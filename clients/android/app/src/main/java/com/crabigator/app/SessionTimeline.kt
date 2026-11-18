package com.crabigator.app

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

internal fun compactDuration(seconds: Long?): String = sessionDuration(seconds).substringBefore(' ')

/** A List row's lines under its title, recap and computer: timing, counts, git status and changes. [items] is the row's saved set. */
@Composable internal fun SessionStatsFooter(session: Session, clock: State<Long>, items: Set<String>) {
    val now = clock.value
    val stats = session.stats
    val duration = stats.duration(session.active, now)
    val prompt = if (stats.promptAt > 0) sessionAge(stats.promptAt, now) else "—"
    val completion = if (stats.completionAt > 0) sessionAge(stats.completionAt, now) else "—"
    val summary = buildList {
        if ("sessionTime" in items) add("Session time: ${sessionDuration(duration)}")
        if ("thinkingTime" in items) add("Thinking time: ${sessionDuration(stats.thinkingSeconds)}")
        if ("promptRecency" in items) add("Last prompt: $prompt")
        if ("completionRecency" in items) add("Last completion: $completion")
    }.joinToString("; ")
    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
        OneLineTimeline(summary, statusLabel = if (!session.active) "Ended" else if (session.state == "thinking") "Working" else session.state.replaceFirstChar { it.uppercase() }, prioritizeDuration = "promptRecency" !in items && "completionRecency" !in items, start = { compact ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if ("sessionTime" in items) Stat("◉", if (compact) compactDuration(duration) else sessionDuration(duration), "Session time: ${sessionDuration(duration)}", CrabColors.Blue)
                if ("thinkingTime" in items) Stat("◐", if (compact) compactDuration(stats.thinkingSeconds) else sessionDuration(stats.thinkingSeconds), "Thinking time: ${sessionDuration(stats.thinkingSeconds)}", Mint)
            }
        }, end = {
            BoardRecency(session, clock, "promptRecency" in items, "completionRecency" in items, showStatus = true)
        })
        StatLine(countStats(session, items))
        if (showsGit(session, items)) RowGit(session)
        if (showsChanges(session, items)) RowChanges(session)
    }
}

/** A PR board row's optional lines: times, counts, git status and changes. Its two recency columns already sit beside the title. */
@Composable internal fun BoardRowLines(session: Session, clock: State<Long>, items: Set<String>) {
    val times = listOfNotNull(
        RowStat("◉", "Session time", session.stats.duration(session.active, clock.value)?.let(::sessionDuration), CrabColors.Blue).takeIf { "sessionTime" in items },
        RowStat("◐", "Thinking time", session.stats.thinkingSeconds?.let(::sessionDuration), Mint).takeIf { "thinkingTime" in items })
    val counts = countStats(session, items)
    val git = showsGit(session, items)
    val changes = showsChanges(session, items)
    // Emit nothing when no line has anything to show, so the row keeps its spacing.
    if ((times + counts).none { it.value != null } && !git && !changes) return
    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
        StatLine(times)
        StatLine(counts)
        if (git) RowGit(session)
        if (changes) RowChanges(session)
    }
}
/** Tools, compactions and how full the context window is. Context appears only when the board sent both token counts. */
private fun countStats(session: Session, items: Set<String>) = listOfNotNull(
    RowStat("⚒", "Tools", session.stats.tools?.toString(), Peach).takeIf { "tools" in items },
    RowStat("⊜", "Compactions", session.stats.compactions?.toString(), Peach).takeIf { "compactions" in items },
    session.stats.context?.let { RowStat("◔", "Context", it.label, TerminalText.palette(it.palette)) }.takeIf { "context" in items })
