package com.crabigator.app

import android.graphics.Paint
import android.graphics.Typeface
import android.provider.Settings
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import org.json.JSONObject
import kotlin.math.roundToInt

/** The flow scenes (crates/flow-render), built into the app as libflow_render.so. */
internal object FlowNative {
    init { System.loadLibrary("flow_render") }
    /** A scene, or 0 for an unknown name. `hue`: NaN for the scene's own colours. */
    @JvmStatic external fun create(name: String, seed: Int, hue: Double): Long
    @JvmStatic external fun free(handle: Long)
    /** `tint`: 0 normal, 1 smoke, 2 blue. */
    @JvmStatic external fun set(handle: Long, level: Double, waiting: Boolean, night: Boolean, tint: Int, boost: Double)
    /** Step once into `out` (`columns * rows * 3` ints); returns milliseconds until the next frame. */
    @JvmStatic external fun frame(handle: Long, columns: Int, rows: Int, out: IntArray): Int
}

private class FlowFrame(val cells: IntArray, val columns: Int, val rows: Int)

/**
 * A session's flow scene, run on the phone with the desktop's own scene code,
 * following the dials the desktop sends in "flow" events: the same scene in the
 * same colours, moving with the same work.
 */
@Composable internal fun FlowScene(event: JSONObject, modifier: Modifier = Modifier) {
    val name = event.optString("scene")
    val seed = event.optLong("seed").toInt()
    val hue = if (event.has("hue")) event.optDouble("hue") else Double.NaN
    val handle = remember(name, seed, hue) { runCatching { FlowNative.create(name, seed, hue) }.getOrDefault(0L) }
    DisposableEffect(handle) { onDispose { if (handle != 0L) FlowNative.free(handle) } }
    val dials by rememberUpdatedState(event)
    var size by remember { mutableStateOf(IntSize.Zero) }
    var frame by remember { mutableStateOf<FlowFrame?>(null) }
    val cellHeight = with(LocalDensity.current) { 7.5.dp.toPx() }
    val context = LocalContext.current
    val still = remember { Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f }
    LaunchedEffect(handle, size) {
        if (handle == 0L || size.width == 0 || size.height == 0) return@LaunchedEffect
        // Cells shaped like a terminal's (0.6 wide to 1.45 tall).
        val rows = maxOf(3, (size.height / cellHeight).roundToInt())
        val columns = maxOf(6, (size.width / (size.height.toFloat() / rows * 0.6f / 1.45f)).roundToInt())
        var applied = ""
        while (isActive) {
            val event = dials
            if (event.toString() != applied) {
                FlowNative.set(handle, event.optDouble("level", 1.0), event.optBoolean("waiting"), event.optBoolean("night"),
                    when (event.optString("tint")) { "smoke" -> 1; "blue" -> 2; else -> 0 }, event.optDouble("boost", 0.0))
                applied = event.toString()
            }
            val cells = IntArray(columns * rows * 3)
            val pace = FlowNative.frame(handle, columns, rows, cells)
            frame = FlowFrame(cells, columns, rows)
            if (still) break
            delay(pace.toLong())
            // Only step while the screen is drawing (not in the background).
            withFrameNanos { }
        }
    }
    Canvas(modifier.background(FlowCells.Backdrop).onSizeChanged { size = it }) { frame?.let { drawFlow(it) } }
}

private val glyphPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { typeface = Typeface.MONOSPACE; textAlign = Paint.Align.CENTER }

