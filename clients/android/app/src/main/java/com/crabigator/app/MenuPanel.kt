package com.crabigator.app

import android.content.Intent
import android.provider.Settings
import androidx.activity.compose.BackHandler
import androidx.compose.animation.*
import androidx.compose.animation.core.tween
import androidx.compose.foundation.*
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import kotlinx.coroutines.launch
import org.json.JSONObject

private val Cyan = CrabColors.Cyan

/** Which screen a View panel changes, or null for every other page. */
private fun viewScope(page: String) = page.removePrefix("View ").takeIf { page.startsWith("View ") }

@Composable internal fun MenuPanel(menu: String?, anchor: Rect, model: SessionModel, notifications: () -> Unit, navigate: (String) -> Unit, close: () -> Unit) {
    val prefs by model.preferences.collectAsStateWithLifecycle()
    var lastMenu by remember { mutableStateOf("Account") }
    if (menu != null) lastMenu = menu
    val scope = viewScope(lastMenu)
    val title = when (lastMenu) { "Create session" -> "New session"; "Watch PR" -> "Watch a pull request"; "Session voice transcripts" -> "Voice transcripts"; else -> lastMenu }
    // Pages opened from another menu page, and the page Back returns to.
    val parents = mapOf("Notifications" to "Account", "Voice transcripts" to "Account", "Session voice transcripts" to "View Session")
    val focus = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(menu) { if (menu != null) { focus.clearFocus(); keyboard?.hide() } }
    BackHandler(menu != null) { parents[menu]?.let(navigate) ?: close() }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val density = LocalDensity.current
        val phone = LocalConfiguration.current.smallestScreenWidthDp < 600
        val direction = LocalLayoutDirection.current
        val viewport = listOf(maxWidth.value, maxHeight.value, density.density, density.fontScale)
        var previousViewport by remember { mutableStateOf(viewport) }
        LaunchedEffect(viewport) { if (previousViewport != viewport) { previousViewport = viewport; close() } }
        val insetLeft = with(density) { WindowInsets.safeDrawing.getLeft(this, direction).toDp() }
        val insetRight = with(density) { WindowInsets.safeDrawing.getRight(this, direction).toDp() }
        val insetBottom = with(density) { WindowInsets.safeDrawing.getBottom(this).toDp() }
        val safeLeft = insetLeft + 12.dp
        val safeRight = insetRight + 12.dp
        val safeTop = with(density) { WindowInsets.safeDrawing.getTop(this).toDp() } + 8.dp
        val safeBottom = insetBottom + 12.dp
        // On a phone the View and Account panels are sheets: full width, from under their button to the foot of the screen.
        val sheet = phone && lastMenu !in listOf("Create session", "Watch PR")
        // A View panel holds drawn tiles side by side, so its popover is wider.
        val viewFamily = scope != null || parents[lastMenu]?.let(::viewScope) != null
        val menuWidth = if (sheet) maxWidth else (if (viewFamily) 440.dp else 360.dp).coerceAtMost((maxWidth - safeLeft - safeRight).coerceAtLeast(1.dp))
        val left = if (sheet) 0.dp else (with(density) { anchor.right.toDp() } - menuWidth).coerceIn(safeLeft, (maxWidth - menuWidth - safeRight).coerceAtLeast(safeLeft))
        val latestTop = (maxHeight - safeBottom - 120.dp).coerceAtLeast(safeTop)
        // The Account pages take the whole phone screen. A View panel starts under its button, so the screen it changes stays in sight.
        val top = if (sheet && !viewFamily) safeTop else (with(density) { anchor.bottom.toDp() } + 8.dp).coerceIn(safeTop, latestTop)
        val availableHeight = (maxHeight - top - safeBottom).coerceAtLeast(1.dp)
        val menuHeight = if (phone) availableHeight * .75f else availableHeight
        val surface by animateColorAsState(if (scope != null) Panel else Ink, tween(160), label = "Menu surface")
        AnimatedVisibility(menu != null, enter = fadeIn(tween(160)), exit = fadeOut(tween(140))) {
            Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = .12f)).clickable(onClickLabel = "Close menu", onClick = close))
        }
        AnimatedVisibility(menu != null, modifier = Modifier.offset(x = left, y = top),
            enter = if (sheet) fadeIn(tween(160)) + slideInVertically(tween(240)) { it / 10 } else fadeIn(tween(160)) + scaleIn(tween(200), initialScale = .96f, transformOrigin = TransformOrigin(1f, 0f)),
            exit = if (sheet) fadeOut(tween(140)) + slideOutVertically(tween(160)) { it / 10 } else fadeOut(tween(140)) + scaleOut(tween(160), targetScale = .96f, transformOrigin = TransformOrigin(1f, 0f))) {
            Surface(Modifier.width(menuWidth).then(if (sheet) Modifier.height(maxHeight - top) else Modifier.heightIn(max = menuHeight))
                .semantics { paneTitle = title }.pointerInput(Unit) { detectTapGestures {} }, color = surface,
                shape = if (sheet) RoundedCornerShape(topStart = 22.dp, topEnd = 22.dp) else RoundedCornerShape(16.dp), shadowElevation = 20.dp, border = BorderStroke(1.dp, Color(0xFF37414E))) {
                Column(if (sheet) Modifier.padding(start = insetLeft, end = insetRight) else Modifier) {
                    Row(Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 4.dp, bottom = 2.dp), verticalAlignment = Alignment.CenterVertically) {
                        parents[lastMenu]?.let { parent -> Control(R.drawable.ic_back, "Back to ${if (viewScope(parent) != null) "View" else parent.lowercase()}") { navigate(parent) } }
                        // A View panel names the screen it changes, in the brand's gold.
                        Text(buildAnnotatedString {
                            append(if (scope != null) "View" else title)
                            if (scope != null) withStyle(SpanStyle(color = CrabColors.Orange, fontWeight = FontWeight.Medium)) { append(" $scope") }
                        }, Modifier.weight(1f).semantics { heading() }, fontSize = if (lastMenu == "Account") 20.sp else 17.sp, fontWeight = FontWeight.Bold)
                        TextButton(close, Modifier.heightIn(min = 44.dp)) { Text("Done", color = Cyan, fontSize = 16.sp, fontWeight = FontWeight.SemiBold) }
                    }
                    if (scope == null && lastMenu != "Account") HorizontalDivider(color = Hairline)
                    AnimatedContent(lastMenu, label = "Menu page", modifier = Modifier.weight(1f, fill = false)) { page ->
                        Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(start = 16.dp, end = 16.dp, top = if (viewScope(page) != null || page == "Account") 6.dp else 16.dp, bottom = if (sheet) insetBottom + 24.dp else 16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                            when (page) {
                                "View List" -> BoardViewPanel(model, prefs, list = true)
                                "View PR board" -> BoardViewPanel(model, prefs, list = false)
                                "View Session" -> SessionViewPanel(prefs, model::style) { navigate("Session voice transcripts") }
                                "Notifications" -> NotificationMenu(notifications)
                                "Create session" -> BoardActionMenu(model, watch = false)
                                "Watch PR" -> BoardActionMenu(model, watch = true)
                                "Voice transcripts" -> VoiceTranscriptsMenu(model, null)
                                "Session voice transcripts" -> {
                                    // Follow the open session without redrawing this page for each terminal update.
                                    val state = model.state.collectAsStateWithLifecycle()
                                    val sessionId by remember { derivedStateOf { state.value.selected?.id } }
                                    VoiceTranscriptsMenu(model, sessionId)
                                }
                                else -> AccountMenu(model, navigate)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable private fun MenuSection(title: String, content: @Composable ColumnScope.() -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(title, color = Muted, fontSize = 13.sp, fontWeight = FontWeight.Medium)
        Surface(color = Panel, shape = RoundedCornerShape(12.dp), border = BorderStroke(1.dp, Color(0xFF303A46))) {
            Column(Modifier.fillMaxWidth().padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp), content = content)
        }
    }
}
@Composable private fun Choices(values: List<String>, selected: Int, choose: (Int) -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        values.forEachIndexed { index, label ->
            Surface(onClick = { choose(index) }, modifier = Modifier.weight(1f).semantics { this.selected = index == selected }, shape = RoundedCornerShape(7.dp),
                color = if (index == selected) Cyan else Ink, contentColor = if (index == selected) Ink else Muted) {
                Box(Modifier.heightIn(min = 44.dp).padding(horizontal = 4.dp, vertical = 10.dp), contentAlignment = Alignment.Center) { Text(label, fontSize = 12.sp) }
            }
        }
    }
}
@Composable private fun CheckRow(label: String, checked: Boolean, change: () -> Unit) {
    Row(Modifier.fillMaxWidth().clickable(onClick = change), verticalAlignment = Alignment.CenterVertically) {
        Text(label, Modifier.weight(1f), fontSize = 14.sp)
        Checkbox(checked, { change() }, colors = CheckboxDefaults.colors(checkedColor = Cyan))
    }
}
/** Where the Account page reads the account from. Tests and the debug preview supply their own; otherwise it asks the server. */
internal val LocalAccountSource = compositionLocalOf<(suspend () -> JSONObject)?> { null }

/** The account, its devices and its alerts. Nothing here changes how a screen looks. */
@Composable private fun AccountMenu(model: SessionModel, navigate: (String) -> Unit) {
    val context = LocalContext.current
    val uri = LocalUriHandler.current
    val scope = rememberCoroutineScope()
    var account by remember { mutableStateOf<JSONObject?>(null) }
    var message by remember { mutableStateOf("") }
    var invite by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var confirmUnpair by remember { mutableStateOf(false) }
    val source = LocalAccountSource.current
    LaunchedEffect(Unit) { try { account = source?.invoke() ?: model.api.call("/api/account") } catch (e: Exception) { if (e is kotlinx.coroutines.CancellationException) throw e; message = e.message.orEmpty() } }
    AccountGroup("Alerts and history") {
        val status = remember { Notifications.status(context) }
        val on = status == "Notifications on"
        AccountRow("Notifications", value = if (on) (if (Notifications.soundEnabled(context)) "On, with sound" else "On, silent") else if (status == "Notifications off") "Off" else status, valueColor = if (on) Mint else MutedText, chevron = true) { navigate("Notifications") }
        AccountDivider()
        AccountRow("Voice transcripts", chevron = true) { navigate("Voice transcripts") }
    }
    AccountGroup("Sign-in") {
        val identities = account?.array("identities").orEmpty()
        if (account == null && message.isEmpty()) LinearProgressIndicator(Modifier.fillMaxWidth().padding(vertical = 24.dp))
        else if (identities.isEmpty()) AccountText("Pairing code")
        identities.forEach { identity ->
            val who = identity.text("email", identity.text("name", identity.text("username")))
            val provider = when (identity.text("provider")) { "github" -> "GitHub"; "google" -> "Google"; else -> identity.text("provider").replaceFirstChar { it.uppercase() } }
            AccountText(if (who.isBlank()) provider else if (provider.isBlank()) who else "$who with $provider")
        }
        AccountDivider()
        AccountRow("Manage sign-in on the web", color = Cyan) { uri.openUri(model.credentials.origin + "/dashboard") }
    }
    AccountGroup("Devices and agents") {
        AccountRow(if (busy) "Generating…" else "Pair another device", color = Cyan, enabled = !busy) { busy = true; scope.launch { try { invite = model.api.call("/api/pairing/invite", JSONObject()).text("code") } catch (e: Exception) { if (e is kotlinx.coroutines.CancellationException) throw e; message = e.message.orEmpty() } finally { busy = false } } }
        if (invite.isNotEmpty()) SelectionContainer { Text(invite, fontSize = 24.sp, color = Cyan, modifier = Modifier.padding(bottom = 12.dp)) }
        AccountDivider()
        Column(Modifier.padding(top = 10.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("Connect an agent with MCP", color = PrimaryText, fontSize = 15.sp)
            SelectionContainer { Text(model.credentials.origin + "/mcp", color = SecondaryText, fontSize = 13.sp, fontFamily = FontFamily.Monospace) }
            AccountRow("Tools and examples", color = Cyan, height = 44, fontSize = 14) { uri.openUri(model.credentials.origin + "/mcp-tools") }
        }
    }
    AccountGroup("This device") {
        Column(Modifier.padding(vertical = 10.dp), verticalArrangement = Arrangement.spacedBy(3.dp)) {
            Text(android.os.Build.MODEL, color = PrimaryText, fontSize = 15.sp)
            Text("${model.credentials.origin.removePrefix("https://")} · Version ${BuildConfig.VERSION_NAME}", color = MutedText, fontSize = 13.sp)
        }
        AccountDivider()
        AccountRow("Privacy policy", color = Cyan) { uri.openUri("https://drinkcrabigator.com/privacy") }
        AccountDivider()
        if (confirmUnpair) Row(Modifier.heightIn(min = 52.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Unpair this device?", Modifier.weight(1f), color = PrimaryText, fontSize = 15.sp)
            TextButton({ model.unpair() }) { Text("Unpair", color = CrabColors.Red) }
            TextButton({ confirmUnpair = false }) { Text("Cancel") }
        } else AccountRow("Unpair this device", color = CrabColors.Red) { confirmUnpair = true }
    }
    if (message.isNotEmpty()) Text(message, color = MaterialTheme.colorScheme.error, fontSize = 13.sp)
}
@Composable private fun AccountGroup(title: String, content: @Composable ColumnScope.() -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(title, color = MutedText, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, modifier = Modifier.semantics { heading() })
        Surface(color = Panel, shape = RoundedCornerShape(14.dp), border = BorderStroke(1.dp, Hairline)) {
            Column(Modifier.fillMaxWidth().padding(horizontal = 14.dp), content = content)
        }
    }
}
@Composable private fun AccountRow(label: String, value: String = "", color: Color = PrimaryText, valueColor: Color = MutedText, chevron: Boolean = false, enabled: Boolean = true, height: Int = 52, fontSize: Int = 15, onClick: () -> Unit) {
    Row(Modifier.fillMaxWidth().heightIn(min = height.dp).clickable(enabled, role = Role.Button, onClick = onClick), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, Modifier.weight(1f), color = color, fontSize = fontSize.sp)
        if (value.isNotBlank()) Text(value, color = valueColor, fontSize = 14.sp, maxLines = 1)
        if (chevron) Icon(painterResource(R.drawable.ic_chevron_down), null, tint = MutedText, modifier = Modifier.size(16.dp).rotate(-90f))
    }
}
@Composable private fun AccountText(text: String) { Box(Modifier.heightIn(min = 52.dp), contentAlignment = Alignment.CenterStart) { Text(text, color = PrimaryText, fontSize = 15.sp) } }
@Composable private fun AccountDivider() { Box(Modifier.fillMaxWidth().height(1.dp).background(Hairline)) }
@Composable internal fun NewSessionForm(model: SessionModel, request: suspend (String, JSONObject) -> Unit = { path, body -> model.api.call(path, body); model.refresh() }) {
    val state by model.state.collectAsStateWithLifecycle()
    val launch by model.sessionLaunch.collectAsStateWithLifecycle()
    val pending = launch.takeIf { it.busy }
    val desktops = state.sessions.filter { it.active }.distinctBy { it.machineGroup }
    var computer by remember { mutableStateOf(pending?.computer ?: state.selected?.takeIf { it.active }?.machineGroup ?: desktops.firstOrNull()?.machineGroup.orEmpty()) }
    val source = desktops.firstOrNull { it.machineGroup == computer }
    val projects = state.sessions.filter { it.machineGroup == computer && it.cwd.isNotBlank() }.distinctBy { it.cwd }.sortedBy { it.repo.ifBlank { it.cwd.substringAfterLast('/') } }
    var folder by remember(computer) { mutableStateOf(if (pending != null) pending.path.takeIf { path -> projects.any { it.cwd == path } }.orEmpty() else state.selected?.takeIf { it.machineGroup == computer }?.cwd ?: projects.firstOrNull()?.cwd.orEmpty()) }
    var customFolder by remember(computer) { mutableStateOf(pending?.path.orEmpty()) }
    var platform by remember { mutableStateOf(pending?.platform ?: "codex") }
    var projectMenu by remember { mutableStateOf(false) }
    var computerMenu by remember { mutableStateOf(false) }
    val busy = launch.busy
    LaunchedEffect(desktops) { if (computer.isEmpty()) computer = desktops.firstOrNull()?.machineGroup.orEmpty() }
    val path = (if (folder.isBlank()) customFolder else folder).trim()
    MenuSection("Project") {
        Box {
            TextButton({ projectMenu = true }, enabled = !busy, modifier = Modifier.fillMaxWidth()) {
                Text(projects.firstOrNull { it.cwd == folder }?.let { it.repo.ifBlank { it.cwd.substringAfterLast('/') } } ?: "Other folder…", Modifier.weight(1f), color = Peach)
                Text("⌄", color = Muted)
            }
            DropdownMenu(projectMenu, { projectMenu = false }) {
                projects.forEach { project ->
                    DropdownMenuItem(text = { Column {
                        Text(project.repo.ifBlank { project.cwd.substringAfterLast('/') })
                        Text(project.cwd, fontSize = 11.sp, color = Muted)
                    } }, onClick = { folder = project.cwd; projectMenu = false })
                }
                DropdownMenuItem(text = { Text("Other folder…") }, onClick = { folder = ""; projectMenu = false })
            }
        }
        if (folder.isBlank()) OutlinedTextField(customFolder, { customFolder = it }, label = { Text("Folder") }, placeholder = { Text("/path/to/project") }, enabled = !busy, singleLine = true, modifier = Modifier.fillMaxWidth())
        else Text(folder, fontSize = 11.sp, color = Muted, modifier = Modifier.padding(horizontal = 8.dp))
        if (desktops.size > 1 || source == null && desktops.isNotEmpty()) Box {
            TextButton({ computerMenu = true }, enabled = !busy) { Text(if (source == null) "$computer · Offline" else "On $computer", color = Muted, fontSize = 12.sp) }
            DropdownMenu(computerMenu, { computerMenu = false }) {
                desktops.forEach { desktop -> DropdownMenuItem(text = { Text(desktop.machineGroup) }, onClick = { computer = desktop.machineGroup; computerMenu = false }) }
            }
        } else Text(source?.let { "On ${it.machineGroup}" } ?: "Open Crabigator on a desktop to start a session.", fontSize = 12.sp, color = Muted, modifier = Modifier.padding(8.dp))
    }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Assistant", color = Muted, fontSize = 13.sp)
        listOf("Codex" to "codex", "Claude" to "claude", "OpenCode" to "opencode", "Grok" to "grok").chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { (label, value) ->
                    FilterChip(platform == value, { platform = value }, label = { Text(label) }, enabled = !busy, modifier = Modifier.weight(1f).heightIn(min = 44.dp))
                }
            }
        }
    }
    Button(onClick = {
        val desktop = source ?: return@Button
        model.startSession(desktop, path, platform, request)
    }, enabled = !busy && source != null && path.startsWith("/") && !path.contains('\u0000'), modifier = Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
        if (busy) { CircularProgressIndicator(Modifier.size(16.dp), strokeWidth = 2.dp); Spacer(Modifier.width(8.dp)) }
        Text(if (busy) "Starting…" else "Start session")
    }
    if (launch.message.isNotBlank()) Text(launch.message, color = if (launch.failed) MaterialTheme.colorScheme.error else Mint, fontSize = 13.sp)
}

