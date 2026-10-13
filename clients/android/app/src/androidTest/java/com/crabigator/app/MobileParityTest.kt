package com.crabigator.app

import android.app.Application
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.core.app.ApplicationProvider
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

class MobileParityTest {
    @get:Rule val compose = createComposeRule()

    @Test fun boardSettingsPersistAndExposeActions() {
        val context = ApplicationProvider.getApplicationContext<Application>()
        val model = SessionModel(context)
        val saved = model.preferences.value
        val menu = mutableStateOf<String?>("Settings")
        val session = Session.parse(JSONObject("""{"id":"settings-test","title":"Test session","active":true,"device_name":"Studio","cwd":"/tmp/project"}"""))
        model.state.value = AppState(paired = true, sessions = listOf(session), rosterStatus = RosterStatus.Ready)
        try {
            compose.runOnUiThread { model.style { it.copy(boardAgeHours = 24) } }
            compose.setContent { MaterialTheme {
                MenuPanel(menu.value, androidx.compose.ui.geometry.Rect(0f, 0f, 800f, 0f), model, {}, { menu.value = it }, { menu.value = null })
            } }
            compose.onNodeWithText("Last 24 hours").performClick()
            compose.onNodeWithText("Last hour").performClick()
            compose.runOnIdle { assertEquals(1, PreferenceStore(context).load().boardAgeHours) }
            compose.onNodeWithText("Show recaps").performClick()
            compose.runOnIdle { assertEquals(!saved.boardRecaps, PreferenceStore(context).load().boardRecaps) }
            compose.onNodeWithText("Watch PR").performScrollTo().performClick()
            compose.onNode(hasText("Watch PR") and hasClickAction()).assertIsNotEnabled()
            compose.onNode(hasSetTextAction()).performTextInput("owner/repo#12")
            compose.onNode(hasText("Watch PR") and hasClickAction()).assertIsEnabled()
            compose.onNodeWithContentDescription("Back to settings").performClick()
            compose.onNodeWithText("New session").performScrollTo().performClick()
            compose.onNodeWithText("/tmp/project").assertExists()
            compose.onNodeWithText("Start session").assertIsEnabled()
            compose.onNodeWithContentDescription("Back to settings").performClick()
            compose.onNodeWithText("Search sessions").performScrollTo().performClick()
            compose.runOnIdle { assertEquals("Search", menu.value) }
        } finally { model.style { saved } }
    }

    @Test fun prGroupingKeepsOlderTouchingSessionsWithinRecentGroup() {
        val model = SessionModel(ApplicationProvider.getApplicationContext<Application>())
        val savedStyle = model.preferences.value
        val savedNavigation = model.navigation.value
        val now = System.currentTimeMillis() / 1000
        val recent = Session.parse(JSONObject("""{"id":"group-new","title":"Recent work","active":true,"device_name":"Studio","state":"complete"}"""))
            .copy(stats = SessionStats(promptAt = now - 10))
        val old = recent.copy(id = "group-old", title = "Older work on same PR", stats = SessionStats(promptAt = now - 172800))
        val pr = PullRequest("owner/repo#1", "Shared pull request", "owner/repo", 1, "OPEN", "", "", listOf(recent, old), primary = true)
        model.state.value = AppState(paired = true, sessions = listOf(recent, old), prs = listOf(pr), rosterStatus = RosterStatus.Ready)
        model.style { it.copy(boardAgeHours = 24, boardGrouping = "sessions", boardComputer = "") }
        model.navigate { it.copy(listMode = false, expanded = true, selectedId = null) }
        try {
            compose.setContent { MaterialTheme { App(model) {} } }
            compose.onNode(hasText(old.title) and hasClickAction()).assertDoesNotExist()
            compose.runOnIdle { model.style { it.copy(boardGrouping = "prs") } }
            compose.onNode(hasText(old.title) and hasClickAction()).assertExists()
            compose.onNode(hasText(recent.title) and hasClickAction()).assertExists()
            compose.onNodeWithText("Older", substring = false).assertDoesNotExist()
            compose.onAllNodesWithText("Shared pull request", substring = true).assertCountEquals(1)
        } finally { model.style { savedStyle }; model.navigate { savedNavigation } }
    }

