package com.crabigator.app

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class TerminalScrollTest {
    @get:Rule val compose = createComposeRule()

    @Test fun readingHistorySurvivesNewOutputAndPinReturnsToLiveEnd() {
        val pinned = mutableStateOf(true)
        val screen = mutableStateOf("Live output\nLIVE FOOTER")
        val history = (0 until 1000).joinToString("\n") { "History row $it" }
        compose.setContent {
            MaterialTheme {
                Box(Modifier.fillMaxSize().testTag("terminal")) {
                    TerminalViewport(history, screen.value, UiPreferences(), pinned.value) { pinned.value = it }
                }
            }
        }
        compose.waitUntil(10_000) { compose.onAllNodesWithText("LIVE FOOTER", substring = true).fetchSemanticsNodes().isNotEmpty() }
        val list = compose.onNode(SemanticsMatcher.keyIsDefined(SemanticsProperties.CollectionInfo))
        // 42 history blocks and one live block. Wait for both background parses.
        compose.waitUntil(10_000) { list.fetchSemanticsNode().config[SemanticsProperties.CollectionInfo].rowCount == 43 }
        compose.onNodeWithText("LIVE FOOTER", substring = true).assertIsDisplayed()
        compose.onNodeWithTag("terminal").performTouchInput { swipeDown(durationMillis = 600) }
        compose.waitForIdle()
        compose.runOnIdle { assertFalse(pinned.value) }
        val reading = compose.onAllNodes(hasText("History row", substring = true)).fetchSemanticsNodes()
            .filter { it.boundsInRoot.height > 0f }.maxByOrNull { it.boundsInRoot.height }
        assertNotNull("History must be visible after scrolling", reading)
        val readingText = reading!!.config[SemanticsProperties.Text].single().text
        val readingTop = reading.positionInRoot.y
        compose.runOnIdle { screen.value = (0 until 80).joinToString("\n") { "New output $it" } + "\nNEW LIVE FOOTER" }
        // The new live frame has four blocks. Await its parse and lazy-list layout before checking.
        compose.waitUntil(10_000) { list.fetchSemanticsNode().config[SemanticsProperties.CollectionInfo].rowCount == 46 }
        compose.onNodeWithText(readingText).assertIsDisplayed()
        assertEquals("New output must not move the history being read", readingTop,
            compose.onNodeWithText(readingText).fetchSemanticsNode().positionInRoot.y, 1f)
        val axis = SemanticsProperties.VerticalScrollAxisRange
        val beforeFollow = list.fetchSemanticsNode().config[axis].value()
        compose.mainClock.autoAdvance = false
        compose.runOnIdle { pinned.value = true }
        compose.mainClock.advanceTimeBy(64)
        val duringFollow = list.fetchSemanticsNode().config[axis]
        assertTrue("Follow must move through intermediate positions", duringFollow.value() > beforeFollow)
        assertTrue("Follow must animate instead of jumping to the end", duringFollow.value() < duringFollow.maxValue())
        // A new live frame must not cancel the user's animated return.
        compose.runOnIdle { screen.value = screen.value.replace("New output 0", "Updated output 0") }
        compose.mainClock.advanceTimeBy(32)
        compose.mainClock.autoAdvance = true
        compose.waitUntil(10_000) { compose.onAllNodesWithText("NEW LIVE FOOTER", substring = true).fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("NEW LIVE FOOTER", substring = true).assertIsDisplayed()
    }
    @Test fun noWrapKeepsTheEndOfALongAnsiRowReachable() {
        val line = "X".repeat(1000) + " LAST CELL"
        compose.setContent {
            MaterialTheme {
                Box(Modifier.fillMaxSize().testTag("terminal")) {
                    TerminalViewport("", "\u001b[33m$line\u001b[0m", UiPreferences(wrap = false), true) {}
                }
            }
        }
        compose.waitUntil(10_000) { compose.onAllNodesWithText(line).fetchSemanticsNodes().isNotEmpty() }
        val text = compose.onNodeWithText(line)
        val layouts = mutableListOf<TextLayoutResult>()
        text.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        val layout = layouts.single()
        assertEquals("No-wrap rows must remain on one line", 1, layout.lineCount)
        val viewport = compose.onNodeWithTag("terminal").fetchSemanticsNode().boundsInRoot
        fun lastCellRight() = text.fetchSemanticsNode().positionInRoot.x + layout.getBoundingBox(line.lastIndex).right
        assertTrue("The test row must extend beyond the viewport", lastCellRight() > viewport.right)

        compose.onNode(SemanticsMatcher.keyIsDefined(SemanticsProperties.HorizontalScrollAxisRange))
            .performSemanticsAction(SemanticsActions.ScrollBy) { it(100_000f, 0f) }
        compose.waitForIdle()
        assertTrue("Horizontal scrolling must reveal the final cell", lastCellRight() <= viewport.right + 1f)
        assertTrue("The final cell must remain inside the viewport", lastCellRight() > viewport.left)
    }
    @Test fun capturedHistoryMeetsTheLiveScreenWithoutAnExtraBlankRow() {
        compose.setContent {
            MaterialTheme {
                TerminalViewport("\u001b[0m\u001b[31mSaved terminal row\u001b[0m\n",
                    "Live terminal row\n", UiPreferences(), true) {}
            }
        }
        compose.waitUntil(10_000) {
            compose.onAllNodesWithText("Saved terminal row").fetchSemanticsNodes().isNotEmpty() &&
                compose.onAllNodesWithText("Live terminal row").fetchSemanticsNodes().isNotEmpty()
        }
        val history = compose.onNodeWithText("Saved terminal row").fetchSemanticsNode().boundsInRoot
        val live = compose.onNodeWithText("Live terminal row").fetchSemanticsNode().boundsInRoot
        assertEquals("History and live rows must be adjacent", history.bottom, live.top, 1f)
    }

}
