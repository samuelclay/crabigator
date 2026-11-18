package com.crabigator.app

import androidx.compose.foundation.*
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** The three board filters. The chips and the View panel both read and write them here. */
internal object BoardFilters {
    /** Saved activity windows in hours; 0 means every age. */
    val hours = listOf(1, 3, 6, 9, 12, 24, 0)
    fun short(hours: Int) = if (hours == 0) "All" else "${hours}h"
    fun long(hours: Int) = if (hours == 0) "All ages" else if (hours == 1) "Last hour" else "Last $hours hours"
    /** Each window takes the colour of the board's time band that it reaches. */
    fun color(hours: Int) = TerminalText.palette(BoardAge.entries[BoardFilters.hours.indexOf(hours).coerceAtLeast(0)].palette)
    fun sessions(showEnded: Boolean) = if (showEnded) "Live + ended" else "Live"
    fun computers(sessions: List<Session>, selected: String) = (sessions.map { it.machineGroup } + selected).filter { it.isNotBlank() }.distinct().sorted()
}

/** Sits under the board header. The chips scroll sideways when they do not fit; View stays at the trailing edge. */
@Composable internal fun BoardFilterBar(p: UiPreferences, computers: List<String>, update: ((UiPreferences) -> UiPreferences) -> Unit, viewOpen: Boolean, modifier: Modifier = Modifier, centerGap: Dp = 0.dp, view: (Rect) -> Unit) {
    var viewAnchor by remember { mutableStateOf(Rect.Zero) }
    Row(modifier.fillMaxWidth().padding(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        Row(Modifier.weight(1f).horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            FilterMenuChip(BoardFilters.short(p.boardAgeHours), "Activity", BoardFilters.hours.map(BoardFilters::long), BoardFilters.hours.indexOf(p.boardAgeHours), dot = BoardFilters.color(p.boardAgeHours)) { n -> update { it.copy(boardAgeHours = BoardFilters.hours[n]) } }
            FilterMenuChip(BoardFilters.sessions(p.showEnded), "Sessions", listOf("Live", "Live and ended"), if (p.showEnded) 1 else 0) { n -> update { it.copy(showEnded = n == 1) } }
            FilterMenuChip(p.boardComputer.ifBlank { "All computers" }, "Computer", listOf("All computers") + computers, computers.indexOf(p.boardComputer) + 1,
                color = if (p.boardComputer.isBlank()) PrimaryText else CrabColors.computer(p.boardComputer), optionColors = listOf(null) + computers.map(CrabColors::computer)) { n -> update { it.copy(boardComputer = if (n == 0) "" else computers[n - 1]) } }
        }
        // A floating toolbar sits in the middle of this row on a full-width tablet board.
        if (centerGap > 0.dp) Spacer(Modifier.width(centerGap))
        Box(if (centerGap > 0.dp) Modifier.weight(1f) else Modifier, contentAlignment = Alignment.CenterEnd) {
            ViewButton(viewOpen, Modifier.onGloballyPositioned { viewAnchor = it.boundsInRoot() }) { view(viewAnchor) }
        }
    }
}

/** A chip that names the filter's value and opens its choices. */
@Composable private fun FilterMenuChip(label: String, filter: String, options: List<String>, selected: Int, dot: Color? = null, color: Color = PrimaryText, optionColors: List<Color?> = emptyList(), choose: (Int) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    Box {
        Row(Modifier.heightIn(min = 44.dp).clip(RoundedCornerShape(22.dp)).background(Panel).border(1.dp, Hairline, RoundedCornerShape(22.dp))
            .clickable(onClickLabel = "Change $filter", role = Role.DropdownList) { expanded = true }.semantics { stateDescription = filter }.padding(horizontal = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
            if (dot != null) Box(Modifier.size(8.dp).background(dot, CircleShape))
            Text(label, color = color, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.widthIn(max = 180.dp))
            Icon(painterResource(R.drawable.ic_chevron_down), null, tint = MutedText, modifier = Modifier.size(12.dp))
        }
        DropdownMenu(expanded, { expanded = false }) {
            options.forEachIndexed { index, option ->
                DropdownMenuItem(text = { Text(option, color = optionColors.getOrNull(index) ?: PrimaryText, fontWeight = if (index == selected) FontWeight.SemiBold else FontWeight.Normal) },
                    trailingIcon = { if (index == selected) Icon(painterResource(R.drawable.ic_check), null, tint = CrabColors.Cyan, modifier = Modifier.size(18.dp)) },
                    modifier = Modifier.semantics { this.selected = index == selected },
                    onClick = { choose(index); expanded = false })
            }
        }
    }
}

/** Opens the View panel for the screen it sits on. Filled with the accent while that panel is open. */
@Composable internal fun ViewButton(open: Boolean, modifier: Modifier = Modifier, onClick: () -> Unit) {
    Row(modifier.heightIn(min = 44.dp).clip(RoundedCornerShape(22.dp)).background(if (open) CrabColors.Cyan else Panel)
        .then(if (open) Modifier else Modifier.border(1.dp, Hairline, RoundedCornerShape(22.dp)))
        .clickable(role = Role.Button, onClick = onClick).semantics { stateDescription = if (open) "Open" else "Closed" }.padding(horizontal = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        val content = if (open) Ink else PrimaryText
        Icon(painterResource(R.drawable.ic_view), null, tint = content, modifier = Modifier.size(14.dp))
        Text("View", color = content, fontSize = 13.sp, fontWeight = FontWeight.Bold, maxLines = 1)
    }
}
