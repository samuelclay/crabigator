package com.crabigator.app

import android.content.Context
import org.json.JSONObject
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

data class NavigationState(val listMode: Boolean = true, val expanded: Boolean = true, val selectedId: String? = null, val conductor: Boolean = false)

class NavigationStore(context: Context) {
    private val storage = context.getSharedPreferences("navigation", Context.MODE_PRIVATE)
    fun load() = NavigationState(storage.getBoolean("listMode", true), storage.getBoolean("expanded", true), storage.getString("selectedId", null), storage.getBoolean("conductor", false))
    fun save(state: NavigationState) {
        storage.edit().putBoolean("listMode", state.listMode).putBoolean("expanded", state.expanded)
            .putString("selectedId", state.selectedId).putBoolean("conductor", state.conductor).apply()
    }
}

data class UiPreferences(
    val fontSize: Int = 13,
    val lineSpacing: Int = 145,
    val wrap: Boolean = true,
    val columns: Int = 0,
    val terminalHeight: Int = 0,
    val widgets: Boolean = true,
    val grouping: String = "all",
    val order: String = "recent",
    val density: String = "comfortable",
    val sidebarRight: Boolean = false,
    val hidden: Set<String> = setOf("tools", "compactions"),
    val showEnded: Boolean = false,
    val boardFontSize: Int = 14,
    val boardAgeHours: Int = 24,
    val boardComputer: String = "",
    val boardRecaps: Boolean = false,
    val boardGrouping: String = "sessions",
    val groupByComputer: Boolean = true,
    /** Opt-in stats turned on; see [OPT_IN_STATS]. */
    val shown: Set<String> = emptySet(),
    /** What a List row, a PR board row and the session's stats strip show. Null until first changed; see the three sets below. */
    val listRow: Set<String>? = null,
    val boardRow: Set<String>? = null,
    val sessionStats: Set<String>? = null,
) {
    fun includesActivity(timestamp: Long, now: Long): Boolean =
        boardAgeHours == 0 || BoardAge.fromSeconds((now - timestamp).coerceAtLeast(0)) <=
            if (boardAgeHours == 24) BoardAge.Day else BoardAge.fromSeconds(boardAgeHours * 3600L - 1)
    fun visible(key: String) = if (key in OPT_IN_STATS) key in shown else key !in hidden
    fun toggle(key: String) = when {
        key in OPT_IN_STATS -> copy(shown = if (key in shown) shown - key else shown + key)
        else -> copy(hidden = if (key in hidden) hidden - key else hidden + key)
    }

    // A set that was never saved follows the older choices, so each screen looks as it did before the sets existed.
    val listRowItems: Set<String> get() = listRow ?: (oldStats(listOf("sessionTime", "thinkingTime", "promptRecency", "completionRecency", "tools", "compactions")) + listOfNotNull("recap".takeIf { boardRecaps })).toSet()
    /** The old PR board rows showed recency and no stats, so context starts off here. */
    val boardRowItems: Set<String> get() = boardRow ?: setOfNotNull("promptRecency", "completionRecency", "recap".takeIf { boardRecaps })
    /** Prompts and completions never had a switch, so they start on. */
    val sessionStatItems: Set<String> get() = sessionStats ?: oldStats(SESSION_STATS - "context", setOf("prompts", "completions")).toSet()
    /**
     * The stats of [others] that the old Visible stats switches showed. Context joins them when its own
     * old switch was on, or when every other stat is on: someone who shows everything wants context too.
     */
    private fun oldStats(others: List<String>, noSwitch: Set<String> = emptySet()): List<String> {
        val on = others.filter { it in noSwitch || visible(it) }
        return on + listOfNotNull("context".takeIf { visible("context") || on.size == others.size })
    }
    fun toggleListRow(key: String) = copy(listRow = listRowItems.flip(key))
    fun toggleBoardRow(key: String) = copy(boardRow = boardRowItems.flip(key))
    fun toggleSessionStat(key: String) = copy(sessionStats = sessionStatItems.flip(key))
    private fun Set<String>.flip(key: String) = if (key in this) this - key else this + key

    /** The one Group by choice, read from and written to the two saved preferences behind it. */
    val groupBy: GroupBy get() = GroupBy.entries.first { it.computer == groupByComputer && it.project == (grouping == "project") }
    fun grouped(by: GroupBy) = copy(groupByComputer = by.computer, grouping = if (by.project) "project" else "all")
    /** The PR board shares the computer half of that choice and leaves the List's project grouping alone. */
    val boardGroupBy: GroupBy get() = if (groupByComputer) GroupBy.Computer else GroupBy.OneList
    fun boardGrouped(by: GroupBy) = copy(groupByComputer = by.computer)

    companion object {
        /** Stats most people never need start hidden, even in settings saved before they existed. */
        val OPT_IN_STATS = setOf("context")
        val SESSION_STATS = listOf("sessionTime", "thinkingTime", "prompts", "completions", "tools", "compactions", "context")
    }
}

