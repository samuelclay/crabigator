package com.crabigator.app

import android.Manifest
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.SystemBarStyle
import androidx.compose.ui.res.painterResource
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items as gridItems
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.flow
import org.json.JSONObject

internal val Ink = Color(0xFF101419)
internal val Panel = Color(0xFF1B2129)
internal val Muted = Color(0xFF9CA9B8)
internal val Peach = Color(0xFFFFAD7F)
internal val Mint = Color(0xFF98D9B1)
private val CrabTheme = darkColorScheme(primary = Peach, onPrimary = Ink, background = Ink, surface = Ink, surfaceContainer = Panel, onSurface = Color(0xFFE9EEF4), secondary = Mint, secondaryContainer = Color(0xFF3D3028), onSecondaryContainer = Peach, outline = Color(0xFF37414E))

class MainActivity : ComponentActivity() {
    private val model: SessionModel by viewModels()
    private val permission = registerForActivityResult(ActivityResultContracts.RequestPermission()) { Notifications.register(this, model.api); Notifications.reconcile(this) }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState); enableEdgeToEdge(statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT), navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT))
        setContent { MaterialTheme(colorScheme = CrabTheme) { Surface(Modifier.fillMaxSize()) { App(model) {
            if (android.os.Build.VERSION.SDK_INT >= 33 && androidx.core.content.ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) != android.content.pm.PackageManager.PERMISSION_GRANTED) permission.launch(Manifest.permission.POST_NOTIFICATIONS)
            else startActivity(Intent(android.provider.Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(android.provider.Settings.EXTRA_APP_PACKAGE, packageName))
        } } } }
        handleIntent(intent)
    }
    override fun onStart() { super.onStart(); model.start() }
    override fun onStop() { model.stop(); super.onStop() }
    override fun onNewIntent(intent: Intent) { super.onNewIntent(intent); setIntent(intent); handleIntent(intent) }
    private fun handleIntent(intent: Intent) { intent.getStringExtra("session_id")?.let(model::open) }
}

@Composable private fun App(model: SessionModel, notifications: () -> Unit) {
    val s by model.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    var menu by remember { mutableStateOf<String?>(null) }
    var menuAnchor by remember { mutableStateOf(Rect.Zero) }
    val preferences by model.preferences.collectAsStateWithLifecycle()
    if (!s.paired) { Pairing(s, model); return }
    BackHandler(s.selected != null) { model.close() }
    Box(Modifier.fillMaxSize()) {
    Column(Modifier.fillMaxSize().safeDrawingPadding().then(if (menu != null) Modifier.clearAndSetSemantics {} else Modifier)) {
        if (s.error != null) Surface(color = Color(0xFF4D2C2C)) { Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) { Text(s.error!!, Modifier.weight(1f), fontSize = 13.sp); TextButton(onClick = model::dismissError) { Text("Dismiss") } } }
        BoxWithConstraints(Modifier.weight(1f)) {
            val wide = maxWidth >= 720.dp && s.selected != null
            Row(Modifier.fillMaxSize()) {
                if (!preferences.sidebarRight && (wide || s.selected == null)) Box(if (wide) Modifier.width(360.dp).fillMaxHeight() else Modifier.fillMaxSize()) { Board(s, model, { menuAnchor = it; menu = "Settings" }) }
                if (wide && !preferences.sidebarRight) VerticalDivider(color = Color(0xFF303944))
                if (s.selected != null) Box(Modifier.weight(1f).fillMaxHeight()) { SessionDetail(s, model, wide) { menuAnchor = it; menu = "Style" } }
                if (wide && preferences.sidebarRight) VerticalDivider(color = Color(0xFF303944))
                if (preferences.sidebarRight && (wide || s.selected == null)) Box(if (wide) Modifier.width(360.dp).fillMaxHeight() else Modifier.fillMaxSize()) { Board(s, model, { menuAnchor = it; menu = "Settings" }) }
            }
        }
    }
    MenuPanel(menu, menuAnchor, model, notifications, { menu = it }, { menu = null })
    }

}

