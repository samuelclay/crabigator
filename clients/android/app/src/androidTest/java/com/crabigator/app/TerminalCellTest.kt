package com.crabigator.app

import android.graphics.Paint
import android.graphics.Typeface
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Text
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.sp
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class TerminalCellTest {
    @get:Rule val compose = createComposeRule()

    @Test fun animatedSymbolsKeepFollowingTextInTheSameCell() {
        val cellWidthEm = Paint().apply {
            typeface = Typeface.MONOSPACE
            textSize = 1000f
        }.measureText(" ") / 1000f
        val layouts = mutableMapOf<Pair<Int, Char>, TextLayoutResult>()
        val frames = " ⏺·✢✳✶✻✽"
        val sizes = listOf(10, 14, 20)
        compose.setContent {
            Column {
                for (size in sizes) for (symbol in frames) {
                    Text(TerminalText.parse("\u001b[38;2;153;153;153m$symbol \u001b[1mBash\u001b[22m (38m 18s)", cellWidthEm),
                        fontFamily = FontFamily.Monospace, fontSize = size.sp, lineHeight = (size * 1.3f).sp,
                        onTextLayout = { layouts[size to symbol] = it })
                }
            }
        }
        compose.runOnIdle {
            for (size in sizes) {
                val blank = layouts.getValue(size to ' ')
                for (symbol in frames) {
                    val frame = layouts.getValue(size to symbol)
                    for (offset in listOf(2, 8)) {
                        assertEquals("$size sp, $symbol, offset $offset",
                            blank.getBoundingBox(offset).left, frame.getBoundingBox(offset).left, .05f)
                    }
                    assertEquals("Line height at $size sp, $symbol", blank.size.height, frame.size.height)
                }
            }
        }
    }
}
