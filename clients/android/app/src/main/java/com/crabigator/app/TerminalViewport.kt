package com.crabigator.app

import android.graphics.Paint
import android.graphics.Typeface
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
    var lines = 0
    text.text.forEachIndexed { index, char ->
        if (char == '\n' && ++lines == linesPerBlock) {
            blocks += text.subSequence(start, index)
            start = index + 1
            lines = 0
        }
    }
    blocks += text.subSequence(start, text.length)
    return blocks
}

private data class TerminalDocument(val blocks: List<AnnotatedString> = emptyList(), val widthEm: Float = 0f, val columns: Int = 0)

@Composable private fun terminalDocument(source: String, wrap: Boolean, cellWidthEm: Float): State<TerminalDocument> =
    produceState(TerminalDocument(), source, wrap, cellWidthEm) {
        value = withContext(Dispatchers.Default) {
            val parsed = TerminalText.parse(source, cellWidthEm).let { if (wrap) TerminalText.trimLineEnds(it) else it }
            var widthEm = 0f
            var columns = 0
            if (!wrap) {
                val paint = Paint().apply { typeface = Typeface.MONOSPACE; textSize = 1000f }
                for (line in parsed.text.lineSequence()) {
                    widthEm = maxOf(widthEm, paint.measureText(line) / 1000f)
                    columns = maxOf(columns, line.length)
                }
            }
            TerminalDocument(terminalBlocks(parsed), widthEm, columns)
        }
    }

@Composable internal fun TerminalViewport(history: String, screen: String, prefs: UiPreferences, pinned: Boolean, setPinned: (Boolean) -> Unit) {
    val cellWidthEm = remember {
        Paint().apply { typeface = Typeface.MONOSPACE; textSize = 1000f }.measureText(" ") / 1000f
    }
    val historyDocument by terminalDocument(history, prefs.wrap, cellWidthEm)
    val screenDocument by terminalDocument(screen, prefs.wrap, cellWidthEm)
    val scroll = rememberLazyListState()
    val horizontal = rememberScrollState()
    val currentSetPinned by rememberUpdatedState(setPinned)
    val userScroll = remember(scroll) { object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
            if (source == NestedScrollSource.UserInput && available.y > 0) currentSetPinned(false)
            return Offset.Zero
        }
        override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
            if (consumed.y < 0 && !scroll.canScrollBackward) currentSetPinned(true)
            return Offset.Zero
        }
    } }
    // Reverse layout anchors the live end, even when a new frame changes height.
    LaunchedEffect(pinned, historyDocument, screenDocument) { if (pinned) scroll.scrollToItem(0) }
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
            LazyColumn(reverseLayout = true, state = scroll,
                modifier = Modifier.fillMaxHeight().nestedScroll(userScroll)
                    .then(if (prefs.wrap) Modifier.fillMaxWidth() else Modifier.horizontalScroll(horizontal).width(contentWidth)),
                contentPadding = PaddingValues(16.dp)) {
                items(screenDocument.blocks.size, key = { "screen-${screenDocument.blocks.lastIndex - it}" }, contentType = { "terminal" }) { index ->
                    TerminalBlock(screenDocument.blocks[screenDocument.blocks.lastIndex - index], prefs)
                }
                if (historyDocument.blocks.isNotEmpty() && screenDocument.blocks.isNotEmpty()) item(key = "separator", contentType = "separator") { Spacer(Modifier.height(8.dp)) }
                items(historyDocument.blocks.size, key = { "history-${historyDocument.blocks.lastIndex - it}" }, contentType = { "terminal" }) { index ->
                    TerminalBlock(historyDocument.blocks[historyDocument.blocks.lastIndex - index], prefs)
                }
            }
        }
    }
}

@Composable private fun TerminalBlock(text: AnnotatedString, prefs: UiPreferences) {
    Text(text, fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace, fontSize = prefs.fontSize.sp,
        lineHeight = (prefs.fontSize * prefs.lineSpacing / 100f).sp, softWrap = prefs.wrap,
        style = LocalTextStyle.current.copy(lineHeightStyle = LineHeightStyle(LineHeightStyle.Alignment.Center, LineHeightStyle.Trim.None)))
}
