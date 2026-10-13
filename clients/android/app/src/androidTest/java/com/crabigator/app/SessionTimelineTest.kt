package com.crabigator.app

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class SessionTimelineTest {
    @get:Rule val compose = createComposeRule()

    @Test fun recencyStaysOnOneLineAtTheRightEdgeWhenTheCardNarrows() {
        val width = mutableStateOf(340.dp)
        val fontScale = mutableStateOf(1f)
        val prefs = mutableStateOf(UiPreferences())
        val now = mutableStateOf(1_000_000L)
        val session = mutableStateOf(Session.parse(JSONObject()).copy(active = true, state = "complete", stats = SessionStats(startedAt = 400_000, thinkingSeconds = 18_800,
            promptAt = now.value - 59 * 60, completionAt = now.value - 42 * 60)))
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(LocalDensity.current.density, fontScale.value)) {
                MaterialTheme {
                    Box(Modifier.width(width.value).testTag("timeline")) { SessionStatsFooter(session.value, now, prefs.value) }
                }
            }
        }
        fun bounds(label: String) = compose.onNodeWithContentDescription(label).fetchSemanticsNode().boundsInRoot
        fun checkAlignment() {
            val container = compose.onNodeWithTag("timeline").fetchSemanticsNode().boundsInRoot
            val duration = bounds("Session time: 6d 22h")
            val prompt = bounds("Last prompt: 59m ago")
            val completion = bounds("Last completion: 42m ago")
            assertEquals(container.left, duration.left, 1f)
            assertEquals(container.right, completion.right, 1f)
            assertEquals(prompt.top, completion.top, 1f)
            assertTrue(prompt.right <= completion.left)
            assertEquals(duration.center.y, prompt.center.y, 1f)
        }
        compose.waitForIdle()
        checkAlignment()
        val promptLeft = bounds("Last prompt: 59m ago").left
        val completionLeft = bounds("Last completion: 42m ago").left
        compose.runOnIdle { session.value = session.value.copy(stats = session.value.stats.copy(promptAt = now.value, completionAt = now.value - 3 * 86400)) }
        assertEquals(promptLeft, bounds("Last prompt: now").left, 1f)
        assertEquals(completionLeft, bounds("Last completion: 3d ago").left, 1f)
        compose.runOnIdle {
            session.value = session.value.copy(stats = session.value.stats.copy(promptAt = now.value - 59 * 60, completionAt = now.value - 42 * 60))
            width.value = 200.dp
        }
        compose.waitForIdle()
        compose.runOnIdle { fontScale.value = 1.8f }
        compose.waitForIdle()
        val container = compose.onNodeWithTag("timeline").fetchSemanticsNode().boundsInRoot
        // Recency stays on one line at large text sizes; the accessible summary retains hidden durations.
        val summary = bounds("Complete; Session time: 6d 22h; Thinking time: 5h 13m; Last prompt: 59m ago; Last completion: 42m ago")
        assertEquals(container.right, summary.right, 1f)
        assertTrue(summary.left >= container.left)
        assertEquals(container.top, summary.top, 1f)
        compose.runOnIdle { fontScale.value = 3f }
        compose.waitForIdle()
        val largest = bounds("Complete; Session time: 6d 22h; Thinking time: 5h 13m; Last prompt: 59m ago; Last completion: 42m ago")
        assertTrue(largest.left >= container.left)
        assertEquals(container.right, largest.right, 1f)
        compose.runOnIdle {
            width.value = 170.dp
            fontScale.value = 4f
            prefs.value = UiPreferences(hidden = setOf("promptRecency", "completionRecency", "tools", "compactions"))
        }
        compose.waitForIdle()
        val durationsOnly = bounds("Session time: 6d 22h; Thinking time: 5h 13m")
        assertTrue(durationsOnly.height > 0)
        assertEquals(container.left, durationsOnly.left, 1f)
        assertTrue(durationsOnly.right <= container.right)
    }
}
