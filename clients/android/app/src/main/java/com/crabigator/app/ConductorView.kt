package com.crabigator.app

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject
import java.text.DateFormat
import java.util.Date
import java.util.UUID

@Composable internal fun ConductorView(
    model: SessionModel,
    modifier: Modifier = Modifier,
    newSession: (Rect) -> Unit = {},
    request: suspend (String, JSONObject?) -> JSONObject = { path, body -> model.api.call(path, body) },
) {
    var newSessionAnchor by remember { mutableStateOf(Rect.Zero) }
    var snapshot by remember { mutableStateOf(JSONObject()) }
    var error by remember { mutableStateOf<String?>(null) }
    var sending by remember { mutableStateOf(false) }
    var draft by rememberSaveable { mutableStateOf("") }
    var messageId by rememberSaveable { mutableStateOf(UUID.randomUUID().toString()) }
    var sentText by rememberSaveable { mutableStateOf("") }
    var pane by rememberSaveable { mutableStateOf("Conversation") }
    var history by rememberSaveable { mutableStateOf(false) }
    var recentOutcome by rememberSaveable { mutableStateOf("all") }
    var query by rememberSaveable { mutableStateOf("") }
    var outcome by rememberSaveable { mutableStateOf("all") }
    var archive by remember { mutableStateOf(JSONObject()) }
    var archiveError by remember { mutableStateOf<String?>(null) }
    var searching by remember { mutableStateOf(false) }
    var searchEpoch by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    suspend fun refresh() {
        try { snapshot = request("/api/conductor", null); error = snapshot.text("error").takeIf { it.isNotBlank() } }
        catch (e: CancellationException) { throw e }
        catch (e: Exception) { error = e.message ?: "Could not connect to Conductor." }
    }
    var archiveCriteria by remember { mutableStateOf<Pair<String, String>?>(null) }
    suspend fun search(more: Boolean = false) {
        if (more && (searching || archiveCriteria != (query to outcome))) return
        val epoch = ++searchEpoch; val currentQuery = query; val currentOutcome = outcome
        searching = true; archiveError = null
        try {
            val page = request("/api/conductor/archive", JSONObject().put("query", currentQuery).put("status", currentOutcome).put("offset", if (more) archive.optInt("next_offset") else 0))
            if (epoch == searchEpoch && query == currentQuery && outcome == currentOutcome) {
                archiveCriteria = currentQuery to currentOutcome
                if (more) {
                    val entries = archive.array("entries"); val ids = entries.map { it.text("id") }.toSet()
                    page.put("entries", JSONArray(entries + page.array("entries").filter { it.text("id") !in ids }))
                }
                archive = page
            }
        } catch (e: CancellationException) { throw e }
        catch (e: Exception) { if (epoch == searchEpoch) archiveError = e.message }
        finally { if (epoch == searchEpoch) searching = false }
    }
    LaunchedEffect(lifecycle) { lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) { while (true) { refresh(); delay(3000) } } }
    LaunchedEffect(pane, query, outcome) { if (pane == "Archive") { delay(250); search() } }
    val tracks = snapshot.array("tracks")
    Column(modifier.imePadding()) {
        Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            ConductorText("Conductor", weight = FontWeight.SemiBold, size = 17)
            Spacer(Modifier.weight(1f))
            Control(R.drawable.ic_add, "New session", modifier = Modifier.onGloballyPositioned { newSessionAnchor = it.boundsInRoot() }) { newSession(newSessionAnchor) }
            if (snapshot.optBoolean("working")) CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
            ConductorText(if (snapshot.optBoolean("host_online")) "Desktop ready" else "Desktop offline", size = 11, color = Muted)
        }
        val tabs = listOf("Conversation", "Sessions", "Archive")
        TabRow(tabs.indexOf(pane), containerColor = Ink, contentColor = Peach) {
            tabs.forEach { tab -> Tab(pane == tab, { pane = tab }, text = { ConductorText(tab, size = 11) }) }
        }
        error?.let { message ->
            Row(Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.errorContainer).padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
                ConductorText(message, Modifier.weight(1f), size = 12)
                TextButton({ scope.launch { refresh() } }) { ConductorText("Retry", size = 12) }
            }
        }
        when (pane) {
            "Conversation" -> {
                val exchanges = mutableListOf<MutableList<JSONObject>>()
                snapshot.array("messages").forEach { message ->
                    if (message.text("role") == "user" || exchanges.isEmpty()) exchanges.add(mutableListOf())
                    exchanges.last().add(message)
                }
                LazyColumn(Modifier.weight(1f).fillMaxWidth(), contentPadding = PaddingValues(20.dp), verticalArrangement = Arrangement.spacedBy(22.dp)) {
                    if (exchanges.isEmpty()) item { ConductorText("One conversation. Your existing sessions.\n\nAsk here. Conductor finds the right sessions and brings their answers back.", color = Muted) }
                    items(if (history) exchanges else exchanges.takeLast(1), key = { it.first().text("id") }) { exchange ->
                        Column(Modifier.widthIn(max = 760.dp), verticalArrangement = Arrangement.spacedBy(20.dp)) {
                            exchange.firstOrNull { it.text("role") == "user" }?.let { question ->
                                Column(Modifier.padding(start = 12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                                    ConductorText("You · " + conductorDate(question.optLong("at")), size = 11, color = Muted)
                                    ConductorRichText(question.array("blocks"), question.text("text"))
                                }
                            }
                            val replies = exchange.filter { it.text("role") != "user" }
                            replies.lastOrNull()?.let { reply ->
                                ConductorText("Latest reply · " + conductorDate(reply.optLong("at")), size = 11, color = Peach)
                                ConductorRichText(reply.array("blocks"), reply.text("text"))
                                ConductorHighlights(reply, model)
                            }
                            if (replies.lastOrNull()?.array("highlights").isNullOrEmpty()) {
                                tracks.filter { it.text("request_id") == exchange.first().text("id") }.forEach { track ->
                                    Surface(color = Panel, shape = RoundedCornerShape(6.dp)) {
                                        Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                                            ConductorIdentity(track)
                                            ConductorText(conductorStatus(track) + " · " + conductorDate(track.optLong("updated_at")), color = Peach, size = 11)
                                            TextButton({ pane = "Sessions" }) { ConductorText("View requests", size = 12) }
                                        }
                                    }
                                }
                            }
                            if (replies.size > 1) ConductorDisclosure("Earlier updates · ${replies.size - 1}") {
                                replies.dropLast(1).forEach { reply ->
                                    ConductorText(conductorDate(reply.optLong("at")), size = 10, color = Muted)
                                    ConductorRichText(reply.array("blocks"), reply.text("text"))
                                ConductorHighlights(reply, model)
                                }
                            }
                        }
                    }
                    if (exchanges.size > 1) item { TextButton({ history = !history }) { ConductorText(if (history) "Show latest exchange" else "Earlier conversation", size = 12) } }
                    if (!snapshot.optBoolean("host_online")) item { ConductorText("Start on your computer: crabigator conductor", color = Muted, size = 12) }
                }
                Row(Modifier.fillMaxWidth().padding(12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Bottom) {
                    OutlinedTextField(draft, { draft = it.take(16000) }, placeholder = { ConductorText("Ask your sessions…", color = Muted) }, modifier = Modifier.weight(1f).semantics { contentDescription = "Conductor message" }, maxLines = 5, shape = RoundedCornerShape(6.dp), textStyle = LocalTextStyle.current.copy(fontFamily = FontFamily.Monospace))
                    Button(enabled = !sending && draft.isNotBlank(), onClick = {
                        val text = draft.trim()
                        if (sentText != text) { messageId = UUID.randomUUID().toString(); sentText = text }
                        val id = messageId; sending = true
                        scope.launch {
                            try {
                                request("/api/conductor/message", JSONObject().put("id", id).put("text", text))
                                if (draft.trim() == text) draft = ""
                                sentText = ""; history = false; refresh()
                            } catch (e: CancellationException) { throw e }
                            catch (e: Exception) { error = e.message }
                            finally { sending = false }
                        }
                    }) { ConductorText("Send", size = 12) }
                }
            }
            "Sessions" -> {
                ConductorText("One row per actual session. Requests appear underneath.", Modifier.padding(16.dp), size = 12, color = Muted)
                ConductorFilters(recentOutcome, { recentOutcome = it }, archive = false)
                LazyColumn(Modifier.weight(1f).fillMaxWidth(), contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                    if (tracks.isEmpty()) item { ConductorText("No requests sent yet. Start in Conversation.", color = Muted) }
                    tracks.groupBy { it.text("project").ifBlank { "Unknown project" } }.toSortedMap().forEach { (project, projectTracks) ->
                        item(key = "project-$project") { ConductorText(project, weight = FontWeight.SemiBold) }
                        items(projectTracks.groupBy { it.text("session_id") }.values.toList(), key = { it.first().text("session_id") }) { sessionTracks ->
                            val first = sessionTracks.first()
                            Surface(color = Panel, shape = RoundedCornerShape(6.dp)) {
                                Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                    ConductorIdentity(first)
                                    ConductorContext(first)
                                    ConductorDisclosure("Requests") {
                                        val visible = sessionTracks.filter { recentOutcome == "all" || (if (recentOutcome == "active") !conductorEnded(it) else it.text("state") == recentOutcome) }
                                        visible.forEach { entry -> key(entry.text("id")) { ConductorEntry(entry, model, live = true) } }
                                        if (visible.isEmpty()) ConductorText("No matching requests.", color = Muted, size = 12)
                                    }
                                    ConductorText("${sessionTracks.count { !conductorEnded(it) }} active · ${sessionTracks.count { it.text("state") == "answered" }} answered · ${sessionTracks.count { it.text("state") == "failed" }} failed", size = 10, color = Muted)
                                    ConductorText(if (first.optBoolean("connected")) "● Online" else "○ Offline", size = 10, color = Muted)
                                }
                            }
                        }
                    }
                    item { ConductorText("Older requests and answers are in Archive.", size = 12, color = Muted) }
                }
            }
            else -> {
                OutlinedTextField(query, { query = it.take(200) }, placeholder = { ConductorText("Find a question or answer…", size = 12) }, singleLine = true, modifier = Modifier.fillMaxWidth().padding(12.dp).semantics { contentDescription = "Search Conductor archive" }, textStyle = LocalTextStyle.current.copy(fontFamily = FontFamily.Monospace))
                ConductorFilters(outcome, { outcome = it }, archive = true)
                if (searching) LinearProgressIndicator(Modifier.fillMaxWidth())
                archiveError?.let { ConductorText(it, Modifier.padding(12.dp), color = MaterialTheme.colorScheme.error, size = 12) }
                ConductorText("${archive.optInt("total")} saved matches · by project and session", Modifier.padding(12.dp), color = Muted, size = 11)
                val entries = archive.array("entries")
                LazyColumn(Modifier.weight(1f).fillMaxWidth(), contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                    entries.groupBy { it.text("project").ifBlank { "Unknown project" } }.toSortedMap(compareBy<String> { it == "Conductor" }.thenBy { it }).forEach { (project, group) ->
                        item(key = "project-$project") { ConductorText(project, weight = FontWeight.SemiBold) }
                        group.groupBy { it.text("session_id") }.forEach { (id, sessionEntries) ->
                            item(key = "session-$project-$id") { if (id.isBlank()) ConductorText("Coordinator conversation", color = Muted, size = 11) else ConductorIdentity(sessionEntries.first()) }
                            items(sessionEntries, key = { it.text("kind") + it.text("id") }) { entry -> ConductorEntry(entry, model, live = false) }
                        }
                    }
                    if (entries.isEmpty() && !searching) item { ConductorText("No matches. Try another word or outcome.", color = Muted) }
                    if (archive.has("next_offset") && !archive.isNull("next_offset")) item { TextButton(enabled = !searching, onClick = { scope.launch { search(true) } }) { ConductorText("Load more", size = 12) } }
                }
            }
        }
        Box(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp), contentAlignment = Alignment.Center) {
            BoardModeFooter(model.navigation.value.listMode, { list -> model.navigate { it.copy(conductor = false, listMode = list) } }, null, conductor = true)
        }
    }
}

