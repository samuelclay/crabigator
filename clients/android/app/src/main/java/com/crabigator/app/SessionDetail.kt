package com.crabigator.app

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.delay

@Composable internal fun SessionDetail(s: AppState, model: SessionModel, wide: Boolean, close: () -> Unit, swipe: Modifier = Modifier, active: Boolean = true, style: (Rect) -> Unit) {
    val session = s.selected ?: return
    val p by model.preferences.collectAsStateWithLifecycle()
    var styleAnchor by remember { mutableStateOf(Rect.Zero) }
    var pinned by rememberSaveable(session.id) { mutableStateOf(true) }
    var loadingSlowly by remember(session.id) { mutableStateOf(false) }
    LaunchedEffect(session.id) { delay(8000); loadingSlowly = true }
    val modules = remember(s.details, session) { SessionModuleData(session, s.details) }
    val prs = remember(s.details["prs"], s.prs, session.id) { sessionPullRequests(s) }
    val hasOutput = s.history.isNotEmpty() || s.screen.isNotEmpty()
    val loaded = s.historyLoaded && s.screenLoaded
    Column(Modifier.fillMaxSize().imePadding()) {
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            Row(Modifier.fillMaxWidth().padding(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Control(if (wide) R.drawable.ic_close else R.drawable.ic_back, if (wide) "Close session" else "Back to list", onClick = close)
                Column(Modifier.weight(1f)) {
                    Text(session.title, color = CrabColors.Title, maxLines = 2, overflow = TextOverflow.Ellipsis, fontSize = 15.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium)
                    Text(if (s.connected) "${session.machine} · Live" else if (!session.active) "${session.machine} · Ended" else "Reconnecting…", fontSize = 11.sp, lineHeight = 15.sp, color = if (s.connected) Mint else Muted)
                }
                Control(R.drawable.ic_modules, if (p.widgets) "Hide session modules" else "Show session modules", selected = p.widgets) { model.style { it.copy(widgets = !it.widgets) } }
                Control(R.drawable.ic_follow_output, if (pinned) "Stop following output" else "Follow latest output", selected = pinned) { pinned = !pinned }
                Control(R.drawable.ic_style, "Style", modifier = Modifier.onGloballyPositioned { styleAnchor = it.boundsInRoot() }) { style(styleAnchor) }
            }
        }
        HorizontalDivider(color = Color(0xFF303A46))
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth()) {
            val availableHeight = maxHeight
            val terminalLimit = if (p.widgets) maxHeight * .55f else maxHeight
            val terminalHeight = p.terminalHeight.dp.coerceAtMost(terminalLimit)
            Column(Modifier.fillMaxSize()) {
                Box(Modifier.then(if (p.terminalHeight == 0) Modifier.weight(1f) else Modifier.height(terminalHeight)).fillMaxWidth().then(swipe).background(Color(0xFF0B0F14))) {
                    if (!hasOutput) {
                        TerminalPlaceholder(session.active, loaded, loadingSlowly, model::reconnectSession, Modifier.align(Alignment.Center))
                    } else key(session.id) { TerminalViewport(s.history, s.screen, p, pinned) { pinned = it } }
                }
                if (p.widgets) SessionModules(session, modules, p, prs, Modifier.heightIn(max = availableHeight * .45f))
            }
        }
        if (s.prompt != null) PromptPanel(s.prompt, s.sending || !s.connected || s.revision == null) { action -> model.send(action.route, action.body, guarded = true, expectedRevision = s.revision) }
        key(session.id) { SessionComposer(s, model, active) }
    }
}
@Composable internal fun TerminalPlaceholder(active: Boolean, loaded: Boolean, loadingSlowly: Boolean, reconnect: () -> Unit, modifier: Modifier = Modifier) {
    Column(modifier, horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        when {
            loaded -> Text("No terminal output", color = Muted, fontSize = 13.sp)
            !active -> Text("Terminal unavailable for this ended session", color = Muted, fontSize = 13.sp)
            else -> {
                CircularProgressIndicator(Modifier.size(24.dp), strokeWidth = 2.dp)
                if (loadingSlowly) {
                    Text("Waiting for terminal output…", color = Muted, fontSize = 13.sp)
                    TextButton(reconnect) { Text("Reconnect") }
                }
            }
        }
    }
}
internal fun sessionPullRequests(s: AppState): List<PullRequest> {
    val session = s.selected ?: return emptyList()
    val live = s.details["prs"]
    return (if (live != null) live.array("prs").filterNot { it.optBoolean("dismissed") }.map(PullRequest::parse)
        else s.prs.filter { pr -> pr.sessions.any { it.id == session.id } })
        .distinctBy { it.key }
}
