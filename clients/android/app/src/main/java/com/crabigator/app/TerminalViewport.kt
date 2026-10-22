package com.crabigator.app

import android.graphics.Paint
import android.graphics.Typeface
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.*
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.style.LineHeightStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Keep ANSI styles across block boundaries, without laying out the whole transcript. */
internal fun terminalBlocks(text: AnnotatedString, linesPerBlock: Int = 24): List<AnnotatedString> {
    require(linesPerBlock > 0)
    if (text.isEmpty()) return emptyList()
    val blocks = mutableListOf<AnnotatedString>()
    var start = 0
    var lineStart = 0
    var lines = 0
    fun finishLine(end: Int) {
        if (TerminalText.isRule(text.text.subSequence(lineStart, end))) {
            // A rule gets its own Text so it can clip while nearby text wraps.
            if (start < lineStart) blocks += text.subSequence(start, lineStart - 1)
            blocks += text.subSequence(lineStart, end)
            start = end + 1
            lines = 0
        } else if (++lines == linesPerBlock) {
            blocks += text.subSequence(start, end)
            start = end + 1
            lines = 0
        }
        lineStart = end + 1
    }
    text.text.forEachIndexed { index, char -> if (char == '\n') finishLine(index) }
    finishLine(text.length)
    if (start <= text.length) blocks += text.subSequence(start, text.length)
    return blocks
}

private data class TerminalDocument(val blocks: List<AnnotatedString> = emptyList(), val widthEm: Float = 0f, val columns: Int = 0)

@Composable private fun terminalDocument(source: String, wrap: Boolean, cellWidthEm: Float): State<TerminalDocument> =
    produceState(TerminalDocument(), source, wrap, cellWidthEm) {
        value = withContext(Dispatchers.Default) {
            val parsed = TerminalText.parse(source.removeSuffix("\n"), cellWidthEm).let { if (wrap) TerminalText.trimLineEnds(it) else it }
            var widthEm = 0f
            var columns = 0
            if (!wrap) {
                val paint = Paint().apply { typeface = Typeface.MONOSPACE; textSize = 1000f }
                for (line in parsed.text.lineSequence()) {
                    widthEm = maxOf(widthEm, paint.measureText(line) / 1000f)
                    columns = maxOf(columns, line.length)
                }
            }
            // A single blank terminal row is still one row after its delimiter is removed.
            val blocks = if (parsed.isEmpty() && source.isNotEmpty()) listOf(parsed) else terminalBlocks(parsed)
            TerminalDocument(blocks, widthEm, columns)
        }
    }

@Composable internal fun TerminalViewport(history: String, screen: String, prefs: UiPreferences, pinned: Boolean, setPinned: (Boolean) -> Unit) {
    val cellWidthEm = remember {
        Paint().apply { typeface = Typeface.MONOSPACE; textSize = 1000f }.measureText(" ") / 1000f
    }
    val historyDocument by terminalDocument(history, prefs.wrap, cellWidthEm)
    val screenDocument by terminalDocument(screen, prefs.wrap, cellWidthEm)
    // Capture one immutable list per composition so lazy item counts, keys,
    // and content cannot read different background parse results.
    val historyBlocks = historyDocument.blocks
    val screenBlocks = screenDocument.blocks
    val scroll = rememberLazyListState()
    val horizontal = rememberScrollState()
    val currentSetPinned by rememberUpdatedState(setPinned)
    val userScroll = remember(scroll) { object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
            if (source == NestedScrollSource.UserInput && available.y > 0) currentSetPinned(false)
            return Offset.Zero
        }
        override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
            if (consumed.y < 0 && !scroll.canScrollForward) currentSetPinned(true)
            return Offset.Zero
        }
    } }
    var wasPinned by remember { mutableStateOf(pinned) }
    var followingAnimation by remember { mutableStateOf(false) }
    val currentCount by rememberUpdatedState(historyBlocks.size + screenBlocks.size)
    val currentPinned by rememberUpdatedState(pinned)
    // Only an explicit return to following animates. New output must not cancel
    // that animation or animate every arriving frame.
    LaunchedEffect(pinned) {
        val animate = pinned && !wasPinned
        wasPinned = pinned
        if (animate && currentCount > 0) {
            followingAnimation = true
            try {
                scroll.animateScrollToItem(currentCount - 1)
                val layout = scroll.layoutInfo
                val last = layout.visibleItemsInfo.lastOrNull()
                val remaining = last?.let { it.offset + it.size + layout.afterContentPadding - layout.viewportEndOffset } ?: 0
                if (remaining > 0) scroll.animateScrollBy(remaining.toFloat())
            } finally { followingAnimation = false }
        }
    }
    // Normal order keeps the row being read anchored when the live frame grows.
    LaunchedEffect(historyDocument, screenDocument, followingAnimation) {
        if (currentPinned && !followingAnimation && currentCount > 0) scroll.scrollToItem(currentCount - 1, Int.MAX_VALUE)
    }
    BoxWithConstraints(Modifier.fillMaxSize().graphicsLayer()) {
        val density = LocalDensity.current
        val letterSpacing = LocalTextStyle.current.letterSpacing
        val contentWidth = with(density) {
            val spacingPx = if (letterSpacing.isSp) letterSpacing.toPx() else 0f
            val widthPx = maxOf(historyDocument.widthEm, screenDocument.widthEm) * prefs.fontSize.sp.toPx() +
                maxOf(historyDocument.columns, screenDocument.columns) * spacingPx
            maxOf(maxWidth, widthPx.toDp() + 32.dp)
        }
        SelectionContainer {
            LazyColumn(state = scroll,
                modifier = Modifier.fillMaxHeight().nestedScroll(userScroll)
                    .then(if (prefs.wrap) Modifier.fillMaxWidth() else Modifier.horizontalScroll(horizontal).width(contentWidth)),
                contentPadding = PaddingValues(16.dp)) {
                items(historyBlocks.size, key = { "history-$it" }, contentType = { "terminal" }) { index ->
                    TerminalBlock(historyBlocks[index], prefs)
                }
                items(screenBlocks.size, key = { "screen-$it" }, contentType = { "terminal" }) { index ->
                    TerminalBlock(screenBlocks[index], prefs)
                }
            }
        }
    }
}

@Composable private fun TerminalBlock(text: AnnotatedString, prefs: UiPreferences) {
    val rule = TerminalText.isRule(text.text)
    Text(text, fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace, fontSize = prefs.fontSize.sp,
        lineHeight = (prefs.fontSize * prefs.lineSpacing / 100f).sp, softWrap = prefs.wrap && !rule,
        maxLines = if (rule) 1 else Int.MAX_VALUE,
        style = LocalTextStyle.current.copy(lineHeightStyle = LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.None)))
}
