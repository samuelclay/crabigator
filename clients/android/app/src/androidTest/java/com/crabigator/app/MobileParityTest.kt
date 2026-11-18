package com.crabigator.app

import android.app.Application
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
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

    private val panelAnchor = androidx.compose.ui.geometry.Rect(0f, 0f, 800f, 0f)
    private fun studioSession() = Session.parse(JSONObject("""{"id":"settings-test","title":"Test session","active":true,"device_name":"Studio","cwd":"/tmp/project"}"""))

    @Test fun viewPanelSavesFiltersGroupingAndEachBoardsOwnRowSet() {
        val context = ApplicationProvider.getApplicationContext<Application>()
        val model = SessionModel(context)
        val saved = model.preferences.value
        val menu = mutableStateOf<String?>("View List")
        model.state.value = AppState(paired = true, sessions = listOf(studioSession()), rosterStatus = RosterStatus.Ready)
        fun stored() = PreferenceStore(context).load()
        try {
            compose.runOnUiThread { model.style { UiPreferences() } }
            compose.setContent { MaterialTheme { MenuPanel(menu.value, panelAnchor, model, {}, { menu.value = it }, { menu.value = null }) } }
            compose.onNodeWithText("View List").assertExists()
            compose.onNodeWithText("24h").assertIsSelected()
            compose.onNodeWithText("1h").performClick().assertIsSelected()
            compose.runOnIdle { assertEquals(1, stored().boardAgeHours) }
            compose.onNodeWithText("Ended sessions").assertIsOff().performClick().assertIsOn()
            compose.runOnIdle { assertTrue(stored().showEnded) }
            compose.onNodeWithText("Studio").performClick().assertIsSelected()
            compose.runOnIdle { assertEquals("Studio", stored().boardComputer) }
            // One picker reads and writes group-by-computer and project grouping together.
            compose.onNodeWithText("Computer").assertIsSelected()
            compose.onNodeWithText("Sort projects").assertDoesNotExist()
            compose.onNodeWithText("Project").performScrollTo().performClick().assertIsSelected()
            compose.runOnIdle { assertEquals(false to "project", stored().let { it.groupByComputer to it.grouping }) }
            compose.onNodeWithText("A to Z").performScrollTo().performClick().assertIsSelected()
            compose.runOnIdle { assertEquals("alpha", stored().order) }
            compose.onNodeWithText("Computer, then project").performScrollTo().performClick().assertIsSelected()
            compose.runOnIdle { assertEquals(true to "project", stored().let { it.groupByComputer to it.grouping }) }
            // The List's row set is saved on its first change and leaves the PR board's alone.
            compose.onNodeWithText("Last prompt").performScrollTo().assertIsOn()
            compose.onNodeWithText("Git status").performScrollTo().assertIsOff().performClick().assertIsOn()
            compose.runOnIdle { assertTrue("git" in stored().listRow!!); assertEquals(null, stored().boardRow) }
            compose.onNodeWithText("The PR board keeps its own set.").assertExists()
            compose.onNodeWithText("Relaxed").performScrollTo().performClick().assertIsSelected()
            compose.onNodeWithContentDescription("Larger text").performClick()
            compose.onNodeWithText("2").performScrollTo().performClick().assertIsSelected()
            compose.runOnIdle { assertEquals(Triple("relaxed", 15, 2), stored().let { Triple(it.density, it.boardFontSize, it.columns) }) }
            compose.onNodeWithText("Each block starts with").assertDoesNotExist()
            // The PR board offers no project choice or columns, and keeps its own row set.
            compose.runOnIdle { menu.value = "View PR board" }
            compose.onNodeWithText("View PR board").assertExists()
            compose.onNodeWithText("Project").assertDoesNotExist()
            compose.onNodeWithText("Columns").assertDoesNotExist()
            compose.onNodeWithText("One list").performScrollTo().performClick().assertIsSelected()
            compose.runOnIdle { assertEquals(false to "project", stored().let { it.groupByComputer to it.grouping }) }
            compose.onNodeWithText("The pull request").performScrollTo().performClick().assertIsSelected()
            compose.runOnIdle { assertEquals("prs", stored().boardGrouping) }
            compose.onNodeWithText("Git status").performScrollTo().assertIsOff()
            compose.onNodeWithText("Changes").performScrollTo().performClick().assertIsOn()
            compose.runOnIdle { assertEquals(setOf("promptRecency", "completionRecency", "changes"), stored().boardRow) }
            compose.onNodeWithText("The List keeps its own set.").assertExists()
            compose.onNodeWithText("Done").performClick()
            compose.runOnIdle { assertEquals(null, menu.value) }
        } finally { model.style { saved } }
    }

    @Test fun accountPageHoldsNoViewChoicesOrBoardActions() {
        val model = SessionModel(ApplicationProvider.getApplicationContext<Application>())
        val menu = mutableStateOf<String?>("Account")
        val account = JSONObject("""{"identities":[{"provider":"github","email":"dev@example.com"}]}""")
        compose.setContent { MaterialTheme { CompositionLocalProvider(LocalAccountSource provides { account }) {
            MenuPanel(menu.value, panelAnchor, model, {}, { menu.value = it }, { menu.value = null })
        } } }
        listOf("Alerts and history", "Sign-in", "Devices and agents", "This device").forEach { compose.onNodeWithText(it).performScrollTo().assertExists() }
        compose.onNodeWithText("dev@example.com with GitHub").assertExists()
        listOf("Voice transcripts", "Manage sign-in on the web", "Pair another device", "Connect an agent with MCP", "Tools and examples", "Privacy policy").forEach { compose.onNodeWithText(it).assertExists() }
        listOf("Search sessions", "New session", "Watch PR", "Style", "Show recaps", "Group by computer", "Session time").forEach { compose.onNodeWithText(it).assertDoesNotExist() }
        compose.onNodeWithText("Notifications").performScrollTo().performClick()
        compose.runOnIdle { assertEquals("Notifications", menu.value) }
        compose.onNodeWithText("Questions and permissions").assertExists()
        compose.onNodeWithContentDescription("Back to account").performClick()
        compose.onNodeWithText("Unpair this device").performScrollTo().performClick()
        compose.onNodeWithText("Unpair this device?").assertExists()
        compose.onNodeWithText("Cancel").performClick()
        compose.onNodeWithText("Unpair this device").assertExists()
    }

    @Test fun boardHeaderHoldsTheFiltersAndTheActionsThatLeftSettings() {
        val context = ApplicationProvider.getApplicationContext<Application>()
        val model = SessionModel(context)
        val savedStyle = model.preferences.value
        val savedNavigation = model.navigation.value
        model.state.value = AppState(paired = true, sessions = listOf(studioSession()), rosterStatus = RosterStatus.Ready)
        model.style { UiPreferences() }
        model.navigate { it.copy(conductor = false, listMode = true, expanded = true, selectedId = null) }
        try {
            compose.setContent { MaterialTheme { CompositionLocalProvider(LocalAccountSource provides { JSONObject() }) { App(model) {} } } }
            compose.onNodeWithContentDescription("Settings").assertDoesNotExist()
            compose.onNodeWithContentDescription("New session or watch a pull request").performClick()
            compose.onNodeWithText("Watch a pull request").performClick()
            compose.onNode(hasText("Watch PR") and hasClickAction()).assertIsNotEnabled()
            compose.onNode(hasSetTextAction()).performTextInput("owner/repo#12")
            compose.onNode(hasText("Watch PR") and hasClickAction()).assertIsEnabled()
            compose.onNodeWithText("Done").performClick()
            // Each chip names its filter's value and changes the saved preference.
            compose.onNodeWithText("24h").performClick()
            compose.onNodeWithText("Last hour").performClick()
            compose.onNodeWithText("1h").assertExists()
            compose.onNodeWithText("Live").performClick()
            compose.onNodeWithText("Live and ended").performClick()
            compose.onNodeWithText("Live + ended").assertExists()
            compose.onNodeWithText("All computers").performClick()
            compose.onNodeWithText("Studio").performClick()
            compose.runOnIdle { assertEquals(Triple(1, true, "Studio"), PreferenceStore(context).load().let { Triple(it.boardAgeHours, it.showEnded, it.boardComputer) }) }
            // View opens the panel for the mode on screen, and shows the same filters.
            compose.onNodeWithText("View").performClick()
            compose.onNodeWithText("View List").assertExists()
            compose.onNodeWithText("1h").assertIsSelected()
            compose.onNodeWithText("Done").performClick()
            compose.onNodeWithText("PR board").performClick()
            compose.onNodeWithText("View").performClick()
            compose.onNodeWithText("View PR board").assertExists()
            compose.onNodeWithText("Done").performClick()
            compose.onNodeWithContentDescription("Account and notifications").performClick()
            compose.onNodeWithText("Account").assertExists()
        } finally { model.style { savedStyle }; model.navigate { savedNavigation } }
    }

    @Test fun tabletHeaderButtonExpandsTheBoardAndReturnsToTheSession() {
        val model = SessionModel(ApplicationProvider.getApplicationContext<Application>())
        val savedNavigation = model.navigation.value
        val session = Session.parse(JSONObject("""{"id":"expand-test","title":"Keep this session","active":true}""")).copy(stats = SessionStats(promptAt = System.currentTimeMillis() / 1000))
        model.state.value = AppState(paired = true, sessions = listOf(session), selected = session, rosterStatus = RosterStatus.Ready)
        model.navigate { it.copy(conductor = false, listMode = true, expanded = false, selectedId = session.id) }
        try {
            // A density of 1 gives the tablet layout on every test runner.
            compose.setContent { CompositionLocalProvider(LocalDensity provides Density(1f)) { MaterialTheme { App(model) {} } } }
            val row = compose.onNode(hasText(session.title) and hasClickAction())
            row.assertIsSelected()
            compose.onNodeWithContentDescription("Expand List / PR board").performClick()
            compose.runOnIdle { assertTrue(model.navigation.value.expanded) }
            row.assertIsNotSelected()
            compose.onNodeWithContentDescription("Return to session").performClick()
            compose.runOnIdle { assertEquals(false, model.navigation.value.expanded); assertEquals(session.id, model.state.value.selected?.id) }
            row.assertIsSelected()
        } finally { model.navigate { savedNavigation } }
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
                compose.runOnIdle { model.style { it.copy(density = "comfortable", boardFontSize = 14) } }
                val row = compose.onNode(hasText(session.title) and hasClickAction())
                val normalHeight = row.fetchSemanticsNode().boundsInRoot.height
                compose.mainClock.autoAdvance = false
                compose.runOnIdle { model.style { it.copy(density = "compact", boardFontSize = 10) } }
                compose.mainClock.advanceTimeBy(160)
                val movingHeight = row.fetchSemanticsNode().boundsInRoot.height
                compose.mainClock.advanceTimeBy(600)
                val tightHeight = row.fetchSemanticsNode().boundsInRoot.height
                val rowBounds = row.fetchSemanticsNode().boundsInRoot
                val glyph = compose.onAllNodesWithText(session.glyph, useUnmergedTree = true).fetchSemanticsNodes().single { rowBounds.contains(it.boundsInRoot.center) }.boundsInRoot
                val title = compose.onAllNodesWithText(session.title, useUnmergedTree = true).fetchSemanticsNodes().single { rowBounds.contains(it.boundsInRoot.center) }.boundsInRoot
                assertEquals("Session glyph must center on the one-line title", title.center.y, glyph.center.y, 2f)
                assertTrue("Row should be shrinking midway through the animation", movingHeight < normalHeight && movingHeight > tightHeight)
                compose.mainClock.autoAdvance = true
                compose.onNodeWithText(mode).performClick()
                val search = compose.onNodeWithContentDescription("Search")
                val logo = compose.onNodeWithContentDescription("Crabigator")
                assertTrue(search.fetchSemanticsNode().boundsInRoot.left - logo.fetchSemanticsNode().boundsInRoot.right > 20)
                val logoBounds = logo.fetchSemanticsNode().boundsInRoot
                val searchBounds = search.fetchSemanticsNode().boundsInRoot
                compose.onRoot().performTouchInput { click(androidx.compose.ui.geometry.Offset((logoBounds.right + searchBounds.left) / 2, searchBounds.center.y)) }
                row.assertExists()
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

    @Test fun sessionViewPanelChangesTheTerminalTheDrawerAndTheStatsStrip() {
        val prefs = mutableStateOf(UiPreferences())
        var transcripts = 0
        compose.setContent { MaterialTheme { Column(Modifier.verticalScroll(rememberScrollState())) {
            SessionViewPanel(prefs.value, { prefs.value = it(prefs.value) }) { transcripts++ }
        } } }
        compose.onNodeWithText("Normal").assertIsSelected()
        compose.onNodeWithText("Relaxed").performClick().assertIsSelected()
        compose.onNodeWithContentDescription("Larger text").performClick()
        // The terminal's size and spacing are its own: the board's stay as they were.
        compose.runOnIdle { assertEquals(listOf(170, 14, "comfortable", 14), prefs.value.let { listOf(it.lineSpacing, it.fontSize, it.density, it.boardFontSize) }) }
        compose.onNodeWithText("Wrap").assertIsSelected()
        compose.onNodeWithText("Scroll sideways").performScrollTo().performClick().assertIsSelected()
        compose.onNodeWithText("Fill").assertIsSelected()
        compose.onNodeWithText("Tall").performScrollTo().performClick().assertIsSelected()
        compose.onNodeWithText("Tucked away").performScrollTo().performClick().assertIsSelected()
        compose.runOnIdle { assertEquals(Triple(false, 500, false), prefs.value.let { Triple(it.wrap, it.terminalHeight, it.widgets) }) }
        compose.onNodeWithText("Flow scene").performScrollTo().assertIsOn().performClick().assertIsOff()
        compose.runOnIdle { assertEquals(false, prefs.value.visible("flow")) }
        // The stats strip keeps its own set; prompts and completions start on. Context starts off on a fresh install.
        compose.onNodeWithText("Completions").performScrollTo().assertIsOn()
        compose.onNodeWithText("Context").performScrollTo().assertIsOff()
        compose.onNodeWithText("Prompts").performScrollTo().assertIsOn().performClick().assertIsOff()
        compose.runOnIdle { assertEquals(setOf("sessionTime", "thinkingTime", "completions"), prefs.value.sessionStats); assertEquals(null, prefs.value.listRow) }
        compose.onNodeWithText("Voice transcripts for this session").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(1, transcripts) }
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
            MaterialTheme { SessionPages(state.value, model, false, {}, Modifier, view = {}) }
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
