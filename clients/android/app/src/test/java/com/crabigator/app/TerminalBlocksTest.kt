package com.crabigator.app

import androidx.compose.ui.text.AnnotatedString
import org.junit.Assert.*
import org.junit.Test

class TerminalBlocksTest {
    @Test fun splittingPreservesBlankLinesAndStylesThatCrossBoundaries() {
        val input = "\u001b[33mfirst\n\nthird\nfourth\u001b[0m plain\n"
        val parsed = TerminalText.parse(input)
        val blocks = terminalBlocks(parsed, linesPerBlock = 2)
        assertEquals(parsed.text, blocks.joinToString("\n") { it.text })
        for (block in blocks) {
            for (word in listOf("first", "third", "fourth")) {
                val offset = block.text.indexOf(word)
                if (offset >= 0) assertEquals(TerminalText.palette(3), block.spanStyles.first { offset in it.start until it.end }.item.color)
            }
        }
        assertTrue(terminalBlocks(AnnotatedString("")).isEmpty())
    }
}
