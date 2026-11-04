package com.crabigator.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.activity.ComponentActivity
import androidx.activity.enableEdgeToEdge
import androidx.activity.SystemBarStyle
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import android.view.KeyEvent
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import androidx.test.platform.app.InstrumentationRegistry
import android.graphics.Bitmap
import java.io.File

class DemoTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test fun sampleChoiceAndMessageStayInDemo() {
        var closed = false
        if (android.os.Build.VERSION.SDK_INT >= 27) compose.activity.setTurnScreenOn(true)
        compose.activity.runOnUiThread {
            compose.activity.enableEdgeToEdge(statusBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT), navigationBarStyle = SystemBarStyle.dark(android.graphics.Color.TRANSPARENT))
            compose.activity.window.setSoftInputMode(android.view.WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
            compose.activity.window.addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
        compose.setContent { MaterialTheme(colorScheme = CrabTheme) { Surface { DemoView { closed = true } } } }
        capture("demo-sessions")
        compose.onNodeWithText("PR board").performClick()
        capture("demo-pr-board")
        compose.onNodeWithText("PR #23 · Add dark mode").performClick()
        capture("demo-question")
        compose.onNodeWithText("Try a sample message").performClick()
        compose.waitUntil(5000) { compose.onAllNodesWithText("Dark", useUnmergedTree = true).fetchSemanticsNodes().isEmpty() }
        InstrumentationRegistry.getInstrumentation().sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        compose.waitUntil(5000) { compose.onAllNodesWithText("Dark", useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Dark", useUnmergedTree = true).performClick()
        compose.onNodeWithText("Dark", useUnmergedTree = true).assertDoesNotExist()
        compose.onNodeWithText("Try a sample message").performTextInput("Show the tests")
        compose.onNodeWithText("Send").performClick()
        compose.waitUntil(5000) {
            compose.onAllNodesWithText("your message stayed on this device", substring = true).fetchSemanticsNodes().isNotEmpty()
        }
        capture("demo-reply")
        compose.onNodeWithText("Back").performClick()
        compose.onNodeWithText("PR #24 · Improve search").assertExists()
        compose.onNodeWithText("Done").performClick()
        assertTrue(closed)
    }

    private fun capture(name: String) {
        compose.waitForIdle()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val screenshot = instrumentation.uiAutomation.takeScreenshot()
        val screenshots = File(instrumentation.targetContext.filesDir, "demo-screenshots").apply { mkdirs() }
        File(screenshots, "$name.png").outputStream().use {
            screenshot.compress(Bitmap.CompressFormat.PNG, 100, it)
        }
        screenshot.recycle()
    }
}
