package com.crabigator.app

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class TerminalLinksInteractionTest {
    @get:Rule val compose = createComposeRule()

    @Test fun tapsOpenHistoryAndLiveLinksWithOrWithoutWrapping() {
        var opened = ""
        val prefs = mutableStateOf(UiPreferences())
        val handler = object : UriHandler { override fun openUri(uri: String) { opened = uri } }
        compose.setContent {
            CompositionLocalProvider(LocalUriHandler provides handler) {
                MaterialTheme {
                    Box(Modifier.fillMaxSize()) {
                        TerminalViewport(
                            "🦀 \u001b]8;id=history;https://example.com/history\u001b\\History PR\u001b]8;;\u0007\n",
                            "\u001b]8;;https://example.com/docs\u0007Codex docs\u001b]8;;\u001b\\\nhttps://example.com/plain",
                            prefs.value, true, {})
                    }
                }
            }
        }
        repeat(2) {
            for ((label, url) in listOf("History PR" to "https://example.com/history", "Codex docs" to "https://example.com/docs", "https://example.com/plain" to "https://example.com/plain")) {
                compose.waitUntil(10_000) { compose.onAllNodesWithText(label, substring = true).fetchSemanticsNodes().isNotEmpty() }
                val text = compose.onNodeWithText(label, substring = true)
                var layout: TextLayoutResult? = null
                text.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { action ->
                    val results = mutableListOf<TextLayoutResult>(); action(results); layout = results.single()
                }
                val result = layout!!
                val offset = result.layoutInput.text.text.indexOf(label)
                text.performTouchInput { click(result.getBoundingBox(offset).center) }
                compose.runOnIdle { assertEquals(url, opened) }
            }
            compose.runOnIdle { prefs.value = prefs.value.copy(wrap = !prefs.value.wrap) }
        }
    }
}
