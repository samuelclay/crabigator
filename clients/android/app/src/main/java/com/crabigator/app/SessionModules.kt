package com.crabigator.app

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.json.JSONObject

internal data class ModuleChange(val name: String, val kind: String, val additions: Int, val deletions: Int)
internal class SessionModuleData(session: Session, details: Map<String, JSONObject>) {
    val history = details["recap_history"]?.array("history").orEmpty()
    private val latest = details["recap"]?.optJSONObject("latest") ?: history.lastOrNull()
    val recap = latest?.text("headline", session.recap) ?: session.recap
    val bullets = latest?.optJSONArray("bullets")?.let { a -> (0 until a.length()).map { a.getString(it) } }.orEmpty()
    val recapAt = latest?.optLong("generated_at") ?: 0L
    private val git = details["git"]
    val branch = git?.text("branch", session.branch) ?: session.branch
    val files = git?.array("files").orEmpty().map { ModuleChange(it.text("path"), it.text("status"), it.optInt("additions"), it.optInt("deletions")) }
    val changes = details["changes"]?.array("by_language").orEmpty().flatMap { it.array("changes") }.map { ModuleChange(it.text("name"), it.text("kind"), it.optInt("additions"), it.optInt("deletions")) }
    val commits = details["commit_history"]?.array("history")?.reversed() ?: git?.array("recent_commits").orEmpty()
    val stats = details["stats"] ?: JSONObject()
}

