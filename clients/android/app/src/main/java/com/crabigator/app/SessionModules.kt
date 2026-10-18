package com.crabigator.app

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.material3.Icon
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.json.JSONObject
import kotlinx.coroutines.delay

internal data class ModuleChange(val name: String, val kind: String, val additions: Int, val deletions: Int)
internal class SessionModuleData(session: Session, details: Map<String, JSONObject>) {
    val history = details["recap_history"]?.array("history").orEmpty().filter { recapHeadline(it.text("headline")).isNotBlank() }
    private val latest = details["recap"]?.optJSONObject("latest")?.takeIf { recapHeadline(it.text("headline")).isNotBlank() } ?: history.lastOrNull()
    val recap = recapHeadline(latest?.text("headline", session.recap) ?: session.recap)
    val bullets = latest?.optJSONArray("bullets")?.let { a -> (0 until a.length()).map { a.getString(it) } }.orEmpty()
    val recapAt = latest?.let { it.optLong("generated_at") / 1000 } ?: session.recapAt
    private val git = details["git"]
    val branch = git?.text("branch", session.branch) ?: session.branch
    val files = git?.array("files").orEmpty().map { ModuleChange(it.text("path"), it.text("status"), it.optInt("additions"), it.optInt("deletions")) }
    val changes = details["changes"]?.array("by_language").orEmpty().flatMap { it.array("changes") }.map { ModuleChange(it.text("name"), it.text("kind"), it.optInt("additions"), it.optInt("deletions")) }
    val commits = details["commit_history"]?.array("history")?.reversed() ?: git?.array("recent_commits").orEmpty()
    val stats = details["stats"] ?: JSONObject()
}

