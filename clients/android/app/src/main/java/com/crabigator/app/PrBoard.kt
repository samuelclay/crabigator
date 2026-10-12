package com.crabigator.app

import androidx.compose.animation.core.tween
import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.selected
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable internal fun PrBoard(
    groups: List<BoardMachine>, watched: List<PullRequest>, clock: State<Long>,
    query: String, selectedId: String?, collapsed: List<String>, state: LazyListState, toggle: (String) -> Unit,
    select: (Session, List<Session>) -> Unit,
) {
    val order = remember(groups) { groups.flatMap { it.sessions }.map { it.session } }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val wide = maxWidth >= 840.dp
        if (groups.isEmpty() && watched.isEmpty()) Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text(if (query.isNotBlank()) "No matching sessions" else "No active sessions", color = Muted)
        }
        LazyColumn(state = state, contentPadding = PaddingValues(start = 12.dp, end = 12.dp, bottom = 88.dp)) {
            groups.forEach { machine ->
                stickyHeader(key = "machine-${machine.machine}", contentType = "machine") { index ->
                    MachineHeader(machine.machine, machine.sessions.size, machine.machine in collapsed, Modifier.animateItem(placementSpec = tween(320))) {
                        if (machine.machine !in collapsed && state.firstVisibleItemIndex > index) state.requestScrollToItem(index)
                        toggle(machine.machine)
                    }
                }
                if (machine.machine in collapsed) return@forEach
                machine.bands.forEach { band ->
                    val prefix = "${machine.machine.length}:${machine.machine}-${band.age}"
                    item(key = "$prefix-age", contentType = "age") {
                        val color = TerminalText.palette(band.age.palette)
                        Text(band.age.label, color = color, fontSize = 12.sp, fontWeight = FontWeight.Medium,
                            modifier = Modifier.semantics { heading() }.fillMaxWidth().background(color.copy(alpha = .13f), RoundedCornerShape(4.dp)).padding(horizontal = 10.dp, vertical = 5.dp))
                    }
                    band.repositories.forEach { repo ->
                        item(key = "$prefix-${repo.repo}-repo", contentType = "repo") {
                            Text(repo.repo, color = CrabColors.Yellow, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.semantics { heading() }.padding(start = 10.dp, top = 10.dp, bottom = 3.dp))
                        }
                        items(repo.sessions, key = { "session-${it.session.id}" }, contentType = { "session" }) { entry ->
                            Column(Modifier.animateItem(placementSpec = tween(320), fadeInSpec = tween(220), fadeOutSpec = tween(180)).padding(bottom = 5.dp)) {
                                BoardSessionBlock(entry, selectedId == entry.session.id, wide, clock) { select(entry.session, order) }
                                HorizontalDivider(color = Color(0xFF252D37), modifier = Modifier.padding(top = 4.dp))
                            }
                        }
                    }
                }
            }
            if (watched.isNotEmpty()) {
                stickyHeader(key = "watched", contentType = "machine") { index ->
                    MachineHeader("Tracked PRs", watched.size, "\u0000watched" in collapsed, countNoun = "pull request") {
                        if ("\u0000watched" !in collapsed && state.firstVisibleItemIndex > index) state.requestScrollToItem(index)
                        toggle("\u0000watched")
                    }
                }
                if ("\u0000watched" !in collapsed) items(watched, key = { "watched-${it.key}" }, contentType = { "pr" }) {
                    BoardPrRow(it, wide, Modifier.padding(horizontal = 8.dp, vertical = 7.dp))
                }
            }
        }
    }
}

@Composable internal fun BoardSessionBlock(entry: BoardSession, selected: Boolean, wide: Boolean, clock: State<Long>, open: () -> Unit) {
    val session = entry.session
    Surface(onClick = open, color = if (selected) Peach.copy(alpha = .13f) else Color.Transparent,
        border = if (selected) BorderStroke(2.dp, Peach) else null,
        shape = RoundedCornerShape(6.dp), modifier = Modifier.fillMaxWidth().semantics { this.selected = selected }) {
        Row(Modifier.heightIn(min = 44.dp).padding(horizontal = 8.dp, vertical = 7.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.Top) {
            val fg = remember(session.color) { runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(Peach) }
            val bg = remember(session.background) { runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel) }
            Surface(color = bg, shape = RoundedCornerShape(4.dp), modifier = Modifier.width(38.dp)) {
                Text(session.glyph, color = fg, fontSize = 13.sp, maxLines = 1, softWrap = false, textAlign = androidx.compose.ui.text.style.TextAlign.Center, modifier = Modifier.padding(vertical = 2.dp))
            }
            if (!wide) SessionStatus(session)
            // PRs share the title column, so their indent stays correct at every font size.
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(7.dp)) {
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.Top) {
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Text(session.title, color = if (session.active) CrabColors.Title else Muted, fontSize = 14.sp, lineHeight = 19.sp,
                            maxLines = if (wide) 1 else 2, overflow = TextOverflow.Ellipsis)
                        if (!wide) Row(verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(session.branch, color = Muted, fontSize = 11.sp, lineHeight = 15.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                            BoardRecency(session, clock)
                        }
                    }
                    if (wide) {
                        Text(session.branch, color = Muted, fontSize = 12.sp, lineHeight = 17.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.width(180.dp))
                        Row(Modifier.width(300.dp), verticalAlignment = Alignment.Top) {
                            SessionStatus(session)
                            Spacer(Modifier.weight(1f))
                            BoardRecency(session, clock)
                        }
                    }
                }
                entry.prs.forEach { BoardPrRow(it, wide) }
            }
        }
    }
}

