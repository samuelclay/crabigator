package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class SessionModulesTest {
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
