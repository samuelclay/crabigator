package com.crabigator.app

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.*
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.compositeOver
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import org.json.JSONObject

private val Accent = CrabColors.Cyan
private val TileFill = Color(0xFF151B22)
/** A selected tile or chip: the accent at 13% over the tile's own fill. */
private val Tint = Accent.copy(alpha = .13f).compositeOver(TileFill)
private val Screen = Color(0xFF0B0F14)

private val ROW_LABELS = listOf("recap" to "Recap", "sessionTime" to "Session time", "thinkingTime" to "Thinking time", "promptRecency" to "Last prompt", "completionRecency" to "Last completion", "tools" to "Tools", "compactions" to "Compactions", "context" to "Context", "git" to "Git status", "changes" to "Changes")
private val STAT_LABELS = listOf("sessionTime" to "Session time", "thinkingTime" to "Thinking time", "prompts" to "Prompts", "completions" to "Completions", "tools" to "Tools", "compactions" to "Compactions", "context" to "Context")
/** The drawer's widgets, with the colour each one's heading takes in the drawer drawing. */
private val DRAWER = listOf(Triple("recap", "Recap", 0xFF5F87FF), Triple("prs", "Pull requests", 0xFFAF87FF), Triple("commits", "Commits", 0xFFFF8700), Triple("git", "Git status", 0xFF5FFF5F), Triple("changes", "Changes", 0xFFFF87D7), Triple("flow", "Flow scene", 0xFFD7AF5F))

/** What the List or the PR board shows: which sessions, how they are grouped, what a row holds and how large it is drawn. */
@Composable internal fun BoardViewPanel(model: SessionModel, p: UiPreferences, list: Boolean) {
    val state by model.state.collectAsStateWithLifecycle()
    val update = model::style
    val computers = remember(state.sessions, p.boardComputer) { BoardFilters.computers(state.sessions, p.boardComputer) }
    ViewSection("Which sessions") {
        TimeRuler(p.boardAgeHours) { hours -> update { it.copy(boardAgeHours = hours) } }
        SwitchRow("Ended sessions", p.showEnded) { update { it.copy(showEnded = !it.showEnded) } }
        Segments(listOf("All computers") + computers, computers.indexOf(p.boardComputer) + 1, colors = listOf(null) + computers.map(CrabColors::computer), perRow = 3, fontSize = 13) { n -> update { it.copy(boardComputer = if (n == 0) "" else computers[n - 1]) } }
        Caption("The same three filters as the chips above. Change them in either place.")
    }
    ViewSection("Group by") {
        // The PR board's rows are already in time bands and repositories, so it offers no project choice.
        val current = if (list) p.groupBy else p.boardGroupBy
        // Four tiles share one row on the List, so they carry a name only. The PR board's two have room for a second line.
        TileRow(compact = list) {
            (if (list) GroupBy.entries else GroupBy.board).forEach { by -> Tile(by.label, by == current, by.detail.takeUnless { list }, { update { if (list) it.grouped(by) else it.boardGrouped(by) } }, compact = list) { GroupThumbnail(by, if (list) 4 else 6) } }
        }
        if (list && current.project) Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
            FieldLabel("Sort projects")
            Segments(listOf("Most recent first", "A to Z"), if (p.order == "alpha") 1 else 0, Modifier.weight(1f), label = "Sort projects") { n -> update { it.copy(order = if (n == 1) "alpha" else "recent") } }
        }
    }
    if (!list) ViewSection("Each block starts with") {
        TileRow {
            Tile("The session", p.boardGrouping != "prs", "Its pull requests sit under it", { update { it.copy(boardGrouping = "sessions") } }) { LeadThumbnail(pullRequest = false) }
            Tile("The pull request", p.boardGrouping == "prs", "Its sessions sit under it", { update { it.copy(boardGrouping = "prs") } }) { LeadThumbnail(pullRequest = true) }
        }
    }
    ViewSection(if (list) "A List row shows" else "A PR board row shows") {
        SampleRow(list, p)
        ToggleChips(ROW_LABELS, if (list) p.listRowItems else p.boardRowItems) { key -> update { if (list) it.toggleListRow(key) else it.toggleBoardRow(key) } }
        Caption(if (list) "The PR board keeps its own set." else "The List keeps its own set.")
    }
    ViewSection("Size and spacing") {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            TextSizeStepper(p.boardFontSize, 10..24) { size -> update { it.copy(boardFontSize = size) } }
            val densities = listOf("compact", "comfortable", "relaxed")
            Segments(listOf("Tight", "Normal", "Relaxed"), densities.indexOf(p.density).coerceAtLeast(0), Modifier.weight(1f), label = "Row spacing") { n -> update { it.copy(density = densities[n]) } }
        }
        if (list) {
            FieldLabel("Columns")
            ColumnsPicker(p.columns) { n -> update { it.copy(columns = n) } }
        }
        if (LocalConfiguration.current.smallestScreenWidthDp >= 600) {
            FieldLabel("Session list side")
            TileRow {
                Tile("Left", !p.sidebarRight, onClick = { update { it.copy(sidebarRight = false) } }) { SideThumbnail(right = false) }
                Tile("Right", p.sidebarRight, onClick = { update { it.copy(sidebarRight = true) } }) { SideThumbnail(right = true) }
            }
        }
    }
}

