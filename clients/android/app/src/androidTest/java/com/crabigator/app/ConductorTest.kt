package com.crabigator.app

import android.app.Application
import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File

class ConductorTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    @Test fun showsParallelTracksAndRetriesTheSameMessageWithoutLosingTheDraft() {
        compose.activityRule.scenario.onActivity {
            it.setShowWhenLocked(true)
            it.setTurnScreenOn(true)
            it.window.addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val model = SessionModel(context.applicationContext as Application)
        val fixture = JSONObject("""{"host_online": true, "host_name": "Test desktop", "messages": [{"id": "question", "role": "user", "text": "Where are the mobile releases?", "at": 1792288800000}, {"id": "answer", "role": "assistant", "text": "Both apps are waiting on **Apple review**. Android is live. These are fixture results.", "at": 1792288860000, "highlights": [{"session_id": "news-session", "project": "NewsBlur", "session_title": "Check app store review", "platform": "Codex", "session_mark": {"glyph": "¬‿¬", "fg_hex": "#AFFFD7", "bg_hex": "#243B35"}, "recap": "Checked both store fixtures", "additions": 12, "deletions": 3, "prompts": 4, "session_state": "complete", "text": "iOS **2.0 · In review**\nAndroid **2.1 · Published**\nReview duration: unknown.", "state": "answered", "updated_at": 1792288840000}, {"session_id": "touch-session", "project": "Turn Touch", "session_title": "Prepare the mobile release", "platform": "Claude", "session_mark": {"glyph": "⌬⌬", "fg_hex": "#FFD7AF", "bg_hex": "#443524"}, "text": "iOS **2.0 · In review**\nAndroid **2.1 · Published**\nReview duration: unknown.", "state": "answered", "updated_at": 1792288840000}]}], "tracks": [{"session_id": "news-session", "project": "NewsBlur", "session_title": "Check app store review", "platform": "Codex", "session_mark": {"glyph": "¬‿¬", "fg_hex": "#AFFFD7", "bg_hex": "#243B35"}, "recap": "Checked both store fixtures", "additions": 12, "deletions": 3, "prompts": 4, "session_state": "complete", "id": "one", "request_id": "question", "title": "App store versions", "state": "answered", "connected": true, "question": "Where are the mobile releases?", "events": [], "result": "Fixture: iOS 2.0 in review. Android 2.1 is live."}, {"session_id": "news-session", "project": "NewsBlur", "session_title": "Check app store review", "platform": "Codex", "session_mark": {"glyph": "¬‿¬", "fg_hex": "#AFFFD7", "bg_hex": "#243B35"}, "recap": "Checked both store fixtures", "additions": 12, "deletions": 3, "prompts": 4, "session_state": "complete", "id": "retry", "request_id": "old", "title": "Earlier store check", "state": "failed", "connected": true, "events": [], "result": "Desktop was unavailable."}, {"session_id": "touch-session", "project": "Turn Touch", "session_title": "Prepare the mobile release", "platform": "Claude", "session_mark": {"glyph": "⌬⌬", "fg_hex": "#FFD7AF", "bg_hex": "#443524"}, "id": "two", "request_id": "question", "title": "App store versions", "state": "answered", "connected": false, "events": [], "result": "Fixture: Android 2.1 is live."}]}""")
        val sent = mutableListOf<JSONObject>()
        compose.setContent { MaterialTheme(colorScheme = CrabTheme) { Surface(Modifier.fillMaxSize(), color = Ink) { ConductorView(model, Modifier.fillMaxSize()) { path, body ->
            if (path.endsWith("/archive")) {
                val query = body?.text("query").orEmpty()
                val entries = fixture.array("tracks").filter { query.isBlank() || it.text("result").contains(query, ignoreCase = true) }
                JSONObject().put("entries", org.json.JSONArray(entries)).put("total", entries.size)
            } else if (body != null) {
                sent.add(JSONObject(body.toString()))
                if (sent.size == 1) throw Exception("Connection lost after send")
                JSONObject()
            } else fixture
        } } } }
        compose.waitUntil(10000) { runCatching { compose.onNodeWithText("Check app store review").assertIsDisplayed(); true }.getOrDefault(false) }
        compose.onNodeWithText("Check app store review").assertIsDisplayed()
        val dir = File(context.getExternalFilesDir(null), "conductor-evidence").apply { mkdirs() }
        File(dir, "android-conductor.png").outputStream().use { compose.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
        compose.onNodeWithText("Sessions", useUnmergedTree = true).performClick()
        compose.onNodeWithText("0 active · 1 answered · 1 failed").assertIsDisplayed()
        compose.onNodeWithText("Archive", useUnmergedTree = true).performClick()
        compose.onNodeWithContentDescription("Search Conductor archive").performTextInput("unavailable")
        compose.waitUntil(5000) { compose.onAllNodesWithText("Prepare the mobile release").fetchSemanticsNodes().isEmpty() }
        compose.onNodeWithText("Prepare the mobile release").assertDoesNotExist()
        compose.onNodeWithText("Conversation", useUnmergedTree = true).performClick()
        compose.onNode(hasSetTextAction()).performTextInput("Check the fixtures again")
        compose.onNodeWithText("Send", useUnmergedTree = true).performClick()
        compose.waitUntil(5000) { sent.size == 1 }
        compose.onNodeWithText("Connection lost after send").assertIsDisplayed()
        compose.onNode(hasSetTextAction()).assertTextContains("Check the fixtures again")
        compose.onNodeWithText("Send", useUnmergedTree = true).performClick()
        compose.waitUntil(5000) { sent.size == 2 }
        assertEquals(sent[0].getString("id"), sent[1].getString("id"))
        assertEquals("", compose.onNode(hasSetTextAction()).fetchSemanticsNode().config[SemanticsProperties.EditableText].text)
        compose.onNodeWithText("List", useUnmergedTree = true).performClick()
        compose.runOnIdle { assertFalse(model.navigation.value.conductor); assertTrue(model.navigation.value.listMode) }
    }
}
