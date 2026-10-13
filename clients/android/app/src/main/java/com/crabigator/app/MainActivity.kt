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
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.grid.items as gridItems
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.compositeOver
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.flow
import org.json.JSONObject

internal val Ink = Color(0xFF101419)
internal val Panel = Color(0xFF1B2129)
internal val Muted = CrabColors.Gray
internal val Peach = CrabColors.Orange
internal val Mint = CrabColors.Green
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
        if (savedInstanceState == null) handleIntent(intent)
    }
    override fun onStart() { super.onStart(); model.start() }
    override fun onStop() { model.stop(); super.onStop() }
    override fun onNewIntent(intent: Intent) { super.onNewIntent(intent); setIntent(intent); handleIntent(intent) }
    private fun handleIntent(intent: Intent) { intent.getStringExtra("session_id")?.let(model::open) }
}

@Composable internal fun App(model: SessionModel, notifications: () -> Unit) {
    val s by model.state.collectAsStateWithLifecycle()
    val navigation by model.navigation.collectAsStateWithLifecycle()
    val listMode = navigation.listMode
    val browserExpanded = navigation.expanded
    var handledNavigation by rememberSaveable { mutableLongStateOf(0L) }
    var menu by remember { mutableStateOf<String?>(null) }
    var menuAnchor by remember { mutableStateOf(Rect.Zero) }
    var boardSearchRequest by remember { mutableIntStateOf(0) }
    var closingSession by remember(s.selected?.id) { mutableStateOf<String?>(null) }
    LaunchedEffect(s.navigationId) {
        if (handledNavigation != s.navigationId) {
            handledNavigation = s.navigationId
            if (s.selected != null) { closingSession = null }
        }
    }
    val closing = s.selected != null && closingSession == s.selected?.id
    val closeSession: () -> Unit = { closingSession = s.selected?.id }
    val preferences by model.preferences.collectAsStateWithLifecycle()
    if (!s.paired) { Pairing(s, model); return }
    BackHandler(s.selected != null, onBack = closeSession)
    Box(Modifier.fillMaxSize()) {
    Column(Modifier.fillMaxSize().safeDrawingPadding().then(if (menu != null) Modifier.clearAndSetSemantics {} else Modifier)) {
        if (s.error != null) Surface(color = Color(0xFF4D2C2C)) { Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) { Text(s.error!!, Modifier.weight(1f), fontSize = 13.sp); TextButton(onClick = model::dismissError) { Text("Dismiss") } } }
        BoxWithConstraints(Modifier.weight(1f)) {
            val wide = maxWidth >= 720.dp
            val detailWidth = maxWidth - 361.dp
            val fullBoard = wide && browserExpanded
            val focusManager = LocalFocusManager.current
            val keyboard = LocalSoftwareKeyboardController.current
            LaunchedEffect(fullBoard) { if (fullBoard) { focusManager.clearFocus(); keyboard?.hide() } }
            val sidebarWidth by animateDpAsState(if (fullBoard) maxWidth else 360.dp, tween(360, easing = FastOutSlowInEasing), label = "Board width")
            val board: @Composable () -> Unit = {
                Board(s.sessions, s.prs, s.selected?.id, s.rosterStatus, model,
                    listMode, { next ->
                        if (listMode == next) { if (wide) model.navigate { it.copy(expanded = !it.expanded) } }
                        else { model.navigate { it.copy(listMode = next) } }
                    }, if (wide) fullBoard else null, { model.navigate { it.copy(expanded = false) } }, boardSearchRequest) { menuAnchor = it; menu = "Settings" }
            }
            val entrance = remember(s.selected != null) { Animatable(1f) }
            var swipe by remember(s.selected?.id) { mutableStateOf<Float?>(null) }
            val scope = rememberCoroutineScope()
            var settleJob by remember { mutableStateOf<Job?>(null) }
            val currentlyClosing by rememberUpdatedState(closing)
            val density = LocalDensity.current
            val travel = with(density) { (if (wide) detailWidth else maxWidth).toPx() }
            val progress = swipe ?: entrance.value
            LaunchedEffect(s.selected?.id, closing) {
                settleJob?.cancel()
                swipe = null
                entrance.animateTo(if (closing) 1f else 0f, tween(360, easing = FastOutSlowInEasing))
                if (closing && model.state.value.selected?.id == closingSession) {
                    closingSession = null
                    model.close()
                }
            }
            val swipeModifier = if (closing) Modifier else Modifier.sessionSwipe(s.selected?.id to travel,
                start = {
                    if (!currentlyClosing && model.state.value.selected?.id == s.selected?.id) {
                        settleJob?.cancel()
                        swipe = entrance.value
                        settleJob = scope.launch { entrance.stop() }
                    }
                },
                drag = { delta -> swipe?.let { swipe = (it + delta / travel).coerceIn(0f, 1f) } },
                finish = finish@{ velocity, cancelled ->
                    val fraction = swipe ?: return@finish
                    if (currentlyClosing || model.state.value.selected?.id != s.selected?.id) return@finish
                    val threshold = with(density) { 600.dp.toPx() }
                    val dismiss = !cancelled && (velocity > threshold || (fraction > .33f && velocity > -threshold))
                    settleJob?.cancel()
                    settleJob = scope.launch {
                        entrance.snapTo(fraction)
                        if (currentlyClosing || model.state.value.selected?.id != s.selected?.id) return@launch
                        swipe = null
                        if (dismiss) closeSession()
                        else entrance.animateTo(0f, tween(280, easing = FastOutSlowInEasing))
                    }
                })
            val detail: @Composable () -> Unit = {
                if (s.selected != null) Box(Modifier.fillMaxSize().clipToBounds()) {
                    Box(Modifier.fillMaxSize().graphicsLayer { translationX = size.width * progress }
                        .background(Ink).pointerInput(Unit) { detectTapGestures {} }) {
                        SessionPages(s, model, wide, closeSession, swipeModifier, visible = !fullBoard) { menuAnchor = it; menu = "Style" }
                    }
                }
            }
            if (wide) Box(Modifier.fillMaxSize().clipToBounds()) {
                Box(Modifier.align(if (preferences.sidebarRight) Alignment.CenterStart else Alignment.CenterEnd)
                    .width(detailWidth).fillMaxHeight()) {
                    if (s.selected == null && !fullBoard) Column(Modifier.align(Alignment.Center), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Icon(painterResource(R.drawable.ic_terminal), null, tint = Muted.copy(alpha = .4f), modifier = Modifier.size(40.dp))
                        Text("Select a session", color = Muted, fontSize = 14.sp)
                    }
                    Box(Modifier.fillMaxSize().then(if (fullBoard) Modifier.clearAndSetSemantics {} else Modifier)) { detail() }
                }
                Box(Modifier.align(if (preferences.sidebarRight) Alignment.CenterEnd else Alignment.CenterStart)
                    .width(sidebarWidth).fillMaxHeight().background(Ink)) { board() }
            } else Box(Modifier.fillMaxSize().clipToBounds()) {
                Box(Modifier.fillMaxSize().then(if (s.selected != null) Modifier.clearAndSetSemantics {} else Modifier)) {
                    board()
                }
                detail()
            }
        }
    }
    MenuPanel(menu, menuAnchor, model, notifications, { destination ->
        when (destination) {
            "Search" -> { menu = null; boardSearchRequest++ }
            "Toggle fullscreen" -> { menu = null; model.navigate { it.copy(expanded = !it.expanded) } }
            else -> menu = destination
        }
    }, { menu = null })
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

@Composable private fun Board(roster: List<Session>, pullRequests: List<PullRequest>, selectedId: String?, rosterStatus: RosterStatus, model: SessionModel,
    list: Boolean, mode: (Boolean) -> Unit, expanded: Boolean?, selected: () -> Unit, searchRequest: Int, settings: (Rect) -> Unit) {
    var query by rememberSaveable { mutableStateOf("") }; var searching by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(searchRequest) { if (searchRequest > 0) searching = true }
    val now = remember { flow { while (true) { emit(System.currentTimeMillis() / 1000); delay(1000) } } }
        .collectAsStateWithLifecycle(initialValue = System.currentTimeMillis() / 1000)
    var settingsAnchor by remember { mutableStateOf(Rect.Zero) }
    val prefs by model.preferences.collectAsStateWithLifecycle()
    val all = prefs.showEnded
    var machineOrder by rememberSaveable { mutableStateOf(emptyList<String>()) }
    var collapsedMachines by rememberSaveable { mutableStateOf(emptyList<String>()) }
    val gridState = rememberLazyGridState()
    val boardState = rememberLazyListState()
    val machines = remember(roster, machineOrder) { retainMachineOrder(machineOrder, roster) }
    SideEffect { if (machineOrder != machines) machineOrder = machines }
    val ordering = remember { RosterOrder() }
    val minute by remember { derivedStateOf { now.value / 60 } }
    val layout = remember(roster, pullRequests, machines, minute, prefs.grouping, prefs.order) {
        ordering.update(roster, pullRequests, machines, now.value, android.os.SystemClock.elapsedRealtime() / 1000, prefs.grouping, prefs.order)
    }
    val visible = remember(layout, all, query, prefs.boardAgeHours, prefs.boardComputer, prefs.boardGrouping, minute) { layout.filtered(all, query, prefs, now.value) }
    fun select(session: Session, order: List<Session>) {
        val direction = if (order.indexOfFirst { it.id == session.id } < order.indexOfFirst { it.id == selectedId }) -1 else 1
        model.select(session, direction)
        selected()
    }
    val density = LocalDensity.current
    val spacing = animatedBoardSpacing(prefs.density)
    val floating = expanded == true
    val headerSpace by animateDpAsState(if (floating && !searching) 0.dp else 60.dp, tween(360), label = "Board header space")
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val toolbarWidth by animateDpAsState(if (floating) 228.dp else maxWidth, tween(360), label = "Board toolbar width")
        Column(Modifier.fillMaxSize()) {
            Spacer(Modifier.height(headerSpace))
            if (searching) OutlinedTextField(query, { query = it }, placeholder = { Text("Search") }, singleLine = true, shape = RoundedCornerShape(12.dp), modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp))
            CompositionLocalProvider(LocalDensity provides Density(density.density, density.fontScale * prefs.boardFontSize / 14f), LocalBoardSpacing provides spacing, LocalFloatingBoardToolbar provides floating) {
                if (roster.isEmpty() && pullRequests.isEmpty()) Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                    RosterPlaceholder(rosterStatus, model::retryRoster) { EmptyPairing(model) }
                }
                else if (list) BoxWithConstraints(Modifier.weight(1f)) {
                    val count = if (prefs.columns == 0) (maxWidth.value / 320).toInt().coerceAtLeast(1) else prefs.columns.coerceAtMost((maxWidth.value / 280).toInt().coerceAtLeast(1))
                    val groups = visible.list
                    val order = remember(groups) { groups.flatMap { it.sessions } }
                    val counts = remember(groups) { groups.groupBy { it.machine }.mapValues { (_, sections) -> sections.sumOf { it.sessions.size } } }
                    LazyVerticalGrid(columns = GridCells.Fixed(count), state = gridState, contentPadding = PaddingValues(12.dp, 6.dp, 12.dp, 88.dp), verticalArrangement = Arrangement.spacedBy(spacing.gap(8)), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        groups.forEachIndexed { index, group ->
                            if (index == 0 || groups[index - 1].machine != group.machine) stickyHeader(key = "machine-${group.machine}", contentType = "machine") { headerIndex ->
                                MachineHeader(group.machine, counts.getValue(group.machine), group.machine in collapsedMachines, Modifier.animateItem(placementSpec = tween(320))) {
                                    if (group.machine !in collapsedMachines && gridState.firstVisibleItemIndex > headerIndex) gridState.requestScrollToItem(headerIndex)
                                    collapsedMachines = if (group.machine in collapsedMachines) collapsedMachines - group.machine else collapsedMachines + group.machine
                                }
                            }
                            if (group.machine in collapsedMachines) return@forEachIndexed
                            if (group.project.isNotEmpty()) item(key = "project-${group.machine.length}:${group.machine}${group.project}", contentType = "group", span = { GridItemSpan(maxLineSpan) }) {
                                Text(group.project, color = Muted, fontSize = 12.sp, modifier = Modifier.animateItem(placementSpec = tween(320), fadeInSpec = tween(220), fadeOutSpec = tween(180)).padding(vertical = spacing.gap(6)))
                            }
                            gridItems(group.sessions, key = { it.id }, contentType = { "session" }) { SessionCard(it, expanded != true && it.id == selectedId, now, prefs, showMachine = false, modifier = Modifier.animateItem(placementSpec = tween(320), fadeInSpec = tween(220), fadeOutSpec = tween(180))) { select(it, order) } }
                        }
                    }
                }
                else PrBoard(visible.board, visible.watched, now, query, selectedId.takeUnless { expanded == true }, collapsedMachines, boardState,
                    { machine -> collapsedMachines = if (machine in collapsedMachines) collapsedMachines - machine else collapsedMachines + machine },
                    ::select, prefs)
            }
        }
        Surface(modifier = Modifier.align(Alignment.TopCenter).width(toolbarWidth),
            color = if (floating) Panel.copy(alpha = .96f) else Ink,
            shape = RoundedCornerShape(if (floating) 28.dp else 0.dp),
            border = if (floating) BorderStroke(1.dp, Muted.copy(alpha = .2f)) else null,
            shadowElevation = if (floating) 8.dp else 0.dp) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = if (floating) 2.dp else 6.dp), horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                Image(painterResource(R.drawable.ic_crabigator), "Crabigator", Modifier.padding(8.dp).size(28.dp))
                Spacer(Modifier.weight(1f))
                Control(R.drawable.ic_search, "Search", selected = searching) { searching = !searching; if (!searching) query = "" }
                Control(R.drawable.ic_settings, "Settings", modifier = Modifier.onGloballyPositioned { settingsAnchor = it.boundsInRoot() }) { settings(settingsAnchor) }
            }
        }
        BoardModeFooter(list, mode, expanded, Modifier.align(Alignment.BottomCenter).padding(12.dp))
    }
}
@Composable internal fun MachineHeader(name: String, count: Int, collapsed: Boolean, modifier: Modifier = Modifier, countNoun: String = "session", toggle: () -> Unit) {
    val spacing = LocalBoardSpacing.current
    val floating = LocalFloatingBoardToolbar.current
    val angle by animateFloatAsState(if (collapsed) -90f else 0f, tween(240), label = "Machine folder")
    Surface(onClick = toggle, color = Ink, modifier = modifier.fillMaxWidth().semantics {
        heading()
        stateDescription = if (collapsed) "Collapsed" else "Expanded"
    }) {
        BoxWithConstraints {
            val nameWidth = if (floating) ((maxWidth - 260.dp) / 2 - 36.dp).coerceAtLeast(60.dp) else maxWidth
            Row(Modifier.heightIn(min = if (floating) 56.dp else 44.dp).padding(horizontal = 8.dp, vertical = spacing.gap(8)), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Icon(painterResource(if (countNoun == "session") R.drawable.ic_machine else R.drawable.ic_board), null, tint = Muted, modifier = Modifier.size(18.dp))
                Text(name, color = Muted, fontSize = 13.sp, fontWeight = FontWeight.Medium, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f).wrapContentWidth(Alignment.Start).widthIn(max = nameWidth))
                Text(if (count == 1) "1 $countNoun" else "$count ${countNoun}s", color = Muted, fontSize = 12.sp,
                    modifier = Modifier.background(Panel, RoundedCornerShape(6.dp)).padding(horizontal = 7.dp, vertical = 2.dp))
                Icon(painterResource(R.drawable.ic_chevron_down), null, tint = Muted, modifier = Modifier.size(18.dp).rotate(angle))
            }
        }
    }
}

