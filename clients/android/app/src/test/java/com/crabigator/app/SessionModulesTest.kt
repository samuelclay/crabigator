package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class SessionModulesTest {
    @Test fun summaryUsesWorkingTreeTotalsAndVisibleModules() {
        val session = Session.parse(JSONObject("""{"id":"summary"}"""))
        val data = SessionModuleData(session, mapOf(
            "git" to JSONObject("""{"files":[{"path":"a","additions":12,"deletions":3},{"path":"b","additions":8,"deletions":4}]}"""),
            "changes" to JSONObject("""{"by_language":[{"changes":[{"name":"symbol","additions":999}]}]}""")))
        val prs = listOf(
            PullRequest("a#1", "First", "o/a", 1, "OPEN", "", "", emptyList(), additions = 1000),
            PullRequest("a#42", "Primary", "o/a", 42, "OPEN", "", "", emptyList(), primary = true))
        val summary = ModuleSummary(data, prs, UiPreferences())
        assertEquals("PR #42 +1", summary.prLabel)
        assertEquals(2, summary.fileCount)
        assertEquals(20, summary.additions)
        assertEquals(7, summary.deletions)
        assertTrue(ModuleSummary(data, prs, UiPreferences(hidden = setOf("git", "prs"))).isEmpty)
        assertTrue(ModuleSummary(SessionModuleData(session, emptyMap()), emptyList(), UiPreferences()).isEmpty)
        val clean = ModuleSummary(SessionModuleData(session, mapOf("git" to JSONObject("""{"files":[]}"""))), emptyList(), UiPreferences())
        assertEquals(0, clean.fileCount)
        assertNull(clean.additions)
        assertTrue(ModuleSummary(SessionModuleData(session, mapOf("git" to JSONObject("""{"files":[],"is_repo":false}"""))), emptyList(), UiPreferences()).isEmpty)
    }
    @Test fun lastRecapAndStructuredDiffsRemainAvailableWhileRefreshing() {
        assertTrue(UiPreferences().widgets)
        val session = Session.parse(JSONObject("""{"id":"modules","recap":{"headline":"Roster recap"}}"""))
        val data = SessionModuleData(session, mapOf(
            "recap" to JSONObject("""{"status":"waiting"}"""),
            "recap_history" to JSONObject("""{"history":[{"headline":"Previous turn","bullets":["Keep this detail"]}]}"""),
            "git" to JSONObject("""{"branch":"main","files":[{"path":"Session.kt","status":"M","additions":12,"deletions":3}]}"""),
            "stats" to JSONObject("""{"prompts":3,"tools":9}""")))
        assertEquals("Previous turn", data.recap)
        assertEquals(listOf("Keep this detail"), data.bullets)
        assertEquals("Session.kt", data.files.first().name)
        assertEquals(12, data.files.first().additions)
        assertEquals(9, data.stats.optInt("tools"))
        assertEquals("Roster recap", SessionModuleData(session, emptyMap()).recap)
    }
}
