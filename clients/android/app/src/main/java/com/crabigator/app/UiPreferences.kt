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

data class NavigationState(val listMode: Boolean = true, val expanded: Boolean = true, val selectedId: String? = null)

class NavigationStore(context: Context) {
    private val storage = context.getSharedPreferences("navigation", Context.MODE_PRIVATE)
    fun load() = NavigationState(storage.getBoolean("listMode", true), storage.getBoolean("expanded", true), storage.getString("selectedId", null))
    fun save(state: NavigationState) {
        storage.edit().putBoolean("listMode", state.listMode).putBoolean("expanded", state.expanded)
            .putString("selectedId", state.selectedId).apply()
    }
}

data class UiPreferences(
    val fontSize: Int = 13,
    val lineSpacing: Int = 145,
    val wrap: Boolean = true,
    val columns: Int = 0,
    val terminalHeight: Int = 0,
    val widgets: Boolean = false,
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
) {
    fun includesActivity(timestamp: Long, now: Long): Boolean =
        boardAgeHours == 0 || BoardAge.fromSeconds((now - timestamp).coerceAtLeast(0)) <=
            if (boardAgeHours == 24) BoardAge.Day else BoardAge.fromSeconds(boardAgeHours * 3600L - 1)
    fun visible(key: String) = key !in hidden
    fun toggle(key: String) = copy(hidden = if (key in hidden) hidden - key else hidden + key)
}

class PreferenceStore(context: Context) {
    private val storage = context.getSharedPreferences("appearance", Context.MODE_PRIVATE)
    fun load(): UiPreferences {
        val j = runCatching { JSONObject(storage.getString("settings", "{}")!!) }.getOrDefault(JSONObject())
        return UiPreferences(
            j.optInt("fontSize", 13).coerceIn(9, 24), j.optInt("lineSpacing", 145).coerceIn(110, 180),
            j.optBoolean("wrap", true), j.optInt("columns", 0).coerceIn(0, 4),
            j.optInt("terminalHeight").coerceIn(0, 700), j.optBoolean("widgets"),
            j.text("grouping", "all"), j.text("order", "recent"), j.text("density", "comfortable"),
            j.optBoolean("sidebarRight"), j.optJSONArray("hidden")?.let { a -> (0 until a.length()).map { a.getString(it) }.toSet() } ?: setOf("tools", "compactions"),
            j.optBoolean("showEnded"), j.optInt("boardFontSize", 14).coerceIn(10, 24),
            j.optInt("boardAgeHours", 24).takeIf { it in listOf(0, 1, 3, 6, 9, 12, 24) } ?: 24,
            j.text("boardComputer"), j.optBoolean("boardRecaps"), j.text("boardGrouping", "sessions"),
        )
    }
    fun save(p: UiPreferences) {
        val j = JSONObject().put("boardFontSize", p.boardFontSize).put("fontSize", p.fontSize).put("lineSpacing", p.lineSpacing).put("wrap", p.wrap)
            .put("columns", p.columns).put("terminalHeight", p.terminalHeight)
            .put("widgets", p.widgets).put("grouping", p.grouping).put("order", p.order).put("density", p.density)
            .put("boardAgeHours", p.boardAgeHours).put("boardComputer", p.boardComputer)
            .put("boardRecaps", p.boardRecaps).put("boardGrouping", p.boardGrouping)
            .put("showEnded", p.showEnded).put("sidebarRight", p.sidebarRight).put("hidden", org.json.JSONArray(p.hidden.toList()))
        storage.edit().putString("settings", j.toString()).apply()
    }
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
internal val LocalFloatingBoardToolbar = compositionLocalOf { false }