@Composable internal fun SessionModules(session: Session, data: SessionModuleData, prefs: UiPreferences, prs: List<PullRequest>, modifier: Modifier = Modifier) {
    var expanded by rememberSaveable(session.id) { mutableStateOf(emptyList<String>()) }
    fun toggle(key: String) { expanded = if (key in expanded) expanded - key else expanded + key }
    val uri = LocalUriHandler.current
    Column(modifier.fillMaxWidth().background(Panel)) {
        Column(Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState())) {
            if (prefs.visible("recap") && data.recap.isNotBlank()) {
                ModuleSection("Recap", data.recap, color = CrabColors.Cyan, expanded = "recap" in expanded, toggle = { toggle("recap") }) {
                    Column(Modifier.padding(horizontal = 12.dp).padding(bottom = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(data.recap, color = Color(0xFFE2E7ED), fontSize = 13.sp, lineHeight = 18.sp)
                        data.bullets.forEach { bullet ->
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) { Text("·", color = CrabColors.Cyan); Text(bullet, color = Muted, fontSize = 12.sp, lineHeight = 17.sp) }
                        }
                        if (data.recapAt > 0) Text(sessionAge(data.recapAt, System.currentTimeMillis() / 1000), color = Muted, fontSize = 10.sp)
                        data.history.dropLast(1).reversed().forEach { previous ->
                            ModuleDivider()
                            Text(previous.text("headline"), color = Muted, fontSize = 12.sp, lineHeight = 17.sp)
                        }
                    }
                }
            }
            if (prefs.visible("prs") && prs.isNotEmpty()) {
                val open = prs.count { it.state == "OPEN" }; val merged = prs.count { it.state == "MERGED" }
                val summary = listOfNotNull(if (open > 0) "$open open" else null, if (merged > 0) "$merged merged" else null).joinToString(" · ")
                ModuleSection("Pull requests", "${prs.size} PR${if (prs.size == 1) "" else "s"}", summary, CrabColors.Purple, "prs" in expanded, { toggle("prs") }) {
                    Column(Modifier.padding(horizontal = 10.dp).padding(bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        prs.forEach { pr -> ModulePrRow(pr) { if (pr.url.startsWith("https://")) uri.openUri(pr.url) } }
                    }
                }
            }
            if (prefs.visible("git") && (data.branch.isNotBlank() || data.files.isNotEmpty())) {
                ModuleSection("Git", data.branch, if (data.files.isEmpty()) "Clean" else "${data.files.size} file${if (data.files.size == 1) "" else "s"}", CrabColors.Green, "git" in expanded, { toggle("git") }) {
                    if (data.files.isEmpty()) Text("Working tree clean", color = Muted, fontSize = 12.sp, modifier = Modifier.padding(horizontal = 12.dp).padding(bottom = 10.dp))
                    else ModuleChanges(data.files)
                }
            }
            if (prefs.visible("changes") && data.changes.isNotEmpty()) {
                ModuleSection("Changes", "${data.changes.size} symbol${if (data.changes.size == 1) "" else "s"}", color = CrabColors.Orange, expanded = "changes" in expanded, toggle = { toggle("changes") }) { ModuleChanges(data.changes) }
            }
            if (prefs.visible("commits") && data.commits.isNotEmpty()) {
                ModuleSection("Commits", data.commits.first().text("subject"), "${data.commits.size}", Muted, "commits" in expanded, { toggle("commits") }) {
                    Column(Modifier.padding(horizontal = 12.dp).padding(bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        data.commits.forEach { commit ->
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Text(commit.text("short_hash"), color = CrabColors.Orange, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                                Text(commit.text("subject"), color = Muted, fontSize = 11.sp, lineHeight = 16.sp, fontFamily = FontFamily.Monospace)
                            }
                        }
                    }
                }
            }
        }
        if (prefs.visible("stats")) ModuleStats(session, data.stats, prefs)
    }
}

@Composable private fun ModuleSection(title: String, summary: String, suffix: String = "", color: Color, expanded: Boolean, toggle: () -> Unit, content: @Composable () -> Unit) {
    Column {
        Row(Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(role = Role.Button, onClick = toggle).semantics { contentDescription = title; stateDescription = if (expanded) "Expanded" else "Collapsed" }.padding(horizontal = 10.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(if (expanded) "⌄" else "›", color = color, fontSize = 12.sp, modifier = Modifier.width(10.dp))
            Text(title, color = color, fontSize = 11.sp, fontWeight = FontWeight.SemiBold, maxLines = 1)
            Text(summary, color = Muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            if (suffix.isNotBlank()) Text(suffix, color = Muted, fontSize = 10.sp, maxLines = 1)
        }
        if (expanded) content()
        ModuleDivider()
    }
}
@Composable private fun ModuleDivider() { Box(Modifier.fillMaxWidth().height(.5.dp).background(Color.White.copy(alpha = .07f))) }
@Composable private fun ModuleChanges(rows: List<ModuleChange>) {
    Column(Modifier.padding(horizontal = 12.dp).padding(bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        rows.forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
                Text(row.kind, color = if (row.kind == "D") CrabColors.Red else CrabColors.Orange, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                Text(row.name, color = Muted, fontSize = 11.sp, lineHeight = 16.sp, fontFamily = FontFamily.Monospace, modifier = Modifier.weight(1f))
                ModuleDiff(row.additions, row.deletions)
            }
        }
    }
}
@Composable private fun ModuleDiff(additions: Int?, deletions: Int?) {
    Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
        if (additions != null) Text("+$additions", color = CrabColors.Green, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
        if (deletions != null) Text("−$deletions", color = CrabColors.Red, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
    }
}
@Composable private fun ModulePrRow(pr: PullRequest, open: () -> Unit) {
    val stateColor = when { pr.state == "MERGED" -> CrabColors.Purple; pr.state == "CLOSED" -> CrabColors.Red; pr.draft -> Muted; else -> CrabColors.Green }
    Box(Modifier.fillMaxWidth().background(Color.White.copy(alpha = .025f), RoundedCornerShape(5.dp)).border(.5.dp, Color.White.copy(alpha = .08f), RoundedCornerShape(5.dp)).clickable(enabled = pr.url.startsWith("https://"), role = Role.Button, onClick = open).semantics { contentDescription = "Open ${pr.repo}#${pr.number} on GitHub: ${pr.title}, ${pr.state.lowercase()}" }) {
        Box(Modifier.matchParentSize()) { Box(Modifier.width(2.dp).fillMaxHeight().background(CrabColors.Purple.copy(alpha = .6f))) }
        Column(Modifier.padding(horizontal = 10.dp, vertical = 9.dp), verticalArrangement = Arrangement.spacedBy(5.dp)) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text((if (pr.primary) "★ " else "") + pr.repo.substringAfterLast('/') + " #${pr.number}", color = CrabColors.Purple, fontSize = 12.sp, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                Text(if (pr.draft) "draft" else pr.state.lowercase(), color = stateColor, fontSize = 10.sp, lineHeight = 14.sp, modifier = Modifier.background(stateColor.copy(alpha = .08f), RoundedCornerShape(3.dp)).border(.5.dp, stateColor.copy(alpha = .5f), RoundedCornerShape(3.dp)).padding(horizontal = 5.dp, vertical = 2.dp))
            }
            Text(pr.title, color = Muted, fontSize = 12.sp, lineHeight = 17.sp)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(if (pr.branch.isNotBlank()) "⑂ ${pr.branch}" else "", color = Muted, fontSize = 10.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                ModuleDiff(pr.additions, pr.deletions)
                if (pr.checks.isNotBlank()) Text(pr.checks, color = if (pr.checksFailed > 0) CrabColors.Red else if (pr.checksPending > 0) CrabColors.Yellow else CrabColors.Green, fontSize = 10.sp, maxLines = 1)
            }
        }
    }
}
@Composable private fun ModuleStats(session: Session, stats: JSONObject, prefs: UiPreferences) {
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).height(34.dp).padding(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
        SessionStatus(session)
        if (prefs.visible("sessionTime")) ModuleStat("◉", sessionDuration(if (stats.has("work_seconds")) stats.optLong("work_seconds") else session.stats.workSeconds), "Session time", CrabColors.Blue)
        if (prefs.visible("thinkingTime")) ModuleStat("◐", sessionDuration(if (stats.has("thinking_seconds")) stats.optLong("thinking_seconds") else session.stats.thinkingSeconds), "Thinking time", CrabColors.Green)
        if (prefs.visible("prompts")) ModuleStat("⟩", stats.opt("prompts")?.toString() ?: session.stats.prompts?.toString() ?: "—", "Prompts", Muted)
        if (prefs.visible("completions")) ModuleStat("⋖", stats.opt("completions")?.toString() ?: session.stats.completions?.toString() ?: "—", "Completions", Muted)
        if (prefs.visible("tools")) ModuleStat("⚒", stats.opt("tools")?.toString() ?: session.stats.tools?.toString() ?: "—", "Tools", CrabColors.Orange)
        if (prefs.visible("compactions")) ModuleStat("⊜", stats.opt("compressions")?.toString() ?: session.stats.compactions?.toString() ?: "—", "Compactions", CrabColors.Purple)
    }
}
@Composable private fun ModuleStat(symbol: String, value: String, label: String, color: Color) {
    Row(Modifier.clearAndSetSemantics { contentDescription = "$label: $value" }, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(symbol, color = color, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
        Text(value, color = Muted, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
    }
}
