package com.crabigator.app

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp

/** A local walkthrough. It never creates an API client or changes pairing credentials. */
@OptIn(ExperimentalLayoutApi::class)
@Composable internal fun DemoView(done: () -> Unit) {
    var selected by rememberSaveable { mutableStateOf<Int?>(null) }
    var board by rememberSaveable { mutableStateOf(false) }
    var answer by rememberSaveable { mutableStateOf<String?>(null) }
    var draft by rememberSaveable { mutableStateOf("") }
    var messages by rememberSaveable { mutableStateOf("") }
    var fontSize by rememberSaveable { mutableIntStateOf(13) }
    var pinned by rememberSaveable { mutableStateOf(true) }
    val keyboardVisible = WindowInsets.ime.getBottom(LocalDensity.current) > 0
    val focus = LocalFocusManager.current
    val titles = listOf("Add dark mode", "Improve search")
    BackHandler { if (selected != null) selected = null else done() }
    Column(Modifier.fillMaxSize().safeDrawingPadding().imePadding().background(Ink)) {
        Row(Modifier.fillMaxWidth().padding(12.dp)) {
            if (selected != null) TextButton(onClick = { selected = null }) { Text("Back") }
            Text("Crabigator demo", modifier = Modifier.weight(1f).padding(vertical = 12.dp), style = MaterialTheme.typography.titleMedium)
            TextButton(onClick = done) { Text("Done") }
        }
        Text("Sample data · Nothing is sent to a computer or AI service", color = Mint,
            style = MaterialTheme.typography.bodySmall, modifier = Modifier.fillMaxWidth().background(Panel).padding(12.dp))
        if (selected == null) {
            Row(Modifier.padding(12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                FilterChip(selected = !board, onClick = { board = false }, label = { Text("Sessions") })
                FilterChip(selected = board, onClick = { board = true }, label = { Text("PR board") })
            }
            Text("Demo computer · example / website", modifier = Modifier.padding(16.dp), color = Muted)
            LazyColumn(Modifier.weight(1f), contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                itemsIndexed(titles) { index, title ->
                    Surface(color = Panel, shape = MaterialTheme.shapes.medium,
                        modifier = Modifier.fillMaxWidth().clickable { selected = index; messages = ""; draft = ""; pinned = true }) {
                        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(if (board) "PR #${23 + index} · $title" else title, color = CrabColors.Title)
                            Text(if (index == 0 && answer == null) "Question · Choose a default theme" else "Complete · Checks passed",
                                color = if (index == 0 && answer == null) Peach else Mint, style = MaterialTheme.typography.bodySmall)
                            Text(if (index == 0) "+42 −8 · Claude Code" else "+64 −12 · Codex", color = Muted, style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
            Text("Pair your computer to see live sessions, send prompts, and receive notifications.", color = Muted,
                style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(16.dp))
        } else {
            Row(Modifier.padding(12.dp)) {
                Text(titles[selected!!], modifier = Modifier.weight(1f).padding(vertical = 12.dp), style = MaterialTheme.typography.titleMedium)
                TextButton(onClick = { fontSize = (fontSize - 1).coerceAtLeast(9) }) { Text("A−") }
                TextButton(onClick = { fontSize = (fontSize + 1).coerceAtMost(24) }) { Text("A+") }
            }
            val screen = if (selected == 1) {
                "Sample session · Codex\n\n› Make search easier to use.\n\nAdded keyboard navigation and clearer empty states.\n\n  ✓ 18 tests passed\n  + 64 lines  − 12 lines\n\nPR #24 · Improve search · Checks passed\n\nReady for review."
            } else {
                "Sample session · Claude Code\n\n› Add a dark theme to the settings page.\n\nUpdated the theme colors and saved the appearance preference.\n\n  ✓ 12 tests passed\n  + 42 lines  − 8 lines\n\n" +
                    (answer?.let { "› $it\n\nSample answer received. The preview now uses ${it.lowercase()}." } ?: "Which theme should be the default?")
            }
            Box(Modifier.weight(1f)) { TerminalViewport("", screen + messages, UiPreferences(fontSize = fontSize), pinned) { pinned = it } }
            if (selected == 0 && answer == null && !keyboardVisible) {
                Column(Modifier.fillMaxWidth().background(Panel).padding(16.dp)) {
                    Text("Which theme should be the default?", style = MaterialTheme.typography.titleSmall)
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf("System", "Dark", "Light").forEach { choice ->
                            OutlinedButton(onClick = { answer = choice; pinned = true }) { Text(choice) }
                        }
                    }
                }
            }
            Row(Modifier.padding(12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(draft, { draft = it }, label = { Text("Try a sample message") }, maxLines = 4, modifier = Modifier.weight(1f))
                TextButton(enabled = draft.isNotBlank(), onClick = {
                    messages += "\n\n› $draft\n\nDemo only: your message stayed on this device. Pair a computer to send real prompts."
                    draft = ""; pinned = true; focus.clearFocus()
                }) { Text("Send") }
            }
        }
    }
}
