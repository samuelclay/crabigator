package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class RosterOrderTest {
    private val now = 100_000L
    private fun session(id: String, age: Long, repo: String = "app", machine: String = "Studio") = Session.parse(JSONObject()
        .put("id", id).put("title", id).put("device_name", machine).put("repo_name", repo)
        .put("prompts_changed_at", now - age).put("state", "complete"))
    private fun update(cache: RosterOrder, sessions: List<Session>, at: Long, prs: List<PullRequest> = emptyList(), grouping: String = "all", order: String = "recent") =
        cache.update(sessions, prs, listOf("Studio", "Book"), now + at, at, grouping, order)
    private fun RosterLayout.ids() = list.flatMap { it.sessions }.map { it.id }
    private fun RosterLayout.boardIds() = board.flatMap { it.sessions }.map { it.session.id }

    @Test fun statusAndTimingStayLiveButPositionsOnlyRefreshAfterTenMinutes() {
        val cache = RosterOrder()
        val a = session("a", 10)
        val b = session("b", 100)
        assertEquals(listOf("a", "b"), update(cache, listOf(a, b), 0).ids())
        val asking = b.copy(title = "New question", state = "question", stats = b.stats.copy(promptAt = now + 20))
        for (at in listOf(20L, 120L, 599L)) {
            val live = update(cache, listOf(asking, a), at)
            assertEquals(listOf("a", "b"), live.ids())
            assertEquals(listOf("a", "b"), live.boardIds())
            assertEquals(asking, live.list.single().sessions.last())
            assertEquals(asking, live.board.single().sessions.last().session)
            assertEquals(listOf("b"), live.filtered(false, "New question").ids())
        }
        val refreshed = update(cache, listOf(a, asking), 600)
        assertEquals(listOf("b", "a"), refreshed.ids())
        assertEquals(listOf("b", "a"), refreshed.boardIds())
        val renewed = a.copy(state = "question", stats = a.stats.copy(promptAt = now + 601))
        assertEquals(listOf("b", "a"), update(cache, listOf(renewed, asking), 1199).ids())
        assertEquals(listOf("a", "b"), update(cache, listOf(renewed, asking), 1200).ids())
    }

    @Test fun timeBandsStayCurrentWhileRepoAndPrOrderRemainStable() {
        val cache = RosterOrder()
        val a = session("a", 3590, "z")
        val b = session("b", 7200, "a")
        val first = PullRequest("z#1", "First", "z", 1, "OPEN", "", "", listOf(a), primary = true)
        val second = first.copy(key = "other#2", title = "Second", repo = "other", number = 2, primary = false)
        val initial = update(cache, listOf(a, b), 0, listOf(first, second), "project")
        val working = b.copy(state = "thinking", stats = b.stats.copy(promptAt = now + 50))
        val failing = second.copy(checks = "1 failed", checksFailed = 1)
        val held = update(cache, listOf(a, working), 599, listOf(failing, first), "project")
        assertEquals(listOf(BoardAge.Hour, BoardAge.Three), held.board.single().bands.map { it.age })
        assertEquals("b", held.board.single().bands.first().repositories.single().sessions.single().session.id)
        assertEquals("a", held.board.single().bands.last().repositories.single().sessions.single().session.id)
        assertEquals(initial.list.map { it.project }, held.list.map { it.project })
        val heldA = held.board.single().sessions.first { it.session.id == "a" }
        assertEquals(listOf(first.key, second.key), heldA.prs.map { it.key })
        assertEquals("1 failed", heldA.prs.last().checks)
        val refreshed = update(cache, listOf(a, working), 600, listOf(failing, first), "project")
        assertEquals(listOf(BoardAge.Hour, BoardAge.Three), refreshed.board.single().bands.map { it.age })
        assertEquals("b", refreshed.boardIds().first())
        assertEquals("other", refreshed.board.single().bands.last().repositories.single().repo)
        assertEquals(listOf(second.key, first.key), refreshed.board.single().sessions.last().prs.map { it.key })
        assertEquals(listOf("a", "z"), refreshed.list.map { it.project })
    }

    @Test fun newPromptMovesIntoLastHourWithoutReorderingOtherRows() {
        val cache = RosterOrder()
        val recent = session("recent", 10)
        val other = session("other-recent", 20)
        val portal = session("portal", 7200)
        val old = session("old", 8000)
        val initial = update(cache, listOf(recent, other, portal, old), 0)
        assertEquals(listOf("portal", "old"), initial.board.single().bands.last().repositories.single().sessions.map { it.session.id })

        // A quick response may already be complete at the next roster update.
        val prompted = portal.copy(stats = portal.stats.copy(promptAt = now + 1))
        val completed = old.copy(stats = old.stats.copy(completionAt = now + 1))
        val live = update(cache, listOf(prompted, completed, other, recent), 1)
        assertEquals(listOf(BoardAge.Hour, BoardAge.Three), live.board.single().bands.map { it.age })
        assertEquals(listOf("recent", "other-recent", "portal"), live.board.single().bands.first().repositories.single().sessions.map { it.session.id })
        assertEquals(listOf("old"), live.board.single().bands.last().repositories.single().sessions.map { it.session.id })
        assertEquals(listOf("recent", "other-recent", "portal", "old"), live.ids())
        assertEquals("portal", update(cache, listOf(prompted, completed, other, recent), 600).boardIds().first())
    }

    @Test fun workingSessionMovesUpBeforeItsPromptTimestampArrives() {
        val cache = RosterOrder()
        val portal = session("portal", 7200)
        update(cache, listOf(portal), 0)
        val working = portal.copy(state = "thinking")
        assertEquals(listOf(BoardAge.Hour), update(cache, listOf(working), 1).board.single().bands.map { it.age })
        val settled = portal.copy(stats = portal.stats.copy(promptAt = now + 1))
        val aged = cache.update(listOf(settled), emptyList(), listOf("Studio"), now + 3601, 2, "all", "recent")
        assertEquals(listOf(BoardAge.Three), aged.board.single().bands.map { it.age })
    }

    @Test fun filtersHideOldSessionsInBothViewsWithoutChangingSavedOrder() {
        val cache = RosterOrder()
        val fresh = session("fresh", 10)
        val old = session("old", 7200)
        val working = session("working", 100_000).copy(state = "thinking")
        val book = session("book", 20, machine = "Book")
        val source = update(cache, listOf(fresh, old, working, book), 0)
        val prefs = UiPreferences(boardAgeHours = 1, boardComputer = "Studio")
        val filtered = source.filtered(false, "", prefs, now)
        assertEquals(setOf("fresh", "working"), filtered.ids().toSet())
        assertEquals(filtered.ids().toSet(), filtered.boardIds().toSet())
        assertEquals(source.ids(), source.filtered(true, "", prefs.copy(boardAgeHours = 0, boardComputer = ""), now).ids())
        for (hours in listOf(1, 3, 6, 9, 12, 24)) {
            assertTrue(prefs.copy(boardAgeHours = hours).includesActivity(now - hours * 3600 + 1, now))
            assertFalse(prefs.copy(boardAgeHours = hours).includesActivity(now - hours * 3600 - 1, now))
        }
    }

    @Test fun prGroupingCombinesTouchingSessionsAcrossTimeBands() {
        val a = session("a", 10)
        val b = session("b", 7200)
        val pr = PullRequest("owner/repo#1", "Shared PR", "owner/repo", 1, "OPEN", "", "", listOf(a, b), primary = true)
        val prefs = UiPreferences(boardAgeHours = 1, boardGrouping = "prs")
        val layout = update(RosterOrder(), listOf(a, b), 0, listOf(pr))
        val visible = layout.filtered(true, "", prefs, now)
        assertEquals(listOf("a"), visible.ids())
        assertEquals(listOf("a"), layout.filtered(true, "", prefs.copy(boardGrouping = "sessions"), now).boardIds())
        val grouped = groupBoardByPullRequest(visible.board, now) { prefs.includesActivity(it, now) }.single()
        assertEquals(listOf(BoardAge.Hour), grouped.bands.map { it.age })
        val block = grouped.bands.single().repositories.single().pullRequests.single()
        assertEquals(listOf("a", "b"), block.sessions.map { it.session.id })
        assertTrue(grouped.bands.single().repositories.single().sessions.isEmpty())
    }

    @Test fun prGroupsUseTheirOwnEventsOnlyWhenNoSessionHasPrompted() {
        val never = session("never", now)
        val primary = PullRequest("owner/repo#1", "Old primary", "owner/repo", 1, "OPEN", "", "", listOf(never), primary = true, updatedAt = now - 7200)
        val secondary = primary.copy(key = "owner/repo#2", number = 2, primary = false, updatedAt = now)
        val prefs = UiPreferences(boardAgeHours = 1, boardGrouping = "prs")
        val source = prBoardGroups(listOf(never), listOf(primary, secondary), listOf("Studio"), now, true, "")
        assertTrue(groupBoardByPullRequest(source, now) { prefs.includesActivity(it, now) }.isEmpty())
        val fresh = primary.copy(updatedAt = now)
        val noPrompt = prBoardGroups(listOf(never), listOf(fresh), listOf("Studio"), now, true, "")
        assertEquals(BoardAge.Hour, groupBoardByPullRequest(noPrompt, now).single().bands.single().age)
        val prompted = session("never", 7200)
        val oldPrompt = prBoardGroups(listOf(prompted), listOf(fresh), listOf("Studio"), now, true, "")
        assertTrue(groupBoardByPullRequest(oldPrompt, now) { prefs.includesActivity(it, now) }.isEmpty())
    }

    @Test fun watchTargetParsingRejectsInvalidTargetsAndUsesNewWatchTime() {
        assertEquals(12, BoardActions.watchTarget("owner/repo#12")?.optInt("number"))
        assertEquals("repo", BoardActions.watchTarget("https://github.com/owner/repo/pull/12/files")?.text("repo"))
        assertNull(BoardActions.watchTarget("https://other.test/owner/repo/pull/12"))
        assertNull(BoardActions.watchTarget("owner/repo#0"))
        val watched = PullRequest.parse(JSONObject("""{"owner":"owner","repo":"repo","number":12,"updated_at":100,"pr":{"watched":true}}"""))
        assertEquals(100L, watched.updatedAt)
    }

    @Test fun newSessionsRefreshRemovedRowsDisappearAndFiltersDoNotResetPositions() {
        val cache = RosterOrder()
        val a = session("a", 100)
        val b = session("b", 200)
        val book = session("book", 300, machine = "Book")
        update(cache, listOf(a, b, book), 0)
        val fresh = session("new", 0)
        val extraRepo = session("extra", 1, "other")
        val ended = b.copy(active = false)
        val live = update(cache, listOf(fresh, extraRepo, a, ended, book), 30)
        assertEquals(listOf("new", "extra", "a", "b", "book"), live.ids())
        assertEquals(listOf("new", "a", "b", "extra", "book"), live.boardIds())
        assertEquals(listOf("new", "extra", "a", "book"), live.filtered(false, "").ids())
        assertEquals(listOf("new", "extra", "a", "b", "book"), live.filtered(true, "").ids())
        val removed = update(cache, listOf(fresh, extraRepo, ended, book), 40)
        assertEquals(listOf("new", "extra", "b", "book"), removed.ids())
        assertEquals(listOf("Studio", "Book"), removed.board.map { it.machine })
        val changedGrouping = update(cache, listOf(fresh, extraRepo, ended, book), 50, grouping = "project", order = "alpha")
        assertEquals(listOf("app", "other", "app"), changedGrouping.list.map { it.project })
    }

    @Test fun aNewSessionRefreshesBothViewsAndRestartsTheTenMinuteTimer() {
        val cache = RosterOrder()
        val a = session("a", 100)
        val b = session("b", 200)
        update(cache, listOf(a, b), 0)
        val activeB = b.copy(stats = b.stats.copy(promptAt = now + 50))
        assertEquals(listOf("a", "b"), update(cache, listOf(a, activeB), 50).ids())

        val fresh = session("new", -120)
        val refreshed = update(cache, listOf(a, activeB, fresh), 120)
        assertEquals(listOf("new", "b", "a"), refreshed.ids())
        assertEquals(listOf("new", "b", "a"), refreshed.boardIds())

        val activeA = a.copy(stats = a.stats.copy(promptAt = now + 121))
        for (at in listOf(121L, 600L, 719L)) {
            val held = update(cache, listOf(activeA, activeB, fresh), at)
            assertEquals(listOf("new", "b", "a"), held.ids())
            assertEquals(listOf("new", "b", "a"), held.boardIds())
        }
        val next = update(cache, listOf(activeA, activeB, fresh), 720)
        assertEquals(listOf("a", "new", "b"), next.ids())
        assertEquals(listOf("a", "new", "b"), next.boardIds())
    }

    @Test fun aPreviouslySeenSessionReturningDoesNotRestartTheTimer() {
        val cache = RosterOrder()
        val a = session("a", 100)
        val b = session("b", 200)
        update(cache, listOf(a, b), 0)
        update(cache, listOf(a), 50)
        val returning = b.copy(stats = b.stats.copy(promptAt = now + 100))
        val held = update(cache, listOf(returning, a), 100)
        assertEquals(listOf("a", "b"), held.ids())
        assertEquals(listOf("a", "b"), held.boardIds())
        val refreshed = update(cache, listOf(returning, a), 600)
        assertEquals(listOf("b", "a"), refreshed.ids())
        assertEquals(listOf("b", "a"), refreshed.boardIds())
    }

    @Test fun returningMachineKeepsItsPlaceBeforeTheNextRefresh() {
        val cache = RosterOrder()
        val studio = session("studio", 100)
        val book = session("book", 200, machine = "Book")
        update(cache, listOf(studio, book), 0)

        val absent = update(cache, listOf(book), 10)
        assertEquals(listOf("Book"), absent.list.map { it.machine })
        assertEquals(listOf("Book"), absent.board.map { it.machine })

        val returning = session("new-studio", 0)
        val restored = update(cache, listOf(book, returning), 20)
        assertEquals(listOf("Studio", "Book"), restored.list.map { it.machine })
        assertEquals(listOf("Studio", "Book"), restored.board.map { it.machine })
        assertEquals(listOf("new-studio", "book"), restored.ids())
        assertEquals(listOf("new-studio", "book"), restored.boardIds())
    }
}