@Composable internal fun SessionModules(session: Session, data: SessionModuleData, prefs: UiPreferences, prs: List<PullRequest>, modifier: Modifier = Modifier, revealScope: AnimatedVisibilityScope? = null) {
    var expanded by rememberSaveable(session.id) { mutableStateOf(emptyList<String>()) }
    fun toggle(key: String) { expanded = if (key in expanded) expanded - key else expanded + key }
    val uri = LocalUriHandler.current
    Column(modifier.fillMaxWidth().background(Panel)) {
        Column(Modifier.weight(1f, fill = false).verticalScroll(rememberScrollState())) {
            if (prefs.visible("recap") && data.recap.isNotBlank()) {
                ModuleSection("Recap", data.recap, updatedAt = data.recapAt, color = CrabColors.Cyan, expanded = "recap" in expanded, toggle = { toggle("recap") }, modifier = revealScope.moduleEntrance(0)) {
                    Column(Modifier.padding(horizontal = 12.dp).padding(bottom = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(data.recap, color = Color(0xFFE2E7ED), fontSize = 13.sp, lineHeight = 18.sp)
                        data.bullets.forEach { bullet ->
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) { Text("·", color = CrabColors.Cyan); Text(bullet, color = Muted, fontSize = 12.sp, lineHeight = 17.sp) }
                        }
                        data.history.dropLast(1).reversed().forEach { previous ->
                            ModuleDivider()
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
                                Text(previous.text("headline"), color = Muted, fontSize = 12.sp, lineHeight = 17.sp, modifier = Modifier.weight(1f))
                                ModuleAge(previous.optLong("generated_at") / 1000)
                            }
                        }
                    }
                }
            }
            if (prefs.visible("prs") && prs.isNotEmpty()) {
                val open = prs.count { it.state == "OPEN" }; val merged = prs.count { it.state == "MERGED" }
                val summary = listOfNotNull(if (open > 0) "$open open" else null, if (merged > 0) "$merged merged" else null).joinToString(" · ")
                ModuleSection("Pull requests", listOf("${prs.size} PR${if (prs.size == 1) "" else "s"}", summary).filter { it.isNotBlank() }.joinToString(" · "), "", CrabColors.Purple, "prs" in expanded, { toggle("prs") }, modifier = revealScope.moduleEntrance(1), updatedAt = prs.maxOf { it.updatedAt }) {
                    Column(Modifier.padding(horizontal = 10.dp).padding(bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        prs.forEach { pr -> ModulePrRow(pr) { if (pr.url.startsWith("https://")) uri.openUri(pr.url) } }
                    }
                }
            }
            if (prefs.visible("git") && (data.branch.isNotBlank() || data.files.isNotEmpty())) {
                ModuleSection("Git", data.branch, if (data.files.isEmpty()) "Clean" else "${data.files.size} file${if (data.files.size == 1) "" else "s"}", CrabColors.Green, "git" in expanded, { toggle("git") }, modifier = revealScope.moduleEntrance(2)) {
                    if (data.files.isEmpty()) Text("Working tree clean", color = Muted, fontSize = 12.sp, modifier = Modifier.padding(horizontal = 12.dp).padding(bottom = 10.dp))
                    else ModuleChanges(data.files)
                }
            }
            if (prefs.visible("changes") && data.changes.isNotEmpty()) {
                ModuleSection("Changes", "${data.changes.size} symbol${if (data.changes.size == 1) "" else "s"}", color = CrabColors.Orange, expanded = "changes" in expanded, toggle = { toggle("changes") }, modifier = revealScope.moduleEntrance(3)) { ModuleChanges(data.changes) }
            }
            if (prefs.visible("commits") && data.commits.isNotEmpty()) {
                ModuleSection("Commits", data.commits.first().text("subject"), "${data.commits.size}", Muted, "commits" in expanded, { toggle("commits") }, modifier = revealScope.moduleEntrance(4), updatedAt = data.commits.maxOf { it.optLong("timestamp") }) {
                    Column(Modifier.padding(horizontal = 12.dp).padding(bottom = 10.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        data.commits.forEach { commit ->
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Text(commit.text("short_hash"), color = CrabColors.Orange, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                                Text(commit.text("subject"), color = Muted, fontSize = 11.sp, lineHeight = 16.sp, fontFamily = FontFamily.Monospace, modifier = Modifier.weight(1f))
                                ModuleAge(commit.optLong("timestamp"))
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable private fun ModuleSection(title: String, summary: String, suffix: String = "", color: Color, expanded: Boolean, toggle: () -> Unit, modifier: Modifier = Modifier, updatedAt: Long = 0, content: @Composable () -> Unit) {
    val angle by animateFloatAsState(if (expanded) 90f else 0f, spring(dampingRatio = .8f), label = "Section chevron")
    Column(modifier) {
        Row(Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable(role = Role.Button, onClick = toggle).semantics { contentDescription = title; stateDescription = if (expanded) "Expanded" else "Collapsed" }.padding(horizontal = 10.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(painterResource(R.drawable.ic_chevron_down), contentDescription = null, tint = color, modifier = Modifier.size(10.dp).rotate(angle - 90f))
            Text(title, color = color, fontSize = 11.sp, fontWeight = FontWeight.SemiBold, maxLines = 1)
            Text(summary, color = Muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            if (suffix.isNotBlank()) Text(suffix, color = Muted, fontSize = 10.sp, maxLines = 1)
            ModuleAge(updatedAt)
        }
        AnimatedVisibility(expanded, enter = expandVertically(spring(dampingRatio = .85f, stiffness = 500f)) + fadeIn(tween(150)), exit = shrinkVertically(tween(180)) + fadeOut(tween(120))) { content() }
        ModuleDivider()
    }
}
// Rise from the summary bar, nearest row first. Collapse is a shorter sweep.
@Composable private fun AnimatedVisibilityScope?.moduleEntrance(index: Int): Modifier {
    val offset = with(LocalDensity.current) { 18.dp.roundToPx() }
    return this?.run {
        val delay = (4 - index) * 35
        val lift = tween<IntOffset>(320, delay, CubicBezierEasing(.18f, 1.25f, .36f, 1f))
        Modifier.animateEnterExit(
            enter = fadeIn(tween(180, delay)) + slideInVertically(lift) { offset } +
                scaleIn(tween(320, delay, CubicBezierEasing(.18f, 1.25f, .36f, 1f)), initialScale = .96f, transformOrigin = TransformOrigin(.5f, 1f)),
            exit = fadeOut(tween(100, index * 16)) + slideOutVertically(tween(150, index * 16)) { offset } +
                scaleOut(tween(150, index * 16), targetScale = .96f, transformOrigin = TransformOrigin(.5f, 1f)))
    } ?: Modifier
}

@Composable private fun ModuleAge(timestamp: Long) {
    if (timestamp <= 0) return
    val now by produceState(System.currentTimeMillis() / 1000) {
        while (true) { value = System.currentTimeMillis() / 1000; delay(30_000) }
    }
    Text(sessionAge(timestamp, now), color = Muted, fontSize = 10.sp, maxLines = 1, softWrap = false)
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
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.Top) {
                Text(pr.title, color = Muted, fontSize = 12.sp, lineHeight = 17.sp, modifier = Modifier.weight(1f))
                ModuleAge(pr.updatedAt)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(if (pr.branch.isNotBlank()) "⑂ ${pr.branch}" else "", color = Muted, fontSize = 10.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                ModuleDiff(pr.additions, pr.deletions)
                if (pr.checks.isNotBlank()) Text(pr.checks, color = if (pr.checksFailed > 0) CrabColors.Red else if (pr.checksPending > 0) CrabColors.Yellow else CrabColors.Green, fontSize = 10.sp, maxLines = 1)
            }
        }
    }
}
@Composable internal fun SessionSummaryBar(session: Session, stats: JSONObject, prefs: UiPreferences, expanded: Boolean, toggle: () -> Unit) {
    val angle by animateFloatAsState(if (expanded) 0f else 180f, spring(dampingRatio = .75f, stiffness = 500f), label = "Modules chevron")
    Column(Modifier.fillMaxWidth().background(Panel)) {
        ModuleDivider()
        Row(Modifier.fillMaxWidth().height(44.dp).clickable(role = Role.Button, onClick = toggle)
            .semantics { contentDescription = "Session summary"; stateDescription = if (expanded) "Expanded" else "Collapsed" }, verticalAlignment = Alignment.CenterVertically) {
            val foreground = remember(session.color) { runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(CrabColors.Title) }
            val background = remember(session.background) { runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel) }
            Box(Modifier.padding(start = 6.dp).width(38.dp).height(25.dp).background(background, RoundedCornerShape(5.dp)), contentAlignment = Alignment.Center) {
                Text(session.glyph, color = foreground, fontSize = 13.sp, fontFamily = FontFamily.Monospace, maxLines = 1)
            }
            ModuleStats(session, stats, prefs, Modifier.weight(1f))
            Box(Modifier.size(44.dp).background(CrabColors.Orange.copy(alpha = if (expanded) .08f else .03f)), contentAlignment = Alignment.Center) {
                Icon(painterResource(R.drawable.ic_chevron_down), contentDescription = null, tint = CrabColors.Orange, modifier = Modifier.size(16.dp).rotate(angle))
            }
        }
    }
}
@Composable private fun ModuleStats(session: Session, stats: JSONObject, prefs: UiPreferences, modifier: Modifier) {
    val now by produceState(System.currentTimeMillis() / 1000) {
        while (true) { value = System.currentTimeMillis() / 1000; delay(1000) }
    }
    SubcomposeLayout(modifier.height(44.dp)) { constraints ->
        // Measure the full wording first, then shorten only when it overflows.
        val full = subcompose("measure") {
            ModuleStatsRow(session, stats, prefs, now, compact = false, Modifier.clearAndSetSemantics {})
        }
            .single().measure(constraints.copy(minWidth = 0, maxWidth = Constraints.Infinity))
        val row = subcompose("visible") {
            ModuleStatsRow(session, stats, prefs, now, compact = full.width > constraints.maxWidth,
                Modifier.horizontalScroll(rememberScrollState()))
        }.single().measure(constraints)
        layout(constraints.maxWidth, constraints.maxHeight) { row.placeRelative(0, 0) }
    }
}
@Composable private fun ModuleStatsRow(session: Session, stats: JSONObject, prefs: UiPreferences, now: Long, compact: Boolean, modifier: Modifier = Modifier) {
    fun age(key: String, fallback: Long): String {
        val timestamp = stats.optLong(key, fallback)
        return if (timestamp > 0) sessionAge(timestamp, now) else "—"
    }
    Row(modifier.height(44.dp).padding(horizontal = 8.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
        SessionStatus(session)
        if (prefs.visible("sessionTime")) ModuleStat("◉", sessionDuration(if (stats.has("work_seconds")) stats.optLong("work_seconds") else session.stats.workSeconds), "Session time", CrabColors.Blue)
        if (prefs.visible("thinkingTime")) ModuleStat("◐", sessionDuration(if (stats.has("thinking_seconds")) stats.optLong("thinking_seconds") else session.stats.thinkingSeconds), "Thinking time", CrabColors.Green)
        if (prefs.visible("prompts")) ModuleStat("⟩", stats.opt("prompts")?.toString() ?: session.stats.prompts?.toString() ?: "—", "Prompts", CrabColors.Orange, age("prompts_changed_at", session.stats.promptAt), compact)
        if (prefs.visible("completions")) ModuleStat("⋖", stats.opt("completions")?.toString() ?: session.stats.completions?.toString() ?: "—", "Completions", CrabColors.DarkOrange, age("completions_changed_at", session.stats.completionAt), compact)
        if (prefs.visible("tools")) ModuleStat("⚒", stats.opt("tools")?.toString() ?: session.stats.tools?.toString() ?: "—", "Tools", CrabColors.Orange)
        if (prefs.visible("compactions")) ModuleStat("⊜", stats.opt("compressions")?.toString() ?: session.stats.compactions?.toString() ?: "—", "Compactions", CrabColors.Purple)
    }
}
@Composable private fun ModuleStat(symbol: String, value: String, label: String, color: Color, age: String = "", compact: Boolean = false) {
    Row(Modifier.clearAndSetSemantics { contentDescription = "$label: $value" + if (age.isBlank()) "" else ", $age" }, horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(symbol, color = color, fontSize = 11.sp, fontFamily = FontFamily.Monospace, maxLines = 1)
        Text(value, color = if (age.isBlank()) color else CrabColors.Title, fontSize = 11.sp, fontFamily = FontFamily.Monospace, maxLines = 1)
        if (age.isNotBlank()) Text(if (compact) age.removeSuffix(" ago") else age, color = color, fontSize = 11.sp, fontFamily = FontFamily.Monospace, maxLines = 1)
    }
}
