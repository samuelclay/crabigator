package com.crabigator.app

import android.app.Application
import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.unit.dp
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File

class SessionModulesLayoutTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    // Adaptive sizing keeps an unplaced measurement row in the unmerged tree.
    private fun metric(description: String) = compose.onNode(
        hasContentDescription(description) and SemanticsMatcher("Placed") { it.layoutInfo.isPlaced },
        useUnmergedTree = true)

    @Test fun crowdedSummaryKeepsAgoOnTheMoreRecentActivity() {
        val now = System.currentTimeMillis() / 1000
        val stats = mutableStateOf(JSONObject("""{"work_seconds":600,"thinking_seconds":9120,"prompts":56,"completions":26,"prompts_changed_at":${now - 180},"completions_changed_at":${now - 120}}"""))
        val session = Session.parse(JSONObject("""{"id":"ages","state":"complete","active":true}"""))
        compose.setContent {
            MaterialTheme { Box(Modifier.width(360.dp)) {
                SessionSummaryBar(session, stats.value, UiPreferences(), false) {}
            } }
        }
        fun age(text: String) = compose.onNode(hasText(text) and SemanticsMatcher("Placed") { it.layoutInfo.isPlaced }, useUnmergedTree = true)
        capture("summary-recent")
        age("3m").assertIsDisplayed()
        age("2m ago").assertIsDisplayed()
        compose.runOnIdle { stats.value = JSONObject(stats.value.toString()).put("prompts_changed_at", now - 60) }
        age("1m ago").assertIsDisplayed()
        age("2m").assertIsDisplayed()
    }

    @Test fun modulesExpandIndependentlyAndKeepTheComposerVisible() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val model = SessionModel(context.applicationContext as Application)
        model.preferences.value = UiPreferences()
        val session = Session.parse(JSONObject("""{"id":"module-layout","title":"Internationalize Turn Touch","device_name":"Studio","state":"complete","branch":"codex/localization","session_mark":{"glyph":"ᛝ","fg_hex":"#5FD7FF","bg_hex":"#17333D"}}"""))
        val now = System.currentTimeMillis() / 1000
        val prs = listOf(
            PullRequest("ios#15", "Add 21-language localization and iOS language selection", "samuelclay/turntouch-ios", 15, "OPEN", "✓ CI", "https://github.com/samuelclay/turntouch-ios/pull/15", listOf(session), branch = "codex/localization-21-languages", additions = 41683, deletions = 738, primary = true, updatedAt = now - 10800),
            PullRequest("android#1", "Add 21-language localization and Android language selection", "samuelclay/turntouch-android", 1, "MERGED", "", "https://github.com/samuelclay/turntouch-android/pull/1", listOf(session), branch = "codex/localization-21-languages", additions = 20920, deletions = 299, primary = true, updatedAt = now - 14400))
        val state = AppState(selected = session, connected = true, screenLoaded = true, historyLoaded = true, prs = prs,
            screen = "Added language selection on all three platforms.\n\n  ✓ iOS build passed\n  ✓ Android tests passed\n  ✓ Mac build passed\n\nThe pull requests are ready for review.\n\n› Ask Codex to do anything",
            details = mapOf(
                "stats" to JSONObject("""{"work_seconds":17160,"thinking_seconds":9120,"prompts":56,"completions":26,"prompts_changed_at":${now - 180},"completions_changed_at":${now - 120}}"""),
                "recap" to JSONObject("""{"latest":{"generated_at":${(now - 420) * 1000},"headline":"Added localization and language selection","bullets":["Translated the app into 21 languages.","Added native language selection on each platform."]}}"""),
                "commit_history" to JSONObject("""{"history":[{"short_hash":"abc1234","subject":"Add language selection","timestamp":${now - 7200}}]}"""),
                "git" to JSONObject("""{"branch":"codex/localization","files":[{"path":"Settings/LanguagePicker.kt","status":"M","additions":42,"deletions":8}]}"""),
                "changes" to JSONObject("""{"by_language":[{"changes":[{"name":"LanguagePicker","kind":"class","additions":42,"deletions":8}]}]}""")))
        compose.activityRule.scenario.onActivity {
            it.enableEdgeToEdge(statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT), navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT))
        }
        compose.setContent { MaterialTheme(colorScheme = darkColorScheme(background = Ink, surface = Ink)) { Surface(Modifier.fillMaxSize()) {
            Box(Modifier.safeDrawingPadding()) { SessionDetail(state, model, false, {}, active = false, style = {}) }
        } } }
        compose.onNodeWithText("Translated the app into 21 languages.").assertDoesNotExist()
        compose.onNodeWithText("7m ago", useUnmergedTree = true).assertExists()
        compose.onNodeWithText("3h ago", useUnmergedTree = true).assertExists()
        compose.onNodeWithText("2h ago", useUnmergedTree = true).assertExists()
        metric("Prompts: 56, 3m ago").assertIsDisplayed()
        metric("Completions: 26, 2m ago").assertExists()
        val glyph = compose.onNodeWithText("ᛝ", useUnmergedTree = true).assertIsDisplayed().fetchSemanticsNode().boundsInRoot
        val sessionTime = metric("Session time: 4h 46m").fetchSemanticsNode().boundsInRoot
        assertTrue("Identity stays left of the metrics", glyph.right < sessionTime.left)
        val prompts = metric("Prompts: 56, 3m ago").fetchSemanticsNode().boundsInRoot
        assertEquals("Metrics share one line", sessionTime.height, prompts.height, 2f)
        capture("modules-collapsed")
        val completion = metric("Completions: 26, 2m ago")
        if (!completion.isDisplayed()) completion.performScrollTo()
        completion.assertIsDisplayed()
        val sessionMetric = metric("Session time: 4h 46m")
        if (!sessionMetric.isDisplayed()) sessionMetric.performScrollTo()
        compose.onNodeWithContentDescription("Recap").performClick()
        compose.onNodeWithText("Translated the app into 21 languages.").assertExists()
        compose.onNodeWithText(prs.first().title).assertDoesNotExist()
        compose.onNodeWithContentDescription("Recap").performClick()
        compose.onNodeWithContentDescription("Pull requests").performClick()
        compose.onNodeWithText(prs.first().title, useUnmergedTree = true).assertExists()
        compose.onNodeWithText("Translated the app into 21 languages.").assertDoesNotExist()
        capture("modules-prs-expanded")
        compose.onNodeWithContentDescription("Pull requests").performClick()
        compose.onNodeWithContentDescription("Git").performClick()
        compose.onNodeWithText("Settings/LanguagePicker.kt").assertExists()
        capture("modules-git-expanded")
        val summary = compose.onNodeWithContentDescription("Session summary")
        val barBefore = summary.fetchSemanticsNode().boundsInRoot
        compose.onNodeWithContentDescription("Hide session modules").assertDoesNotExist()
        summary.performTouchInput { click() }
        compose.onNodeWithContentDescription("Recap").assertDoesNotExist()
        summary.assertIsDisplayed().assert(SemanticsMatcher.expectValue(androidx.compose.ui.semantics.SemanticsProperties.StateDescription, "Collapsed"))
        assertEquals(barBefore.bottom, summary.fetchSemanticsNode().boundsInRoot.bottom, 2f)
        metric("Thinking time: 2h 32m").assertIsDisplayed()
        capture("modules-summary-only")
        summary.performClick()
        compose.onNodeWithText("Settings/LanguagePicker.kt").assertExists()
        assertEquals(barBefore.bottom, summary.fetchSemanticsNode().boundsInRoot.bottom, 2f)
        // Reverse an in-flight animation rather than waiting for it to finish.
        compose.mainClock.autoAdvance = false
        summary.performClick()
        compose.mainClock.advanceTimeBy(80)
        summary.performClick()
        compose.mainClock.autoAdvance = true
        compose.onNodeWithText("Settings/LanguagePicker.kt").assertExists()
        capture("modules-restored")
        // A fixed terminal taller than the available space must not push the bar away.
        compose.runOnIdle { model.preferences.value = model.preferences.value.copy(terminalHeight = 1000) }
        summary.assertIsDisplayed()
        compose.mainClock.autoAdvance = false
        summary.performClick()
        compose.mainClock.advanceTimeBy(80)
        assertEquals(barBefore.bottom, summary.fetchSemanticsNode().boundsInRoot.bottom, 2f)
        compose.mainClock.autoAdvance = true
        summary.assertIsDisplayed()
        assertEquals(barBefore.bottom, summary.fetchSemanticsNode().boundsInRoot.bottom, 2f)
    }
    private fun capture(name: String) {
        compose.waitForIdle()
        // Native ripple drawing uses the device clock, not Compose's test clock.
        android.os.SystemClock.sleep(600)
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val output = File(context.getExternalFilesDir(null), "$name.png")
        val bitmap = compose.onRoot().captureToImage().asAndroidBitmap()
        output.outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }
}
