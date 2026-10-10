package com.crabigator.app

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import kotlin.math.abs

/** Claim only a rightward drag; vertical scrolling and long-press selection keep their gestures. */
internal fun Modifier.sessionSwipe(
    key: Any?,
    start: () -> Unit,
    drag: (Float) -> Unit,
    finish: (Float, Boolean) -> Unit,
) = composed {
    var coordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }
    Modifier.onGloballyPositioned { coordinates = it }.pointerInput(key) {
        fun rootPosition(position: Offset): Offset = coordinates?.takeIf { it.isAttached }?.localToRoot(position) ?: position
        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
            val origin = rootPosition(down.position)
            var previous = origin
            val velocity = VelocityTracker()
            velocity.addPosition(down.uptimeMillis, origin)
            var claimed = false
            var released = false
            try {
                while (true) {
                    val event = awaitPointerEvent(PointerEventPass.Initial)
                    val change = event.changes.firstOrNull { it.id == down.id } ?: break
                    if (event.changes.count { it.pressed } > 1 || change.isConsumed) break
                    // The pane follows the finger, so local coordinates move too.
                    val position = rootPosition(change.position)
                    velocity.addPosition(change.uptimeMillis, position)
                    if (!change.pressed) {
                        released = true
                        if (claimed) change.consume()
                        break
                    }
                    val distance = position - origin
                    if (!claimed) {
                        if (change.uptimeMillis - down.uptimeMillis >= viewConfiguration.longPressTimeoutMillis) break
                        if (abs(distance.y) > viewConfiguration.touchSlop && abs(distance.y) >= abs(distance.x)) break
                        if (distance.x < -viewConfiguration.touchSlop) break
                        if (distance.x <= viewConfiguration.touchSlop) continue
                        claimed = true
                        start()
                        drag(distance.x - viewConfiguration.touchSlop)
                    } else drag(position.x - previous.x)
                    previous = position
                    change.consume()
                }
            } finally {
                if (claimed) finish(velocity.calculateVelocity().x, !released)
            }
        }
    }
}
