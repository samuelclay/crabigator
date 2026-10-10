package com.crabigator.app

import android.content.Context
import org.json.JSONObject

data class UiPreferences(
    val fontSize: Int = 13,
    val lineSpacing: Int = 145,
    val wrap: Boolean = true,
    val columns: Int = 1,
    val terminalHeight: Int = 0,
    val widgets: Boolean = false,
    val grouping: String = "all",
    val order: String = "recent",
    val density: String = "comfortable",
    val sidebarRight: Boolean = false,
    val hidden: Set<String> = setOf("tools", "compactions"),
) {
    fun visible(key: String) = key !in hidden
    fun toggle(key: String) = copy(hidden = if (key in hidden) hidden - key else hidden + key)
}

class PreferenceStore(context: Context) {
    private val storage = context.getSharedPreferences("appearance", Context.MODE_PRIVATE)
    fun load(): UiPreferences {
        val j = runCatching { JSONObject(storage.getString("settings", "{}")!!) }.getOrDefault(JSONObject())
        return UiPreferences(
            j.optInt("fontSize", 13).coerceIn(9, 24), j.optInt("lineSpacing", 145).coerceIn(110, 180),
            j.optBoolean("wrap", true), j.optInt("columns", 1).coerceIn(0, 4),
            j.optInt("terminalHeight").coerceIn(0, 700), j.optBoolean("widgets"),
            j.text("grouping", "all"), j.text("order", "recent"), j.text("density", "comfortable"),
            j.optBoolean("sidebarRight"), j.optJSONArray("hidden")?.let { a -> (0 until a.length()).map { a.getString(it) }.toSet() } ?: setOf("tools", "compactions"),
        )
    }
    fun save(p: UiPreferences) {
        val j = JSONObject().put("fontSize", p.fontSize).put("lineSpacing", p.lineSpacing).put("wrap", p.wrap)
            .put("columns", p.columns).put("terminalHeight", p.terminalHeight)
            .put("widgets", p.widgets).put("grouping", p.grouping).put("order", p.order).put("density", p.density)
            .put("sidebarRight", p.sidebarRight).put("hidden", org.json.JSONArray(p.hidden.toList()))
        storage.edit().putString("settings", j.toString()).apply()
    }
}
