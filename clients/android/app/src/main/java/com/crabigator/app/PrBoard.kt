package com.crabigator.app

import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.tween
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.*
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.TextAutoSize
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
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable internal fun PrBoard(
    groups: List<BoardMachine>, watched: List<PullRequest>, clock: State<Long>,
    query: String, selectedId: String?, collapsed: List<String>, state: LazyListState, toggle: (String) -> Unit,
    select: (Session, List<Session>) -> Unit, prefs: UiPreferences = UiPreferences(),
) {
    val spacing = LocalBoardSpacing.current
    val uri = LocalUriHandler.current
    val displayGroups = if (prefs.boardGrouping == "prs") groupBoardByPullRequest(groups, clock.value) { prefs.includesActivity(it, clock.value) } else groups
    val order = remember(displayGroups) { displayGroups.flatMap { it.sessions }.map { it.session } }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val wide = maxWidth >= 840.dp
        if (displayGroups.isEmpty() && watched.isEmpty()) Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Text("No sessions match your filters", color = Muted)
        }
        LazyColumn(state = state, contentPadding = PaddingValues(start = 12.dp, end = 12.dp, bottom = 88.dp)) {
            displayGroups.forEach { machine ->
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
                            modifier = Modifier.semantics { heading() }.fillMaxWidth().background(color.copy(alpha = .13f), RoundedCornerShape(4.dp)).padding(horizontal = 10.dp, vertical = spacing.gap(5)))
                    }
                    band.repositories.forEach { repo ->
                        item(key = "$prefix-${repo.repo}-repo", contentType = "repo") {
                            Text(repo.repo, color = CrabColors.Yellow, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.semantics { heading() }.padding(start = 10.dp, top = spacing.gap(10), bottom = spacing.gap(3)))
                        }
                        items(repo.pullRequests, key = { "pr-${machine.machine}-${it.pr.key}" }, contentType = { "pr-group" }) { block ->
                            Column(Modifier.padding(bottom = spacing.gap(5))) {
                                val link = block.pr.url.takeIf { it.startsWith("https://") }
                                BoardPrRow(block.pr, wide, Modifier.padding(horizontal = 8.dp).then(if (link != null) Modifier.clickable { uri.openUri(link) } else Modifier))
                                block.sessions.forEach { entry ->
                                    BoardSessionBlock(entry.copy(prs = emptyList()), selectedId == entry.session.id, wide, clock, prefs.boardRecaps) { select(entry.session, order) }
                                }
                            }
                        }
                        items(repo.sessions, key = { "session-${it.session.id}" }, contentType = { "session" }) { entry ->
                            Column(Modifier.animateItem(placementSpec = tween(320), fadeInSpec = tween(220), fadeOutSpec = tween(180)).padding(bottom = spacing.gap(5))) {
                                BoardSessionBlock(entry, selectedId == entry.session.id, wide, clock, prefs.boardRecaps) { select(entry.session, order) }
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
                    BoardPrRow(it, wide, Modifier.padding(horizontal = 8.dp, vertical = spacing.gap(7)))
                }
            }
        }
    }
}

