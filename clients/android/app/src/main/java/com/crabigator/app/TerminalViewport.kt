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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.BorderStroke
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
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
import kotlinx.coroutines.launch
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private fun promptExcerpt(line: CharSequence): String? {
    if (line.getOrNull(1)?.isWhitespace() != true) return null
    return line.drop(1).trim().take(80).toString().ifEmpty { null }
}

private data class PromptTarget(val block: Int, val text: String)

/** Keep ANSI styles across block boundaries, without laying out the whole transcript. */
internal fun terminalBlocks(text: AnnotatedString, linesPerBlock: Int = 24): List<AnnotatedString> {
    require(linesPerBlock > 0)
    if (text.isEmpty()) return emptyList()
    val blocks = mutableListOf<AnnotatedString>()
    val markers = text.getStringAnnotations("prompt", 0, text.length).mapTo(mutableSetOf()) { it.start }
    var start = 0
    var lineStart = 0
    var lines = 0
    fun finishLine(end: Int) {
        val line = text.text.subSequence(lineStart, end)
        if (start < lineStart && lineStart in markers && promptExcerpt(line) != null) {
            blocks += text.subSequence(start, lineStart - 1)
            start = lineStart
            lines = 0
        }
        if (TerminalText.isRule(line)) {
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

private data class TerminalDocument(val blocks: List<AnnotatedString> = emptyList(), val widthEm: Float = 0f, val columns: Int = 0, val prompts: List<PromptTarget> = emptyList())

@Composable private fun terminalDocument(source: String, wrap: Boolean, cellWidthEm: Float, indentCellWidthEm: Float): State<TerminalDocument> =
    produceState(TerminalDocument(), source, wrap, cellWidthEm, indentCellWidthEm) {
        value = withContext(Dispatchers.Default) {
            val parsed = TerminalText.parse(source.removeSuffix("\n"), cellWidthEm).let { if (wrap) TerminalText.trimLineEnds(TerminalText.reflowParagraphs(it, indentCellWidthEm)) else it }
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
            val prompts = blocks.mapIndexedNotNull { index, block ->
                if (!block.hasStringAnnotations("prompt", 0, 1)) return@mapIndexedNotNull null
                promptExcerpt(block.text.substringBefore('\n'))?.let { PromptTarget(index, it) }
            }
            TerminalDocument(blocks, widthEm, columns, prompts)
        }
    }

@Composable internal fun TerminalViewport(history: String, screen: String, prefs: UiPreferences, pinned: Boolean, setPinned: (Boolean) -> Unit) {
    val cellWidthEm = remember {
        Paint().apply { typeface = Typeface.MONOSPACE; textSize = 1000f }.measureText(" ") / 1000f
    }
    val letterSpacing = LocalTextStyle.current.letterSpacing
    val indentCellWidthEm = cellWidthEm + when {
        letterSpacing.isSp -> letterSpacing.value / prefs.fontSize
        letterSpacing.isEm -> letterSpacing.value
        else -> 0f
    }
    val historyDocument by terminalDocument(history, prefs.wrap, cellWidthEm, indentCellWidthEm)
    val screenDocument by terminalDocument(screen, prefs.wrap, cellWidthEm, indentCellWidthEm)
    // Capture one immutable list per composition so lazy item counts, keys,
    // and content cannot read different background parse results.
    val historyBlocks = historyDocument.blocks
    val screenBlocks = screenDocument.blocks
    val prompts = remember(historyDocument, screenDocument) {
        historyDocument.prompts + screenDocument.prompts.map { it.copy(block = historyBlocks.size + it.block) }
    }
    var selected by remember { mutableStateOf<PromptTarget?>(null) }
    val scope = rememberCoroutineScope()
    val scroll = rememberLazyListState()
    val horizontal = rememberScrollState()
    val currentSetPinned by rememberUpdatedState(setPinned)
    val userScroll = remember(scroll) { object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
            if (source == NestedScrollSource.UserInput && available.y != 0f) {
                selected = null
                if (available.y > 0) currentSetPinned(false)
            }
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
        if (pinned) selected = null
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
        val current = if (pinned) -1 else prompts.indexOf(selected)
        val after = when {
            pinned -> prompts.size
            current >= 0 -> current + 1
            else -> prompts.indexOfFirst {
                it.block > scroll.firstVisibleItemIndex ||
                    (it.block == scroll.firstVisibleItemIndex && scroll.firstVisibleItemScrollOffset == 0)
            }.let { if (it < 0) prompts.size else it }
        }
        val before = if (current >= 0) current - 1 else after - 1
        fun jump(target: PromptTarget) {
            selected = target
            setPinned(false)
            scope.launch {
                horizontal.scrollTo(0)
                scroll.scrollToItem(target.block)
            }
        }
        prompts.getOrNull(before)?.let { target ->
            PromptPill("Previous prompt", before, prompts.size, target.text, Modifier.align(Alignment.TopEnd)) { jump(target) }
        }
        prompts.getOrNull(after)?.let { target ->
            PromptPill("Next prompt", after, prompts.size, target.text, Modifier.align(Alignment.BottomEnd)) { jump(target) }
        }
    }
}

@Composable private fun PromptPill(label: String, index: Int, total: Int, text: String, modifier: Modifier, onClick: () -> Unit) {
    Surface(onClick = onClick, modifier = modifier.padding(6.dp).widthIn(max = 210.dp),
        shape = RoundedCornerShape(14.dp), color = Color(0xFF19232D), border = BorderStroke(1.dp, Color(0xFF374654))) {
        Column(Modifier.padding(horizontal = 12.dp, vertical = 5.dp)) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(label, color = Color.White, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                Text("${index + 1}/$total", color = Color(0xFF79C9DF), fontSize = 12.sp)
            }
            Text(text, maxLines = 1, overflow = TextOverflow.Ellipsis, fontSize = 12.sp, color = Color(0xFFB8C6D1))
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
