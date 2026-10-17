package com.crabigator.app

import android.app.Application
import android.graphics.Bitmap
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File

class SessionModulesLayoutTest {
    @get:Rule val compose = createComposeRule()

    @Test fun modulesExpandIndependentlyAndKeepTheComposerVisible() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val model = SessionModel(context.applicationContext as Application)
        model.preferences.value = UiPreferences()
        val session = Session.parse(JSONObject("""{"id":"module-layout","title":"Internationalize Turn Touch","device_name":"Studio","state":"complete","branch":"codex/localization"}"""))
        val prs = listOf(
            PullRequest("ios#15", "Add 21-language localization and iOS language selection", "samuelclay/turntouch-ios", 15, "OPEN", "✓ CI", "https://github.com/samuelclay/turntouch-ios/pull/15", listOf(session), branch = "codex/localization-21-languages", additions = 41683, deletions = 738, primary = true),
            PullRequest("android#1", "Add 21-language localization and Android language selection", "samuelclay/turntouch-android", 1, "MERGED", "", "https://github.com/samuelclay/turntouch-android/pull/1", listOf(session), branch = "codex/localization-21-languages", additions = 20920, deletions = 299, primary = true))
        val state = AppState(selected = session, connected = true, screenLoaded = true, historyLoaded = true, prs = prs,
            screen = "Added language selection on all three platforms.\n\n  ✓ iOS build passed\n  ✓ Android tests passed\n  ✓ Mac build passed\n\nThe pull requests are ready for review.\n\n› Ask Codex to do anything",
            details = mapOf(
                "stats" to JSONObject("""{"work_seconds":17160,"thinking_seconds":9120,"prompts":6,"completions":4}"""),
                "recap" to JSONObject("""{"latest":{"headline":"Added localization and language selection","bullets":["Translated the app into 21 languages.","Added native language selection on each platform."]}}"""),
                "git" to JSONObject("""{"branch":"codex/localization","files":[{"path":"Settings/LanguagePicker.kt","status":"M","additions":42,"deletions":8}]}"""),
                "changes" to JSONObject("""{"by_language":[{"changes":[{"name":"LanguagePicker","kind":"class","additions":42,"deletions":8}]}]}""")))
        compose.setContent { MaterialTheme(colorScheme = darkColorScheme(background = Ink, surface = Ink)) { Surface(Modifier.fillMaxSize()) { SessionDetail(state, model, false, {}, active = false, style = {}) } } }
        compose.onNodeWithText("Translated the app into 21 languages.").assertDoesNotExist()
        capture("modules-collapsed")
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
