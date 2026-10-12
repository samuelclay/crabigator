package com.crabigator.app

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlin.math.cos

@Composable internal fun RosterPlaceholder(status: RosterStatus, retry: () -> Unit, pairing: @Composable () -> Unit) {
    Crossfade(status, animationSpec = tween(180), label = "Session loading") { current ->
        Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(28.dp),
            horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
            when (current) {
                RosterStatus.Loading -> {
                    val animation = rememberInfiniteTransition(label = "Loading sessions")
                    val phase by animation.animateFloat(0f, 1f, infiniteRepeatable(tween(1100, easing = LinearEasing)), label = "Loading dots")
                    Canvas(Modifier.size(56.dp, 24.dp).clearAndSetSemantics {
                        contentDescription = "Loading sessions"
                        progressBarRangeInfo = ProgressBarRangeInfo.Indeterminate
                    }) {
                        for (dot in 0..2) {
                            val pulse = ((cos((phase - dot * .18f) * Math.PI * 2) + 1) / 2).toFloat()
                            drawCircle(CrabColors.Cyan.copy(alpha = .25f + .75f * pulse), 3.dp.toPx(),
                                Offset(size.width * (.2f + dot * .3f), size.height / 2 - 3.dp.toPx() * pulse))
                        }
                    }
                }
                RosterStatus.Failed -> {
                    Text("Couldn’t load sessions", color = Muted, fontSize = 16.sp)
                    OutlinedButton(onClick = retry) { Text("Retry") }
                }
                RosterStatus.Ready -> {
                    Text("No sessions", color = Muted, fontSize = 16.sp)
                    pairing()
                }
            }
        }
    }
}

@Composable internal fun EmptyPairing(model: SessionModel) {
    val state by model.state.collectAsStateWithLifecycle()
    var code by rememberSaveable { mutableStateOf("") }
    Text("Run on your computer", color = Muted, fontSize = 13.sp)
    SelectionContainer { Text("crabigator pair", fontFamily = FontFamily.Monospace, color = Mint, fontSize = 18.sp) }
    OutlinedTextField(code, { code = it.uppercase().take(11) }, label = { Text("Pairing code") },
        placeholder = { Text("ABC-DEF-GHI") }, singleLine = true, modifier = Modifier.fillMaxWidth())
    Button(onClick = { model.pair(code, model.credentials.origin) }, enabled = code.isNotBlank() && !state.loading) {
        Text(if (state.loading) "Connecting…" else "Pair computer")
    }
}