@Composable private fun ConductorIdentity(entry: JSONObject) {
    val session = Session.parse(entry)
    val fg = runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(Peach)
    val bg = runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel)
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Surface(color = bg, shape = RoundedCornerShape(5.dp)) {
            Box(Modifier.width(42.dp).padding(vertical = 5.dp), contentAlignment = Alignment.Center) { ConductorText(session.glyph, color = fg, size = 13) }
        }
        Column(verticalArrangement = Arrangement.spacedBy(5.dp)) {
            ConductorText(conductorSessionName(entry), size = 14, weight = FontWeight.SemiBold)
            ConductorText(listOf(entry.text("project"), entry.text("platform")).filter { it.isNotBlank() }.joinToString(" · "), color = Muted, size = 10)
        }
    }
}
@Composable private fun ConductorContext(entry: JSONObject) {
    if (entry.text("recap").isNotBlank()) ConductorText(entry.text("recap"), color = Muted, size = 11)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        if (entry.optInt("additions") > 0 || entry.optInt("deletions") > 0) {
            ConductorText("+${entry.optInt("additions")}", color = Color(0xFFA3D8BF), size = 10)
            ConductorText("−${entry.optInt("deletions")}", color = Color(0xFFF1AFAB), size = 10)
        }
        ConductorText(entry.text("session_state"), color = Muted, size = 10)
        if (entry.optInt("prompts") > 0) ConductorText("${entry.optInt("prompts")} prompts", color = Muted, size = 10)
    }
}
@Composable private fun ConductorHighlights(message: JSONObject, model: SessionModel) {
    message.array("highlights").forEach { entry ->
        val color = when(entry.text("state")) { "answered" -> Color(0xFFA3D8BF); "failed" -> Color(0xFFF1AFAB); else -> Peach }
        Surface(color = Panel, shape = RoundedCornerShape(6.dp)) {
            Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(0.dp)) {
                Box(Modifier.fillMaxWidth().height(2.dp).background(color))
                Column(Modifier.padding(18.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                    Box(Modifier.clickable { model.open(entry.text("session_id")) }) { ConductorIdentity(entry) }
                    ConductorText(conductorStatus(entry) + " · " + conductorDate(entry.optLong("updated_at")), size = 10, color = color)
                    ConductorRichText(entry.array("blocks"), entry.text("text"))
                }
            }
        }
    }
}

