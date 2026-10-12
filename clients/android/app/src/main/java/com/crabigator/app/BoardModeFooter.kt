package com.crabigator.app

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable internal fun BoardModeFooter(list: Boolean, mode: (Boolean) -> Unit, expanded: Boolean?, modifier: Modifier = Modifier) {
    Surface(modifier, color = Panel, shape = RoundedCornerShape(20.dp), shadowElevation = 12.dp,
        border = BorderStroke(1.dp, Muted.copy(alpha = .2f))) {
        Row(Modifier.padding(4.dp), horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
            SingleChoiceSegmentedButtonRow(Modifier.width(if (expanded == null) 218.dp else 252.dp)) {
                listOf(true, false).forEachIndexed { index, isList ->
                    SegmentedButton(selected = list == isList, onClick = { mode(isList) },
                        modifier = Modifier.semantics { if (list == isList && expanded != null) stateDescription = if (expanded) "Full width; tap for sidebar" else "Sidebar; tap for full width" },
                        shape = SegmentedButtonDefaults.itemShape(index, 2, RoundedCornerShape(16.dp)),
                        colors = SegmentedButtonDefaults.colors(activeContainerColor = Peach.copy(alpha = .16f), activeContentColor = Peach,
                            inactiveContainerColor = Panel, inactiveContentColor = Muted, activeBorderColor = Peach.copy(alpha = .3f), inactiveBorderColor = Muted.copy(alpha = .2f)),
                        icon = { Icon(painterResource(if (isList) R.drawable.ic_list else R.drawable.ic_board), null, Modifier.size(18.dp)) }) {
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(if (isList) "List" else "PR board", fontSize = 12.sp)
                            if (list == isList && expanded != null) Icon(painterResource(if (expanded) R.drawable.ic_collapse else R.drawable.ic_expand),
                                if (expanded) "Show sidebar" else "Show full width", Modifier.size(15.dp))
                        }
                    }
                }
            }

        }
    }
}