/** The git line needs a branch or files not committed; the changes line needs a line added or removed. */
private fun showsGit(session: Session, items: Set<String>) = "git" in items && (session.branch.isNotBlank() || session.uncommitted > 0)
private fun showsChanges(session: Session, items: Set<String>) = "changes" in items && (session.additions > 0 || session.deletions > 0)
@Composable private fun RowGit(session: Session) {
    val files = if (session.uncommitted == 1) "1 file not committed" else "${session.uncommitted} files not committed"
    Row(Modifier.clearAndSetSemantics { contentDescription = listOfNotNull("Branch ${session.branch}".takeIf { session.branch.isNotBlank() }, files.takeIf { session.uncommitted > 0 }).joinToString(", ") }, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        if (session.branch.isNotBlank()) Text("⎇ ${session.branch}", color = GitGreen, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f, fill = false))
        if (session.uncommitted > 0) Text(files, color = SecondaryText, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1, softWrap = false)
    }
}
@Composable private fun RowChanges(session: Session) {
    Row(Modifier.clearAndSetSemantics { contentDescription = "Changes: ${session.additions} lines added, ${session.deletions} removed" }, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("Δ", color = CrabColors.Pink, fontSize = 11.sp, lineHeight = 16.sp)
        Text("+${session.additions}", color = Mint, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1, softWrap = false)
        Text("−${session.deletions}", color = CrabColors.Red, fontSize = 11.sp, lineHeight = 16.sp, maxLines = 1, softWrap = false)
    }
}
private val GitGreen = Color(0xFF7EE787)

private class RowStat(val symbol: String, val label: String, val value: String?, val color: Color)
/** A stat with no data shows a dash beside one that has some. A line with no data at all is left out. */
@Composable private fun StatLine(stats: List<RowStat>) {
    if (stats.none { it.value != null }) return
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        stats.forEach { Stat(it.symbol, it.value ?: "—", "${it.label}: ${it.value ?: "unknown"}", it.color) }
    }
}

/** Shorten durations before giving up space reserved for the two right-aligned recency values. */
@Composable private fun OneLineTimeline(summary: String, statusLabel: String, prioritizeDuration: Boolean, modifier: Modifier = Modifier, start: @Composable (Boolean) -> Unit, end: @Composable () -> Unit) {
    SubcomposeLayout(modifier.fillMaxWidth()) { constraints ->
        val loose = Constraints(maxHeight = constraints.maxHeight)
        // Probes choose a format only. Hide their semantics so only the visible values are announced.
        val rightProbe = subcompose("measure-recency") { Box(Modifier.clearAndSetSemantics {}) { end() } }.single().measure(loose)
        var leftProbe = subcompose("measure-duration") { Box(Modifier.clearAndSetSemantics {}) { start(false) } }.single().measure(loose)
        val gap = if (leftProbe.width > 0 && rightProbe.width > 0) 8.dp.roundToPx() else 0
        val compact = leftProbe.width + gap + rightProbe.width > constraints.maxWidth
        if (compact) leftProbe = subcompose("measure-compact") { Box(Modifier.clearAndSetSemantics {}) { start(true) } }.single().measure(loose)
        val showLeft = leftProbe.width + gap + rightProbe.width <= constraints.maxWidth
        if (!showLeft && (rightProbe.width == 0 || prioritizeDuration)) {
            val durationOnly = subcompose("visible") {
                Box(Modifier.clearAndSetSemantics { contentDescription = summary }) { start(true) }
            }.single().measure(loose.copy(maxWidth = (constraints.maxWidth - rightProbe.width - gap).coerceAtLeast(0)))
            val status = subcompose("visible-status") { end() }.single().measure(loose.copy(maxWidth = constraints.maxWidth))
            val height = maxOf(durationOnly.height, status.height)
            return@SubcomposeLayout layout(constraints.maxWidth, height) {
                durationOnly.placeRelative(0, (height - durationOnly.height) / 2)
                status.placeRelative(constraints.maxWidth - status.width, (height - status.height) / 2)
            }
        }
        val visible = subcompose("visible") {
            if (showLeft) Box { start(compact) }
            Box(if (showLeft) Modifier else Modifier.clearAndSetSemantics { contentDescription = "$statusLabel; $summary" }) {
                end()
            }
        }.map { it.measure(loose.copy(maxWidth = constraints.maxWidth)) }
        val right = visible.last()
        val height = visible.maxOf { it.height }
        layout(constraints.maxWidth, height) {
            if (showLeft) visible.first().placeRelative(0, (height - visible.first().height) / 2)
            right.placeRelative(constraints.maxWidth - right.width, (height - right.height) / 2)
        }
    }
}

@Composable private fun Stat(symbol: String, value: String, description: String, color: Color) {
    Row(Modifier.clearAndSetSemantics { contentDescription = description }, horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(symbol, color = color, fontSize = 13.sp, lineHeight = 16.sp, maxLines = 1, softWrap = false)
        Text(value, color = color, fontSize = 11.sp, lineHeight = 16.sp, letterSpacing = 0.sp, maxLines = 1, softWrap = false)
    }
}
