package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class PromptActionsTest {
    private fun prompt(extra: String = "") = JSONObject("""{"prompt_type":"question","cursor_row":3,"questions":[{"question":"Which one?","options":[{"label":"One","value":"1"},{"label":"Two","value":"2"}],"allows_other":true}]$extra}""")
    @Test fun screenIntensityMatchesCodexWorkingAndBuildOutput() {
        val parsed = TerminalText.parse("\u001b[2mBUILD SUCCESSFUL\u001b[0m normal\n\u001b[2mWorking (\u001b[1mesc\u001b[2m to interrupt)\u001b[22m normal")
        fun style(word: String) = parsed.spanStyles.first { parsed.text.indexOf(word) in it.start until it.end }.item
        assertEquals(.5f, style("BUILD").color.alpha, .01f)
        assertEquals(.5f, style("Working").color.alpha, .01f)
        assertEquals(1f, style("esc").color.alpha, .01f)
        assertEquals(androidx.compose.ui.text.font.FontWeight.Bold, style("esc").fontWeight)
        assertEquals(.5f, style("to interrupt").color.alpha, .01f)
        assertEquals(androidx.compose.ui.text.font.FontWeight.Normal, style("to interrupt").fontWeight)
        assertEquals(1f, parsed.spanStyles.last().item.color.alpha, .01f)
    }
    @Test fun wrappingRemovesDesktopPaddingAndPreservesTextStyles() {
        val parsed = TerminalText.parse("\u001b[31m  red text    \n\u001b[0mnext  ")
        val wrapped = TerminalText.trimLineEnds(parsed)
        assertEquals("  red text\nnext", wrapped.text)
        assertEquals(parsed.spanStyles.first().item.color, wrapped.spanStyles.first().item.color)
        assertEquals("  red text    \nnext  ", parsed.text)
    }
    @Test fun choosingFromTextRowLeavesItWithoutSendingAnExtraEnter() {
        val steps = PromptActions.action(prompt(), option = 1).body.getJSONArray("steps")
        assertEquals(2, steps.length())
        assertEquals("up", steps.getJSONObject(0).getString("key"))
        assertEquals("2", steps.getJSONObject(1).getString("text"))
    }
    @Test fun customReplyReplacesExistingTextAndSubmits() {
        val steps = PromptActions.action(prompt(",\"custom_text\":\"abc\""), text = "new").body.getJSONArray("steps")
        assertEquals(6, steps.length())
        assertEquals("backspace", steps.getJSONObject(2).getString("key"))
        assertEquals("new", steps.getJSONObject(3).getString("text"))
        assertEquals(50, steps.getJSONObject(4).getInt("ms"))
        assertEquals("enter", steps.getJSONObject(5).getString("key"))
    }
    @Test fun customReplyUsesTheTextRowShortcut() {
        val p = prompt().apply { remove("cursor_row") }
        val steps = PromptActions.action(p, text = "phone reply").body.getJSONArray("steps")
        assertEquals("3", steps.getJSONObject(0).getString("text"))
        assertEquals("phone reply", steps.getJSONObject(1).getString("text"))
        assertEquals("enter", steps.getJSONObject(3).getString("key"))
    }
    @Test fun permissionsUseActualValueNotTheirLabel() {
        val p = JSONObject("""{"prompt_type":"permission","options":[{"label":"Deny","value":"n"}]}""")
        val a = PromptActions.action(p, option = 0)
        assertEquals("answer", a.route); assertEquals("n", a.body.getString("text"))
    }
    @Test fun formattedScreenKeepsColorsAndColumnSpacing() {
        val text = TerminalText.parse("\u001b[38;5;196mRed\u001b[0m\u001b[3Cnext\r\n\u001b]8;;https://example.com\u0007link\u001b]8;;\u0007")
        assertEquals("Red   next\nlink", text.text)
        assertEquals(TerminalText.palette(196), text.spanStyles.first().item.color)
    }
    @Test fun grokTextAnswerSubmitsBeforeAdvancingPages() {
        val p = prompt().put("ui", "grok_card").put("current_question", 0)
        p.getJSONArray("questions").put(p.getJSONArray("questions").getJSONObject(0))
        val steps = PromptActions.action(p, text = "typed answer").body.getJSONArray("steps")
        assertEquals("enter", steps.getJSONObject(steps.length() - 1).getString("key"))
        assertFalse(steps.toString().contains("escape"))
        assertFalse(steps.toString().contains("right"))
    }
    @Test fun legacyMultiSelectTogglesWithoutSubmitting() {
        val p = prompt().apply { remove("cursor_row") }
        p.getJSONArray("questions").getJSONObject(0).put("multi_select", true)
        val steps = PromptActions.action(p, option = 1).body.getJSONArray("steps")
        assertEquals(1, steps.length())
        assertEquals("2", steps.getJSONObject(0).getString("text"))
    }
    @Test fun pairingUsesTheNewOriginWithoutAnExistingCredential() {
        val request = pairingRequest("https://self-host.example/", "abc def ghi", "mobile-id", "Phone")
        assertEquals("https://self-host.example/api/pairing/claim", request.url.toString())
        assertNull(request.header("Authorization"))
        val body = okio.Buffer().also { request.body!!.writeTo(it) }.readUtf8()
        assertEquals("ABC-DEF-GHI", JSONObject(body).getString("pairing_token"))
    }
    @Test fun pairingRejectsInsecureOrEmbeddedCredentialOrigins() {
        for (origin in listOf("http://example.com", "https://user:password@example.com", "https://example.com/other", "https://example.com?token=secret")) {
            assertThrows(IllegalArgumentException::class.java) { pairingRequest(origin, "ABC-DEF-GHI", "id", "Phone") }
        }
    }
}
