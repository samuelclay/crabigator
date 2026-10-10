package com.crabigator.app

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable internal fun TerminalKeyboard(open: Boolean, close: () -> Unit, enabled: Boolean, codex: Boolean, send: (String) -> Unit) {
    var shift by remember { mutableStateOf(false) }
    var control by remember { mutableStateOf(false) }
    var alt by remember { mutableStateOf(false) }
    var letters by remember { mutableStateOf(false) }
    val modified = shift || control || alt
    LaunchedEffect(open) { if (!open) { shift = false; control = false; alt = false; letters = false } }
    DropdownMenu(open, close, modifier = Modifier.widthIn(max = 352.dp).width(352.dp),
        containerColor = Ink, shape = RoundedCornerShape(18.dp), border = BorderStroke(1.dp, Color(0xFF37414E))) {
        Column(Modifier.padding(horizontal = 12.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Keyboard", Modifier.weight(1f), fontSize = 16.sp, fontWeight = FontWeight.Medium)
                if (modified) TextButton({ shift = false; control = false; alt = false }) { Text("Reset", fontSize = 12.sp) }
                Control(R.drawable.ic_close, "Close keyboard", onClick = close)
            }
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                val combos = listOf(
                    Shortcut("⇧ Tab", "Mode", "Tab", shift = true),
                    Shortcut("Ctrl C", "Interrupt", "C", control = true),
                    if (codex) Shortcut("Alt ↑", "Questions", "↑", alt = true) else Shortcut("Ctrl L", "Clear screen", "L", control = true),
                    Shortcut("Ctrl A", "Line start", "A", control = true),
                    Shortcut("Ctrl E", "Line end", "E", control = true),
                    Shortcut("Ctrl U", "Clear line", "U", control = true),
                )
                combos.chunked(3).forEach { row -> Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    row.forEach { combo -> Keycap(combo.label, Modifier.weight(1f), enabled, caption = combo.caption) {
                        send(terminalKey(combo.key, combo.shift, combo.control, combo.alt))
                    } }
                } }
            }
            HorizontalDivider(color = Color(0xFF303A46))
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                ModifierKey("⇧ Shift", shift, Modifier.weight(1f)) { shift = !shift }
                ModifierKey("⌃ Ctrl", control, Modifier.weight(1f)) { control = !control }
                ModifierKey("⌥ Alt", alt, Modifier.weight(1f)) { alt = !alt }
            }
            AnimatedContent(letters, label = "Keyboard keys") { alphabet ->
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    val rows = if (alphabet) ('A'..'Z').map(Char::toString).chunked(6) else listOf(
                        listOf("Esc", "Tab", "Enter", "⌫"), listOf("Home", "↑", "End", "PgUp"),
                        listOf("←", "↓", "→", "PgDn"), listOf("Space", "Insert", "Delete"))
                    rows.forEach { row -> Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        row.forEach { key -> Keycap(key, Modifier.weight(if (key == "Space") 2f else 1f), enabled, modified) {
                            send(terminalKey(key, shift, control, alt))
                        } }
                        if (alphabet) repeat(6 - row.size) { Spacer(Modifier.weight(1f)) }
                    } }
                }
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                TextButton({ letters = !letters }) { Text(if (letters) "← Navigation" else "A–Z →", fontSize = 12.sp) }
            }
        }
    }
}

private data class Shortcut(val label: String, val caption: String, val key: String,
    val shift: Boolean = false, val control: Boolean = false, val alt: Boolean = false)

@Composable private fun ModifierKey(label: String, active: Boolean, modifier: Modifier, toggle: () -> Unit) {
    Surface(onClick = toggle, modifier = modifier.semantics { selected = active }, shape = RoundedCornerShape(9.dp),
        color = if (active) CrabColors.Title else Color(0xFF202A36), contentColor = if (active) Ink else Muted,
        border = BorderStroke(1.dp, if (active) CrabColors.Title else Color(0xFF405064))) {
        Box(Modifier.height(44.dp), contentAlignment = Alignment.Center) { Text(label, fontSize = 13.sp, fontWeight = FontWeight.SemiBold) }
    }
}

@Composable private fun Keycap(label: String, modifier: Modifier, enabled: Boolean, modified: Boolean = false,
    caption: String? = null, press: () -> Unit) {
    Surface(onClick = press, enabled = enabled, modifier = modifier, shape = RoundedCornerShape(8.dp),
        color = if (modified) Color(0xFF1B3047) else Panel,
        contentColor = (if (modified) CrabColors.Title else MaterialTheme.colorScheme.onSurface).copy(alpha = if (enabled) 1f else .4f),
        border = BorderStroke(1.dp, if (modified) CrabColors.Title.copy(alpha = .5f) else Color(0xFF364150)), shadowElevation = 2.dp) {
        Column(Modifier.height(if (caption == null) 44.dp else 54.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
            Text(label, fontFamily = FontFamily.Monospace, fontSize = 12.sp, fontWeight = FontWeight.Medium)
            caption?.let { Text(it, color = Muted, fontSize = 10.sp, lineHeight = 14.sp) }
        }
    }
}
