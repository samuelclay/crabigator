package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ModuleRecapTest {
    @Test fun cachedPlaceholdersAreHiddenAndModuleAgesUseSeconds() {
        val empty = "The agent completed the turn without captured transcript output."
        val now = 1_800_000_000L
        val session = Session.parse(JSONObject().put("id", "recap").put("recap", JSONObject().put("headline", empty)))
        assertEquals("", session.recap)
        val data = SessionModuleData(session, mapOf(
            "recap" to JSONObject().put("latest", JSONObject().put("headline", empty)),
            "recap_history" to JSONObject("""{"history":[{"headline":"Shipped the fix","generated_at":${(now - 420) * 1000}},{"headline":"$empty"}]}"""),
            "commit_history" to JSONObject("""{"history":[{"subject":"Fix capture","timestamp":${now - 7200}}]}""")))
        assertEquals("Shipped the fix", data.recap)
        assertEquals(1, data.history.size)
        assertEquals("7m ago", sessionAge(data.recapAt, now))
        assertEquals("2h ago", sessionAge(data.commits.first().optLong("timestamp"), now))
        val roster = Session.parse(JSONObject("""{"id":"roster","recap":{"headline":"Roster recap","generated_at":${(now - 420) * 1000}}}"""))
        assertEquals(now - 420, SessionModuleData(roster, emptyMap()).recapAt)
        val pr = PullRequest.parse(JSONObject("""{"owner":"o","repo":"r","number":1,"updated_at":${(now - 10800) * 1000}}"""))
        assertEquals("3h ago", sessionAge(pr.updatedAt, now))
    }
}
