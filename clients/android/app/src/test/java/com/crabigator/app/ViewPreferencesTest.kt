package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ViewPreferencesTest {
    private val timing = setOf("sessionTime", "thinkingTime", "promptRecency", "completionRecency")
    private val recency = setOf("promptRecency", "completionRecency")
    private val strip = setOf("sessionTime", "thinkingTime", "prompts", "completions")

    @Test fun freshInstallShowsNoContextInAnySet() {
        // Tools and compactions start hidden, so not every stat is on, and the old Context switch starts off.
        val fresh = UiPreferences()
        assertEquals(timing, fresh.listRowItems)
        assertEquals(recency, fresh.boardRowItems)
        assertEquals(strip, fresh.sessionStatItems)
    }

    @Test fun oldContextSwitchTurnsContextOnForListRowsAndTheStatsStripOnly() {
        val old = UiPreferences(shown = setOf("context"))
        assertEquals(timing + "context", old.listRowItems)
        assertEquals(strip + "context", old.sessionStatItems)
        assertEquals("PR board rows start without context", recency, old.boardRowItems)
    }

    @Test fun contextStartsOnWhenEveryOtherStatTheSetShowsIsOn() {
        val everything = UiPreferences(hidden = emptySet())
        assertEquals(timing + setOf("tools", "compactions", "context"), everything.listRowItems)
        assertEquals(strip + setOf("tools", "compactions", "context"), everything.sessionStatItems)
        assertEquals(recency, everything.boardRowItems)
        // A hidden drawer widget is not a stat, so it does not count.
        assertTrue("context" in UiPreferences(hidden = setOf("flow", "git")).listRowItems)
        // One stat off is enough to leave context off.
        val noTools = UiPreferences(hidden = setOf("tools"))
        assertFalse("context" in noTools.listRowItems || "context" in noTools.sessionStatItems)
        // Each set looks only at the switches it shows. The strip has no recency switch; prompts and completions never had one and count as on.
        val noRecency = UiPreferences(hidden = setOf("promptRecency"))
        assertFalse("context" in noRecency.listRowItems)
        assertTrue("context" in noRecency.sessionStatItems)
        assertEquals(strip + setOf("tools", "compactions", "context"), UiPreferences(hidden = setOf("prompts", "completions")).sessionStatItems)
    }

    @Test fun aSavedContextChoiceSticksWhateverTheOldSwitchesSay() {
        // Turned off where it started on: it stays off, also after a save and reload.
        val off = UiPreferences(hidden = emptySet(), shown = setOf("context")).toggleListRow("context").toggleSessionStat("context")
        assertFalse("context" in off.listRow!! || "context" in off.sessionStats!!)
        val reloaded = preferencesFromJson(JSONObject(off.toJson().toString()))
        assertEquals(off, reloaded)
        assertFalse("context" in reloaded.listRowItems || "context" in reloaded.sessionStatItems)
        // Turned on where it started off: it stays on, also when the old switches would leave it off.
        val on = UiPreferences().toggleListRow("context").toggleBoardRow("context").toggleSessionStat("context")
        assertEquals(listOf(timing + "context", recency + "context", strip + "context"), listOf(on.listRow, on.boardRow, on.sessionStats))
        val later = preferencesFromJson(JSONObject(on.copy(hidden = setOf("tools", "compactions", "sessionTime")).toJson().toString()))
        assertTrue("context" in later.listRowItems && "context" in later.boardRowItems && "context" in later.sessionStatItems)
    }

    @Test fun listRowStartsFromTheOldVisibleStatsAndShowRecaps() {
        val old = UiPreferences(hidden = setOf("sessionTime"), shown = setOf("context"), boardRecaps = true)
        assertEquals(setOf("thinkingTime", "promptRecency", "completionRecency", "tools", "compactions", "context", "recap"), old.listRowItems)
        // Git status and changes are new on rows, so they start off whatever the old choices were.
        assertFalse(UiPreferences(hidden = emptySet(), shown = setOf("context"), boardRecaps = true).listRowItems.any { it == "git" || it == "changes" })
    }

    @Test fun boardRowStartsWithRecencyAndFollowsOnlyShowRecaps() {
        assertEquals(recency + "recap", UiPreferences(boardRecaps = true).boardRowItems)
        // The old PR board rows had no stats, so the visible-stats choices do not reach them.
        assertEquals(recency, UiPreferences(hidden = setOf("promptRecency", "sessionTime"), shown = setOf("context")).boardRowItems)
    }

    @Test fun sessionStatsStartFromTheOldChoicesWithPromptsAndCompletionsOn() {
        val old = UiPreferences(hidden = setOf("thinkingTime", "prompts", "completions"), shown = setOf("context"))
        assertEquals(setOf("sessionTime", "prompts", "completions", "tools", "compactions", "context"), old.sessionStatItems)
    }

    @Test fun aChangedSetIsSavedAndStopsFollowingTheOldChoices() {
        val changed = UiPreferences().toggleListRow("git")
        assertEquals(timing + "git", changed.listRow)
        assertEquals(timing + "git", changed.copy(hidden = emptySet(), shown = setOf("context"), boardRecaps = true).listRowItems)
        assertEquals(timing, changed.toggleListRow("git").listRowItems)
        // The three sets are separate: changing one leaves the others following the old choices.
        assertNull(changed.boardRow)
        assertNull(changed.sessionStats)
        assertEquals(recency + "tools", UiPreferences().toggleBoardRow("tools").boardRow)
        assertEquals(strip - "prompts", UiPreferences().toggleSessionStat("prompts").sessionStats)
        // Turning off every item leaves an empty saved set, not the derived one.
        val empty = recency.fold(UiPreferences()) { p, key -> p.toggleBoardRow(key) }
        assertEquals(emptySet<String>(), empty.boardRowItems)
    }

    @Test fun oldSavesKeepEveryChoiceAndNewSetsRoundTrip() {
        val old = preferencesFromJson(JSONObject("""{"fontSize":15,"lineSpacing":170,"wrap":false,"columns":2,"terminalHeight":350,"modulesExpanded":false,"grouping":"project","order":"alpha","density":"compact","sidebarRight":true,"hidden":["tools","flow"],"shown":["context"],"showEnded":true,"boardFontSize":16,"boardAgeHours":6,"boardComputer":"Studio","boardRecaps":true,"boardGrouping":"prs","groupByComputer":false}"""))
        assertEquals(UiPreferences(15, 170, false, 2, 350, false, "project", "alpha", "compact", true, setOf("tools", "flow"), true, 16, 6, "Studio", true, "prs", false, setOf("context")), old)
        assertNull(old.listRow); assertNull(old.boardRow); assertNull(old.sessionStats)
        // A set that was never changed is not written, so it keeps following the old choices after a save.
        val saved = old.toJson()
        assertFalse(saved.has("listRow") || saved.has("boardRow") || saved.has("sessionStats"))
        assertEquals(old, preferencesFromJson(saved))
        val changed = old.toggleListRow("changes").toggleBoardRow("git").toggleSessionStat("tools")
        val restored = preferencesFromJson(JSONObject(changed.toJson().toString()))
        assertEquals(changed, restored)
        assertTrue("changes" in restored.listRowItems && "git" in restored.boardRowItems && "tools" in restored.sessionStatItems)
        assertEquals(emptySet<String>(), preferencesFromJson(JSONObject("""{"boardRow":[]}""")).boardRowItems)
        assertEquals(UiPreferences(), preferencesFromJson(JSONObject()))
    }

    @Test fun groupByReadsAndWritesTheTwoSavedPreferences() {
        val saved = mapOf(GroupBy.OneList to (false to "all"), GroupBy.Computer to (true to "all"), GroupBy.Project to (false to "project"), GroupBy.ComputerThenProject to (true to "project"))
        for ((choice, stored) in saved) {
            assertEquals(choice, UiPreferences(groupByComputer = stored.first, grouping = stored.second).groupBy)
            val written = UiPreferences(order = "alpha").grouped(choice)
            assertEquals(stored, written.groupByComputer to written.grouping)
            assertEquals("Project order must survive a change of grouping", "alpha", written.order)
        }
        assertEquals(listOf("One list", "Computer", "Project", "Computer, then project"), GroupBy.entries.map { it.label })
        // The default is today's default: a section for each computer, projects together.
        assertEquals(GroupBy.Computer, UiPreferences().groupBy)
    }

    @Test fun prBoardGroupingSharesTheComputerChoiceAndLeavesProjectsAlone() {
        assertEquals(listOf(GroupBy.OneList, GroupBy.Computer), GroupBy.board)
        val list = UiPreferences().grouped(GroupBy.ComputerThenProject)
        assertEquals(GroupBy.Computer, list.boardGroupBy)
        val board = list.boardGrouped(GroupBy.OneList)
        assertEquals(GroupBy.OneList, board.boardGroupBy)
        assertEquals("The PR board's choice also moves the List, which keeps its projects", GroupBy.Project, board.groupBy)
    }

    @Test fun filterChipsNameTheSavedFilters() {
        assertEquals(listOf("1h", "3h", "6h", "9h", "12h", "24h", "All"), BoardFilters.hours.map(BoardFilters::short))
        assertEquals(listOf("Last hour", "Last 3 hours", "Last 6 hours", "Last 9 hours", "Last 12 hours", "Last 24 hours", "All ages"), BoardFilters.hours.map(BoardFilters::long))
        assertEquals(listOf(51, 45, 39, 33, 27, 25, 245).map(TerminalText::palette), BoardFilters.hours.map(BoardFilters::color))
        assertEquals("Live", BoardFilters.sessions(false)); assertEquals("Live + ended", BoardFilters.sessions(true))
        fun session(machine: String) = Session.parse(JSONObject().put("id", machine).put("device_name", machine))
        assertEquals(listOf("Book", "Gone", "Studio"), BoardFilters.computers(listOf(session("Studio"), session("Book"), session("Studio")), "Gone"))
    }

    @Test fun boardSessionsCarryContextAndUncommittedWork() {
        val session = Session.parse(JSONObject("""{"session_id":"s","branch":"main","uncommitted":3,"additions":212,"deletions":48,"stats":{"tools":212,"compressions":3,"context_used":186002,"context_limit":1000000}}"""))
        assertEquals(listOf(3, 212, 48), listOf(session.uncommitted, session.additions, session.deletions))
        assertEquals("186k / 1M · 19%", session.stats.context?.label)
        assertEquals(75, session.stats.context?.palette)
        assertEquals(listOf(75, 220, 220, 203), listOf(69, 70, 84, 85).map { ContextUsage(it.toLong(), 100).palette })
        // The Worker sends null for a count it does not have; context then shows nothing.
        for (stats in listOf("""{"context_used":null,"context_limit":1000000}""", """{"context_used":5,"context_limit":null}""", """{"tools":1}""")) {
            assertNull(stats, Session.parse(JSONObject("""{"session_id":"s","stats":$stats}""")).stats.context)
        }
        val bare = Session.parse(JSONObject("""{"session_id":"s"}"""))
        assertEquals(listOf(0, 0, 0), listOf(bare.uncommitted, bare.additions, bare.deletions))
        assertNull(bare.stats.context)
    }
}
