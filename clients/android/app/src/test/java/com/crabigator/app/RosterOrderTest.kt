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

    @Test fun ageBandsReposAndPrPriorityStayPutWhileTheirContentsUpdate() {
        val cache = RosterOrder()
        val a = session("a", 3590, "z")
        val b = session("b", 7200, "a")
        val first = PullRequest("z#1", "First", "z", 1, "OPEN", "", "", listOf(a), primary = true)
        val second = first.copy(key = "other#2", title = "Second", repo = "other", number = 2, primary = false)
        val initial = update(cache, listOf(a, b), 0, listOf(first, second), "project")
        val working = b.copy(state = "thinking", stats = b.stats.copy(promptAt = now + 50))
        val failing = second.copy(checks = "1 failed", checksFailed = 1)
        val held = update(cache, listOf(a, working), 599, listOf(failing, first), "project")
        assertEquals(initial.board.map { it.bands.map { band -> band.age } }, held.board.map { it.bands.map { band -> band.age } })
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

    @Test fun newRowsAppendRemovedRowsDisappearAndFiltersDoNotResetPositions() {
        val cache = RosterOrder()
        val a = session("a", 100)
        val b = session("b", 200)
        val book = session("book", 300, machine = "Book")
        update(cache, listOf(a, b, book), 0)
        val fresh = session("new", 0)
        val extraRepo = session("extra", 1, "other")
        val ended = b.copy(active = false)
        val live = update(cache, listOf(fresh, extraRepo, a, ended, book), 30)
        assertEquals(listOf("a", "b", "new", "extra", "book"), live.ids())
        assertEquals(listOf("a", "b", "new", "extra", "book"), live.boardIds())
        assertEquals(listOf("a", "new", "extra", "book"), live.filtered(false, "").ids())
        assertEquals(listOf("a", "b", "new", "extra", "book"), live.filtered(true, "").ids())
        val removed = update(cache, listOf(fresh, extraRepo, ended, book), 40)
        assertEquals(listOf("b", "new", "extra", "book"), removed.ids())
        assertEquals(listOf("Studio", "Book"), removed.board.map { it.machine })
        val changedGrouping = update(cache, listOf(fresh, extraRepo, ended, book), 50, grouping = "project", order = "alpha")
        assertEquals(listOf("app", "other", "app"), changedGrouping.list.map { it.project })
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
