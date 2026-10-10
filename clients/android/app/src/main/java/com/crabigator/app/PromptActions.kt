package com.crabigator.app

import org.json.JSONArray
import org.json.JSONObject

/** Share the exact same input translation between the app and notification replies. */
object PromptActions {
    data class Action(val route: String, val body: JSONObject)
    fun details(prompt: JSONObject): String {
        val input = prompt.opt("tool_input")?.takeUnless { it == JSONObject.NULL } ?: return ""
        if (prompt.text("tool_name") == "Bash" && input is JSONObject
            && input.keys().asSequence().all { it == "command" || it == "description" }) {
            input.text("command").takeIf { it.isNotBlank() }?.let { return it }
        }
        return input.toString()
    }
    fun question(prompt: JSONObject) = prompt.optJSONArray("questions")?.optJSONObject(prompt.optInt("current_question", 0))
    fun title(prompt: JSONObject): String = when (prompt.text("prompt_type")) {
        "question" -> if (prompt.has("review")) "Review your answers" else question(prompt)?.text("question") ?: "A question needs your answer"
        "text" -> if (prompt.text("state") == "permission") "Permission requested. Open the session to review." else "Your session has a question. Reply or open it for context."
        "permission" -> "Allow ${prompt.text("tool_name", "this action")}?"
        else -> "Ready to start the plan?"
    }
    fun options(prompt: JSONObject): List<JSONObject> = if (prompt.text("prompt_type") == "question") {
        if (prompt.has("review")) listOf(JSONObject().put("label", "Submit answers").put("value", "1"), JSONObject().put("label", "Cancel").put("value", "2")) else question(prompt)?.array("options").orEmpty()
    } else prompt.array("options")
    fun action(prompt: JSONObject, option: Int? = null, text: String? = null, submit: Boolean = false): Action {
        if (prompt.text("prompt_type") == "text") {
            require(prompt.text("state") == "question" && !text.isNullOrBlank()) { "Open this session to review the request." }
            return Action("answer", JSONObject().put("text", text))
        }
        if (prompt.text("prompt_type") != "question") {
            require(option != null) { "Choose an option in the app." }
            return Action("answer", JSONObject().put("text", options(prompt)[option].text("value")))
        }
        if (option != null) require(option in options(prompt).indices) { "This choice is no longer available." }
        val steps = JSONArray()
        fun key(k: String) { steps.put(JSONObject().put("type", "key").put("key", k)) }
        fun type(t: String) { steps.put(JSONObject().put("type", "text").put("text", t)) }
        if (prompt.has("review")) { type((requireNotNull(option) + 1).toString()); return Action("key-sequence", JSONObject().put("steps", steps)) }
        val q = question(prompt) ?: error("This question is no longer available.")
        val count = q.array("options").size
        var cursor = prompt.optInt("cursor_row", 1)
        val grok = prompt.text("ui") == "grok_card"
        fun move(target: Int) { repeat(kotlin.math.abs(target - cursor).coerceAtMost(100)) { key(if (target > cursor) "down" else "up") }; cursor = target }
        if (option != null) {
            if (cursor == count + 1) { key(if (grok) "escape" else "up"); cursor-- }
            when {
                grok && q.optBoolean("multi_select") -> { move(option + 1); key("space") }
                grok -> type(('a'.code + option).toChar().toString())
                prompt.has("cursor_row") || q.optBoolean("multi_select") -> type((option + 1).toString())
                else -> { move(option + 1); key("enter") }
            }
        } else if (text != null || submit) {
            if (!text.isNullOrBlank()) {
                require(q.optBoolean("allows_other", true)) { "This question requires one of its choices." }
                if (grok) type("z")
                else if (!q.optBoolean("multi_select")) {
                    if (cursor != count + 1) type((count + 1).toString())
                    cursor = count + 1
                } else move(count + 1)
                repeat(prompt.text("custom_text").length.coerceAtMost(2000)) { key("backspace") }
                type(text)
                steps.put(JSONObject().put("type", "delay").put("ms", 50))
                if (grok && q.optBoolean("multi_select")) key("escape")
            }
            // Grok accepts single-choice text with Enter. Escape/right is the
            // multi-select page navigation and can leave typed text unsubmitted.
            if (grok && q.optBoolean("multi_select")) key(if (prompt.optInt("current_question") + 1 < prompt.array("questions").size) "right" else "enter")
            else if (q.optBoolean("multi_select")) { move(count + 2); key("enter") }
            else key("enter")
        }
        return Action("key-sequence", JSONObject().put("steps", steps))
    }
}
