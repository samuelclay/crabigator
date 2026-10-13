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

@Composable internal fun SessionStatsFooter(session: Session, clock: State<Long>, prefs: UiPreferences) {
    val now = clock.value
    val stats = session.stats
    val duration = stats.duration(session.active, now)
    val prompt = if (stats.promptAt > 0) sessionAge(stats.promptAt, now) else "—"
    val completion = if (stats.completionAt > 0) sessionAge(stats.completionAt, now) else "—"
    val summary = buildList {
        if (prefs.visible("sessionTime")) add("Session time: ${sessionDuration(duration)}")
        if (prefs.visible("thinkingTime")) add("Thinking time: ${sessionDuration(stats.thinkingSeconds)}")
        if (prefs.visible("promptRecency")) add("Last prompt: $prompt")
        if (prefs.visible("completionRecency")) add("Last completion: $completion")
    }.joinToString("; ")
    Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
        OneLineTimeline(summary, start = { compact ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (prefs.visible("sessionTime")) Stat("◉", if (compact) compactDuration(duration) else sessionDuration(duration), "Session time: ${sessionDuration(duration)}", CrabColors.Blue)
                if (prefs.visible("thinkingTime")) Stat("◐", if (compact) compactDuration(stats.thinkingSeconds) else sessionDuration(stats.thinkingSeconds), "Thinking time: ${sessionDuration(stats.thinkingSeconds)}", Mint)
            }
        }, end = {
            BoardRecency(session, clock, prefs.visible("promptRecency"), prefs.visible("completionRecency"))
        })
        if (prefs.visible("tools") || prefs.visible("compactions")) FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (prefs.visible("tools")) Stat("⚒", stats.tools?.toString() ?: "—", "Tools: ${stats.tools ?: "—"}", Peach)
            if (prefs.visible("compactions")) Stat("⊜", stats.compactions?.toString() ?: "—", "Compactions: ${stats.compactions ?: "—"}", CrabColors.Pink)
        }
    }
}

/** Shorten durations before giving up space reserved for the two right-aligned recency values. */
@Composable private fun OneLineTimeline(summary: String, modifier: Modifier = Modifier, start: @Composable (Boolean) -> Unit, end: @Composable () -> Unit) {
    SubcomposeLayout(modifier.fillMaxWidth()) { constraints ->
        val loose = Constraints(maxHeight = constraints.maxHeight)
        // Probes choose a format only. Hide their semantics so only the visible values are announced.
        val rightProbe = subcompose("measure-recency") { Box(Modifier.clearAndSetSemantics {}) { end() } }.single().measure(loose)
        var leftProbe = subcompose("measure-duration") { Box(Modifier.clearAndSetSemantics {}) { start(false) } }.single().measure(loose)
        val gap = if (leftProbe.width > 0 && rightProbe.width > 0) 8.dp.roundToPx() else 0
        val compact = leftProbe.width + gap + rightProbe.width > constraints.maxWidth
        if (compact) leftProbe = subcompose("measure-compact") { Box(Modifier.clearAndSetSemantics {}) { start(true) } }.single().measure(loose)
        val showLeft = leftProbe.width + gap + rightProbe.width <= constraints.maxWidth
        if (!showLeft && rightProbe.width == 0) {
            val durationOnly = subcompose("visible") {
                Box(Modifier.clearAndSetSemantics { contentDescription = summary }) { start(true) }
            }.single().measure(loose.copy(maxWidth = constraints.maxWidth))
            return@SubcomposeLayout layout(constraints.maxWidth, durationOnly.height) { durationOnly.placeRelative(0, 0) }
        }
        val visible = subcompose("visible") {
            if (showLeft) Box { start(compact) }
            Box(if (showLeft) Modifier else Modifier.clearAndSetSemantics { contentDescription = summary }) {
                end()
            }
        }.map { it.measure(loose.copy(maxWidth = constraints.maxWidth)) }
        val right = visible.last()
        layout(constraints.maxWidth, visible.maxOf { it.height }) {
            if (showLeft) visible.first().placeRelative(0, 0)
            right.placeRelative(constraints.maxWidth - right.width, 0)
        }
    }
}

@Composable private fun Stat(symbol: String, value: String, description: String, color: Color) {
    Row(Modifier.clearAndSetSemantics { contentDescription = description }, horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(symbol, color = Muted, fontSize = 13.sp, lineHeight = 16.sp, maxLines = 1, softWrap = false)
        Text(value, color = color, fontSize = 11.sp, lineHeight = 16.sp, letterSpacing = 0.sp, maxLines = 1, softWrap = false)
    }
}