@Composable internal fun BoardSessionBlock(entry: BoardSession, selected: Boolean, wide: Boolean, clock: State<Long>, recaps: Boolean = false, open: () -> Unit) {
    val spacing = LocalBoardSpacing.current
    val session = entry.session
    val selection by animateFloatAsState(if (selected) 1f else 0f, tween(if (selected) 120 else 350), label = "Session selection")
    Surface(onClick = open, color = Peach.copy(alpha = .13f * selection),
        border = BorderStroke(2.dp, Peach.copy(alpha = selection)),
        shape = RoundedCornerShape(6.dp), modifier = Modifier.fillMaxWidth().semantics { this.selected = selected }) {
        Row(Modifier.heightIn(min = 44.dp).padding(horizontal = 8.dp, vertical = spacing.gap(7)), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.Top) {
            val fg = remember(session.color) { runCatching { Color(android.graphics.Color.parseColor(session.color)) }.getOrDefault(Peach) }
            val bg = remember(session.background) { runCatching { Color(android.graphics.Color.parseColor(session.background)) }.getOrDefault(Panel) }
            Surface(color = bg, shape = RoundedCornerShape(4.dp), modifier = Modifier.width(38.dp).alignBy { it.measuredHeight / 2 }) {
                Text(session.glyph, color = fg, fontSize = 13.sp, maxLines = 1, softWrap = false, textAlign = androidx.compose.ui.text.style.TextAlign.Center, modifier = Modifier.padding(vertical = 2.dp))
            }
            // PRs share the title column, so their indent stays correct at every font size.
            Column(Modifier.weight(1f).alignBy(SessionTitleCenter), verticalArrangement = Arrangement.spacedBy(spacing.gap(7))) {
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.Top) {
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.gap(4))) {
                        Text(session.title, color = if (session.active) CrabColors.Title else Muted, fontSize = 14.sp, lineHeight = spacing.lineHeight(19),
                            maxLines = if (wide || spacing.tight) 1 else 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.sessionTitleAlignment().animateContentSize(tween(300), alignment = Alignment.TopStart))
                        if (!wide) Row(verticalAlignment = Alignment.Top, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(session.branch, color = Muted, fontSize = 11.sp, lineHeight = spacing.lineHeight(15), maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                            BoardRecency(session, clock, showStatus = true)
                        }
                    }
                    if (wide) {
                        Text(session.branch, color = Muted, fontSize = 12.sp, lineHeight = spacing.lineHeight(17), maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.width(180.dp))
                        Row(Modifier.width(360.dp), verticalAlignment = Alignment.Top) {
                            SessionStatus(session)
                            Spacer(Modifier.width(8.dp))
                            BoardRecency(session, clock)
                            Spacer(Modifier.weight(1f))
                        }
                    }
                }
                if (recaps && session.recap.isNotBlank()) Text(session.recap, color = Muted, fontSize = 12.sp)
                entry.prs.filter { it.visibleOnBoard(session.id) }.forEach { BoardPrRow(it, wide) }
            }
        }
    }
}

@Composable internal fun BoardRecency(session: Session, clock: State<Long>, showPrompt: Boolean = true, showCompletion: Boolean = true, showStatus: Boolean = false) {
    val spacing = LocalBoardSpacing.current
    val now = clock.value
    val count = (if (showPrompt) 1 else 0) + (if (showCompletion) 1 else 0)
    if (count == 0 && !showStatus) return
    val columnWidth = with(LocalDensity.current) { 66.sp.toDp() }
    val statusWidth = with(LocalDensity.current) { 14.sp.toDp() } + 8.dp
    Row(Modifier.width(columnWidth * count + (if (count == 2) 8.dp else 0.dp) + if (showStatus) statusWidth else 0.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        if (showStatus) SessionStatus(session)
        if (showPrompt) Text("⟩ ${if (session.stats.promptAt > 0) sessionAge(session.stats.promptAt, now) else "—"}", color = CrabColors.Orange,
            fontSize = 11.sp, lineHeight = spacing.lineHeight(15), maxLines = 1, softWrap = false, overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f).clearAndSetSemantics { contentDescription = "Last prompt: ${if (session.stats.promptAt > 0) sessionAge(session.stats.promptAt, now) else "unknown"}" })
        if (showCompletion) Text("⋖ ${if (session.stats.completionAt > 0) sessionAge(session.stats.completionAt, now) else "—"}", color = CrabColors.DarkOrange,
            fontSize = 11.sp, lineHeight = spacing.lineHeight(15), maxLines = 1, softWrap = false, overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f).clearAndSetSemantics { contentDescription = "Last completion: ${if (session.stats.completionAt > 0) sessionAge(session.stats.completionAt, now) else "unknown"}" })
    }
}

@Composable private fun BoardPrRow(pr: PullRequest, wide: Boolean, modifier: Modifier = Modifier) {
    val spacing = LocalBoardSpacing.current
    val titleColor = if (pr.primary || pr.watched) CrabColors.Purple else Muted
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(spacing.gap(4))) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.Top) {
            Text("#${pr.number}  ${pr.title}", color = titleColor, fontSize = 13.sp, lineHeight = spacing.lineHeight(18),
                maxLines = if (wide || spacing.tight) 1 else 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f).animateContentSize(tween(300), alignment = Alignment.TopStart))
            if (wide) {
                Text(pr.branch, color = Muted, fontSize = 12.sp, lineHeight = spacing.lineHeight(17), maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.width(180.dp))
                PrStatus(pr, Modifier.width(360.dp))
            }
        }
        if (!wide) {
            PrStatus(pr, Modifier.fillMaxWidth())
        }
    }
}

@Composable private fun PrStatus(pr: PullRequest, modifier: Modifier = Modifier) {
    BoxWithConstraints(modifier) {
        val compact = maxWidth < 320.dp
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(if (compact) 3.dp else 6.dp), verticalAlignment = Alignment.CenterVertically) {
            PrSummary(pr, compact, Modifier.weight(1f))
            PrChecks(pr, compact)
        }
    }
}