private fun DrawScope.drawFlow(frame: FlowFrame) {
    val (columns, rows) = frame.columns to frame.rows
    val subColumns = columns * 2; val subRows = rows * 4
    val sx = size.width / subColumns; val sy = size.height / subRows
    val sub = FlowCells.subBlocks(frame.cells, columns, rows)
    // Runs of one colour along each sub-row, so the blocks meet without seams.
    for (y in 0 until subRows) {
        val y0 = (y * sy).roundToInt().toFloat(); val y1 = ((y + 1) * sy).roundToInt().toFloat()
        var x = 0
        while (x < subColumns) {
            val color = sub[y * subColumns + x]
            var end = x + 1
            while (end < subColumns && sub[y * subColumns + end] == color) end++
            if (color >= 0) {
                val x0 = (x * sx).roundToInt().toFloat()
                drawRect(Color(0xFF000000.toInt() or color), Offset(x0, y0), Size((end * sx).roundToInt() - x0, y1 - y0))
            }
            x = end
        }
    }
    for (i in 0 until columns * rows) {
        val cp = frame.cells[i * 3]; val fg = frame.cells[i * 3 + 1]
        if (FlowCells.mask(cp) != null || fg == FlowCells.CLEAR || cp <= 0x20) continue
        glyphPaint.color = 0xFF000000.toInt() or fg
        glyphPaint.textSize = sy * 3.1f
        drawContext.canvas.nativeCanvas.drawText(String(Character.toChars(cp)), ((i % columns) * 2 + 1) * sx, ((i / columns) * 4 + 3) * sy, glyphPaint)
    }
}

/** Draws a frame's cells with flow's 2x4 masks (bit x + 2y), as its desktop renderer does. */
internal object FlowCells {
    const val CLEAR = 0x01000000
    val Backdrop = Color(0xFF0B0F14)
    private val quad = intArrayOf(0x20, 0x2598, 0x259D, 0x2580, 0x2596, 0x258C, 0x259E, 0x259B, 0x2597, 0x259A, 0x2590, 0x259C, 0x2584, 0x2599, 0x259F, 0x2588)
    private val shade = mapOf(0x2591 to .3, 0x2592 to .55, 0x2593 to .8)
    private val braille = arrayOf(intArrayOf(1, 2, 4, 64), intArrayOf(8, 16, 32, 128))

    /** The lit sub-blocks of a glyph, or null when it isn't a block. */
    fun mask(cp: Int): Int? {
        if (cp == 0x20 || cp == 0xA0) return 0
        quad.indexOf(cp).takeIf { it >= 0 }?.let { q -> return (if (q and 1 != 0) 5 else 0) or (if (q and 2 != 0) 10 else 0) or (if (q and 4 != 0) 80 else 0) or (if (q and 8 != 0) 160 else 0) }
        if (cp in 0x2581..0x2587) return (0xFF shl ((4 - ((cp - 0x2580) / 2.0 + .5).toInt()) * 2)) and 0xFF
        if (cp in 0x2589..0x258F) { val c = ((0x2590 - cp) / 4.0 + .5).toInt(); return if (c >= 2) 0xFF else if (c == 1) 0x55 else 0 }
        if (cp in 0x2800..0x28FF) {
            var mask = 0
            for (x in 0..1) for (y in 0..3) if ((cp - 0x2800) and braille[x][y] != 0) mask = mask or (1 shl (x + 2 * y))
            return mask
        }
        return null
    }

    /** Each sub-block's colour (0xRRGGBB), or -1 where the backdrop shows. */
    fun subBlocks(cells: IntArray, columns: Int, rows: Int): IntArray {
        val subColumns = columns * 2
        val sub = IntArray(subColumns * rows * 4) { -1 }
        for (i in 0 until columns * rows) {
            val cp = cells[i * 3]
            val fg = cells[i * 3 + 1].takeIf { it != CLEAR } ?: -1
            val bg = cells[i * 3 + 2].takeIf { it != CLEAR } ?: -1
            val cx = (i % columns) * 2; val cy = (i / columns) * 4
            val mask = mask(cp); val k = shade[cp]
            for (y in 0..3) for (x in 0..1) {
                sub[(cy + y) * subColumns + cx + x] = when {
                    k != null -> if (fg < 0) bg else mix(if (bg < 0) 0x0B0F14 else bg, fg, k)
                    mask != null && mask and (1 shl (x + 2 * y)) != 0 -> if (fg < 0) bg else fg
                    else -> bg
                }
            }
        }
        return sub
    }

    private fun mix(a: Int, b: Int, k: Double): Int {
        fun channel(shift: Int): Int { val from = (a shr shift) and 255; val to = (b shr shift) and 255; return (from + (to - from) * k).roundToInt() }
        return (channel(16) shl 16) or (channel(8) shl 8) or channel(0)
    }
}
