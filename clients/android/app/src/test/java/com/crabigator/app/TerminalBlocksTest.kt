package com.crabigator.app

import androidx.compose.ui.text.AnnotatedString
import org.junit.Assert.*
import org.junit.Test

class TerminalBlocksTest {
    @Test fun rulesKeepTheirTextAndStylesInSeparateBlocks() {
        for (symbol in listOf("-", "─", "━", "=", "x")) {
            val rule = symbol.repeat(120)
            val parsed = TerminalText.parse("before\n\n  \u001b[33m${rule.take(60)}\u001b[31m${rule.takeLast(60)}\u001b[0m   \ninput\n$rule\n")
            val trimmed = TerminalText.trimLineEnds(parsed)
            for (size in listOf(1, 2, 24)) {
                val blocks = terminalBlocks(trimmed, linesPerBlock = size)
                assertEquals(trimmed.text, blocks.joinToString("\n") { it.text })
                val rules = blocks.filter { TerminalText.isRule(it.text) }
                assertEquals(listOf("  $rule", rule), rules.map { it.text })
                assertEquals(TerminalText.palette(3), rules[0].spanStyles.first { 2 in it.start until it.end }.item.color)
                assertEquals(TerminalText.palette(1), rules[0].spanStyles.first { 62 in it.start until it.end }.item.color)
            }
            assertEquals(listOf(rule), terminalBlocks(TerminalText.parse(rule)).map { it.text })
        }
        assertTrue(TerminalText.isRule("🦀".repeat(120)))
        for (row in listOf("--", "----- title -----", "aaab", "   ")) assertFalse(TerminalText.isRule(row))
    }
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