/** The PR board always groups by time band and repository, so it offers only the choices without a project. */
enum class GroupBy(val label: String, val detail: String, val computer: Boolean, val project: Boolean) {
    OneList("One list", "The computer is named on each row", false, false),
    Computer("Computer", "A section for each computer", true, false),
    Project("Project", "A section for each repository", false, true),
    ComputerThenProject("Computer, then project", "Repositories inside each computer", true, true);
    companion object { val board = listOf(OneList, Computer) }
}

class PreferenceStore(context: Context) {
    private val storage = context.getSharedPreferences("appearance", Context.MODE_PRIVATE)
    fun load(): UiPreferences = preferencesFromJson(runCatching { JSONObject(storage.getString("settings", "{}")!!) }.getOrDefault(JSONObject()))
    fun save(p: UiPreferences) { storage.edit().putString("settings", p.toJson().toString()).apply() }
}

/** Reads every saved setting, old or new. A key a save lacks takes its default. */
internal fun preferencesFromJson(j: JSONObject): UiPreferences {
    fun strings(key: String): Set<String>? = j.optJSONArray(key)?.let { a -> (0 until a.length()).map { a.getString(it) }.toSet() }
    return UiPreferences(
        j.optInt("fontSize", 13).coerceIn(9, 24), j.optInt("lineSpacing", 145).coerceIn(110, 180),
        j.optBoolean("wrap", true), j.optInt("columns", 0).coerceIn(0, 4),
        j.optInt("terminalHeight").coerceIn(0, 700), j.optBoolean("modulesExpanded", true),
        j.text("grouping", "all"), j.text("order", "recent"), j.text("density", "comfortable"),
        j.optBoolean("sidebarRight"), strings("hidden") ?: setOf("tools", "compactions"),
        j.optBoolean("showEnded"), j.optInt("boardFontSize", 14).coerceIn(10, 24),
        j.optInt("boardAgeHours", 24).takeIf { it in listOf(0, 1, 3, 6, 9, 12, 24) } ?: 24,
        j.text("boardComputer"), j.optBoolean("boardRecaps"), j.text("boardGrouping", "sessions"),
        j.optBoolean("groupByComputer", true),
        strings("shown") ?: emptySet(),
        strings("listRow"), strings("boardRow"), strings("sessionStats"),
    )
}
internal fun UiPreferences.toJson(): JSONObject {
    val p = this
    val j = JSONObject().put("boardFontSize", p.boardFontSize).put("fontSize", p.fontSize).put("lineSpacing", p.lineSpacing).put("wrap", p.wrap)
        .put("columns", p.columns).put("terminalHeight", p.terminalHeight)
        .put("modulesExpanded", p.widgets).put("grouping", p.grouping).put("order", p.order).put("density", p.density)
        .put("boardAgeHours", p.boardAgeHours).put("boardComputer", p.boardComputer)
        .put("boardRecaps", p.boardRecaps).put("boardGrouping", p.boardGrouping).put("groupByComputer", p.groupByComputer)
        .put("showEnded", p.showEnded).put("sidebarRight", p.sidebarRight).put("hidden", org.json.JSONArray(p.hidden.toList())).put("shown", org.json.JSONArray(p.shown.toList()))
    // Each set is written only once it has been changed; until then it follows the older choices.
    p.listRow?.let { j.put("listRow", org.json.JSONArray(it.toList())) }
    p.boardRow?.let { j.put("boardRow", org.json.JSONArray(it.toList())) }
    p.sessionStats?.let { j.put("sessionStats", org.json.JSONArray(it.toList())) }
    return j
}

internal data class BoardSpacing(
    val density: String = "comfortable",
    val gapScale: Float = if (density == "compact") .45f else if (density == "relaxed") 1.5f else 1f,
    val lineScale: Float = if (density == "compact") .9f else if (density == "relaxed") 1.15f else 1f,
) {
    val tight get() = density == "compact"
    fun gap(normal: Int) = (normal * gapScale).dp
    fun lineHeight(normal: Int) = (normal * lineScale).sp
}
@Composable internal fun animatedBoardSpacing(density: String): BoardSpacing {
    val target = BoardSpacing(density)
    val gaps by animateFloatAsState(target.gapScale, tween(300), label = "Board row spacing")
    val lines by animateFloatAsState(target.lineScale, tween(300), label = "Board line height")
    return target.copy(gapScale = gaps, lineScale = lines)
}
internal val LocalBoardSpacing = compositionLocalOf { BoardSpacing() }
