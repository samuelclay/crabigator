package com.crabigator.app

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.nestedscroll.*
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.delay
import org.json.JSONObject

@Composable internal fun SessionDetail(s: AppState, model: SessionModel, wide: Boolean, style: (Rect) -> Unit) {
    val session = s.selected ?: return
    val p by model.preferences.collectAsStateWithLifecycle()
    var styleAnchor by remember { mutableStateOf(Rect.Zero) }
    var replying by rememberSaveable(session.id) { mutableStateOf(false) }
    var draft by rememberSaveable(session.id) { mutableStateOf("") }
    var pinned by rememberSaveable(session.id) { mutableStateOf(true) }
    var waiting by remember(session.id, p.transcript) { mutableStateOf(true) }
    LaunchedEffect(session.id, p.transcript) { delay(8000); waiting = false }
    val sections = remember(s.details, session, p, s.prs) { sessionWidgetSections(s, p) }
    val focus = remember { FocusRequester() }
    val content = if (p.transcript) s.history else s.screen
    val loaded = if (p.transcript) s.historyLoaded else s.screenLoaded
    val formatted = remember(content, p.wrap) { TerminalText.parse(content).let { if (p.wrap) TerminalText.trimLineEnds(it) else it } }
    val vScroll = key(session.id, p.transcript) { rememberScrollState() }
    val hScroll = rememberScrollState()
    val userScroll = remember(vScroll) { object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
            if (source == NestedScrollSource.UserInput && available.y > 0) pinned = false
            return Offset.Zero
        }
        override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
            if (source == NestedScrollSource.UserInput && consumed.y < 0 && !vScroll.canScrollForward) pinned = true
            return Offset.Zero
        }
    } }
    LaunchedEffect(pinned, vScroll.maxValue, content) { if (pinned) vScroll.scrollTo(vScroll.maxValue) }
    Column(Modifier.fillMaxSize().imePadding()) {
        Row(Modifier.fillMaxWidth().padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Control(if (wide) R.drawable.ic_close else R.drawable.ic_back, if (wide) "Close session" else "Back to list", onClick = model::close)
            Column(Modifier.weight(1f)) {
                Text(session.title, maxLines = 2, overflow = TextOverflow.Ellipsis, fontSize = 15.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium)
                Text(if (s.connected) "${session.machine} · Live" else if (!session.active) "${session.machine} · Ended" else "Reconnecting…", fontSize = 11.sp, lineHeight = 15.sp, color = if (s.connected) Mint else Peach)
            }
            Control(R.drawable.ic_pin, if (pinned) "Unpin scroll" else "Pin scroll to bottom", selected = pinned) { pinned = !pinned }
            Control(if (replying) R.drawable.ic_check else R.drawable.ic_reply, if (replying) "Done" else "Reply", selected = replying) { replying = !replying }
            Control(R.drawable.ic_style, "Style", modifier = Modifier.onGloballyPositioned { styleAnchor = it.boundsInRoot() }) { style(styleAnchor) }
        }
        HorizontalDivider(color = Color(0xFF303A46))
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
            val availableHeight = maxHeight
            val terminalLimit = if (p.widgets && sections.isNotEmpty()) maxHeight * .65f else maxHeight
            val terminalHeight = p.terminalHeight.dp.coerceAtMost(terminalLimit)
            Column(Modifier.fillMaxSize()) {
                Box(Modifier.then(if (p.terminalHeight == 0) Modifier.weight(1f) else Modifier.height(terminalHeight)).fillMaxWidth().background(Color(0xFF0B0F14))) {
                    if (content.isEmpty()) {
                        if (!loaded && waiting && session.active) CircularProgressIndicator(Modifier.align(Alignment.Center).size(24.dp), strokeWidth = 2.dp)
                        else Text(if (p.transcript) "No transcript available" else "No terminal output", color = Muted, fontSize = 13.sp, modifier = Modifier.align(Alignment.Center))
                    } else SelectionContainer {
                        Text(formatted, modifier = Modifier.fillMaxWidth().nestedScroll(userScroll).verticalScroll(vScroll)
                            .then(if (p.wrap) Modifier else Modifier.horizontalScroll(hScroll)).padding(16.dp),
                            fontFamily = FontFamily.Monospace, fontSize = p.fontSize.sp,
                            lineHeight = (p.fontSize * p.lineSpacing / 100f).sp, softWrap = p.wrap)
                    }
                }
                if (p.widgets) SessionWidgets(sections, Modifier.then(if (p.terminalHeight == 0) Modifier.heightIn(max = availableHeight * .35f) else Modifier.weight(1f)))
            }
        }
        if (s.prompt != null) PromptPanel(s.prompt, s.sending || !s.connected || s.revision == null) { action -> model.send(action.route, action.body, guarded = true, expectedRevision = s.revision) }
        if (replying) {
            LaunchedEffect(session.id, replying) { focus.requestFocus() }
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp)) { listOf("Esc" to "escape", "Tab" to "tab", "↑" to "up", "↓" to "down", "←" to "left", "→" to "right", "Enter" to "enter", "Ctrl+C" to "ctrl_c").forEach { (label, key) -> TextButton(onClick = { model.send("key", JSONObject().put("key", key)) }, enabled = !s.sending && s.connected) { Text(label) } } }
            Row(Modifier.fillMaxWidth().padding(12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(draft, { draft = it }, placeholder = { Text("Reply") }, modifier = Modifier.weight(1f).focusRequester(focus), shape = RoundedCornerShape(16.dp), maxLines = 5)
                Control(R.drawable.ic_send, "Send", selected = true, enabled = draft.isNotBlank() && s.connected && !s.sending) { model.send("answer", JSONObject().put("text", draft)) { draft = "" } }
            }
        }
    }
}
private fun sessionWidgetSections(s: AppState, p: UiPreferences): List<Pair<String, List<Pair<String, String?>>>> {
    val session = s.selected ?: return emptyList()
    return buildList {
        if (p.visible("recap")) {
            val recap = s.details["recap"]?.optJSONObject("latest")?.text("headline") ?: session.recap
            if (recap.isNotBlank()) add("Recap" to listOf(recap to null))
        }
        if (p.visible("prs")) {
            val prs = s.details["prs"]?.array("prs")?.map { "#${it.optInt("number")} ${it.text("title")}" to it.text("url").takeIf { url -> url.startsWith("https://") } }
                ?: s.prs.filter { pr -> pr.sessions.any { it.id == session.id } }.map { "#${it.number} ${it.title}" to it.url }
            if (prs.isNotEmpty()) add("Pull requests" to prs)
        }
        val git = s.details["git"]
        if (p.visible("git")) {
            val rows = listOfNotNull((git?.text("branch") ?: session.branch).takeIf { it.isNotBlank() }?.let { "⎇ $it" to null }) +
                git?.array("files").orEmpty().map { "${it.text("status")} ${it.text("path")}  +${it.optInt("additions")} −${it.optInt("deletions")}" to null }
            if (rows.isNotEmpty()) add("Git status" to rows)
        }
        if (p.visible("commits")) {
            val rows = (s.details["commit_history"]?.array("history") ?: git?.array("recent_commits").orEmpty()).map { "${it.text("short_hash")} ${it.text("subject")}" to null }
            if (rows.isNotEmpty()) add("Commits" to rows)
        }
        if (p.visible("changes")) {
            val rows = s.details["changes"]?.array("by_language").orEmpty().flatMap { it.array("changes") }.map { "${it.text("name")}  +${it.optInt("additions")} −${it.optInt("deletions")}" to null }
            if (rows.isNotEmpty()) add("Changes" to rows)
        }
    }
}
@Composable private fun SessionWidgets(sections: List<Pair<String, List<Pair<String, String?>>>>, modifier: Modifier) {
    if (sections.isEmpty()) return
    val uri = LocalUriHandler.current
    Column(modifier.fillMaxWidth().background(Panel).verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        sections.forEach { (title, rows) ->
            Text(title, color = Mint, fontSize = 12.sp)
            rows.forEach { (text, url) -> Text(text, color = if (url != null) Peach else MaterialTheme.colorScheme.onSurface, fontSize = 12.sp, lineHeight = 17.sp,
                modifier = Modifier.fillMaxWidth().then(if (url != null && url.startsWith("https://")) Modifier.clickable { uri.openUri(url) }.padding(vertical = 6.dp) else Modifier)) }
        }
    }
}