/** What the session screen shows: its terminal text, the drawer under the terminal and the stats strip. */
@Composable internal fun SessionViewPanel(p: UiPreferences, update: ((UiPreferences) -> UiPreferences) -> Unit, transcripts: () -> Unit) {
    ViewSection("Terminal text") {
        Column(Modifier.fillMaxWidth().background(Screen, RoundedCornerShape(10.dp)).padding(horizontal = 12.dp, vertical = 10.dp).clearAndSetSemantics { contentDescription = "Sample terminal text" }) {
            // Two lines at the chosen size and line spacing, as the terminal draws them.
            listOf(buildAnnotatedString { withStyle(SpanStyle(color = Mint)) { append("●") }; append(" Read SettingsMenu.swift") }, buildAnnotatedString { append("Mapped every setting on the screen.") }).forEach { line ->
                Text(line, color = SecondaryText, fontFamily = FontFamily.Monospace, fontSize = p.fontSize.sp, lineHeight = (p.fontSize * p.lineSpacing / 100f).sp, maxLines = 1, softWrap = false, overflow = TextOverflow.Clip)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            TextSizeStepper(p.fontSize, 9..24) { size -> update { it.copy(fontSize = size) } }
            Segments(listOf("Tight", "Normal", "Relaxed"), listOf(120, 145, 170).indexOf(p.lineSpacing), Modifier.weight(1f), label = "Line spacing") { n -> update { it.copy(lineSpacing = listOf(120, 145, 170)[n]) } }
        }
    }
    ViewSection("Long lines") {
        TileRow {
            Tile("Wrap", p.wrap, "Continue on the next line", { update { it.copy(wrap = true) } }) { WrapThumbnail(wrap = true) }
            Tile("Scroll sideways", !p.wrap, "Keep each line whole", { update { it.copy(wrap = false) } }) { WrapThumbnail(wrap = false) }
        }
    }
    ViewSection("Terminal height") {
        val heights = listOf(0, 250, 350, 500, 700)
        Segments(listOf("Fill", "Short", "Medium", "Tall", "Tallest"), heights.indexOf(p.terminalHeight)) { n -> update { it.copy(terminalHeight = heights[n]) } }
        Row(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.Top) {
            MiniSessionScreen(p)
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                FieldLabel("Details drawer under the terminal")
                Segments(listOf("Open", "Tucked away"), if (p.widgets) 0 else 1, fontSize = 13, label = "Details drawer under the terminal") { n -> update { it.copy(widgets = n == 0) } }
                Caption("The small screen redraws as you choose a height and what the drawer holds.")
            }
        }
    }
    ViewSection("In the drawer") {
        ToggleChips(DRAWER.map { it.first to it.second }, DRAWER.map { it.first }.filter(p::visible).toSet(), DRAWER.associate { it.first to Color(it.third) }) { key -> update { it.toggle(key) } }
    }
    ViewSection("The stats strip shows") {
        val sample = remember { SampleRows.session(System.currentTimeMillis() / 1000, board = false) }
        Box(Modifier.fillMaxWidth().background(Ink, RoundedCornerShape(10.dp)).horizontalScroll(rememberScrollState()).clearAndSetSemantics { contentDescription = "Sample stats strip" }) {
            ModuleStatsRow(sample, JSONObject(), p, System.currentTimeMillis() / 1000, emptySet())
        }
        ToggleChips(STAT_LABELS, p.sessionStatItems) { key -> update { it.toggleSessionStat(key) } }
    }
    Column {
        Box(Modifier.fillMaxWidth().height(1.dp).background(Hairline))
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).clickable(role = Role.Button, onClick = transcripts), verticalAlignment = Alignment.CenterVertically) {
            Text("Voice transcripts for this session", Modifier.weight(1f), color = PrimaryText, fontSize = 15.sp)
            Icon(painterResource(R.drawable.ic_chevron_down), null, tint = MutedText, modifier = Modifier.size(16.dp).rotate(-90f))
        }
    }
}

