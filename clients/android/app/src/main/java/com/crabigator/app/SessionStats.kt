package com.crabigator.app

import org.json.JSONObject

data class SessionStats(
    val startedAt: Long = 0,
    val endedAt: Long = 0,
    val lastSeenAt: Long = 0,
    val workSeconds: Long? = null,
    val thinkingSeconds: Long? = null,
    val prompts: Long? = null,
    val completions: Long? = null,
    val promptAt: Long = 0,
    val completionAt: Long = 0,
) {
    fun duration(active: Boolean, now: Long): Long? {
        if (!active && workSeconds != null) return workSeconds.coerceAtLeast(0)
        val end = if (active) now else endedAt.takeIf { it > 0 } ?: lastSeenAt
        return if (startedAt > 0 && end > 0) (end - startedAt).coerceAtLeast(0) else workSeconds
    }

    fun activity(count: Long?, changedAt: Long, now: Long): String {
        if (count == null) return "—"
        return if (changedAt > 0 && count > 0) "$count · ${sessionAge(changedAt, now)}" else "$count"
    }

    companion object {
        fun parse(session: JSONObject): SessionStats {
            val stats = session.optJSONObject("stats")
            return SessionStats(
                session.optLong("started_at"), session.optLong("ended_at"), session.optLong("last_seen_at"),
                stats?.number("work_seconds"), stats?.number("thinking_seconds"),
                stats?.number("prompts"), stats?.number("completions"),
                session.optLong("prompts_changed_at"), session.optLong("completions_changed_at"),
            )
        }
        private fun JSONObject.number(key: String): Long? = if (isNull(key)) null else optLong(key).coerceAtLeast(0)
    }
}

internal fun sessionDuration(seconds: Long?): String {
    if (seconds == null) return "—"
    val s = seconds.coerceAtLeast(0)
    return when {
        s < 60 -> "${s}s"
        s < 3600 -> "${s / 60}m"
        s < 86400 -> "${s / 3600}h ${s % 3600 / 60}m"
        else -> "${s / 86400}d ${s % 86400 / 3600}h"
    }
}

internal fun sessionAge(timestamp: Long, now: Long): String {
    val seconds = (now - timestamp).coerceAtLeast(0)
    return when {
        seconds < 60 -> "now"
        seconds < 3600 -> "${seconds / 60}m ago"
        seconds < 86400 -> "${seconds / 3600}h ago"
        else -> "${seconds / 86400}d ago"
    }
}
