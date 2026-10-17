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
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
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

@Composable internal fun MenuPanel(menu: String?, anchor: Rect, model: SessionModel, notifications: () -> Unit, navigate: (String) -> Unit, close: () -> Unit) {
    val prefs by model.preferences.collectAsStateWithLifecycle()
    var lastMenu by remember { mutableStateOf("Settings") }
    if (menu != null) lastMenu = menu
    val title = if (lastMenu == "Board style") "Style" else lastMenu
    val focus = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(menu) { if (menu != null) { focus.clearFocus(); keyboard?.hide() } }
    BackHandler(menu != null) { if (menu in listOf("Notifications", "New session", "Watch PR")) navigate("Settings") else close() }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val density = LocalDensity.current
        val phone = LocalConfiguration.current.smallestScreenWidthDp < 600
        val direction = LocalLayoutDirection.current
        val viewport = listOf(maxWidth.value, maxHeight.value, density.density, density.fontScale)
        var previousViewport by remember { mutableStateOf(viewport) }
        LaunchedEffect(viewport) { if (previousViewport != viewport) { previousViewport = viewport; close() } }
        val safeLeft = with(density) { WindowInsets.safeDrawing.getLeft(this, direction).toDp() } + 12.dp
        val safeRight = with(density) { WindowInsets.safeDrawing.getRight(this, direction).toDp() } + 12.dp
        val safeTop = with(density) { WindowInsets.safeDrawing.getTop(this).toDp() } + 8.dp
        val safeBottom = with(density) { WindowInsets.safeDrawing.getBottom(this).toDp() } + 12.dp
        val menuWidth = 360.dp.coerceAtMost((maxWidth - safeLeft - safeRight).coerceAtLeast(1.dp))
        val left = (with(density) { anchor.right.toDp() } - menuWidth).coerceIn(safeLeft, (maxWidth - menuWidth - safeRight).coerceAtLeast(safeLeft))
        val latestTop = (maxHeight - safeBottom - 120.dp).coerceAtLeast(safeTop)
        val top = (with(density) { anchor.bottom.toDp() } + 8.dp).coerceIn(safeTop, latestTop)
        val availableHeight = (maxHeight - top - safeBottom).coerceAtLeast(1.dp)
        val menuHeight = if (phone) availableHeight * .75f else availableHeight
        AnimatedVisibility(menu != null, enter = fadeIn(tween(160)), exit = fadeOut(tween(140))) {
            Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = .12f)).clickable(onClickLabel = "Close menu", onClick = close))
        }
        AnimatedVisibility(menu != null, modifier = Modifier.offset(x = left, y = top),
            enter = fadeIn(tween(160)) + scaleIn(tween(200), initialScale = .96f, transformOrigin = TransformOrigin(1f, 0f)),
            exit = fadeOut(tween(140)) + scaleOut(tween(160), targetScale = .96f, transformOrigin = TransformOrigin(1f, 0f))) {
            Surface(Modifier.width(menuWidth).heightIn(max = menuHeight)
                .semantics { paneTitle = title }.pointerInput(Unit) { detectTapGestures {} }, color = Ink,
                shape = RoundedCornerShape(16.dp), shadowElevation = 20.dp, border = BorderStroke(1.dp, Color(0xFF37414E))) {
                Column {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        if (lastMenu in listOf("Notifications", "New session", "Watch PR")) Control(R.drawable.ic_back, "Back to settings") { navigate("Settings") }
                        Text(title, Modifier.weight(1f), fontSize = 18.sp, fontWeight = FontWeight.Medium)
                        Control(R.drawable.ic_close, "Close menu", onClick = close)
                    }
                    HorizontalDivider(color = Color(0xFF303A46))
                    AnimatedContent(lastMenu, label = "Menu page", modifier = Modifier.weight(1f, fill = false)) { page ->
                        if (page == "Style" || page == "Board style") {
                            StyleMenu(prefs, terminalInitially = page == "Style", update = model::style)
                        } else {
                            Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                                when (page) {
                                    "Notifications" -> NotificationMenu(notifications)
                                    "New session" -> BoardActionMenu(model, watch = false)
                                    "Watch PR" -> BoardActionMenu(model, watch = true)
                                    else -> SettingsMenu(model, prefs, navigate)
                                }
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
@Composable private fun SettingPicker(label: String, value: String, options: List<String>, choose: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        TextButton({ expanded = true }, Modifier.fillMaxWidth()) {
            Text(label, Modifier.weight(1f), color = MaterialTheme.colorScheme.onSurface, fontSize = 14.sp)
            Text(value, color = Cyan, fontSize = 13.sp)
        }
        DropdownMenu(expanded, { expanded = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(text = { Text(option) }, onClick = { choose(index); expanded = false })
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
@Composable private fun Stepper(label: String, decrease: () -> Unit, increase: () -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        OutlinedButton(decrease) { Text("−") }
        Box(Modifier.weight(1f), contentAlignment = Alignment.Center) { Text(label, color = Cyan) }
        OutlinedButton(increase) { Text("+") }
    }
}
@Composable internal fun StyleMenu(p: UiPreferences, terminalInitially: Boolean, update: ((UiPreferences) -> UiPreferences) -> Unit) {
    var terminal by remember(terminalInitially) { mutableStateOf(terminalInitially) }
    Column {
        Box(Modifier.padding(horizontal = 16.dp, vertical = 10.dp)) {
            Choices(listOf("List / PR board", "Terminal"), if (terminal) 1 else 0) { terminal = it == 1 }
        }
        key(terminal) {
            Column(Modifier.fillMaxWidth().weight(1f, fill = false).verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                if (terminal) {
                    MenuSection("Text") {
                        Text("Text size", color = Muted, fontSize = 12.sp)
                        Stepper("${p.fontSize} sp", { update { it.copy(fontSize = (it.fontSize - 1).coerceAtLeast(9)) } }, { update { it.copy(fontSize = (it.fontSize + 1).coerceAtMost(24)) } })
                        Text("Line spacing", color = Muted, fontSize = 12.sp)
                        Choices(listOf("Tight", "Normal", "Relaxed"), listOf(120,145,170).indexOf(p.lineSpacing)) { n -> update { it.copy(lineSpacing = listOf(120,145,170)[n]) } }
                        Text("Text wrap", color = Muted, fontSize = 12.sp)
                        Choices(listOf("Wrap", "Horizontal scroll"), if (p.wrap) 0 else 1) { n -> update { it.copy(wrap = n == 0) } }
                    }
                    MenuSection("Layout") {
                        Text("Terminal height", color = Muted, fontSize = 12.sp)
                        Choices(listOf("Full", "250", "350", "500", "700"), listOf(0,250,350,500,700).indexOf(p.terminalHeight)) { n -> update { it.copy(terminalHeight = listOf(0,250,350,500,700)[n]) } }
                    }
                    MenuSection("Widgets below the terminal") {
                        Choices(listOf("Expanded", "Collapsed"), if (p.widgets) 0 else 1) { n -> update { it.copy(widgets = n == 0) } }
                        listOf("stats" to "Status", "recap" to "Recap", "prs" to "Pull requests", "commits" to "Commits", "git" to "Git status", "changes" to "Changes").forEach { (key, label) -> CheckRow(label, p.visible(key)) { update { it.toggle(key) } } }
                    }
                } else {
                    MenuSection("Text and spacing") {
                        Text("Text size", color = Muted, fontSize = 12.sp)
                        Stepper("${p.boardFontSize} sp", { update { it.copy(boardFontSize = (it.boardFontSize - 1).coerceAtLeast(10)) } }, { update { it.copy(boardFontSize = (it.boardFontSize + 1).coerceAtMost(24)) } })
                        Text("Row spacing", color = Muted, fontSize = 12.sp)
                        Choices(listOf("Tight", "Normal", "Relaxed"), listOf("compact", "comfortable", "relaxed").indexOf(p.density).coerceAtLeast(0)) { n -> update { it.copy(density = listOf("compact", "comfortable", "relaxed")[n]) } }
                        Text("Tight keeps titles on one line.", color = Muted, fontSize = 12.sp)
                    }
                    MenuSection("Layout") {
                        Text("List columns", color = Muted, fontSize = 12.sp)
                        Choices(listOf("1", "2", "3", "4", "Fit"), if (p.columns == 0) 4 else p.columns - 1) { n -> update { it.copy(columns = if (n == 4) 0 else n + 1) } }
                        Text("Projects", color = Muted, fontSize = 12.sp)
                        Choices(listOf("Together", "Grouped"), if (p.grouping == "all") 0 else 1) { n -> update { it.copy(grouping = if (n == 0) "all" else "project") } }
                        if (p.grouping == "project") {
                            Text("Project order", color = Muted, fontSize = 12.sp)
                            Choices(listOf("Most recent", "Alphabetical"), if (p.order == "recent") 0 else 1) { n -> update { it.copy(order = if (n == 0) "recent" else "alpha") } }
                        }
                    }
                }
            }
        }
    }
}

@Composable private fun SettingsMenu(model: SessionModel, p: UiPreferences, navigate: (String) -> Unit) {
    val context = LocalContext.current
    val uri = LocalUriHandler.current
    val scope = rememberCoroutineScope()
    var account by remember { mutableStateOf<JSONObject?>(null) }
    var message by remember { mutableStateOf("") }
    var invite by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var confirmUnpair by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { try { account = model.api.call("/api/account") } catch (e: Exception) { if (e is kotlinx.coroutines.CancellationException) throw e; message = e.message.orEmpty() } }
    val state by model.state.collectAsStateWithLifecycle()
    MenuSection("List / PR board") {
        val hours = listOf(1, 3, 6, 9, 12, 24, 0)
        val ages = hours.map { if (it == 0) "All ages" else if (it == 1) "Last hour" else "Last $it hours" }
        SettingPicker("Activity", ages[hours.indexOf(p.boardAgeHours).coerceAtLeast(0)], ages) { n -> model.style { it.copy(boardAgeHours = hours[n]) } }
        SettingPicker("Sessions", if (p.showEnded) "Live and ended" else "Live", listOf("Live", "Live and ended")) { n -> model.style { it.copy(showEnded = n == 1) } }
        val computers = (state.sessions.map { it.machineGroup } + p.boardComputer).filter { it.isNotBlank() }.distinct().sorted()
        SettingPicker("Computers", p.boardComputer.ifBlank { "All computers" }, listOf("All computers") + computers) { n -> model.style { it.copy(boardComputer = if (n == 0) "" else computers[n - 1]) } }
        CheckRow("Show recaps", p.boardRecaps) { model.style { it.copy(boardRecaps = !it.boardRecaps) } }
        SettingPicker("PR board groups", if (p.boardGrouping == "prs") "Pull requests" else "Sessions", listOf("Sessions", "Pull requests")) { n -> model.style { it.copy(boardGrouping = if (n == 1) "prs" else "sessions") } }
    }
    val navigation by model.navigation.collectAsStateWithLifecycle()
    MenuSection("Actions") {
        TextButton({ navigate("Search") }) { Text("Search sessions", color = Cyan) }
        TextButton({ navigate("New session") }) { Text("New session", color = Cyan) }
        TextButton({ navigate("Watch PR") }) { Text("Watch PR", color = Cyan) }
        if (LocalConfiguration.current.smallestScreenWidthDp >= 600 && state.selected != null) {
            TextButton({ navigate("Toggle fullscreen") }) { Text(if (navigation.expanded) "Return to session" else "Expand List / PR board", color = Cyan) }
        }
    }
    MenuSection("Appearance") { TextButton({ navigate("Board style") }, Modifier.fillMaxWidth()) { Text("Style", color = Cyan) } }
    MenuSection("Notifications") { TextButton({ navigate("Notifications") }, Modifier.fillMaxWidth()) { Text(Notifications.status(context), color = Cyan) } }
    MenuSection("Session list") {
        Text("Position", color = Muted, fontSize = 12.sp)
        Choices(listOf("Left", "Right"), if (p.sidebarRight) 1 else 0) { n -> model.style { it.copy(sidebarRight = n == 1) } }
    }
    MenuSection("Visible stats") { listOf("sessionTime" to "Session time", "thinkingTime" to "Thinking time", "promptRecency" to "Last prompt", "completionRecency" to "Last completion", "tools" to "Tools", "compactions" to "Compactions").forEach { (key,label) -> CheckRow(label, p.visible(key)) { model.style { it.toggle(key) } } } }
    MenuSection("Sign-in") {
        val identities = account?.array("identities").orEmpty()
        if (account == null && message.isEmpty()) LinearProgressIndicator(Modifier.fillMaxWidth())
        else if (identities.isEmpty()) Text("Pairing code", fontSize = 14.sp)
        identities.forEach { Text(it.text("email", it.text("name", it.text("provider"))), fontSize = 14.sp) }
        TextButton({ uri.openUri(model.credentials.origin + "/dashboard") }) { Text("Manage sign-in", color = Cyan) }
    }
    MenuSection("Pair another device") {
        if (invite.isNotEmpty()) androidx.compose.foundation.text.selection.SelectionContainer { Text(invite, fontSize = 24.sp, color = Cyan) }
        TextButton(onClick = { busy = true; scope.launch { try { invite = model.api.call("/api/pairing/invite", JSONObject()).text("code") } catch (e: Exception) { if (e is kotlinx.coroutines.CancellationException) throw e; message = e.message.orEmpty() } finally { busy = false } } }, enabled = !busy) { Text(if (busy) "Generating…" else "Generate pairing code", color = Cyan) }
    }
    MenuSection("MCP") {
        androidx.compose.foundation.text.selection.SelectionContainer { Text(model.credentials.origin + "/mcp", fontSize = 13.sp) }
        TextButton({ uri.openUri(model.credentials.origin + "/mcp-tools") }) { Text("Tools and examples", color = Cyan) }
    }
    MenuSection("This device") {
        Text(android.os.Build.MODEL, fontSize = 14.sp); Text(model.credentials.origin, fontSize = 12.sp, color = Muted)
        Text("Version ${BuildConfig.VERSION_NAME}", fontSize = 12.sp, color = Muted)
        if (confirmUnpair) { Text("Unpair this device?", fontSize = 14.sp); Row { TextButton({ model.unpair() }) { Text("Unpair", color = MaterialTheme.colorScheme.error) }; TextButton({ confirmUnpair = false }) { Text("Cancel") } } }
        else TextButton({ confirmUnpair = true }) { Text("Unpair this device", color = MaterialTheme.colorScheme.error) }
    }
    if (message.isNotEmpty()) Text(message, color = MaterialTheme.colorScheme.error, fontSize = 13.sp)
}
@Composable private fun BoardActionMenu(model: SessionModel, watch: Boolean) {
    val state by model.state.collectAsStateWithLifecycle()
    val desktops = state.sessions.filter { it.active }.distinctBy { it.machineGroup }
    var computer by remember { mutableStateOf(desktops.firstOrNull()?.machineGroup.orEmpty()) }
    val source = desktops.firstOrNull { it.machineGroup == computer }
    var cwd by remember(computer) { mutableStateOf(source?.cwd.orEmpty()) }
    var platform by remember { mutableStateOf("codex") }
    var target by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    var message by remember { mutableStateOf("") }
    var failed by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    if (watch) {
        OutlinedTextField(target, { target = it }, label = { Text("GitHub URL or owner/repo#123") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
    } else {
        MenuSection("Desktop") {
            val names = desktops.map { it.machineGroup }
            SettingPicker("Computer", computer.ifBlank { "No connected desktop" }, names) { computer = names[it] }
            OutlinedTextField(cwd, { cwd = it }, label = { Text("Folder") }, modifier = Modifier.fillMaxWidth(), singleLine = true)
            val folders = state.sessions.filter { it.machineGroup == computer }.map { it.cwd }.filter { it.isNotBlank() }.distinct().sorted()
            if (folders.isNotEmpty()) SettingPicker("Recent folders", "Choose…", folders) { cwd = folders[it] }
            val platforms = listOf("claude", "codex", "opencode", "grok")
            SettingPicker("Assistant", platform, platforms) { platform = platforms[it] }
            if (desktops.isEmpty()) Text("Open Crabigator on a desktop to start a session.", color = Muted)
        }
    }
    Button(onClick = {
        busy = true; message = ""; failed = false
        scope.launch {
            try {
                if (watch) {
                    val body = BoardActions.watchTarget(target) ?: return@launch
                    model.api.call("/api/prs/watched", body)
                    message = "Watching ${body.text("owner")}/${body.text("repo")}#${body.optInt("number")}."
                } else {
                    val desktop = source ?: return@launch
                    model.api.call("/api/sessions/${desktop.id}/spawn", JSONObject().put("cwd", cwd.trim()).put("platform", platform))
                    message = "Requested on $computer. The session will appear when it connects."
                }
                model.refresh()
            } catch (e: Exception) {
                if (e is kotlinx.coroutines.CancellationException) throw e
                failed = true; message = e.message.orEmpty()
            } finally { busy = false }
        }
    }, enabled = !busy && (if (watch) BoardActions.watchTarget(target) != null else source != null && cwd.trim().startsWith("/"))) {
        Text(if (busy) "Sending…" else if (watch) "Watch PR" else "Start session")
    }
    if (message.isNotBlank()) Text(message, color = if (failed) MaterialTheme.colorScheme.error else Mint)
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