/** The session a sample row or sample stats strip is drawn from. Every item has a value, so each chip shows what it adds. */
private object SampleRows {
    fun session(now: Long, board: Boolean) = Session("sample", if (board) "Live updates for mobile" else "Redesign the settings screen", "example/app", "ClayMac Studio", "complete", true,
        if (board) "◆" else "◈", if (board) "#af87ff" else "#FFAA77", "Mapped every setting and drew four ways to arrange them.", "claude", if (board) "mobile-realtime" else "main", if (board) "#2A2440" else "#352B25",
        SessionStats(startedAt = now - 8040, workSeconds = 8040, thinkingSeconds = 2280, prompts = 14, completions = 13, promptAt = now - if (board) 1500 else 180, completionAt = now - if (board) 1440 else 60,
            tools = 212, compactions = 3, context = ContextUsage(186_000, 1_000_000)), uncommitted = 3, additions = 212, deletions = 48)
    fun board(now: Long): BoardSession = session(now, board = true).let { session ->
        BoardSession(session, listOf(PullRequest("example/app#2217", "Live updates and a new stories pill", "example/app", 2217, "OPEN", "●1 CI", "", listOf(session), branch = "mobile-realtime",
            additions = 9036, deletions = 117, mergeable = "MERGEABLE", primary = true, checksPending = 1)), now)
    }
}

/** The real row view, drawn from a sample session at the chosen text size and spacing. It does not open anything. */
@Composable private fun SampleRow(list: Boolean, p: UiPreferences) {
    val clock = remember { mutableLongStateOf(System.currentTimeMillis() / 1000) }
    val density = LocalDensity.current
    Box(Modifier.fillMaxWidth().background(Ink, RoundedCornerShape(12.dp)).clearAndSetSemantics { contentDescription = "Sample row" }) {
        CompositionLocalProvider(LocalDensity provides Density(density.density, density.fontScale * p.boardFontSize / 14f), LocalBoardSpacing provides animatedBoardSpacing(p.density)) {
            Box(Modifier.padding(if (list) 8.dp else 4.dp)) {
                if (list) SessionCard(remember { SampleRows.session(clock.longValue, board = false) }, false, clock, p, showMachine = !p.groupByComputer) {}
                else BoardSessionBlock(remember { SampleRows.board(clock.longValue) }, false, false, clock, p.boardRowItems, showMachine = !p.groupByComputer) {}
            }
        }
        Box(Modifier.matchParentSize().pointerInput(Unit) { detectTapGestures {} })
    }
}

@Composable private fun ViewSection(title: String, content: @Composable ColumnScope.() -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(title, color = PrimaryText, fontSize = 15.sp, fontWeight = FontWeight.SemiBold, modifier = Modifier.semantics { heading() })
        content()
    }
}
@Composable private fun Caption(text: String) { Text(text, color = MutedText, fontSize = 12.sp, lineHeight = 16.sp) }
@Composable private fun FieldLabel(text: String) { Text(text, color = SecondaryText, fontSize = 13.sp) }