@Composable internal fun Control(icon: Int, label: String, selected: Boolean = false, enabled: Boolean = true, modifier: Modifier = Modifier, onClick: () -> Unit) {
    IconButton(onClick = onClick, enabled = enabled, modifier = modifier.size(48.dp).background(if (selected) Peach.copy(alpha = .12f) else Color.Transparent, RoundedCornerShape(12.dp))) {
        Icon(painterResource(icon), contentDescription = label, tint = if (selected) Peach else Muted, modifier = Modifier.size(22.dp))
    }
}
@Composable private fun SessionCard(session: Session, selected: Boolean, now: State<Long>? = null, prefs: UiPreferences = UiPreferences(), showMachine: Boolean = true, modifier: Modifier = Modifier, open: () -> Unit) {
    val spacing = LocalBoardSpacing.current
    val selection by animateFloatAsState(if (selected) 1f else 0f, tween(if (selected) 120 else 350), label = "Session selection")
    Surface(onClick = open, modifier = modifier.semantics { this.selected = selected },
        color = lerp(Panel, Peach.copy(alpha = .13f).compositeOver(Ink), selection),
        shape = RoundedCornerShape(12.dp), border = BorderStroke(2.dp, Peach.copy(alpha = selection))) {
        Box(Modifier.padding(horizontal = 10.dp, vertical = spacing.gap(10))) {
            SessionRow(session, clickable = false, showMachine = showMachine, footer = {
                if (prefs.boardRecaps && session.recap.isNotBlank()) Text(session.recap, color = Muted, fontSize = 12.sp)
                if (now != null) SessionStatsFooter(session, now, prefs)
            }, open = open)
        }
    }
}
@Composable private fun SessionRow(session: Session, clickable: Boolean = true, showMachine: Boolean = true, footer: @Composable () -> Unit = {}, open: () -> Unit) {
    val spacing = LocalBoardSpacing.current
    val color = remember(session.color) { runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(Peach) }
    val background = remember(session.background) { runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel) }
    val title = remember(session.title, session.machine, showMachine) { buildAnnotatedString {
        append(session.title)
        if (showMachine && session.machine.isNotBlank()) withStyle(SpanStyle(color = Muted, fontSize = 11.sp, fontWeight = FontWeight.Normal)) {
            append("  ·\u00a0${session.machine}")
        }
    } }
    Row(Modifier.fillMaxWidth().then(if (clickable) Modifier.clickable(onClick = open) else Modifier).padding(vertical = spacing.gap(4)), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
        Surface(color = background, shape = RoundedCornerShape(8.dp), modifier = Modifier.width(58.dp)) { Text(session.glyph, color = color, fontSize = 16.sp, lineHeight = 20.sp, textAlign = TextAlign.Center, maxLines = 1, softWrap = false, modifier = Modifier.padding(vertical = 5.dp)) }
        SessionStatus(session)
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.gap(6))) {
            Text(title, color = CrabColors.Title, fontSize = 14.sp, lineHeight = spacing.lineHeight(19), fontWeight = FontWeight.Medium, maxLines = if (spacing.tight) 1 else Int.MAX_VALUE, overflow = TextOverflow.Ellipsis, modifier = Modifier.animateContentSize(tween(300), alignment = Alignment.TopStart))
            footer()
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
