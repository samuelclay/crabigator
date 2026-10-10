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
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import kotlinx.coroutines.launch
import org.json.JSONObject

private val Cyan = Color(0xFF67DCE7)

@Composable internal fun MenuPanel(menu: String?, anchor: Rect, model: SessionModel, notifications: () -> Unit, navigate: (String) -> Unit, close: () -> Unit) {
    val prefs by model.preferences.collectAsStateWithLifecycle()
    var lastMenu by remember { mutableStateOf("Settings") }
    if (menu != null) lastMenu = menu
    val focus = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(menu) { if (menu != null) { focus.clearFocus(); keyboard?.hide() } }
    BackHandler(menu != null) { if (menu == "Notifications") navigate("Settings") else close() }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val density = LocalDensity.current
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
        val menuHeight = (maxHeight - top - safeBottom).coerceAtLeast(1.dp)
        AnimatedVisibility(menu != null, enter = fadeIn(tween(160)), exit = fadeOut(tween(140))) {
            Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = .12f)).clickable(onClickLabel = "Close menu", onClick = close))
        }
        AnimatedVisibility(menu != null, modifier = Modifier.offset(x = left, y = top),
            enter = fadeIn(tween(160)) + scaleIn(tween(200), initialScale = .96f, transformOrigin = TransformOrigin(1f, 0f)),
            exit = fadeOut(tween(140)) + scaleOut(tween(160), targetScale = .96f, transformOrigin = TransformOrigin(1f, 0f))) {
            Surface(Modifier.width(menuWidth).heightIn(max = menuHeight)
                .semantics { paneTitle = lastMenu }.pointerInput(Unit) { detectTapGestures {} }, color = Ink,
                shape = RoundedCornerShape(16.dp), shadowElevation = 20.dp, border = BorderStroke(1.dp, Color(0xFF37414E))) {
                Column {
                    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
                        if (lastMenu == "Notifications") Control(R.drawable.ic_back, "Back to settings") { navigate("Settings") }
                        Text(lastMenu, Modifier.weight(1f), fontSize = 18.sp, fontWeight = FontWeight.Medium)
                        Control(R.drawable.ic_close, "Close menu", onClick = close)
                    }
                    HorizontalDivider(color = Color(0xFF303A46))
                    AnimatedContent(lastMenu, label = "Menu page", modifier = Modifier.weight(1f, fill = false)) { page ->
                        Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                            when (page) {
                                "Style" -> StyleMenu(prefs, model::style)
                                "Notifications" -> NotificationMenu(notifications)
                                else -> SettingsMenu(model, prefs, navigate)
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
            Surface(onClick = { choose(index) }, modifier = Modifier.weight(1f), shape = RoundedCornerShape(7.dp),
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
@Composable private fun Stepper(label: String, decrease: () -> Unit, increase: () -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        OutlinedButton(decrease) { Text("−") }
        Box(Modifier.weight(1f), contentAlignment = Alignment.Center) { Text(label, color = Cyan) }
        OutlinedButton(increase) { Text("+") }
    }
}
@Composable private fun StyleMenu(p: UiPreferences, update: ((UiPreferences) -> UiPreferences) -> Unit) {
    MenuSection("Text size") { Stepper("${p.fontSize} sp", { update { it.copy(fontSize = (it.fontSize - 1).coerceAtLeast(9)) } }, { update { it.copy(fontSize = (it.fontSize + 1).coerceAtMost(24)) } }) }
    MenuSection("Line spacing") { Choices(listOf("Tight", "Normal", "Relaxed"), listOf(120,145,170).indexOf(p.lineSpacing)) { n -> update { it.copy(lineSpacing = listOf(120,145,170)[n]) } } }
    MenuSection("Text wrap") { Choices(listOf("Wrap", "Scroll"), if (p.wrap) 0 else 1) { n -> update { it.copy(wrap = n == 0) } } }
    MenuSection("Content") { Choices(listOf("Terminal", "Transcript"), if (p.transcript) 1 else 0) { n -> update { it.copy(transcript = n == 1) } } }
    MenuSection("Terminal height") { Choices(listOf("Full", "250", "350", "500", "700"), listOf(0,250,350,500,700).indexOf(p.terminalHeight)) { n -> update { it.copy(terminalHeight = listOf(0,250,350,500,700)[n]) } } }
    MenuSection("Widgets") { Choices(listOf("Expanded", "Collapsed"), if (p.widgets) 0 else 1) { n -> update { it.copy(widgets = n == 0) } } }
    MenuSection("Visible sections") {
        listOf("recap" to "Recap", "prs" to "Pull requests", "commits" to "Commits", "git" to "Git status", "changes" to "Changes").forEach { (key, label) -> CheckRow(label, p.visible(key)) { update { it.toggle(key) } } }
    }
    MenuSection("Columns") { Choices(listOf("1", "2", "3", "4", "Fit"), if (p.columns == 0) 4 else p.columns - 1) { n -> update { it.copy(columns = if (n == 4) 0 else n + 1) } } }
    MenuSection("Grouping") { Choices(listOf("All", "By project"), if (p.grouping == "all") 0 else 1) { n -> update { it.copy(grouping = if (n == 0) "all" else "project") } } }
    if (p.grouping == "project") MenuSection("Project order") { Choices(listOf("Most recent", "Alphabetical"), if (p.order == "recent") 0 else 1) { n -> update { it.copy(order = if (n == 0) "recent" else "alpha") } } }
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
    MenuSection("Appearance") { TextButton({ navigate("Style") }, Modifier.fillMaxWidth()) { Text("Style", color = Cyan) } }
    MenuSection("Notifications") { TextButton({ navigate("Notifications") }, Modifier.fillMaxWidth()) { Text(Notifications.status(context), color = Cyan) } }
    MenuSection("Session list") {
        Text("Position", color = Muted, fontSize = 12.sp)
        Choices(listOf("Left", "Right"), if (p.sidebarRight) 1 else 0) { n -> model.style { it.copy(sidebarRight = n == 1) } }
        Spacer(Modifier.height(8.dp)); Text("Density", color = Muted, fontSize = 12.sp)
        Choices(listOf("Compact", "Comfortable"), if (p.density == "compact") 0 else 1) { n -> model.style { it.copy(density = if (n == 0) "compact" else "comfortable") } }
    }
    MenuSection("Visible stats") { listOf("sessionTime" to "Session time", "thinkingTime" to "Thinking time", "prompts" to "Prompt count", "promptRecency" to "Last prompt", "completions" to "Completion count", "completionRecency" to "Last completion", "tools" to "Tools", "compactions" to "Compactions").forEach { (key,label) -> CheckRow(label, p.visible(key)) { model.style { it.toggle(key) } } } }
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
@Composable private fun NotificationMenu(requestPermission: () -> Unit) {
    val context = LocalContext.current
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    var status by remember { mutableStateOf(Notifications.status(context)) }
    DisposableEffect(lifecycle, context) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) status = Notifications.status(context)
        }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer) }
    }
    MenuSection("Questions and permissions") {
        Text(status, color = Cyan, fontSize = 15.sp)
        TextButton(requestPermission) { Text("Enable notifications") }
        TextButton({ context.startActivity(Intent(Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName).putExtra(Settings.EXTRA_CHANNEL_ID, Notifications.CHANNEL)) }) { Text("Sound, vibration and lock screen") }
    }
}