    @Test fun boardSpacingCondensesBothModesAndToolbarSharesMachineHeading() {
        val context = ApplicationProvider.getApplicationContext<Application>()
        val model = SessionModel(context)
        val savedNavigation = model.navigation.value
        val savedStyle = model.preferences.value
        val session = Session.parse(JSONObject("""{"id":"spacing-test","title":"A deliberately long title that wraps across multiple lines in the session sidebar","active":true,"device_name":"Test machine"}""")).copy(stats = SessionStats(promptAt = System.currentTimeMillis() / 1000))
        model.state.value = AppState(paired = true, sessions = listOf(session), selected = session, rosterStatus = RosterStatus.Ready)
        model.navigate { it.copy(listMode = true, expanded = false, selectedId = session.id) }
        try {
            compose.setContent { CompositionLocalProvider(LocalDensity provides Density(1f)) { MaterialTheme { App(model) {} } } }
            for (mode in listOf("List", "PR board")) {
                if (mode == "PR board") compose.onNodeWithText(mode).performClick()
                compose.runOnIdle { model.style { it.copy(density = "comfortable") } }
                val row = compose.onNode(hasText(session.title) and hasClickAction())
                val normalHeight = row.fetchSemanticsNode().boundsInRoot.height
                compose.mainClock.autoAdvance = false
                compose.runOnIdle { model.style { it.copy(density = "compact") } }
                compose.mainClock.advanceTimeBy(160)
                val movingHeight = row.fetchSemanticsNode().boundsInRoot.height
                compose.mainClock.advanceTimeBy(600)
                val tightHeight = row.fetchSemanticsNode().boundsInRoot.height
                assertTrue("Row should be shrinking midway through the animation", movingHeight < normalHeight && movingHeight > tightHeight)
                compose.mainClock.autoAdvance = true
                compose.onNodeWithText(mode).performClick()
                val search = compose.onNodeWithContentDescription("Search")
                val logo = compose.onNodeWithContentDescription("Crabigator")
                assertTrue(search.fetchSemanticsNode().boundsInRoot.left - logo.fetchSemanticsNode().boundsInRoot.right > 20)
                search.performClick()
                compose.onNode(hasSetTextAction()).assertIsDisplayed()
                search.performClick()
                compose.onNodeWithText(mode).performClick()
            }
            compose.runOnIdle {
                val stored = PreferenceStore(context).load()
                assertEquals("compact", stored.density)
                assertEquals(savedStyle.lineSpacing, stored.lineSpacing)
            }
        } finally {
            model.style { savedStyle }
            model.navigate { savedNavigation }
        }
    }

    @Test fun styleTabsSeparateBoardAndTerminalControls() {
        val prefs = mutableStateOf(UiPreferences())
        compose.setContent { MaterialTheme { StyleMenu(prefs.value, terminalInitially = false) { prefs.value = it(prefs.value) } } }
        compose.onNodeWithText("List / PR board").assertIsSelected()
        compose.onNodeWithText("Row spacing").assertExists()
        compose.onNodeWithText("Line spacing").assertDoesNotExist()
        val terminalSpacing = prefs.value.lineSpacing
        compose.onNodeWithText("Tight").performClick()
        compose.runOnIdle { assertEquals("compact", prefs.value.density); assertEquals(terminalSpacing, prefs.value.lineSpacing) }
        compose.onNodeWithText("Terminal").performClick()
        compose.onNodeWithText("Terminal").assertIsSelected()
        compose.onNodeWithText("Row spacing").assertDoesNotExist()
        compose.onNodeWithText("Line spacing").assertExists()
        compose.onNodeWithText("Relaxed").performClick()
        compose.onNodeWithText("List / PR board").performClick()
        compose.onNodeWithText("Tight").assertIsSelected()
        compose.runOnIdle { assertEquals(170, prefs.value.lineSpacing); assertEquals("compact", prefs.value.density) }
    }

    @Test fun newPromptMovesSessionIntoLastHourOnTheOpenBoard() {
        val model = SessionModel(ApplicationProvider.getApplicationContext<Application>())
        val savedNavigation = model.navigation.value
        val now = System.currentTimeMillis() / 1000
        val portal = Session.parse(JSONObject("""{"id":"portal","title":"Portal Fix and Eval Coverage PRs","active":true,"state":"complete","device_name":"Test machine"}"""))
            .copy(stats = SessionStats(promptAt = now - 7200))
        val older = portal.copy(id = "older", title = "Another older session")
        val recent = portal.copy(id = "recent", title = "Already recent", stats = SessionStats(promptAt = now - 60))
        model.state.value = AppState(paired = true, sessions = listOf(recent, portal, older), rosterStatus = RosterStatus.Ready)
        model.navigate { it.copy(listMode = false, expanded = true, selectedId = null) }
        try {
            compose.setContent { CompositionLocalProvider(LocalDensity provides Density(1f)) { MaterialTheme { App(model) {} } } }
            val row = compose.onNode(hasText(portal.title) and hasClickAction())
            val oldBand = compose.onNodeWithText("1–3 hours")
            assertTrue(row.fetchSemanticsNode().boundsInRoot.top > oldBand.fetchSemanticsNode().boundsInRoot.bottom)
            compose.runOnIdle {
                val prompted = portal.copy(stats = portal.stats.copy(promptAt = now))
                model.state.value = model.state.value.copy(sessions = listOf(recent, prompted, older))
            }
            assertTrue(row.fetchSemanticsNode().boundsInRoot.bottom < oldBand.fetchSemanticsNode().boundsInRoot.top)
            assertTrue(row.fetchSemanticsNode().boundsInRoot.top > compose.onNodeWithText("Last hour").fetchSemanticsNode().boundsInRoot.bottom)
        } finally {
            model.navigate { savedNavigation }
        }
    }

