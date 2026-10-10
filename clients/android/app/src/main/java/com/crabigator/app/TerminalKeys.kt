package com.crabigator.app

internal fun terminalKey(key: String, shift: Boolean, control: Boolean, alt: Boolean): String {
    val modifier = 1 + (if (shift) 1 else 0) + (if (alt) 2 else 0) + (if (control) 4 else 0)
    val cursor = mapOf("↑" to "A", "↓" to "B", "→" to "C", "←" to "D", "Home" to "H", "End" to "F")
    cursor[key]?.let { return if (modifier == 1) "\u001b[$it" else "\u001b[1;$modifier$it" }
    val tilde = mapOf("Insert" to 2, "Delete" to 3, "PgUp" to 5, "PgDn" to 6)
    tilde[key]?.let { return if (modifier == 1) "\u001b[$it~" else "\u001b[$it;$modifier~" }
    // Match the desktop's word deletion and modified Tab encoding.
    if (key == "⌫" && (control || alt)) return "\u001b\u007f"
    if (key == "Tab") return when {
        shift -> "\u001b[Z"
        control -> "\u001b[9;${modifier}~"
        else -> "\t"
    }
    val value = when (key) {
        "Esc" -> "\u001b"
        "Enter" -> if (shift || control) "\u001b[13;${modifier}u" else "\r"
        "⌫" -> "\u007f"
        "Space" -> if (control) "\u0000" else " "
        else -> if (control && key.length == 1 && key[0].uppercaseChar() in 'A'..'Z') (key[0].uppercaseChar().code - 64).toChar().toString()
            else if (shift) key.uppercase() else key.lowercase()
    }
    return if (alt && !(key == "Enter" && (shift || control))) "\u001b$value" else value
}
