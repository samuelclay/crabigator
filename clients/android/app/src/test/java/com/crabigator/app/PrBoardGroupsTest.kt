package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class PrBoardGroupsTest {
    private val now = 1_000_000L
    private fun session(id: String, machine: String, age: Long, repo: String = "app", state: String = "complete") =
        Session.parse(JSONObject().put("id", id).put("device_name", machine).put("repo_name", repo)
            .put("prompts_changed_at", now - age).put("state", state))

    @Test fun machinesContainCliAgeBandsAndCompletionDoesNotReorderThem() {
        val oldButWorking = session("working", "Studio", 100_000, state = "thinking")
        val oneHour = session("one", "Studio", 3600)
        val justCompleted = session("nine", "Studio", 32400).let { it.copy(stats = it.stats.copy(completionAt = now)) }
        val book = session("book", "Book", 30)
        val groups = prBoardGroups(listOf(book, justCompleted, oneHour, oldButWorking), emptyList(), listOf("Studio", "Book"), now, false, "")
        assertEquals(listOf("Studio", "Book"), groups.map { it.machine })
        assertEquals(listOf(BoardAge.Hour, BoardAge.Three, BoardAge.Twelve), groups.first().bands.map { it.age })
        assertEquals(listOf("working", "one", "nine"), groups.first().sessions.map { it.session.id })
    }

    @Test fun sharedPrAppearsUnderEachSessionAndSearchKeepsTheMachine() {
        val studio = session("a", "Studio", 10)
        val book = session("b", "Book", 20)
        val pr = PullRequest("owner/repo#1", "Shared fix", "owner/repo", 1, "OPEN", "", "", listOf(studio, book), primary = true)
        val groups = prBoardGroups(listOf(studio, book), listOf(pr), listOf("Studio", "Book"), now, false, "Shared fix")
        assertEquals(listOf("Studio", "Book"), groups.map { it.machine })
        assertEquals(listOf("a", "b"), groups.flatMap { it.sessions }.map { it.session.id })
        assertTrue(groups.all { it.sessions.single().prs.single().key == pr.key })
        assertTrue(prBoardGroups(listOf(studio), listOf(pr), listOf("Studio"), now, false, "missing").isEmpty())
    }

    @Test fun repositoryAndPrOrderFollowCliPriority() {
        val a = session("a", "Studio", 90, "z")
        val b = session("b", "Studio", 20, "a")
        val merged = PullRequest("p#1", "Merged", "p", 1, "MERGED", "", "", listOf(a), primary = true)
        val conflict = merged.copy(key = "p#2", number = 2, state = "OPEN", mergeable = "CONFLICTING", primary = false)
        val group = prBoardGroups(listOf(a, b), listOf(merged, conflict), listOf("Studio"), now, false, "").single()
        assertEquals(listOf("a", "p"), group.bands.single().repositories.map { it.repo })
        assertEquals(listOf("p#2", "p#1"), group.sessions.last().prs.map { it.key })
        assertTrue(prBoardGroups(listOf(a.copy(active = false)), emptyList(), listOf("Studio"), now, false, "").isEmpty())
        assertEquals(1, prBoardGroups(listOf(a.copy(active = false)), emptyList(), listOf("Studio"), now, true, "").single().sessions.size)
    }
}