/** Seven windows from the last hour to every age. Every bar up to the chosen one is lit in the board's time-band colours. */
@Composable private fun TimeRuler(hours: Int, choose: (Int) -> Unit) {
    val selected = BoardFilters.hours.indexOf(hours).coerceAtLeast(0)
    Row(Modifier.fillMaxWidth().selectableGroup(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        BoardFilters.hours.forEachIndexed { index, value ->
            val on = index == selected
            Column(Modifier.weight(1f).heightIn(min = 52.dp).clip(RoundedCornerShape(9.dp)).border(2.dp, if (on) Accent else Color.Transparent, RoundedCornerShape(9.dp))
                .selectable(on, role = Role.RadioButton) { choose(value) }.padding(horizontal = 4.dp, vertical = 7.dp), verticalArrangement = Arrangement.spacedBy(7.dp)) {
                Box(Modifier.fillMaxWidth().height(8.dp).background(if (index <= selected) BoardFilters.color(value) else Color(0xFF2A323D), RoundedCornerShape(4.dp)))
                Text(BoardFilters.short(value), color = if (on) PrimaryText else MutedText, fontSize = 12.sp, fontWeight = if (on) FontWeight.Bold else FontWeight.Medium, textAlign = TextAlign.Center, maxLines = 1, softWrap = false, modifier = Modifier.fillMaxWidth())
            }
        }
    }
}

@Composable private fun SwitchRow(label: String, on: Boolean, toggle: () -> Unit) {
    val side by animateFloatAsState(if (on) 1f else 0f, tween(160), label = "Switch")
    Row(Modifier.fillMaxWidth().heightIn(min = 44.dp).toggleable(on, role = Role.Switch) { toggle() }, verticalAlignment = Alignment.CenterVertically) {
        Text(label, Modifier.weight(1f), color = PrimaryText, fontSize = 15.sp)
        Box(Modifier.size(52.dp, 30.dp).background(lerp(Color(0xFF3A4450), Accent, side), RoundedCornerShape(15.dp)).padding(3.dp)) {
            Box(Modifier.offset { IntOffset((22.dp * side).roundToPx(), 0) }.size(24.dp).background(Color.White, CircleShape))
        }
    }
}

/** One choice of several. The chosen segment is filled with the accent and takes dark text. */
@Composable private fun Segments(labels: List<String>, selected: Int, modifier: Modifier = Modifier, colors: List<Color?> = emptyList(), perRow: Int = labels.size, fontSize: Int = 12, label: String? = null, choose: (Int) -> Unit) {
    Column(modifier.background(Ink, RoundedCornerShape(10.dp)).padding(3.dp).selectableGroup().then(if (label != null) Modifier.semantics { contentDescription = label } else Modifier), verticalArrangement = Arrangement.spacedBy(3.dp)) {
        labels.indices.chunked(perRow.coerceAtLeast(1)).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                row.forEach { index ->
                    val on = index == selected
                    Box(Modifier.weight(1f).heightIn(min = 44.dp).clip(RoundedCornerShape(7.dp)).background(if (on) Accent else Color.Transparent)
                        .selectable(on, role = Role.RadioButton) { choose(index) }.padding(horizontal = 4.dp), contentAlignment = Alignment.Center) {
                        Text(labels[index], color = if (on) Ink else colors.getOrNull(index) ?: SecondaryText, fontSize = fontSize.sp, fontWeight = if (on) FontWeight.Bold else FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center)
                    }
                }
                // A short last row keeps its segments as wide as those above it.
                repeat(perRow.coerceAtMost(labels.size) - row.size) { Spacer(Modifier.weight(1f)) }
            }
        }
    }
}

@Composable private fun TileRow(compact: Boolean = false, content: @Composable RowScope.() -> Unit) {
    Row(Modifier.fillMaxWidth().height(IntrinsicSize.Max).selectableGroup(), horizontalArrangement = Arrangement.spacedBy(if (compact) 6.dp else 10.dp), content = content)
}
/** A drawn choice: its thumbnail, its name and, where there is room, a second line that says what it does. Compact tiles sit four to a row. */
@Composable private fun RowScope.Tile(label: String, selected: Boolean, detail: String? = null, onClick: () -> Unit, compact: Boolean = false, thumbnail: @Composable () -> Unit) {
    val shape = RoundedCornerShape(if (compact) 10.dp else 12.dp)
    Column(Modifier.weight(1f).fillMaxHeight().heightIn(min = 44.dp).clip(shape).background(if (selected) Tint else TileFill).border(2.dp, if (selected) Accent else Hairline, shape)
        .selectable(selected, role = Role.RadioButton, onClick = onClick).padding(if (compact) 7.dp else 8.dp), verticalArrangement = Arrangement.spacedBy(if (compact) 5.dp else 6.dp)) {
        thumbnail()
        Text(label, color = PrimaryText, fontSize = if (compact) 12.sp else 13.sp, lineHeight = if (compact) 15.sp else 17.sp, fontWeight = FontWeight.SemiBold, textAlign = if (compact) TextAlign.Center else TextAlign.Start, modifier = Modifier.fillMaxWidth())
        if (detail != null) Text(detail, color = MutedText, fontSize = 12.sp, lineHeight = 16.sp)
    }
}