    @Test fun fullscreenBoardsRememberSessionWithoutHighlightingIt() {
        val model = SessionModel(ApplicationProvider.getApplicationContext<Application>())
        val savedNavigation = model.navigation.value
        val session = Session.parse(JSONObject("""{"id":"fullscreen-test","title":"Remember this session","active":true}""")).copy(stats = SessionStats(promptAt = System.currentTimeMillis() / 1000))
        model.state.value = AppState(paired = true, sessions = listOf(session), selected = session, rosterStatus = RosterStatus.Ready)
        model.navigate { it.copy(listMode = true, expanded = false, selectedId = session.id) }
        try {
            compose.setContent {
                // Exercise the tablet layout on both tablet and phone test runners.
                CompositionLocalProvider(LocalDensity provides Density(1f)) {
                    MaterialTheme { App(model) {} }
                }
            }
            for (mode in listOf("List", "PR board")) {
                if (mode == "PR board") compose.onNodeWithText(mode).performClick()
                val row = compose.onNode(hasText(session.title) and hasClickAction())
                row.assertIsSelected()
                compose.onNodeWithText(mode).performClick()
                row.assertIsNotSelected()
                compose.runOnIdle { assertEquals(session.id, model.state.value.selected?.id) }
                compose.onNodeWithText(mode).performClick()
                row.assertIsSelected()
                compose.runOnIdle { assertEquals(session.id, model.state.value.selected?.id) }
            }
        } finally {
            model.navigate { savedNavigation }
        }
    }

    @Test fun draftsSurviveSessionSwitchAndClosingTheDetail() {
        val model = SessionModel(ApplicationProvider.getApplicationContext<Application>())
        val first = Session.parse(JSONObject("""{"id":"draft-a","title":"First"}"""))
        val second = Session.parse(JSONObject("""{"id":"draft-b","title":"Second"}"""))
        val state = mutableStateOf(AppState(selected = first))
        compose.setContent {
            MaterialTheme { SessionPages(state.value, model, false, {}, Modifier, style = {}) }
        }
        compose.onNode(hasSetTextAction()).performTextInput("Keep the first draft")
        compose.runOnIdle { state.value = AppState(selected = second) }
        compose.waitForIdle()
        compose.onNode(hasSetTextAction()).performTextInput("Keep the second draft")
        compose.runOnIdle { state.value = AppState(selected = first) }
        compose.waitForIdle()
        compose.onNode(hasSetTextAction()).assertTextEquals("Keep the first draft")
        compose.runOnIdle { state.value = AppState() }
        compose.waitForIdle()
        compose.runOnIdle { state.value = AppState(selected = second) }
        compose.waitForIdle()
        compose.onNode(hasSetTextAction()).assertTextEquals("Keep the second draft")
    }

    @Test fun codexQuestionsShortcutSendsShiftLeft() {
        val sent = mutableListOf<String>()
        compose.setContent { MaterialTheme { TerminalKeyboard(true, {}, true, true) { sent.add(it) } } }
        compose.onNodeWithText("Questions").performClick()
        compose.runOnIdle { assertEquals(listOf("\u001b[1;2D"), sent) }
    }

    @Test fun slowLoadingOffersReconnectAndOnlyLoadedOutputIsEmpty() {
        val loaded = mutableStateOf(false)
        val active = mutableStateOf(true)
        var reconnects = 0
        compose.setContent {
            MaterialTheme { TerminalPlaceholder(active.value, loaded.value, true, { reconnects++ }) }
        }
        compose.onNodeWithText("Waiting for terminal output…").assertIsDisplayed()
        compose.onNodeWithText("No terminal output").assertDoesNotExist()
        compose.onNodeWithText("Reconnect").performClick()
        compose.runOnIdle { assertEquals(1, reconnects); loaded.value = true }
        compose.onNodeWithText("No terminal output").assertIsDisplayed()
        compose.onNodeWithText("Reconnect").assertDoesNotExist()
        compose.runOnIdle { loaded.value = false; active.value = false }
        compose.onNodeWithText("Terminal unavailable for this ended session").assertIsDisplayed()
    }
}