@Composable private fun Pairing(s: AppState, model: SessionModel) {
    var code by rememberSaveable { mutableStateOf("") }; var server by rememberSaveable { mutableStateOf(model.credentials.origin) }; var advanced by remember { mutableStateOf(false) }
    Box(Modifier.fillMaxSize().safeDrawingPadding().imePadding().padding(28.dp), contentAlignment = Alignment.Center) {
        Column(Modifier.widthIn(max = 430.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(20.dp)) {
            Image(painterResource(R.drawable.ic_crabigator), contentDescription = "Crabigator", modifier = Modifier.size(64.dp))
            Text("Crabigator", fontSize = 30.sp, fontWeight = FontWeight.SemiBold)
            Surface(shape = RoundedCornerShape(16.dp), color = Panel) { Column(Modifier.padding(20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) { Text("crabigator pair", fontFamily = FontFamily.Monospace, color = Mint, fontSize = 18.sp) } }
            OutlinedTextField(code, { code = it.uppercase().take(11) }, label = { Text("Pairing code") }, placeholder = { Text("ABC-DEF-GHI") }, singleLine = true, modifier = Modifier.fillMaxWidth(), shape = RoundedCornerShape(14.dp))
            if (s.error != null) Text(s.error, color = MaterialTheme.colorScheme.error)
            Button(onClick = { model.pair(code, server) }, enabled = code.isNotBlank() && !s.loading, modifier = Modifier.fillMaxWidth().height(54.dp)) { Text(if (s.loading) "Connecting…" else "Pair") }
            TextButton(onClick = { advanced = !advanced }) { Text("Server", color = Muted) }
            if (advanced) OutlinedTextField(server, { server = it }, label = { Text("HTTPS server address") }, singleLine = true, modifier = Modifier.fillMaxWidth())
        }
    }
}

@Composable private fun Board(s: AppState, model: SessionModel, settings: (Rect) -> Unit) {
    var list by rememberSaveable { mutableStateOf(true) }; var query by rememberSaveable { mutableStateOf("") }; var all by rememberSaveable { mutableStateOf(false) }; var searching by rememberSaveable { mutableStateOf(false) }
    val now by remember { flow { while (true) { emit(System.currentTimeMillis() / 1000); delay(1000) } } }
        .collectAsStateWithLifecycle(initialValue = System.currentTimeMillis() / 1000)
    var settingsAnchor by remember { mutableStateOf(Rect.Zero) }
    val prefs by model.preferences.collectAsStateWithLifecycle()
    val sessions = s.sessions.filter { (all || it.active) && "${it.title} ${it.repo} ${it.machine}".contains(query, true) }.sortedWith(compareByDescending<Session> { it.attention }.thenByDescending { maxOf(it.stats.promptAt, it.stats.completionAt, it.stats.startedAt) }.thenBy { it.id })
    val prs = s.prs.map { it.copy(sessions = it.sessions.filter { session -> all || session.active }) }.filter { (all || it.watched || it.sessions.isNotEmpty()) && "${it.title} ${it.repo} ${it.number}".contains(query, true) }
    Column(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            Image(painterResource(R.drawable.ic_crabigator), "Crabigator", Modifier.padding(8.dp).size(28.dp))
            Spacer(Modifier.weight(1f))
            Control(R.drawable.ic_board, "PR board", selected = !list) { list = false }
            Control(R.drawable.ic_list, "Session list", selected = list) { list = true }
            Control(if (all) R.drawable.ic_history else R.drawable.ic_live, if (all) "All sessions" else "Live sessions", selected = !all) { all = !all }
            Control(R.drawable.ic_search, "Search", selected = searching) { searching = !searching; if (!searching) query = "" }
            Control(R.drawable.ic_settings, "Settings", modifier = Modifier.onGloballyPositioned { settingsAnchor = it.boundsInRoot() }) { settings(settingsAnchor) }
        }
        if (searching) OutlinedTextField(query, { query = it }, placeholder = { Text("Search") }, singleLine = true, shape = RoundedCornerShape(12.dp), modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp))
        if (s.sessions.isEmpty() && s.prs.isEmpty()) Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) { if (s.loading) CircularProgressIndicator(Modifier.size(24.dp)) else Text("No sessions", color = Muted) }
        else if (list) BoxWithConstraints(Modifier.weight(1f)) {
            val count = if (prefs.columns == 0) (maxWidth.value / 320).toInt().coerceAtLeast(1) else prefs.columns.coerceAtMost((maxWidth.value / 280).toInt().coerceAtLeast(1))
            val groups = if (prefs.grouping == "project") sessions.groupBy { it.repo.ifBlank { "Sessions" } }.entries.let { entries ->
                if (prefs.order == "alpha") entries.sortedBy { it.key.lowercase() } else entries.sortedByDescending { entry -> entry.value.maxOfOrNull { maxOf(it.stats.promptAt, it.stats.completionAt, it.stats.startedAt) } ?: 0 }
            }.map { it.key to it.value } else listOf("" to sessions)
            LazyVerticalGrid(columns = GridCells.Fixed(count), contentPadding = PaddingValues(12.dp, 6.dp, 12.dp, 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                groups.forEach { (group, rows) ->
                    if (group.isNotEmpty()) item(key = "group-$group", span = { GridItemSpan(maxLineSpan) }) { Text(group, color = Muted, fontSize = 13.sp, modifier = Modifier.padding(vertical = 10.dp)) }
                    gridItems(rows, key = { it.id }) { SessionCard(it, it.id == s.selected?.id, now, prefs) { model.select(it) } }
                }
            }
        }
        else LazyColumn(contentPadding = PaddingValues(12.dp, 6.dp, 12.dp, 24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            run {
                val attention = sessions.filter { it.attention }
                if (attention.isNotEmpty()) { items(attention, key = { "attention-${it.id}" }) { SessionCard(it, it.id == s.selected?.id) { model.select(it) } }; item { Spacer(Modifier.height(8.dp)) } }
                items(prs, key = { it.key }) { pr -> PrCard(pr, s.selected?.id, model::select) }
                val owned = prs.flatMap { it.sessions }.map { it.id }.toSet()
                val others = sessions.filter { it.id !in owned && !it.attention }
                if (others.isNotEmpty()) { items(others, key = { it.id }) { SessionCard(it, it.id == s.selected?.id) { model.select(it) } } }
            }
        }
    }
}
@Composable internal fun Control(icon: Int, label: String, selected: Boolean = false, enabled: Boolean = true, modifier: Modifier = Modifier, onClick: () -> Unit) {
    IconButton(onClick = onClick, enabled = enabled, modifier = modifier.size(44.dp).background(if (selected) Peach.copy(alpha = .12f) else Color.Transparent, RoundedCornerShape(12.dp))) {
        Icon(painterResource(icon), contentDescription = label, tint = if (selected) Peach else Muted, modifier = Modifier.size(22.dp))
    }
}
@Composable private fun PrCard(pr: PullRequest, selected: String?, select: (Session) -> Unit) {
    val uri = LocalUriHandler.current
    Surface(color = Panel, shape = RoundedCornerShape(12.dp), border = BorderStroke(1.dp, Color(0xFF303A46))) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) { Text(pr.repo, color = Muted, fontSize = 12.sp, modifier = Modifier.weight(1f)); Text("#${pr.number}", color = Peach, fontSize = 12.sp, lineHeight = 16.sp) }
            if (pr.title.isNotBlank()) Text(pr.title, fontSize = 15.sp, lineHeight = 20.sp, fontWeight = FontWeight.Medium)
            if (pr.state.isNotBlank() || pr.checks.isNotBlank()) Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                if (pr.state.isNotBlank()) Text(pr.state.lowercase().replaceFirstChar { it.uppercase() }, color = if (pr.state == "MERGED") Color(0xFFC5ACFF) else Mint, fontSize = 12.sp, lineHeight = 16.sp)
                if (pr.checks.isNotBlank()) Text(pr.checks, color = if (pr.checks.contains("failed")) Color(0xFFFF9999) else Muted, fontSize = 12.sp, lineHeight = 16.sp)
            }
            pr.sessions.forEach { session -> HorizontalDivider(color = Color(0xFF303A46)); SessionRow(session, selected == session.id) { select(session) } }
            if (pr.sessions.isEmpty() && pr.url.startsWith("https://")) Control(R.drawable.ic_external, "Open pull request") { uri.openUri(pr.url) }
        }
    }
}
@Composable private fun SessionCard(session: Session, selected: Boolean, now: Long? = null, prefs: UiPreferences = UiPreferences(), open: () -> Unit) {
    Surface(onClick = open, color = Panel, shape = RoundedCornerShape(12.dp), border = BorderStroke(1.dp, if (selected) Peach else Color(0xFF303A46))) {
        Column(Modifier.padding(if (prefs.density == "compact") 6.dp else 10.dp)) {
            SessionRow(session, selected, clickable = false, open = open)
            if (now != null) SessionStatsFooter(session, now, prefs)
        }
    }
}
@Composable private fun SessionStatsFooter(session: Session, now: Long, prefs: UiPreferences) {
    val stats = session.stats
    FlowRow(Modifier.fillMaxWidth().padding(top = 6.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
        if (prefs.visible("sessionTime")) Stat("◉", sessionDuration(stats.duration(session.active, now)), "Session time", Color(0xFF79B8FF))
        if (prefs.visible("thinkingTime")) Stat("◐", sessionDuration(stats.thinkingSeconds), "Thinking time", Mint)
        fun activity(countKey: String, ageKey: String, count: Long?, at: Long): String =
            if (prefs.visible(countKey)) stats.activity(count, if (prefs.visible(ageKey)) at else 0, now)
            else if (at > 0) sessionAge(at, now) else "—"
        if (prefs.visible("prompts") || prefs.visible("promptRecency")) Stat("⟩", activity("prompts", "promptRecency", stats.prompts, stats.promptAt), "Prompts; latest prompt age", Muted)
        if (prefs.visible("completions") || prefs.visible("completionRecency")) Stat("⋖", activity("completions", "completionRecency", stats.completions, stats.completionAt), "Completions; latest completion age", Muted)
        if (prefs.visible("tools")) Stat("⚒", stats.tools?.toString() ?: "—", "Tools", Peach)
        if (prefs.visible("compactions")) Stat("⊜", stats.compactions?.toString() ?: "—", "Compactions", Color(0xFFE879F9))
    }
}
@Composable private fun Stat(symbol: String, value: String, label: String, color: Color) {
    Row(Modifier.clearAndSetSemantics { contentDescription = "$label: $value" }, horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(symbol, color = color, fontSize = 13.sp, lineHeight = 16.sp)
        Text(value, color = Muted, fontSize = 11.sp, lineHeight = 16.sp)
    }
}
@Composable private fun SessionRow(session: Session, selected: Boolean, clickable: Boolean = true, open: () -> Unit) {
    val color = remember(session.color) { runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(Peach) }
    val background = remember(session.background) { runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel) }
    val status = if (!session.active) "Ended" else when (session.state) {
        "question" -> "Question"; "permission" -> "Permission"; "thinking" -> "Working"; "complete" -> "Complete"; else -> "Ready"
    }
    Row(Modifier.fillMaxWidth().then(if (clickable) Modifier.clickable(onClick = open) else Modifier).padding(vertical = 4.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.Top) {
        Surface(color = background, shape = RoundedCornerShape(8.dp)) { Text(session.glyph, color = color, fontSize = 16.sp, lineHeight = 20.sp, modifier = Modifier.padding(5.dp)) }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(5.dp)) {
            Text(session.title, color = if (selected) Peach else Color(0xFFE9EEF4), maxLines = 2, overflow = TextOverflow.Ellipsis, fontSize = 14.sp, lineHeight = 19.sp, fontWeight = FontWeight.Medium)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(status, color = if (session.attention) Peach else Mint, fontSize = 11.sp, lineHeight = 14.sp)
                Text(session.machine, color = Muted, fontSize = 11.sp, lineHeight = 14.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable internal fun PromptPanel(prompt: JSONObject, disabled: Boolean, send: (PromptActions.Action) -> Unit) {
    var other by remember(prompt.toString()) { mutableStateOf("") }
    Surface(color = Color(0xFF28251F)) {
        Column(Modifier.fillMaxWidth().heightIn(max = 330.dp).verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(PromptActions.title(prompt), fontWeight = FontWeight.Medium, fontSize = 16.sp)
            if (prompt.has("tool_input")) SelectionContainer { Text(PromptActions.details(prompt), fontFamily = FontFamily.Monospace, fontSize = 11.sp, lineHeight = 15.sp, color = Muted) }
            prompt.array("review").forEach { Text("${it.text("question")}\n${it.text("answer")}", fontSize = 13.sp, color = Muted) }
            val q = PromptActions.question(prompt)
            PromptActions.options(prompt).forEachIndexed { index, option ->
                val checked = prompt.optJSONArray("checked")?.let { a -> (0 until a.length()).any { a.optInt(it) == index + 1 } } == true
                OutlinedButton(onClick = { send(PromptActions.action(prompt, option = index)) }, enabled = !disabled, modifier = Modifier.fillMaxWidth(), shape = RoundedCornerShape(12.dp)) { Column(Modifier.fillMaxWidth()) { Text((if (q?.optBoolean("multi_select") == true) if (checked) "☑ " else "☐ " else "") + option.text("label")); if (option.text("description").isNotBlank()) Text(option.text("description"), fontSize = 11.sp, color = Muted) } }
            }
            if (q != null && !prompt.has("review") && q.optBoolean("allows_other", true)) OutlinedTextField(other, { other = it }, label = { Text("Your answer") }, modifier = Modifier.fillMaxWidth(), maxLines = 3)
            if (q != null && !prompt.has("review")) {
                Button(onClick = { send(PromptActions.action(prompt, text = other.ifBlank { null }, submit = true)) }, enabled = !disabled && (other.isNotBlank() || q.optBoolean("multi_select"))) { Text(if (q.optBoolean("multi_select")) "Submit selections" else "Answer") }
            }
        }
    }
}
