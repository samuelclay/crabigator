package com.crabigator.app

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.ParagraphStyle
import androidx.compose.ui.text.style.TextIndent
import androidx.compose.ui.unit.em
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextGeometricTransform
import java.net.URI

/** Screen frames are complete, row-formatted snapshots, not an incremental VT stream. */
object TerminalText {
    /** Capture has no soft-wrap metadata. Join clear prose only, retaining all spans. */
    fun reflowParagraphs(input: AnnotatedString, cellWidthEm: Float = .6f): AnnotatedString {
        val rows = input.text.split('\n')
        val trimmed = rows.map { it.trimEnd(' ') }
        val indents = trimmed.map { it.takeWhile { c -> c == ' ' }.length }
        val bodies = trimmed.mapIndexed { i, row -> row.drop(indents[i]) }
        val codeWords = setOf("import", "from", "export", "return", "throw", "let", "var", "const", "func", "fun", "def", "class", "if", "else", "for", "while", "switch", "case", "git", "npm", "npx", "yarn", "pnpm", "cargo", "make", "curl", "sudo", "pip", "python", "python3", "node")
        val commandFlag = Regex("(?:^| )--?[a-zA-Z]")
        var fence: Pair<Char, Int>? = null
        val prose = bodies.mapIndexed { i, body ->
            val marker = body.firstOrNull()
            val count = if (marker == '`' || marker == '~') body.takeWhile { it == marker }.length else 0
            if (count >= 3) {
                val open = fence
                if (open == null) fence = marker!! to count
                else if (marker == open.first && count >= open.second && body.drop(count).all { it == ' ' }) fence = null
                false
            } else {
                // Do not guess the terminal width of wide glyphs or combining marks.
                val singleCell = trimmed[i].all { it.code in 32..126 || it.code in 160..255 || it.code in 0x2010..0x2027 }
                fence == null && singleCell && indents[i] in listOf(0, 2) && body.firstOrNull()?.isLetter() == true &&
                    !body.contains("  ") && body.substringBefore(' ') !in codeWords && !commandFlag.containsMatchIn(body) && body.none { it in "{}[]=|\\<>`" }
            }
        }
        val ruleWidths = trimmed.map { row ->
            if (row.length >= 40 && row.first() in "-─━═" && row.all { it == row.first() }) row.length else null
        }
        val offsets = mutableListOf<Int>()
        var offset = 0
        for (row in rows) { offsets += offset; offset += row.length + 1 }
        val joins = mutableListOf<IntRange>()
        var inParagraph = false
        for (i in 0 until rows.lastIndex) {
            if (!prose[i]) { inParagraph = false; continue }
            if (!inParagraph) inParagraph = indents[i] == 2
            val word = bodies[i + 1].substringBefore(' ')
            val previousWord = bodies[i].substringAfterLast(' ')
            if (!inParagraph || !prose[i + 1] || bodies[i].split(' ').size < 3 ||
                bodies[i].last() in ".!?:;-/" || bodies[i + 1].firstOrNull()?.isLowerCase() != true ||
                previousWord.length >= 24 || word.length >= 24 || previousWord.any { it in "/_" } || word.any { it in "/_" }) {
                inParagraph = false; continue
            }
            var nearby: Int? = null
            var conflicting = false
            for (distance in 1..64) {
                val before = ruleWidths.getOrNull(i - distance)
                val after = ruleWidths.getOrNull(i + distance)
                if (before != null || after != null) {
                    conflicting = before != null && after != null && before != after
                    nearby = after ?: before
                    break
                }
            }
            if (conflicting) { inParagraph = false; continue }
            val margin = if (nearby != null) nearby else {
                // Without a separator, require two prose rows at the same margin.
                val widths = (maxOf(0, i - 32)..minOf(rows.lastIndex, i + 32)).filter { prose[it] }.map { trimmed[it].length }
                val longest = widths.maxOrNull() ?: 0
                if (longest < 40 || widths.count { it == longest } < 2) { inParagraph = false; continue }
                longest
            }
            val width = trimmed[i].length
            // Claude sometimes leaves the final cell unused: allow an exact fit.
            if (width > margin || width < margin - 12 || width + 1 + word.length < margin ||
                // A flush unindented continuation could be half of a word.
                (indents[i + 1] != 2 && width == margin && !rows[i].endsWith(' '))) {
                inParagraph = false; continue
            }
            joins += (offsets[i] + trimmed[i].length) until (offsets[i + 1] + indents[i + 1])
        }
        if (joins.isEmpty()) return input
        val result = AnnotatedString.Builder()
        var start = 0
        val joinedOffsets = mutableListOf<Int>()
        for (range in joins) {
            result.append(input.subSequence(start, range.first))
            joinedOffsets += result.length
            result.append(' ')
            start = range.last + 1
        }
        result.append(input.subSequence(start, input.length))
        val joined = result.toAnnotatedString().text
        val paragraphs = joinedOffsets.map { position ->
            val begin = joined.lastIndexOf('\n', position) + 1
            val end = joined.indexOf('\n', position).let { if (it < 0) joined.length else it }
            begin to end
        }.distinct()
        for ((begin, end) in paragraphs) {
            result.addStyle(ParagraphStyle(textIndent = TextIndent(firstLine = 0.em, restLine = (cellWidthEm * 2).em)), begin, end)
        }
        return result.toAnnotatedString()
    }
    fun isRule(line: CharSequence): Boolean {
        val characters = line.trim().toString().codePoints().toArray()
        return characters.size >= 3 && characters.all { it == characters[0] }
    }
    private val linkStyle = TextLinkStyles(SpanStyle(color = Color(0xFF82AAFF), textDecoration = TextDecoration.Underline))
    private val webAddress = Regex("https?://[^\\s<>\"']+", RegexOption.IGNORE_CASE)
    private fun webURL(value: String): String? = runCatching {
        val uri = URI(value)
        value.takeIf { uri.scheme?.lowercase() in listOf("http", "https") && !uri.rawAuthority.isNullOrEmpty() }
    }.getOrNull()
    private val statusSymbols = FontFamily(Font(R.font.noto_sans_symbols2))
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
    fun parse(input: String, cellWidthEm: Float = .6f): AnnotatedString {
        val out = AnnotatedString.Builder()
        var fg = Color(0xFFDDE3ED); var bg = Color.Unspecified; var bold = false; var dim = false; var underline = false; var inverse = false
        var link: String? = null
        var linkStart = 0
        val embedded = mutableListOf<IntRange>()
        var i = 0; var column = 0
        val pending = StringBuilder()
        fun flush() {
            if (pending.isEmpty()) return
            val start = out.length; val text = pending.toString(); out.append(text); pending.clear()
            val foreground = if (inverse) bg.takeOrElse { Color(0xFF11151B) } else fg
            out.addStyle(SpanStyle(color = foreground.copy(alpha = if (dim) .5f else 1f), background = if (inverse) fg else bg, fontWeight = if (bold) FontWeight.Bold else FontWeight.Normal, textDecoration = if (underline) TextDecoration.Underline else TextDecoration.None), start, out.length)
            // The bundled symbols advance 0.6em; match this device's monospace cell exactly.
            text.forEachIndexed { index, char ->
                if (char in '\u23f4'..'\u23fa' || char in "·✢✳✶✻✽") {
                    out.addStyle(SpanStyle(fontFamily = statusSymbols, textGeometricTransform = TextGeometricTransform(scaleX = cellWidthEm / .6f)), start + index, start + index + 1)
                }
            }
        }
        fun finishLink() {
            if (linkStart < out.length) link?.let {
                out.addLink(LinkAnnotation.Url(it, linkStyle), linkStart, out.length)
                embedded += linkStart until out.length
            }
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
                                0 -> { fg = Color(0xFFDDE3ED); bg = Color.Unspecified; bold = false; dim = false; underline = false; inverse = false }
                                1 -> { bold = true; dim = false }; 2 -> { dim = true; bold = false }; 22 -> { bold = false; dim = false }; 4 -> underline = true; 24 -> underline = false
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
                    val start = i
                    while (i < input.length && input[i] != '\u0007' && !(input[i] == '\u001b' && input.getOrNull(i+1) == '\\')) i++
                    val parts = input.substring(start, i).split(';', limit = 3)
                    if (parts.size == 3 && parts[0] == "8") { flush(); finishLink(); link = webURL(parts[2]); linkStart = out.length }
                    i += if (input.getOrNull(i) == '\u001b') 2 else 1
                    continue
                }
                i = (i + 2).coerceAtMost(input.length); continue
            }
            when (input[i]) {
                '\n' -> { pending.append('\n'); column = 0 }
                '\r' -> Unit
                '\t' -> append(" ".repeat(8 - column % 8))
                else -> if (input[i] >= ' ') { append(input[i].toString()) }
            }; i++
        }
        flush()
        finishLink()
        for (match in webAddress.findAll(out.toAnnotatedString().text)) {
            var value = match.value.trimEnd('.', ',', ';', ':', '!', '?')
            for ((open, close) in listOf('(' to ')', '[' to ']', '{' to '}')) {
                while (value.endsWith(close) && value.count { it == close } > value.count { it == open }) value = value.dropLast(1)
            }
            val start = match.range.first; val end = start + value.length
            if (embedded.none { it.first < end && it.last >= start }) {
                webURL(value)?.let { out.addLink(LinkAnnotation.Url(it, linkStyle), start, end) }
            }
        }
        return out.toAnnotatedString()
    }
}
private fun Color.takeOrElse(fallback: () -> Color) = if (this == Color.Unspecified) fallback() else this
