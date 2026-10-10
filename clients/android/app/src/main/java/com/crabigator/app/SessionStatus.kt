package com.crabigator.app

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.sp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay

/** The named xterm colors in src/terminal/escape.rs. */
internal object CrabColors {
    val Green = TerminalText.palette(83)
    val Yellow = TerminalText.palette(220)
    val Orange = TerminalText.palette(179)
    val Red = TerminalText.palette(203)
    val Cyan = TerminalText.palette(45)
    val Blue = TerminalText.palette(39)
    val Title = TerminalText.palette(75)
    val Purple = TerminalText.palette(141)
    val Pink = TerminalText.palette(213)
    val Gray = TerminalText.palette(245)
}

@Composable internal fun SessionStatus(session: Session) {
    // Same frames and 100ms clock as src/ui/stats.rs; rows stay in sync.
    val frames = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
    var frame by remember { mutableIntStateOf(0) }
    val thinking = session.active && session.state == "thinking"
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(thinking, lifecycle) {
        if (thinking) lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                val now = System.currentTimeMillis()
                frame = ((now / 100) % frames.length).toInt()
                delay(100 - now % 100)
            }
        }
    }
    if (thinking) {
        val width = with(LocalDensity.current) { 12.sp.toDp() }
        val height = with(LocalDensity.current) { 16.sp.toDp() }
        Canvas(Modifier.size(width, height).clearAndSetSemantics { contentDescription = "Working" }) {
            // Draw the braille cells on one fixed grid. Font fallback can give
            // different glyphs different bounds and make the row appear to jump.
            val bits = frames[frame].code - 0x2800
            for (column in 0..1) for (row in 0..2) {
                if (bits and (1 shl (column * 3 + row)) != 0) {
                    drawCircle(CrabColors.Green, radius = size.width * .105f,
                        center = Offset(size.width * (if (column == 0) .3f else .7f), size.height * (.25f + row * .25f)))
                }
            }
        }
        return
    }
    val (label, color) = if (!session.active) "○ Ended" to CrabColors.Gray else when (session.state) {
        "permission" -> "» ? « Perm" to CrabColors.Yellow
        "question" -> "» ? « Ask" to CrabColors.Orange
        "complete" -> "✓ Complete" to CrabColors.Purple
        "interrupted" -> "⊘ Interrupted" to CrabColors.Red
        else -> "○ Ready" to CrabColors.Gray
    }
    Text(label, color = color, fontFamily = FontFamily.Monospace, fontSize = 11.sp, lineHeight = 16.sp,
        modifier = Modifier.clearAndSetSemantics { contentDescription = label })
}
