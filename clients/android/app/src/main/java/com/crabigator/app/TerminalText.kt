package com.crabigator.app

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration

/** Screen frames are complete, row-formatted snapshots, not an incremental VT stream. */
object TerminalText {
    /** Screen rows contain padding to the desktop width; wrapping must not wrap that padding. */
    fun trimLineEnds(input: AnnotatedString): AnnotatedString {
        val result = AnnotatedString.Builder()
        var start = 0
        while (start < input.length) {
            val newline = input.text.indexOf('\n', start).let { if (it < 0) input.length else it }
            var end = newline
            while (end > start && input[end - 1] == ' ') end--
            result.append(input.subSequence(start, end))
            if (newline < input.length) result.append('\n')
            start = newline + 1
        }
        return result.toAnnotatedString()
    }
    private val base = intArrayOf(0x20242A, 0xEC7777, 0x98C379, 0xE5C07B, 0x82AAFF, 0xC792EA, 0x89DDFF, 0xDEE4EC, 0x697584, 0xFF8B8B, 0xB2E394, 0xFFE09B, 0xA5C6FF, 0xE1B2FF, 0xB4F0FF, 0xFFFFFF)
    fun palette(n: Int): Color {
        val i = n.coerceIn(0, 255)
        if (i < 16) return Color(0xFF000000L or base[i].toLong())
        if (i >= 232) { val v = 8 + (i - 232) * 10; return Color(v, v, v) }
        val c = i - 16
        fun channel(v: Int) = if (v == 0) 0 else 55 + v * 40
        return Color(channel(c / 36), channel(c / 6 % 6), channel(c % 6))
    }
    fun parse(input: String): AnnotatedString {
        val out = AnnotatedString.Builder()
        var fg = Color(0xFFDDE3ED); var bg = Color.Unspecified; var bold = false; var underline = false; var inverse = false
        var i = 0; var column = 0
        val pending = StringBuilder()
        fun flush() {
            if (pending.isEmpty()) return
            val start = out.length; out.append(pending.toString()); pending.clear()
            out.addStyle(SpanStyle(color = if (inverse) bg.takeOrElse { Color(0xFF11151B) } else fg, background = if (inverse) fg else bg, fontWeight = if (bold) FontWeight.Bold else FontWeight.Normal, textDecoration = if (underline) TextDecoration.Underline else TextDecoration.None), start, out.length)
        }
        fun append(s: String) { pending.append(s); column += s.length }
        while (i < input.length) {
            if (input[i] == '\u001b') {
                if (input.getOrNull(i + 1) == '[') {
                    val start = i + 2; var end = start
                    while (end < input.length && input[end] !in '@'..'~') end++
                    if (end >= input.length) break
                    val nums = input.substring(start, end).split(';').map { it.toIntOrNull() ?: 0 }
                    when (input[end]) {
                        'm' -> { flush(); var p = 0; while (p < nums.size) {
                            when (val n = nums[p]) {
                                0 -> { fg = Color(0xFFDDE3ED); bg = Color.Unspecified; bold = false; underline = false; inverse = false }
                                1 -> bold = true; 22 -> bold = false; 4 -> underline = true; 24 -> underline = false
                                7 -> inverse = true; 27 -> inverse = false
                                in 30..37 -> fg = palette(n - 30); in 90..97 -> fg = palette(n - 90 + 8)
                                in 40..47 -> bg = palette(n - 40); in 100..107 -> bg = palette(n - 100 + 8)
                                39 -> fg = Color(0xFFDDE3ED); 49 -> bg = Color.Unspecified
                                38, 48 -> {
                                    val color = when {
                                        nums.getOrNull(p + 1) == 5 && p + 2 < nums.size -> palette(nums[p + 2]).also { p += 2 }
                                        nums.getOrNull(p + 1) == 2 && p + 4 < nums.size -> Color(nums[p + 2].coerceIn(0,255), nums[p + 3].coerceIn(0,255), nums[p + 4].coerceIn(0,255)).also { p += 4 }
                                        else -> null
                                    }
                                    if (color != null) { if (n == 38) fg = color else bg = color }
                                }
                            }; p++
                        } }
                        'C' -> append(" ".repeat((nums.firstOrNull() ?: 1).coerceIn(1, 500)))
                        'G' -> append(" ".repeat(((nums.firstOrNull() ?: 1) - 1 - column).coerceIn(0, 500)))
                    }
                    i = end + 1; continue
                }
                if (input.getOrNull(i + 1) == ']') {
                    i += 2
                    while (i < input.length && input[i] != '\u0007' && !(input[i] == '\u001b' && input.getOrNull(i+1) == '\\')) i++
                    i += if (input.getOrNull(i) == '\u001b') 2 else 1
                    continue
                }
                i = (i + 2).coerceAtMost(input.length); continue
            }
            when (input[i]) {
                '\n' -> { pending.append('\n'); column = 0 }
                '\r' -> Unit
                '\t' -> append(" ".repeat(8 - column % 8))
                else -> if (input[i] >= ' ') { append(if (input[i] == '\u23fa') "●" else input[i].toString()) }
            }; i++
        }
        flush()
        return out.toAnnotatedString()
    }
}
private fun Color.takeOrElse(fallback: () -> Color) = if (this == Color.Unspecified) fallback() else this