@Composable private fun ConductorFilters(value: String, change: (String) -> Unit, archive: Boolean) {
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        val options = listOf("all" to if (archive) "All outcomes" else "All recent", "active" to "Active", "answered" to "Answered", "failed" to "Failed") + if (archive) listOf("stopped" to "Stopped") else emptyList()
        options.forEach { (key, label) -> FilterChip(value == key, { change(key) }, label = { ConductorText(label, size = 11) }) }
    }
}
@Composable private fun ConductorText(text: String, modifier: Modifier = Modifier, color: Color = MaterialTheme.colorScheme.onSurface, size: Int = 14, weight: FontWeight = FontWeight.Normal) {
    Text(text, modifier, color = color, fontFamily = FontFamily.Monospace, fontSize = size.sp, lineHeight = (size * 1.55).sp, fontWeight = weight)
}
@Composable private fun ConductorDisclosure(title: String, content: @Composable ColumnScope.() -> Unit) {
    var open by rememberSaveable { mutableStateOf(false) }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        TextButton({ open = !open }, contentPadding = PaddingValues(0.dp)) { ConductorText((if (open) "▾ " else "▸ ") + title, color = Peach, size = 12) }
        if (open) content()
    }
}
@Composable private fun ConductorEntry(entry: JSONObject, model: SessionModel, live: Boolean) {
    val clipboard = LocalClipboardManager.current
    val color = when(entry.text("state")) { "answered" -> Color(0xFFA3D8BF); "failed" -> Color(0xFFF1AFAB); else -> Peach }
    Surface(color = Panel, shape = RoundedCornerShape(6.dp)) {
        Column(Modifier.fillMaxWidth().padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            ConductorText(conductorStatus(entry) + " · " + conductorDate(entry.optLong("updated_at", entry.optLong("at"))), color = color, size = 10)
            ConductorDisclosure(entry.text("title").ifBlank { entry.text("text").take(100) }) {
                val question = entry.text("question").ifBlank { entry.text("instruction") }
                if (question.isNotBlank()) ConductorDisclosure("Original question") { ConductorRichText(entry.array("question_blocks"), question) }
                val answer = entry.text("result").ifBlank { entry.text("text") }
                ConductorRichText(entry.array("blocks"), answer.ifBlank { entry.array("events").lastOrNull()?.text("text") ?: "Waiting for a report." })
                ConductorHighlights(entry, model)
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    if (answer.isNotBlank()) TextButton({ clipboard.setText(AnnotatedString((listOf(answer) + entry.array("highlights").map { conductorSessionName(it) + "\n" + it.text("text") }).joinToString("\n\n"))) }) { ConductorText("Copy answer", size = 11) }
                    if (entry.text("session_id").isNotBlank()) TextButton({ model.open(entry.text("session_id")) }) { ConductorText("Open actual session", size = 11) }
                }
                val events = entry.array("events").filter { it.text("text") != entry.text("result") }
                if (events.isNotEmpty()) ConductorDisclosure("Progress · ${events.size} updates") {
                    events.forEach { event -> ConductorText(conductorDate(event.optLong("at")), size = 10, color = Muted); SelectionContainer { ConductorText(event.text("text"), size = 12) } }
                }
                if (live) ConductorDisclosure("Live session activity") {
                    ConductorText("Includes unrelated work in this session.", size = 11, color = Muted)
                    SelectionContainer { Text(TerminalText.parse(entry.text("activity").ifBlank { entry.text("screen") }), fontFamily = FontFamily.Monospace, fontSize = 12.sp) }
                }
            }
            if (!live) ConductorText(entry.text("excerpt"), size = 11, color = Muted)
        }
    }
}
private fun conductorEnded(entry: JSONObject) = entry.text("state") in listOf("answered", "failed", "cancelled")
private fun conductorSessionName(entry: JSONObject) = entry.text("session_title").ifBlank { entry.text("project").ifBlank { "Session" } }
private fun conductorDate(value: Long) = if (value > 0) DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(value)) else ""
private fun conductorStatus(entry: JSONObject): String {
    if (entry.text("kind") == "message") return if (entry.text("role") == "user") "Question" else "Coordinator reply"
    return mapOf("sending" to "Sending", "forwarded" to "Awaiting delivery", "delivered" to "Queued", "working" to "Working", "waiting" to "Needs attention", "delivery_unknown" to "Check delivery", "answered" to "Answered", "failed" to "Failed", "cancelled" to "Stopped")[entry.text("state")] ?: "Pending"
}
private fun conductorInline(text: String): AnnotatedString = buildAnnotatedString {
    val pattern = Regex("(`[^`]+`|\\*\\*[^*]+\\*\\*|\\[[^\\]]+\\]\\(https?://[^)]+\\))")
    var cursor = 0
    pattern.findAll(text).forEach { match ->
        append(text.substring(cursor, match.range.first)); val start = length; val token = match.value
        when {
            token.startsWith("**") -> { append(token.drop(2).dropLast(2)); addStyle(SpanStyle(fontWeight = FontWeight.Bold, color = Color(0xFFF5C99E)), start, length) }
            token.startsWith("`") -> { append(token.drop(1).dropLast(1)); addStyle(SpanStyle(color = Color(0xFFB4D8DD)), start, length) }
            else -> { val split = token.indexOf("]("); append(token.substring(1, split)); addLink(LinkAnnotation.Url(token.substring(split + 2, token.length - 1), TextLinkStyles(SpanStyle(color = Color(0xFF9AC9DF)))), start, length) }
        }
        cursor = match.range.last + 1
    }
    append(text.substring(cursor))
}
@Composable private fun ConductorRichText(blocks: List<JSONObject>, fallback: String) {
    val content = blocks.ifEmpty { listOf(JSONObject().put("type", "paragraph").put("text", fallback)) }
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        content.forEach { block ->
            if (block.text("type") == "table") {
                val headers = block.optJSONArray("headers") ?: JSONArray(); val rows = block.optJSONArray("rows") ?: JSONArray()
                Column(Modifier.horizontalScroll(rememberScrollState()).background(Ink).padding(8.dp)) {
                    for (rowIndex in -1 until rows.length()) {
                        val row = if (rowIndex == -1) headers else rows.optJSONArray(rowIndex) ?: JSONArray()
                        Row {
                            for (col in 0 until row.length()) SelectionContainer {
                                Text(conductorInline(row.optString(col)), Modifier.width(170.dp).padding(10.dp), fontFamily = FontFamily.Monospace, fontSize = 12.sp, lineHeight = 18.sp, fontWeight = if (rowIndex == -1) FontWeight.Bold else FontWeight.Normal)
                            }
                        }
                        if (rowIndex < rows.length() - 1) HorizontalDivider()
                    }
                }
            } else SelectionContainer {
                Text(if (block.text("type") == "code") AnnotatedString(block.text("text")) else conductorInline(block.text("text")), fontFamily = FontFamily.Monospace, fontSize = 14.sp, lineHeight = 23.sp, fontWeight = if (block.text("type") == "heading") FontWeight.SemiBold else FontWeight.Normal)
            }
        }
    }
}
