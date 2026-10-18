package com.crabigator.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test

class RosterPlaceholderTest {
    @get:Rule val compose = createComposeRule()

    @Test fun disconnectedDesktopDoesNotWaitForTerminalOutput() {
        val disconnected = mutableStateOf(false)
        compose.setContent {
            MaterialTheme { TerminalPlaceholder(active = true, loaded = false, loadingSlowly = true,
                reconnect = {}, disconnected = disconnected.value) }
        }
        compose.onNode(hasProgressBarRangeInfo(ProgressBarRangeInfo.Indeterminate)).assertExists()
        compose.runOnIdle { disconnected.value = true }
        compose.onNode(hasProgressBarRangeInfo(ProgressBarRangeInfo.Indeterminate)).assertDoesNotExist()
        compose.onNodeWithText("Desktop disconnected. Terminal output will resume when it reconnects.").assertExists()
        compose.runOnIdle { disconnected.value = false }
        compose.onNode(hasProgressBarRangeInfo(ProgressBarRangeInfo.Indeterminate)).assertExists()
    }

    @Test fun onlyAConfirmedEmptyRosterShowsPairing() {
        val status = mutableStateOf(AppState(paired = true).rosterStatus)
        var retries = 0
        compose.setContent {
            MaterialTheme { RosterPlaceholder(status.value, { retries++ }) { Text("Pair computer") } }
        }
        compose.onNodeWithContentDescription("Loading sessions").assertExists()
        compose.onNodeWithText("No sessions").assertDoesNotExist()
        compose.onNodeWithText("Pair computer").assertDoesNotExist()
        compose.runOnIdle { status.value = RosterStatus.Failed }
        compose.onNodeWithText("Retry").performClick()
        assertEquals(1, retries)
        compose.onNodeWithText("No sessions").assertDoesNotExist()
        compose.runOnIdle { status.value = RosterStatus.Loading }
        compose.onNodeWithContentDescription("Loading sessions").assertExists()
        compose.runOnIdle { status.value = RosterStatus.Ready }
        compose.onNodeWithText("No sessions").assertExists()
        compose.onNodeWithText("Pair computer").assertExists()
        compose.onNodeWithContentDescription("Loading sessions").assertDoesNotExist()
    }
}
