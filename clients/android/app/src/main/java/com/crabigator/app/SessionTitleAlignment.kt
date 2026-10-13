package com.crabigator.app

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.HorizontalAlignmentLine
import androidx.compose.ui.layout.layout
import androidx.compose.ui.platform.LocalDensity

// Only the first title line supplies this line; recaps and PR rows cannot move the chip.
internal val SessionTitleCenter = HorizontalAlignmentLine { first, second -> minOf(first, second) }

@Composable internal fun Modifier.sessionTitleAlignment(): Modifier {
    val spacing = LocalBoardSpacing.current
    val firstLineHeight = with(LocalDensity.current) { spacing.lineHeight(19).roundToPx() }
    return layout { measurable, constraints ->
        val title = measurable.measure(constraints)
        layout(title.width, title.height, mapOf(SessionTitleCenter to minOf(title.height, firstLineHeight) / 2)) {
            title.placeRelative(0, 0)
        }
    }
}
