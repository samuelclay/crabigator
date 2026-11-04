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
    val tools: Long? = null,
    val compactions: Long? = null,
) {
    fun duration(active: Boolean, now: Long): Long? {
        if (!active && workSeconds != null) return workSeconds.coerceAtLeast(0)
        val end = if (active) now else endedAt.takeIf { it > 0 } ?: lastSeenAt
        return if (startedAt > 0 && end > 0) (end - startedAt).coerceAtLeast(0) else workSeconds
    }

    companion object {
        fun parse(session: JSONObject): SessionStats {
            val stats = session.optJSONObject("stats")
            return SessionStats(
                session.optLong("started_at"), session.optLong("ended_at"), session.optLong("last_seen_at"),
                stats?.number("work_seconds"), stats?.number("thinking_seconds"),
                stats?.number("prompts"), stats?.number("completions"),
                session.optLong("prompts_changed_at"), session.optLong("completions_changed_at"),
                stats?.number("tools"), stats?.number("compressions"),
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

/** How full the assistant's context window is, from a stats event: "186k / 1M · 19%". */
data class ContextUsage(val used: Long, val limit: Long) {
    val percent: Long get() = Math.round(used * 100.0 / limit)
    val label: String get() = "${tokens(used)} / ${tokens(limit)} · $percent%"

    companion object {
        fun parse(stats: JSONObject): ContextUsage? {
            if (!stats.has("context_used") || !stats.has("context_limit")) return null
            val limit = stats.optLong("context_limit")
            return if (limit > 0) ContextUsage(stats.optLong("context_used").coerceAtLeast(0), limit) else null
        }

        /** A token count in a few characters, as the CLI shows it: 512, 9.5k, 186k, 1M, 1.5M. */
        fun tokens(count: Long): String {
            fun oneDecimal(value: Double, unit: String): String =
                String.format(java.util.Locale.ROOT, "%.1f", value).removeSuffix(".0") + unit
            if (count < 1000) return count.toString()
            if (count < 9950) return oneDecimal(count / 1000.0, "k")
            val thousands = Math.round(count / 1000.0)
            if (thousands < 1000) return "${thousands}k"
            return oneDecimal(count / 1_000_000.0, "M")
        }
    }
}