@Composable private fun BoardRecency(session: Session, clock: State<Long>) {
    val now = clock.value
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("⟩ ${if (session.stats.promptAt > 0) sessionAge(session.stats.promptAt, now).removeSuffix(" ago") else "—"}", color = CrabColors.Orange, fontSize = 11.sp, lineHeight = 15.sp, maxLines = 1, modifier = Modifier.clearAndSetSemantics { contentDescription = "Last prompt: ${if (session.stats.promptAt > 0) sessionAge(session.stats.promptAt, now) else "unknown"}" })
        Text("⋖ ${if (session.stats.completionAt > 0) sessionAge(session.stats.completionAt, now) else "—"}", color = CrabColors.DarkOrange, fontSize = 11.sp, lineHeight = 15.sp, maxLines = 1, modifier = Modifier.clearAndSetSemantics { contentDescription = "Last completion: ${if (session.stats.completionAt > 0) sessionAge(session.stats.completionAt, now) else "unknown"}" })
    }
}

@Composable private fun BoardPrRow(pr: PullRequest, wide: Boolean, modifier: Modifier = Modifier) {
    val titleColor = if (pr.primary || pr.watched) CrabColors.Purple else Muted
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.Top) {
            Text("#${pr.number}  ${pr.title}", color = titleColor, fontSize = 13.sp, lineHeight = 18.sp,
                maxLines = if (wide) 1 else 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            if (wide) {
                Text(pr.branch, color = Muted, fontSize = 12.sp, lineHeight = 17.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.width(180.dp))
                PrStatus(pr, Modifier.width(300.dp))
            }
        }
        if (!wide) {
            PrStatus(pr, Modifier.fillMaxWidth())
        }
    }
}

@Composable private fun PrStatus(pr: PullRequest, modifier: Modifier = Modifier) {
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        PrState(pr)
        Box(Modifier.weight(1f)) { PrChanges(pr) }
        PrChecks(pr)
    }
}

@Composable private fun PrState(pr: PullRequest) {
    val color = when { pr.state == "MERGED" -> CrabColors.Purple; pr.state == "CLOSED" -> CrabColors.Red; pr.draft || pr.state.isBlank() -> Muted; else -> Mint }
    Text(if (pr.draft && pr.state == "OPEN") "Draft" else pr.state.lowercase().replaceFirstChar { it.uppercase() }.ifBlank { "Pending" },
        color = color, fontSize = 11.sp, lineHeight = 14.sp, maxLines = 1,
        modifier = Modifier.background(color.copy(alpha = .12f), RoundedCornerShape(4.dp)).padding(horizontal = 5.dp, vertical = 2.dp))
}

@Composable private fun PrChanges(pr: PullRequest) {
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        pr.additions?.let { Text("+$it", color = Mint, fontSize = 11.sp, lineHeight = 14.sp, maxLines = 1, overflow = TextOverflow.Ellipsis) }
        pr.deletions?.let { Text("−$it", color = CrabColors.Red, fontSize = 11.sp, lineHeight = 14.sp, maxLines = 1, overflow = TextOverflow.Ellipsis) }
    }
}

@Composable private fun PrChecks(pr: PullRequest) {
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        if (pr.checks.isNotBlank()) Text(pr.checks,
            color = when { pr.checksFailed > 0 -> CrabColors.Red; pr.checksPending > 0 -> CrabColors.Yellow; else -> Mint },
            fontSize = 11.sp, maxLines = 1, lineHeight = 14.sp, modifier = Modifier.clearAndSetSemantics {
                contentDescription = when { pr.checksFailed > 0 -> "${pr.checksFailed} checks failed"; pr.checksPending > 0 -> "${pr.checksPending} checks pending"; else -> "Checks passed" }
            })
        if (pr.state == "OPEN") {
            val merge = when { pr.mergeable == "CONFLICTING" -> "Conflicts"; pr.mergeable == "MERGEABLE" -> if (pr.mergeState == "BEHIND") "Behind" else "Clean"; else -> "" }
            if (merge.isNotBlank()) Text(merge,
                color = when (merge) { "Conflicts" -> CrabColors.Red; "Behind" -> CrabColors.Yellow; else -> Mint },
                fontSize = 11.sp, maxLines = 1, lineHeight = 14.sp)
        }
    }
}