/** A row of switches, one per item. Each has a check box, filled when the item shows. */
@Composable private fun ToggleChips(items: List<Pair<String, String>>, on: Set<String>, tones: Map<String, Color> = emptyMap(), toggle: (String) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        items.forEach { (key, label) ->
            val checked = key in on
            Row(Modifier.heightIn(min = 44.dp).clip(RoundedCornerShape(10.dp)).background(if (checked) Tint else Color.Transparent).border(2.dp, if (checked) Accent else Hairline, RoundedCornerShape(10.dp))
                .toggleable(checked, role = Role.Checkbox) { toggle(key) }.padding(horizontal = 11.dp), horizontalArrangement = Arrangement.spacedBy(7.dp), verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(14.dp).background(if (checked) Accent else Color.Transparent, RoundedCornerShape(4.dp)).border(2.dp, if (checked) Accent else Color(0xFF6B7684), RoundedCornerShape(4.dp)))
                tones[key]?.let { Box(Modifier.size(8.dp).background(it, CircleShape)) }
                Text(label, color = if (checked) PrimaryText else MutedText, fontSize = 13.sp, maxLines = 1)
            }
        }
    }
}

@Composable private fun TextSizeStepper(size: Int, range: IntRange, change: (Int) -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        StepButton(13, "Smaller text", size > range.first) { change(size - 1) }
        Text("$size pt", color = Accent, fontSize = 14.sp, fontWeight = FontWeight.SemiBold, textAlign = TextAlign.Center, maxLines = 1, softWrap = false, modifier = Modifier.widthIn(min = 44.dp).semantics { contentDescription = "Text size $size points" })
        StepButton(19, "Larger text", size < range.last) { change(size + 1) }
    }
}
@Composable private fun StepButton(letterSize: Int, label: String, enabled: Boolean, onClick: () -> Unit) {
    Box(Modifier.size(44.dp).clip(RoundedCornerShape(10.dp)).background(TileFill).border(1.dp, Hairline, RoundedCornerShape(10.dp))
        .clickable(enabled, role = Role.Button, onClick = onClick).clearAndSetSemantics { contentDescription = label }, contentAlignment = Alignment.Center) {
        Text("A", color = if (enabled) PrimaryText else MutedText.copy(alpha = .5f), fontSize = letterSize.sp, fontWeight = FontWeight.SemiBold)
    }
}

