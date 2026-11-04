package com.crabigator.app

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class SessionStatsTest {
    @Test fun contextUsageReadsLikeTheCli() {
        val usage = ContextUsage.parse(JSONObject("""{"context_used":186002,"context_limit":1000000}"""))
        assertEquals("186k / 1M · 19%", usage?.label)
        assertNull(ContextUsage.parse(JSONObject("""{"context_used":5}""")))
        assertNull(ContextUsage.parse(JSONObject("""{"context_used":5,"context_limit":0}""")))
        mapOf(0L to "0", 512L to "512", 1000L to "1k", 9500L to "9.5k", 9960L to "10k", 258400L to "258k", 999600L to "1M", 1500000L to "1.5M")
            .forEach { (count, text) -> assertEquals("$count", text, ContextUsage.tokens(count)) }
    }
    @Test fun contextStatStartsHiddenAndTogglesOn() {
        val prefs = UiPreferences(hidden = setOf("tools"))
        assertTrue(prefs.visible("compactions"))
        assertFalse(prefs.visible("context"))
        val shown = prefs.toggle("context")
        assertTrue(shown.visible("context"))
        assertEquals(setOf("tools"), shown.hidden)
        assertFalse(shown.toggle("context").visible("context"))
    }
    @Test fun liveDurationAdvancesButEndedDurationDoesNot() {
        val stats = SessionStats.parse(JSONObject("""{"started_at":1000,"ended_at":1600,"last_seen_at":1550,"stats":{"work_seconds":590,"thinking_seconds":120,"prompts":4,"completions":3},"prompts_changed_at":1500,"completions_changed_at":1540}"""))
        assertEquals(1000L, stats.duration(true, 2000))
        assertEquals(1100L, stats.duration(true, 2100))
        assertEquals(590L, stats.duration(false, 2000))
        assertEquals(590L, stats.duration(false, 2100))
        assertEquals("8m ago", sessionAge(stats.promptAt, 2000))
        assertEquals("7m ago", sessionAge(stats.completionAt, 2000))
        assertEquals("2m", sessionDuration(stats.thinkingSeconds))
    }
    @Test fun missingStatsAndUncleanEndsDoNotInventLiveTime() {
        val unknown = SessionStats.parse(JSONObject("{}"))
        assertEquals("—", sessionDuration(unknown.duration(true, 2000)))
        val stopped = SessionStats(startedAt = 1000, lastSeenAt = 1500)
        assertEquals(500L, stopped.duration(false, 9000))
        assertEquals(600L, stopped.copy(endedAt = 1600).duration(false, 9000))
        assertEquals(0L, stopped.duration(true, 900))
    }
    @Test fun ageAndDurationBoundariesStayReadable() {
        assertEquals("now", sessionAge(100, 90))
        assertEquals("now", sessionAge(100, 159))
        assertEquals("1m ago", sessionAge(100, 160))
        assertEquals("1h ago", sessionAge(100, 3700))
        assertEquals("1d ago", sessionAge(100, 86500))
        assertEquals("0s", sessionDuration(0))
        assertEquals("1m", sessionDuration(60))
        assertEquals("1h 1m", sessionDuration(3660))
        assertEquals("2d 1h", sessionDuration(176400))
    }
}
