package com.crabigator.app

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test

class PrBoardInteractionTest {
    @get:Rule val compose = createComposeRule()
    private val session = Session.parse(JSONObject("""{"id":"session", "title":"A session title that wraps across several lines", "branch":"session-branch", "state":"complete"}"""))
    private val pr = PullRequest("owner/repo#123", "A pull request with a longer title", "owner/repo", 123, "OPEN", "Checks passed",
        "https://github.com/owner/repo/pull/123", listOf(session), additions = 531763, deletions = 6931, branch = "pr-branch", primary = true, mergeable = "MERGEABLE", mergeState = "BEHIND", unresolvedComments = 7, review = "APPROVED")

    @Test fun prContentOpensSessionAndSharesTheTitleColumn() {
        val wide = mutableStateOf(false)
        val clock = mutableStateOf(1000L)
        var opened = 0
        compose.setContent {
            MaterialTheme {
                Box(Modifier.width(if (wide.value) 1000.dp else 320.dp)) {
                    BoardSessionBlock(BoardSession(session, listOf(pr, pr.copy(number = 124, title = "Already merged", state = "MERGED", checks = "", unresolvedComments = 0, additions = null, deletions = null)), 0), false, wide.value, clock) { opened++ }
                }
            }
        }
        fun bounds(text: String) = compose.onNodeWithText(text, useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
        for (isWide in listOf(false, true)) {
            compose.runOnIdle { wide.value = isWide }
            compose.waitForIdle()
            val title = bounds(session.title)
            val pull = bounds("#123  ${pr.title}")
            assertEquals("PR titles should share the session title indent", title.left, pull.left, 1f)
            val ci = compose.onNodeWithContentDescription("Checks passed", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            val emptyCi = compose.onNodeWithContentDescription("CI status unavailable", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            val behind = compose.onNodeWithContentDescription("Branch status: Behind", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            val emptyBranch = compose.onNodeWithContentDescription("Branch status: —", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            assertEquals("CI column must keep its place when empty", ci.left, emptyCi.left, 1f)
            assertEquals("Branch column must keep its place when empty", behind.left, emptyBranch.left, 1f)
            val comments = compose.onAllNodesWithContentDescription("Unresolved comments: 7", useUnmergedTree = true)[0].fetchSemanticsNode().boundsInRoot
            val emptyComments = compose.onNodeWithContentDescription("Unresolved comments: 0", useUnmergedTree = true)
            assertEquals(comments.left, emptyComments.fetchSemanticsNode().boundsInRoot.left, 1f)
            emptyComments.assertTextEquals("")
            val approval = compose.onNodeWithContentDescription("Approval: Approved", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            val noApproval = compose.onNodeWithContentDescription("Approval: Not applicable", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            assertEquals("Approval column must keep its place", approval.left, noApproval.left, 1f)
            assertTrue("All four status columns must stay separate", ci.right < comments.left && comments.right < approval.left && approval.right < behind.left)
            if (isWide) {
                assertEquals("Session columns should be top-aligned", title.top, bounds(session.branch).top, 1f)
                assertEquals("PR columns should be top-aligned", pull.top,
                    compose.onAllNodesWithText(pr.branch, useUnmergedTree = true)[0].fetchSemanticsNode().boundsInRoot.top, 1f)
                assertEquals("Absent checks must not shift the state column", bounds("Open").left, bounds("Merged").left, 1f)
            } else {
                val status = compose.onNodeWithContentDescription("Complete", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
                val prompt = compose.onNodeWithContentDescription("Last prompt: unknown", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
                assertTrue("Session status must sit immediately before prompt time", status.right < prompt.left && prompt.left - status.right <= 24f)
                assertEquals("Session status must center on the timing line", prompt.center.y, status.center.y, 2f)
            }
            compose.onRoot().performTouchInput { click(pull.center) }
            compose.onRoot().performTouchInput { click(bounds("+531763").center) }
        }
        compose.runOnIdle { assertEquals(4, opened) }
    }

    @Test fun statusColumnsKeepWordsReadableAtAllBoardTextSizes() {
        val clock = mutableStateOf(1000L)
        val fontScale = mutableStateOf(24f / 14f)
        compose.setContent {
            CompositionLocalProvider(LocalDensity provides Density(LocalDensity.current.density, fontScale.value)) {
                MaterialTheme {
                    Box(Modifier.width(320.dp)) {
                        BoardSessionBlock(BoardSession(session, listOf(pr.copy(checks = "✗999 CI", checksFailed = 999, mergeable = "CONFLICTING")), 0),
                            false, false, clock) {}
                    }
                }
            }
        }
        for (scale in listOf(24f / 14f, 10f / 14f)) {
            compose.runOnIdle { fontScale.value = scale }
            val additions = compose.onNodeWithText("+531763", useUnmergedTree = true)
            val deletions = compose.onNodeWithText("−6931", useUnmergedTree = true)
            for (node in listOf(additions, deletions)) {
                node.assertIsDisplayed()
                val results = mutableListOf<TextLayoutResult>()
                node.performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(results) }
                assertTrue("Diff counts must fit without clipping", results.isNotEmpty() && results.none { it.hasVisualOverflow })
            }
            val diffBounds = additions.fetchSemanticsNode().boundsInRoot
            val comments = compose.onNodeWithContentDescription("Unresolved comments: 7", useUnmergedTree = true).fetchSemanticsNode().boundsInRoot
            assertTrue("Narrow PR rows must put review columns below the diff counts", comments.top >= diffBounds.bottom)
            for (label in listOf("999 checks failed", "Branch status: Conflicts", "Unresolved comments: 7", "Approval: Approved")) {
                val results = mutableListOf<TextLayoutResult>()
                compose.onNodeWithContentDescription(label, useUnmergedTree = true)
                    .performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(results) }
                assertTrue("Status text must fit without clipping", results.isNotEmpty() && results.none { it.hasVisualOverflow })
            }
        }
    }

    @Test fun detailHeaderOpensEachExplicitPrLink() {
        val urls = mutableListOf<String>()
        val second = pr.copy(key = "owner/repo#124", number = 124, url = "https://github.com/owner/repo/pull/124")
        val prs = sessionPullRequests(AppState(selected = session, prs = listOf(pr, second, pr)))
        assertEquals(2, prs.size)
        compose.setContent {
            CompositionLocalProvider(LocalUriHandler provides object : UriHandler {
                override fun openUri(uri: String) { urls.add(uri) }
            }) { MaterialTheme { SessionPrLinks(prs, Modifier.widthIn(max = 96.dp)) } }
        }
        compose.onNodeWithText("#123").performClick()
        compose.onNodeWithText("#124").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf(pr.url, second.url), urls) }
        val updated = AppState(selected = session, prs = prs, details = mapOf("prs" to JSONObject("""{"prs":[
            {"number":125,"owner":"owner","repo":"repo","url":"https://github.com/owner/repo/pull/125"},
            {"number":124,"url":"https://github.com/owner/repo/pull/124","dismissed":true}
        ]}""")))
        assertEquals(listOf(125), sessionPullRequests(updated).map { it.number })
        assertTrue(sessionPullRequests(updated.copy(details = mapOf("prs" to JSONObject("{\"prs\":[]}")))).isEmpty())
    }
}
