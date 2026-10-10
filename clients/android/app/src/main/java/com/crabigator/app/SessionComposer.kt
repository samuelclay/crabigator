package com.crabigator.app

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.*
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import kotlinx.coroutines.*
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.sqrt

@Composable internal fun SessionComposer(s: AppState, model: SessionModel, active: Boolean = true) {
    val sessionId = s.selected?.id ?: return
    var draft by rememberSaveable(sessionId) { mutableStateOf("") }
    var shortcuts by remember { mutableStateOf(false) }
    var recording by remember { mutableStateOf(false) }
    var startAfterPermission by remember { mutableStateOf(false) }
    var processing by remember { mutableStateOf(false) }
    var focusAfterProcessing by remember { mutableStateOf(false) }
    var elapsed by remember { mutableIntStateOf(0) }
    var levels by remember { mutableStateOf(List(32) { 0f }) }
    var error by remember { mutableStateOf<String?>(null) }
    var processingJob by remember { mutableStateOf<Job?>(null) }
    val context = LocalContext.current
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    val recorder = remember(sessionId) { VoiceRecorder(context.applicationContext) }
    val scope = rememberCoroutineScope()
    val focus = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current
    val singleLineHeight = with(LocalDensity.current) { MaterialTheme.typography.bodyLarge.lineHeight.toDp() + 24.dp }.coerceAtLeast(48.dp)
    val buttonAlignment = Modifier.height(singleLineHeight).wrapContentHeight(Alignment.CenterVertically)
    val enabled = active && s.connected && !s.sending && !processing
    val currentDraft by rememberUpdatedState(draft)
    fun send(text: String) {
        if (!active || model.state.value.selected?.id != sessionId) return
        model.send("answer", JSONObject().put("text", text)) { if (draft == text) draft = "" }
    }
    fun cancel() {
        processingJob?.cancel(); processingJob = null
        recorder.cancel(); recording = false; processing = false; focusAfterProcessing = false; startAfterPermission = false
    }
    fun start() {
        if (!active || model.state.value.selected?.id != sessionId) return
        error = null
        try {
            recorder.start(); recording = true; elapsed = 0; levels = List(32) { 0f }
            shortcuts = false; keyboard?.hide()
        } catch (e: Exception) { error = e.message ?: "Could not start the microphone." }
    }
    fun finish(sendImmediately: Boolean) {
        if (!recording) return
        recording = false; processing = true
        processingJob = scope.launch {
            var file: java.io.File? = null
            try {
                file = recorder.finish()
                val text = model.api.transcribe(file)
                ensureActive()
                if (text.isBlank()) { error = "No speech detected."; return@launch }
                draft = listOf(currentDraft.trimEnd(), text).filter { it.isNotBlank() }.joinToString("\n")
                if (sendImmediately && model.state.value.selected?.id == sessionId) send(draft)
                else focusAfterProcessing = true
            } catch (e: CancellationException) { throw e }
            catch (e: Exception) { error = e.message ?: "Could not transcribe the recording." }
            finally { file?.delete(); if (isActive) processing = false }
        }
    }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) {
            if (lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)) start()
            else startAfterPermission = true
        } else error = "Microphone access is off."
    }
    DisposableEffect(recorder, lifecycle) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_STOP) cancel()
            if (event == Lifecycle.Event.ON_RESUME && startAfterPermission) { startAfterPermission = false; start() }
        }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer); cancel() }
    }
    LaunchedEffect(active) { if (!active) { shortcuts = false; cancel() } }
    LaunchedEffect(recording) {
        if (recording) {
            val started = android.os.SystemClock.elapsedRealtime()
            while (recording) {
                delay(50)
                levels = levels.drop(1) + sqrt(recorder.amplitude()).coerceIn(0f, 1f)
                elapsed = ((android.os.SystemClock.elapsedRealtime() - started) / 1000).toInt()
                if (elapsed >= 120) finish(false)
            }
        }
    }
    LaunchedEffect(processing, recording, focusAfterProcessing) {
        if (focusAfterProcessing && !processing && !recording) {
            focusAfterProcessing = false
            focus.requestFocus()
            keyboard?.show()
        }
    }
    Column(Modifier.fillMaxWidth().animateContentSize()) {
        HorizontalDivider(color = Muted.copy(alpha = .2f))
        error?.let { message -> Row(Modifier.padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(message, Modifier.weight(1f), color = MaterialTheme.colorScheme.error, fontSize = 12.sp)
            Control(R.drawable.ic_close, "Dismiss recording error") { error = null }
        } }
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 8.dp), verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            if (recording || processing) {
                Control(R.drawable.ic_close, "Cancel recording", modifier = buttonAlignment) { cancel() }
                Column(Modifier.weight(1f).height(56.dp), verticalArrangement = Arrangement.Center) {
                    if (processing) {
                        LinearProgressIndicator(Modifier.fillMaxWidth())
                        Text("Transcribing…", color = Muted, fontSize = 12.sp)
                    } else {
                        Canvas(Modifier.fillMaxWidth().height(28.dp).semantics { contentDescription = "Microphone waveform" }) {
                            val step = size.width / levels.size
                            levels.forEachIndexed { index, level ->
                                val height = (level * size.height).coerceAtLeast(2.dp.toPx())
                                drawLine(Peach, Offset((index + .5f) * step, (size.height - height) / 2),
                                    Offset((index + .5f) * step, (size.height + height) / 2), step * .5f, StrokeCap.Round)
                            }
                        }
                        Text("${elapsed / 60}:${(elapsed % 60).toString().padStart(2, '0')} / 2:00", fontSize = 11.sp, color = Muted)
                    }
                }
                Control(R.drawable.ic_edit, "Edit recording", enabled = recording, modifier = buttonAlignment) { finish(false) }
                Control(R.drawable.ic_send, "Send recording", selected = true, enabled = recording && s.connected && !s.sending, modifier = buttonAlignment) { finish(true) }
            } else {
                Control(R.drawable.ic_microphone, "Record voice", enabled = enabled, modifier = buttonAlignment) {
                    if (ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED) start()
                    else permission.launch(Manifest.permission.RECORD_AUDIO)
                }
                Box(Modifier.height(singleLineHeight), contentAlignment = Alignment.Center) {
                    Control(R.drawable.ic_keyboard, "Terminal keys", selected = shortcuts) { shortcuts = !shortcuts; keyboard?.hide() }
                    TerminalKeyboard(shortcuts, { shortcuts = false }, enabled, s.selected?.platform == "codex") { bytes ->
                        model.send("key-sequence", JSONObject().put("steps", JSONArray().put(JSONObject().put("type", "text").put("text", bytes))))
                    }
                }
                BasicTextField(draft, { draft = it }, minLines = 1, maxLines = 6,
                    textStyle = MaterialTheme.typography.bodyLarge.copy(color = MaterialTheme.colorScheme.onSurface),
                    cursorBrush = SolidColor(CrabColors.Title),
                    modifier = Modifier.weight(1f).focusRequester(focus),
                    decorationBox = { inner ->
                        Box(Modifier.fillMaxWidth().border(BorderStroke(1.dp, MaterialTheme.colorScheme.outline), RoundedCornerShape(14.dp))
                            .padding(horizontal = 12.dp, vertical = 12.dp)) {
                            if (draft.isEmpty()) Text("Message", style = MaterialTheme.typography.bodyLarge, color = Muted)
                            inner()
                        }
                    })
                AnimatedVisibility(draft.isNotBlank(), enter = fadeIn() + expandHorizontally(), exit = fadeOut() + shrinkHorizontally()) {
                    Control(R.drawable.ic_send, "Send", selected = true, enabled = enabled && draft.isNotBlank(), modifier = buttonAlignment) { send(draft) }
                }
            }
        }
    }
}

