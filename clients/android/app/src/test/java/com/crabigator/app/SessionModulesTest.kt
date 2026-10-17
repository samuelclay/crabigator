package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class SessionModulesTest {
    @Test fun liveStatusAndLastRecapRemainAvailableWhileTheNextRecapIsPending() {
        val prefs = UiPreferences()
        assertTrue(prefs.widgets)
        val session = Session.parse(JSONObject("""{"id":"modules","state":"thinking","recap":{"headline":"Previous turn"}}"""))
        val state = AppState(selected = session, details = mapOf(
            "recap" to JSONObject("""{"status":"waiting"}"""),
            "stats" to JSONObject("""{"prompts":3,"tools":9}""")))
        val sections = sessionWidgetSections(state, prefs).toMap()
        assertTrue(sections.getValue("Status").any { it.first.contains("9 tools") })
        assertEquals("Previous turn", sections.getValue("Recap").first().first)
        assertFalse(sessionWidgetSections(state, prefs.toggle("stats")).any { it.first == "Status" })
    }
}