/** Auto fits as many columns as the width allows, so its drawing is an outline. */
@Composable private fun ColumnsPicker(columns: Int, choose: (Int) -> Unit) {
    Row(Modifier.fillMaxWidth().selectableGroup(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        (0..4).forEach { value ->
            val on = columns == value
            Column(Modifier.weight(1f).heightIn(min = 44.dp).clip(RoundedCornerShape(10.dp)).background(if (on) Tint else TileFill).border(2.dp, if (on) Accent else Hairline, RoundedCornerShape(10.dp))
                .selectable(on, role = Role.RadioButton) { choose(value) }.padding(horizontal = 6.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Canvas(Modifier.fillMaxWidth().height(34.dp)) {
                    val count = if (value == 0) 3 else value
                    val gap = 2.dp.toPx(); val line = 1.dp.toPx()
                    val cell = Size((size.width - gap * (count - 1)) / count, (size.height - gap) / 2)
                    for (row in 0..1) for (column in 0 until count) {
                        val at = Offset(column * (cell.width + gap), row * (cell.height + gap))
                        if (value != 0) drawRoundRect(Color(0xFF4A5563), at, cell, CornerRadius(2.dp.toPx()))
                        drawRoundRect(Color(0xFF6B7684), at + Offset(line / 2, line / 2), Size(cell.width - line, cell.height - line), CornerRadius(2.dp.toPx()),
                            style = Stroke(line, pathEffect = if (value == 0) PathEffect.dashPathEffect(floatArrayOf(3.dp.toPx(), 2.dp.toPx())) else null))
                    }
                }
                Text(if (value == 0) "Auto" else "$value", color = PrimaryText, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, textAlign = TextAlign.Center, modifier = Modifier.fillMaxWidth())
            }
        }
    }
}

/** A small session screen: header, terminal, drawer, stats strip and composer. It redraws for the chosen height and drawer. */
@Composable private fun MiniSessionScreen(p: UiPreferences) {
    val terminal = mapOf(250 to 26.dp, 350 to 38.dp, 500 to 54.dp, 700 to 74.dp)[p.terminalHeight]
    val lines = listOf(.80f to 0xFF6B7684, .55f to 0xFF5FFF5F, .90f to 0xFF6B7684, .42f to 0xFF6B7684, .70f to 0xFFD7AF5F, .86f to 0xFF6B7684, .36f to 0xFF6B7684, .66f to 0xFF5FFF5F, .80f to 0xFF6B7684, .52f to 0xFF6B7684, .74f to 0xFF6B7684, .60f to 0xFF6B7684)
    Column(Modifier.size(104.dp, 164.dp).clip(RoundedCornerShape(12.dp)).background(Ink).border(1.dp, Color(0xFF3A4450), RoundedCornerShape(12.dp)).clearAndSetSemantics {}) {
        Box(Modifier.fillMaxWidth().height(14.dp).background(Ink))
        // As on the session screen, the terminal keeps its height inside whatever room the drawer leaves.
        Box(Modifier.fillMaxWidth().weight(1f).clipToBounds()) {
            Column(Modifier.fillMaxWidth().then(if (terminal == null) Modifier.fillMaxHeight() else Modifier.height(terminal)).background(Screen).clipToBounds().padding(horizontal = 7.dp, vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                lines.forEach { (width, color) -> Box(Modifier.fillMaxWidth(width).height(2.dp).background(Color(color))) }
            }
        }
        if (p.widgets) {
            Box(Modifier.fillMaxWidth().height(1.dp).background(Hairline))
            Column(Modifier.fillMaxWidth().background(TileFill).padding(horizontal = 7.dp, vertical = 5.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                val widths = listOf(.60f, .76f, .52f, .48f, .66f, .40f)
                DRAWER.forEachIndexed { index, (key, _, tone) -> if (p.visible(key)) Box(Modifier.fillMaxWidth(widths[index]).height(3.dp).background(Color(tone), RoundedCornerShape(2.dp))) }
            }
        }
        Box(Modifier.fillMaxWidth().height(1.dp).background(Hairline))
        Box(Modifier.fillMaxWidth().height(12.dp).background(Panel))
        Box(Modifier.fillMaxWidth().height(1.dp).background(Hairline))
        Box(Modifier.fillMaxWidth().height(16.dp).background(Ink))
    }
}

// The thumbnails below are the design's drawings, in the design's own units: 120 wide, and 80, 60 or 50 high.
private const val ROW = 0xFF2A323D
private const val TITLE = 0xFFC4CDD8
private const val COMPUTER_A = 0xFF5F87FF
private const val COMPUTER_B = 0xFFFF8700
private const val PROJECT = 0xFFFFD700
private const val PULL_REQUEST = 0xFFAF87FF
private const val DIM = 0xFF6B7684
private const val PEACH = 0xFFFFAA77
private const val GREEN = 0xFF7EE787
private const val GOLD = 0xFFFFD75F
private const val PASSED = 0xFF5FFF5F
private const val FAILED = 0xFFFF5F5F

@Composable private fun Thumbnail(height: Float, background: Color = Color(0xFF12171D), corner: Int = 6, draw: DrawScope.() -> Unit) {
    Canvas(Modifier.fillMaxWidth().aspectRatio(120f / height).clip(RoundedCornerShape(corner.dp)).background(background)) {
        scale(size.width / 120f, size.width / 120f, Offset.Zero) { draw() }
    }
}
private fun DrawScope.bar(x: Float, y: Float, width: Float, height: Float, radius: Float, color: Long) = drawRoundRect(Color(color), Offset(x, y), Size(width, height), CornerRadius(radius))
private fun DrawScope.outline(x: Float, y: Float, width: Float, height: Float, radius: Float, color: Long, stroke: Float) = drawRoundRect(Color(color), Offset(x, y), Size(width, height), CornerRadius(radius), style = Stroke(stroke))
/** A session row: its card, its mark and its title, then the computer's name when the row carries one. */
private fun DrawScope.sessionRow(x: Float, y: Float, height: Float, mark: Long, title: Float, computer: Pair<Float, Long>? = null) {
    bar(x, y, 116f - x, height, 3f, ROW)
    bar(x + 4f, y + (if (computer != null) 4f else (height - 6f) / 2), 7f, 6f, 1.5f, mark)
    bar(x + 15f, y + (if (computer != null) 3.5f else (height - 2.6f) / 2), title, 2.6f, 1.3f, TITLE)
    if (computer != null) bar(x + 15f, y + 8f, computer.first, 2.2f, 1.1f, computer.second)
}
private fun DrawScope.computerHeading(y: Float, width: Float, color: Long) { outline(5f, y, 8f, 5.6f, 1f, color, 1.3f); bar(17f, y + 1.4f, width, 2.8f, 1.4f, color) }

@Composable private fun GroupThumbnail(by: GroupBy, corner: Int = 6) = Thumbnail(80f, corner = corner) {
    when (by) {
        GroupBy.OneList -> {
            sessionRow(4f, 4f, 14f, PEACH, 52f, 22f to COMPUTER_A); sessionRow(4f, 22f, 14f, GREEN, 40f, 18f to COMPUTER_B)
            sessionRow(4f, 40f, 14f, PULL_REQUEST, 58f, 22f to COMPUTER_A); sessionRow(4f, 58f, 14f, GOLD, 46f, 18f to COMPUTER_B)
        }
        GroupBy.Computer -> {
            computerHeading(4f, 34f, COMPUTER_A); sessionRow(4f, 13f, 11f, PEACH, 52f); sessionRow(4f, 26f, 11f, PULL_REQUEST, 58f)
            computerHeading(42.6f, 28f, COMPUTER_B); sessionRow(4f, 52f, 11f, GREEN, 40f); sessionRow(4f, 65f, 11f, GOLD, 46f)
        }
        GroupBy.Project -> {
            bar(5f, 5f, 30f, 2.8f, 1.4f, PROJECT); projectRow(12f, PEACH, 52f, 22f to COMPUTER_A); projectRow(26f, GREEN, 40f, 18f to COMPUTER_B)
            bar(5f, 44f, 24f, 2.8f, 1.4f, PROJECT); projectRow(51f, PULL_REQUEST, 58f, 22f to COMPUTER_A); projectRow(65f, GOLD, 46f, 18f to COMPUTER_B)
        }
        GroupBy.ComputerThenProject -> {
            computerHeading(3f, 34f, COMPUTER_A)
            bar(13f, 12.5f, 26f, 2.4f, 1.2f, PROJECT); sessionRow(12f, 17f, 10f, PEACH, 48f)
            bar(13f, 31.5f, 20f, 2.4f, 1.2f, PROJECT); sessionRow(12f, 36f, 10f, PULL_REQUEST, 56f)
            computerHeading(50.6f, 28f, COMPUTER_B)
            bar(13f, 60.5f, 26f, 2.4f, 1.2f, PROJECT); sessionRow(12f, 65f, 10f, GREEN, 40f)
        }
    }
}
/** A 12-high row under a project heading: the mark and the two lines sit a little lower than in the plain list. */
private fun DrawScope.projectRow(y: Float, mark: Long, title: Float, computer: Pair<Float, Long>) {
    bar(4f, y, 112f, 12f, 3f, ROW); bar(8f, y + 3f, 7f, 6f, 1.5f, mark); bar(19f, y + 2.6f, title, 2.6f, 1.3f, TITLE); bar(19f, y + 7f, computer.first, 2.2f, 1.1f, computer.second)
}

@Composable private fun LeadThumbnail(pullRequest: Boolean) = Thumbnail(50f) {
    if (pullRequest) {
        bar(6f, 8f, 74f, 3f, 1.5f, PULL_REQUEST); bar(98f, 7f, 15f, 5f, 2.5f, PASSED)
        bar(14f, 17f, 9f, 8f, 2f, PEACH); bar(28f, 19.5f, 50f, 2.6f, 1.3f, TITLE)
        bar(14f, 30f, 9f, 8f, 2f, GREEN); bar(28f, 32.5f, 40f, 2.6f, 1.3f, TITLE)
    } else {
        bar(6f, 6f, 9f, 8f, 2f, PEACH); bar(20f, 8.5f, 60f, 3f, 1.5f, TITLE)
        bar(20f, 19f, 66f, 2.6f, 1.3f, PULL_REQUEST); bar(98f, 17.8f, 15f, 5f, 2.5f, PASSED)
        bar(20f, 28f, 52f, 2.6f, 1.3f, PULL_REQUEST); bar(98f, 26.8f, 15f, 5f, 2.5f, FAILED)
        bar(20f, 37f, 60f, 2.6f, 1.3f, PULL_REQUEST); bar(98f, 35.8f, 15f, 5f, 2.5f, PASSED)
    }
}

@Composable private fun WrapThumbnail(wrap: Boolean) = Thumbnail(60f, Screen) {
    val arrow = Stroke(1.4f, cap = StrokeCap.Round, join = StrokeJoin.Round)
    if (wrap) {
        bar(8f, 9f, 104f, 3f, 1.5f, TITLE); bar(8f, 17f, 46f, 3f, 1.5f, TITLE)
        drawPath(Path().apply { moveTo(112f, 10.5f); lineTo(112f, 15.5f); arcTo(Rect(106f, 12.5f, 112f, 18.5f), 0f, 90f, false); lineTo(61f, 18.5f) }, Accent, style = arrow)
        drawPath(Path().apply { moveTo(64f, 15.5f); lineTo(60.5f, 18.5f); lineTo(64f, 21.5f) }, Accent, style = arrow)
        bar(8f, 31f, 70f, 3f, 1.5f, DIM); bar(8f, 42f, 104f, 3f, 1.5f, DIM); bar(8f, 50f, 30f, 3f, 1.5f, DIM)
    } else {
        // The two long lines run off the right edge.
        bar(8f, 9f, 112f, 3f, 0f, TITLE); bar(8f, 20f, 70f, 3f, 1.5f, DIM); bar(8f, 31f, 112f, 3f, 0f, DIM); bar(8f, 42f, 52f, 3f, 1.5f, DIM)
        drawPath(Path().apply { moveTo(104f, 21.5f); lineTo(112f, 21.5f); moveTo(109f, 18.5f); lineTo(112f, 21.5f); lineTo(109f, 24.5f) }, Accent, style = arrow)
        bar(8f, 52f, 104f, 3f, 1.5f, 0xFF232B35); drawRoundRect(Accent, Offset(8f, 52f), Size(44f, 3f), CornerRadius(1.5f))
    }
}

@Composable private fun SideThumbnail(right: Boolean) = Thumbnail(60f) {
    val list = if (right) 77.4f else 6.6f
    val detail = if (right) 14f else 50f
    bar(6f, 6f, 108f, 48f, 5f, 0xFF0B0F14); outline(6f, 6f, 108f, 48f, 5f, 0xFF4A5563, 1.2f)
    bar(list, 6.6f, 36f, 46.8f, 4.4f, ROW)
    bar(list + 4.4f, 13f, 26f, 6f, 2f, 0xFF4A5563); bar(list + 4.4f, 23f, 26f, 6f, 2f, 0xFFD7AF5F); bar(list + 4.4f, 33f, 26f, 6f, 2f, 0xFF4A5563)
    bar(detail, 14f, 52f, 2.6f, 1.3f, DIM); bar(detail, 21f, 36f, 2.6f, 1.3f, DIM); bar(detail, 28f, 46f, 2.6f, 1.3f, PASSED); bar(detail, 35f, 30f, 2.6f, 1.3f, DIM)
}
