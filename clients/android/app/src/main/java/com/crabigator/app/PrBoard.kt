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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable internal fun PrBoard(
    roster: List<Session>, prs: List<PullRequest>, machines: List<String>, clock: State<Long>,
    showEnded: Boolean, query: String, selectedId: String?, collapsed: List<String>, state: LazyListState, toggle: (String) -> Unit,
    select: (Session, List<Session>) -> Unit,
) {
    // Regroup at minute boundaries, while the recency text keeps its normal clock.
    val minute = clock.value / 60
    val groups = remember(roster, prs, machines, minute, showEnded, query) {
        prBoardGroups(roster, prs, machines, minute * 60, showEnded, query)
    }
    val order = remember(groups) { groups.flatMap { it.sessions }.map { it.session } }
    val watched = remember(prs, query) { prs.filter { it.sessions.isEmpty() && "${it.title} ${it.repo} ${it.number}".contains(query, true) } }
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
                                BoardSessionRow(entry.session, selectedId == entry.session.id, wide, clock) { select(entry.session, order) }
                                entry.prs.forEach { pr -> BoardPrRow(pr, wide, Modifier.padding(start = if (wide) 44.dp else 12.dp)) }
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
                if ("\u0000watched" !in collapsed) items(watched, key = { "watched-${it.key}" }, contentType = { "pr" }) { BoardPrRow(it, wide) }
            }
        }
    }
}

@Composable private fun BoardSessionRow(session: Session, selected: Boolean, wide: Boolean, clock: State<Long>, open: () -> Unit) {
    Surface(onClick = open, color = if (selected) CrabColors.Title.copy(alpha = .13f) else Color.Transparent,
        shape = RoundedCornerShape(6.dp), modifier = Modifier.fillMaxWidth()) {
        Row(Modifier.heightIn(min = 44.dp).padding(horizontal = 8.dp, vertical = 5.dp), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
            val fg = remember(session.color) { runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(Peach) }
            val bg = remember(session.background) { runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel) }
            Surface(color = bg, shape = RoundedCornerShape(4.dp), modifier = Modifier.width(38.dp)) {
                Text(session.glyph, color = fg, fontSize = 13.sp, maxLines = 1, softWrap = false, textAlign = androidx.compose.ui.text.style.TextAlign.Center, modifier = Modifier.padding(vertical = 3.dp))
            }
            if (!wide) SessionStatus(session)
            Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(session.title, color = if (session.active) CrabColors.Title else Muted, fontSize = 14.sp, lineHeight = 19.sp,
                    maxLines = if (wide) 1 else 2, overflow = TextOverflow.Ellipsis)
                if (!wide) Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(session.branch, color = Muted, fontSize = 11.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                    BoardRecency(session, clock)
                }
            }
            if (wide) {
                Text(session.branch, color = Muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.width(180.dp))
                Row(Modifier.width(300.dp), verticalAlignment = Alignment.CenterVertically) {
                    SessionStatus(session)
                    Spacer(Modifier.weight(1f))
                    BoardRecency(session, clock)
                }
            }
        }
    }
}

@Composable private fun BoardRecency(session: Session, clock: State<Long>) {
    val now = clock.value
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("⟩ ${if (session.stats.promptAt > 0) sessionAge(session.stats.promptAt, now).removeSuffix(" ago") else "—"}", color = CrabColors.Orange, fontSize = 11.sp, maxLines = 1, modifier = Modifier.clearAndSetSemantics { contentDescription = "Last prompt: ${if (session.stats.promptAt > 0) sessionAge(session.stats.promptAt, now) else "unknown"}" })
        Text("⋖ ${if (session.stats.completionAt > 0) sessionAge(session.stats.completionAt, now) else "—"}", color = CrabColors.DarkOrange, fontSize = 11.sp, maxLines = 1, modifier = Modifier.clearAndSetSemantics { contentDescription = "Last completion: ${if (session.stats.completionAt > 0) sessionAge(session.stats.completionAt, now) else "unknown"}" })
    }
}

@Composable private fun BoardPrRow(pr: PullRequest, wide: Boolean, modifier: Modifier = Modifier) {
    val uri = LocalUriHandler.current
    val canOpen = pr.url.startsWith("https://")
    val titleColor = if (pr.primary || pr.watched) CrabColors.Purple else Muted
    val stateColor = when { pr.state == "MERGED" -> CrabColors.Purple; pr.state == "CLOSED" -> CrabColors.Red; pr.draft -> Muted; else -> Mint }
    Column(modifier.fillMaxWidth().clickable(enabled = canOpen, onClickLabel = "Open pull request") { uri.openUri(pr.url) }
        .heightIn(min = 44.dp).padding(horizontal = 8.dp, vertical = 5.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("${if (pr.primary || pr.watched) "★" else "↳"} #${pr.number}  ${pr.title}", color = titleColor, fontSize = 13.sp, lineHeight = 18.sp,
                maxLines = if (wide) 1 else 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            if (wide) {
                Text(pr.branch, color = Muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.width(180.dp))
                Row(Modifier.width(300.dp), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.weight(1f)) { PrChanges(pr) }
                    Text(if (pr.draft) "Draft" else pr.state.lowercase().replaceFirstChar { it.uppercase() }, color = stateColor, fontSize = 12.sp, modifier = Modifier.width(60.dp))
                    PrChecks(pr, Modifier.width(116.dp))
                }
            }
        }
        if (!wide) FlowRow(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            PrChanges(pr)
            Text(if (pr.draft) "Draft" else pr.state.lowercase().replaceFirstChar { it.uppercase() }, color = stateColor, fontSize = 11.sp)
            PrChecks(pr)
        }
    }
}

@Composable private fun PrChanges(pr: PullRequest) {
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        pr.additions?.let { Text("+$it", color = Mint, fontSize = 11.sp) }
        pr.deletions?.let { Text("−$it", color = CrabColors.Red, fontSize = 11.sp) }
    }
}

@Composable private fun PrChecks(pr: PullRequest, modifier: Modifier = Modifier) {
    val merge = if (pr.state != "OPEN" || pr.mergeState == "UNKNOWN") "" else if (pr.mergeable == "CONFLICTING") "Conflicts" else pr.mergeState.lowercase().replaceFirstChar { it.uppercase() }
    Text(listOf(pr.checks, merge).filter { it.isNotBlank() }.joinToString(" · "),
        color = if (pr.checksFailed > 0 || (pr.state == "OPEN" && pr.mergeable == "CONFLICTING")) CrabColors.Red else Muted,
        fontSize = 11.sp, maxLines = 2, lineHeight = 14.sp, modifier = modifier)
}