@Composable private fun PrSummary(pr: PullRequest, compact: Boolean, modifier: Modifier = Modifier) {
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(if (compact) 3.dp else 6.dp), verticalAlignment = Alignment.CenterVertically) {
        PrState(pr, if (compact) Modifier.width(40.dp) else Modifier, compact)
        PrChanges(pr, Modifier.weight(1f), compact)
    }
}

@Composable private fun PrState(pr: PullRequest, modifier: Modifier = Modifier, compact: Boolean = false) {
    val color = when { pr.state == "MERGED" -> CrabColors.Purple; pr.state == "CLOSED" -> CrabColors.Red; pr.draft || pr.state.isBlank() -> Muted; else -> Mint }
    FittedStatusText(if (pr.draft && pr.state == "OPEN") "Draft" else pr.state.lowercase().replaceFirstChar { it.uppercase() }.ifBlank { "Pending" },
        color, modifier.background(color.copy(alpha = .12f), RoundedCornerShape(4.dp)).padding(horizontal = if (compact) 3.dp else 5.dp, vertical = 2.dp))
}

@Composable private fun PrChanges(pr: PullRequest, modifier: Modifier = Modifier, compact: Boolean = false) {
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(if (compact) 3.dp else 6.dp), verticalAlignment = Alignment.CenterVertically) {
        pr.additions?.let { FittedStatusText("+$it", Mint, Modifier.weight(1f, fill = false)) }
        pr.deletions?.let { FittedStatusText("−$it", CrabColors.Red, Modifier.weight(1f, fill = false)) }
    }
}

@Composable private fun PrChecks(pr: PullRequest, compact: Boolean) {
    Row(horizontalArrangement = Arrangement.spacedBy(if (compact) 2.dp else 6.dp), verticalAlignment = Alignment.CenterVertically) {
        val approval = when {
            pr.state != "OPEN" -> Triple("—", "Not applicable", Muted)
            pr.review == "APPROVED" -> Triple("✓", "Approved", Mint)
            pr.review == "CHANGES_REQUESTED" -> Triple("✗", "Changes requested", CrabColors.Red)
            pr.reviewDismissed -> Triple("⊘", "Review dismissed", CrabColors.Orange)
            else -> Triple("◌", "Awaiting review", Muted)
        }
        StatusColumn(pr.checks.ifBlank { "—" }, width = if (compact) 42.dp else 48.dp,
            color = when { pr.checks.isBlank() -> Muted; pr.checksFailed > 0 -> CrabColors.Red; pr.checksPending > 0 -> CrabColors.Yellow; else -> Mint },
            description = when { pr.checks.isBlank() -> "CI status unavailable"; pr.checksFailed > 0 -> "${pr.checksFailed} checks failed"; pr.checksPending > 0 -> "${pr.checksPending} checks pending"; else -> "Checks passed" })
        StatusColumn(if (pr.unresolvedComments > 0) "💬${pr.unresolvedComments}" else "", width = if (compact) 24.dp else 36.dp,
            color = CrabColors.Orange, description = "Unresolved comments: ${pr.unresolvedComments}")
        StatusColumn(approval.first, width = if (compact) 12.dp else 20.dp, color = approval.third, description = "Approval: ${approval.second}")
        val merge = when {
            pr.state != "OPEN" -> "—"
            pr.mergeable == "CONFLICTING" -> "Conflicts"
            pr.mergeState == "BEHIND" -> "Behind"
            pr.mergeable == "MERGEABLE" -> "Clean"
            else -> "—"
        }
        StatusColumn(merge, width = if (compact) 46.dp else 54.dp, color = when (merge) { "Conflicts" -> CrabColors.Red; "Behind" -> CrabColors.Yellow; "Clean" -> Mint; else -> Muted },
            description = "Branch status: $merge")
    }
}


// Keep status words readable in fixed columns when the surrounding board text is enlarged.
@Composable private fun StatusColumn(text: String, width: Dp, color: Color, description: String) {
    FittedStatusText(text, color, Modifier.width(width).semantics { contentDescription = description })
}

@Composable private fun FittedStatusText(text: String, color: Color, modifier: Modifier = Modifier) {
    val minimumSize = with(LocalDensity.current) { 8.dp.toSp() }.value.coerceAtMost(11f).sp
    BasicText(text, modifier = modifier,
        style = TextStyle(color = color, fontSize = 11.sp, lineHeight = 14.sp),
        maxLines = 1, softWrap = false,
        autoSize = TextAutoSize.StepBased(minFontSize = minimumSize, maxFontSize = 11.sp, stepSize = .25.sp))
}
