package com.crabigator.app

import androidx.compose.ui.text.LinkAnnotation
import org.junit.Assert.*
import org.junit.Test

class TerminalLinksTest {
    @Test fun linksSurviveColorsWrappingAndBlockSplits() {
        val source = "🦀 \u001b]8;id=pr;https://example.com/pr;a=1\u001b\\Read \u001b[1mPR\u001b[0m\nsecond row\u001b]8;;\u0007 plain\n(https://example.com/a_\u001b[34m(b)).\n\u001b]8;;javascript:alert(1)\u0007unsafe\u001b]8;;\u0007"
        val text = TerminalText.trimLineEnds(TerminalText.parse(source))
        assertEquals("🦀 Read PR\nsecond row plain\n(https://example.com/a_(b)).\nunsafe", text.text)
        fun link(word: String): String? {
            val start = text.text.indexOf(word)
            return (text.getLinkAnnotations(start, start + 1).firstOrNull()?.item as? LinkAnnotation.Url)?.url
        }
        assertEquals("https://example.com/pr;a=1", link("Read"))
        assertEquals("https://example.com/pr;a=1", link("PR"))
        assertEquals("https://example.com/pr;a=1", link("second"))
        assertNull(link("plain"))
        assertEquals("https://example.com/a_(b)", link("https"))
        assertNull(link(")."))
        assertNull(link("unsafe"))
        val blocks = terminalBlocks(text, linesPerBlock = 1)
        assertEquals("https://example.com/pr;a=1", (blocks[1].getLinkAnnotations(0, 1).single().item as LinkAnnotation.Url).url)
    }
}
