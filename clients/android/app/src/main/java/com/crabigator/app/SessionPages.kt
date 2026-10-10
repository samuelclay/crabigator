package com.crabigator.app

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.clearAndSetSemantics

/** Only the selected session is live. Exiting pages retain their last frame until hidden. */
@Composable internal fun SessionPages(
    state: AppState,
    model: SessionModel,
    wide: Boolean,
    close: () -> Unit,
    swipe: Modifier,
    style: (Rect) -> Unit,
) {
    val selected = state.selected?.id ?: return
    val frames = remember { mutableMapOf<String, AppState>() }
    SideEffect { frames[selected] = state }
    AnimatedContent(selected, modifier = Modifier.fillMaxSize(), label = "Session pages",
        transitionSpec = {
            (slideInHorizontally(tween(360, easing = FastOutSlowInEasing)) { it * state.pageDirection } togetherWith
                slideOutHorizontally(tween(360, easing = FastOutSlowInEasing)) { -it * state.pageDirection })
                .using(null)
        }) { id ->
        val active = id == selected
        val frame = if (active) state else frames[id]
        DisposableEffect(id) { onDispose { frames.remove(id) } }
        if (frame != null) Box(Modifier.fillMaxSize().then(if (active) Modifier else Modifier
            .clearAndSetSemantics {}
            .pointerInput(Unit) {
                awaitPointerEventScope {
                    while (true) awaitPointerEvent(PointerEventPass.Initial).changes.forEach { it.consume() }
                }
            })) {
            SessionDetail(frame, model, wide, close, if (active) swipe else Modifier, active, style)
        }
    }
}
