package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class SessionGroupsTest {
    private fun session(id: String, machine: String, at: Long, repo: String = "app") = Session.parse(JSONObject()
        .put("id", id).put("device_name", machine).put("prompts_changed_at", at).put("repo_name", repo))

    @Test fun liveUpdatesKeepMachinesStillButLetTheirSessionsMove() {
        val studio = session("studio", "Studio", 100)
        val book = session("book", "Book", 90)
        val initial = retainMachineOrder(emptyList(), listOf(book, studio))
        assertEquals(listOf("Studio", "Book"), initial)
        val latest = listOf(studio, book.copy(stats = book.stats.copy(promptAt = 200)), session("new-studio", "Studio", 250), session("new", "Mini", 300))
            .sortedByDescending { it.activityAt }
        val retained = retainMachineOrder(initial, latest)
        assertEquals(listOf("Studio", "Book", "Mini"), retained)
        val groups = sessionListGroups(latest, retained, "all", "recent")
        assertEquals(retained, groups.map { it.machine })
        assertEquals(listOf("new-studio", "studio"), groups.first().sessions.map { it.id })
        assertEquals(retained, retainMachineOrder(retained, emptyList()))
        assertEquals(listOf("Book"), sessionListGroups(latest.filter { it.machine == "Book" }, retained, "all", "recent").map { it.machine })
        assertEquals(listOf("Mini", "Studio", "Book"), retainMachineOrder(emptyList(), latest))
    }

    @Test fun projectFoldersStayInsideTheirMachine() {
        val sessions = listOf(session("a", "Studio", 100, "z"), session("b", "Book", 90, "z"), session("c", "Studio", 80, "a"))
        val groups = sessionListGroups(sessions, listOf("Studio", "Book"), "project", "alpha")
        assertEquals(listOf("Studio/a", "Studio/z", "Book/z"), groups.map { "${it.machine}/${it.project}" })
        assertEquals(listOf("c", "a", "b"), groups.flatMap { it.sessions }.map { it.id })
    }
}
