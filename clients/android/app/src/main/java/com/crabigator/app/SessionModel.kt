package com.crabigator.app

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update
import okhttp3.*
import org.json.JSONObject

data class AppState(val paired: Boolean = false, val sessions: List<Session> = emptyList(), val prs: List<PullRequest> = emptyList(), val loading: Boolean = false, val error: String? = null, val selected: Session? = null, val screen: String = "", val history: String = "", val prompt: JSONObject? = null, val revision: Long? = null, val connected: Boolean = false, val sending: Boolean = false)
class SessionModel(app: Application) : AndroidViewModel(app) {
    val credentials = Credentials(app)
    val api = Api(credentials)
    val state = MutableStateFlow(AppState(paired = credentials.token.isNotEmpty()))
    private var socket: WebSocket? = null
    private var polling: Job? = null
    private var heartbeat: Job? = null
    private var reconnect: Job? = null
    private var foreground = false
    private var selectionEpoch = 0L
    private var connectionEpoch = 0L
    private var snapshotRequest = 0L
    fun start() {
        foreground = true
        if (!state.value.paired || polling?.isActive == true) return
        polling = viewModelScope.launch { while (isActive) { refresh(); delay(12000) } }
        state.value.selected?.let { connect(it.id) }
        Notifications.register(getApplication(), api)
        Notifications.reconcile(getApplication())
    }
    private fun disconnect() {
        connectionEpoch++; snapshotRequest++
        heartbeat?.cancel(); reconnect?.cancel(); socket?.cancel(); socket = null
        state.update { it.copy(connected = false, revision = null) }
    }
    fun stop() { foreground = false; polling?.cancel(); polling = null; disconnect() }
    fun pair(code: String, origin: String) = viewModelScope.launch {
        state.update { it.copy(loading = true, error = null) }
        try {
            api.pair(code, origin)
            state.update { it.copy(paired = true, loading = false) }
            start()
        } catch (e: Exception) { failure(e) }
    }
    suspend fun refresh() {
        try {
            val data = api.call("/api/prs/board")
            val prs = data.array("prs").map(PullRequest::parse)
            val sessions = (data.array("sessions").map(Session::parse) + prs.flatMap { it.sessions }).distinctBy { it.id }
            state.update { it.copy(prs = prs, sessions = sessions, loading = false, error = null) }
        } catch (e: Exception) { if (e is CancellationException) throw e; failure(e) }
    }
    fun select(session: Session) {
        selectionEpoch++
        state.update { it.copy(selected = session, screen = "", history = "", prompt = null, revision = null, connected = false, sending = false, error = null) }
        connect(session.id)
    }
    fun open(id: String) = viewModelScope.launch {
        if (!state.value.paired) return@launch
        refresh()
        state.value.sessions.find { it.id == id }?.let(::select)
    }
    fun close() {
        selectionEpoch++; disconnect()
        state.update { it.copy(selected = null, prompt = null, screen = "", history = "", sending = false) }
    }
    private fun connect(id: String) {
        disconnect()
        val epoch = connectionEpoch
        heartbeat = viewModelScope.launch {
            while (isActive && foreground && state.value.selected?.id == id) {
                try { api.call("/api/sessions/$id/viewer-active", JSONObject()) }
                catch (e: Exception) { if (e is CancellationException) throw e }
                delay(5000)
            }
        }
        socket = api.client.newWebSocket(api.request("/api/sessions/$id/events").build(), object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) { viewModelScope.launch {
                if (socket === webSocket && connectionEpoch == epoch) state.update { it.copy(error = null) }
            } }
            override fun onMessage(webSocket: WebSocket, text: String) { viewModelScope.launch {
                if (socket !== webSocket || connectionEpoch != epoch || state.value.selected?.id != id) return@launch
                runCatching { JSONObject(text) }.onSuccess { event ->
                    state.update { s -> when (event.text("type")) {
                        "screen" -> s.copy(screen = event.text("content"))
                        "scrollback_history" -> s.copy(history = event.text("content").takeLast(250000))
                        "scrollback" -> s.copy(history = (s.history + event.text("diff")).takeLast(250000))
                        "desktop_status" -> s.copy(connected = event.optBoolean("connected"))
                        "prompt" -> s.copy(prompt = event.optJSONObject("prompt"), revision = null)
                        "state" -> s.copy(selected = s.selected?.copy(state = event.text("state")), revision = null, prompt = if (event.text("state") in listOf("question", "permission")) s.prompt else null)
                        else -> s
                    } }
                    if (event.text("type") in listOf("prompt", "state")) { refreshPrompt(id); Notifications.reconcile(getApplication(), id) }
                }
            } }
            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) { viewModelScope.launch { retry(webSocket, id) } }
            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) { viewModelScope.launch { retry(webSocket, id) } }
        })
        viewModelScope.launch { refreshPrompt(id) }
    }
    private fun retry(ws: WebSocket, id: String) {
        if (socket !== ws || !foreground || state.value.selected?.id != id) return
        state.update { it.copy(connected = false) }
        reconnect?.cancel()
        reconnect = viewModelScope.launch { delay(3000); if (foreground && state.value.selected?.id == id) connect(id) }
    }
    private suspend fun refreshPrompt(id: String) {
        if (state.value.selected?.id != id) return
        val requestId = ++snapshotRequest
        val epoch = connectionEpoch
        try { val snap = api.snapshot(id); if (state.value.selected?.id == id && requestId == snapshotRequest && connectionEpoch == epoch) state.update {
            it.copy(prompt = snap.optJSONObject("prompt"), revision = snap.optLong("prompt_revision"), connected = snap.optBoolean("desktop_connected"))
        } }
        catch (e: Exception) { if (e is CancellationException) throw e /* Older servers still stream their prompt over WebSocket. */ }
    }
    fun send(route: String, body: JSONObject, guarded: Boolean = false, expectedRevision: Long? = null, onSent: () -> Unit = {}) = viewModelScope.launch {
        val s = state.value
        val id = s.selected?.id ?: return@launch
        val epoch = selectionEpoch
        if (s.sending || !s.connected) return@launch
        if (guarded && (expectedRevision == null || expectedRevision != s.revision)) {
            failure(Exception("This question changed. Please review it again."))
            refreshPrompt(id)
            return@launch
        }
        state.update { it.copy(sending = true, error = null) }
        try {
            api.action(id, route, body, if (guarded) expectedRevision else null)
            if (guarded) Notifications.answered(getApplication(), id, api)
            if (selectionEpoch == epoch) { onSent(); refreshPrompt(id) }
        } catch (e: Exception) {
            if (e is CancellationException) throw e
            if (selectionEpoch == epoch) {
                if (e is ApiException && e.status == 409) refreshPrompt(id)
                failure(e)
            }
        } finally {
            if (selectionEpoch == epoch) state.update { it.copy(sending = false) }
        }
    }
    fun dismissError() { state.update { it.copy(error = null) } }
    private fun failure(e: Exception) { if (e is CancellationException) throw e; state.update { it.copy(loading = false, error = e.message ?: "Could not connect. Try again.") } }
    override fun onCleared() { stop() }
}