@Composable private fun BoardActionMenu(model: SessionModel, watch: Boolean) {
    if (!watch) { NewSessionForm(model); return }
    var target by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf("") }
    var failed by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    OutlinedTextField(target, { target = it }, label = { Text("GitHub URL or owner/repo#123") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
    Button(onClick = {
        val body = BoardActions.watchTarget(target) ?: return@Button
        busy = true; message = ""; failed = false
        scope.launch {
            try {
                model.api.call("/api/prs/watched", body)
                message = "Watching ${body.text("owner")}/${body.text("repo")}#${body.optInt("number")}."
                model.refresh()
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                failed = true; message = e.message.orEmpty()
            } finally { busy = false }
        }
    }, enabled = !busy && BoardActions.watchTarget(target) != null) { Text(if (busy) "Sending…" else "Watch PR") }
    if (message.isNotBlank()) Text(message, color = if (failed) MaterialTheme.colorScheme.error else Mint)
}
/**
 * Every voice transcript the account has made. The server saves each one as it is
 * transcribed, so a transcript that never reached its session can still be copied here.
 */
@Composable private fun VoiceTranscriptsMenu(model: SessionModel, sessionId: String?) {
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    val scope = rememberCoroutineScope()
    var everySession by remember { mutableStateOf(sessionId == null) }
    var rows by remember { mutableStateOf(listOf<JSONObject>()) }
    var next by remember { mutableStateOf<Long?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf("") }
    var copied by remember { mutableStateOf("") }
    val filter = if (everySession) null else sessionId
    suspend fun load(more: Boolean) {
        try {
            val page = model.api.voiceTranscripts(filter, if (more) next else null)
            rows = (if (more) rows else emptyList()) + page.array("transcripts")
            next = if (page.isNull("next_cursor")) null else page.optLong("next_cursor")
            message = ""
        } catch (e: kotlinx.coroutines.CancellationException) { throw e }
        catch (e: Exception) { message = e.message ?: "Could not load voice transcripts." }
        loaded = true
    }
    fun copy(text: String, id: String) { clipboard.setText(AnnotatedString(text)); copied = id }
    fun clock(seconds: Double) = "${seconds.toInt() / 60}:${(seconds.toInt() % 60).toString().padStart(2, '0')}"
    fun seconds(row: JSONObject, key: String) = if (row.isNull(key)) null else row.optDouble(key)
    LaunchedEffect(filter) { loaded = false; load(false) }
    LaunchedEffect(copied) { if (copied.isNotEmpty()) { kotlinx.coroutines.delay(2000); copied = "" } }
    if (sessionId != null) Choices(listOf("This session", "All sessions"), if (everySession) 1 else 0) { everySession = it == 1 }
    if (message.isNotEmpty()) Text(message, color = MaterialTheme.colorScheme.error)
    if (!loaded) LinearProgressIndicator(Modifier.fillMaxWidth())
    else if (rows.isEmpty() && message.isEmpty()) Text("No voice transcripts yet. Every recording you transcribe is saved here.", color = Muted, fontSize = 13.sp)
    if (rows.isNotEmpty()) TextButton({ copy(rows.joinToString("\n\n") { it.text("text") }, "all") }) { Text(if (copied == "all") "Copied" else "Copy all", color = Cyan) }
    rows.forEach { row ->
        val id = row.text("id")
        val recorded = seconds(row, "recorded_seconds")
        val audio = seconds(row, "audio_seconds")
        key(id) {
            Surface(color = Panel, shape = RoundedCornerShape(12.dp), border = BorderStroke(1.dp, Color(0xFF303A46))) {
                Column(Modifier.fillMaxWidth().padding(start = 12.dp, end = 4.dp, bottom = 12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        val date = android.text.format.DateUtils.formatDateTime(context, row.optLong("created_at") * 1000,
                            android.text.format.DateUtils.FORMAT_SHOW_DATE or android.text.format.DateUtils.FORMAT_SHOW_TIME or android.text.format.DateUtils.FORMAT_ABBREV_MONTH)
                        Text(listOfNotNull(date, (audio ?: recorded)?.let(::clock)).joinToString(" · "), Modifier.weight(1f), color = Muted, fontSize = 12.sp)
                        TextButton({ copy(row.text("text"), id) }) { Text(if (copied == id) "Copied" else "Copy", color = Cyan) }
                        TextButton({ scope.launch {
                            try { model.api.deleteVoiceTranscript(id); rows = rows.filter { it.text("id") != id } }
                            catch (e: Exception) { message = e.message ?: "Could not delete the transcript." }
                        } }) { Text("Delete", color = Muted) }
                    }
                    Column(Modifier.padding(end = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        if (filter == null && row.text("session_title").isNotBlank()) Text(row.text("session_title"), color = Cyan, fontSize = 12.sp, maxLines = 1)
                        // The app timed more speech than the server received: audio was lost before upload.
                        if (recorded != null && audio != null && recorded - audio > 5) Text("Recorded ${clock(recorded)}, but only ${clock(audio)} of audio arrived.", color = Peach, fontSize = 12.sp)
                        SelectionContainer { Text(row.text("text"), fontSize = 14.sp) }
                    }
                }
            }
        }
    }
    if (next != null) TextButton({ scope.launch { load(true) } }) { Text("Load more", color = Cyan) }
}

@Composable private fun NotificationMenu(requestPermission: () -> Unit) {
    val context = LocalContext.current
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    var status by remember { mutableStateOf(Notifications.status(context)) }
    var sound by remember { mutableStateOf(Notifications.soundEnabled(context)) }
    DisposableEffect(lifecycle, context) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                status = Notifications.status(context)
                sound = Notifications.soundEnabled(context)
            }
        }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer) }
    }
    MenuSection("Questions and permissions") {
        Text(status, color = Cyan, fontSize = 15.sp)
        CheckRow("Sound", sound) { sound = !sound; Notifications.setSoundEnabled(context, sound) }
        TextButton(requestPermission) { Text("Enable notifications") }
        TextButton({ context.startActivity(Intent(Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName).putExtra(Settings.EXTRA_CHANNEL_ID, Notifications.CHANNEL)) }) { Text("Android notification settings") }
    }
}
