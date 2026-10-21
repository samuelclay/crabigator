package com.crabigator.app

import android.app.Application
import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import java.io.File
import kotlinx.coroutines.CompletableDeferred

class NewSessionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    @Test fun headerOpensPickerFromListBoardAndConductor() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val model = SessionModel(context.applicationContext as Application)
        val saved = model.navigation.value
        val session = Session.parse(JSONObject().put("session_id", "header-project").put("device_name", "Studio").put("cwd", "/projects/NewsBlur").put("repo_name", "NewsBlur").put("active", true))
        model.state.value = AppState(paired = true, sessions = listOf(session), rosterStatus = RosterStatus.Ready)
        model.navigate { it.copy(conductor = false, listMode = true, expanded = true, selectedId = null) }
        try {
            compose.setContent { MaterialTheme(colorScheme = CrabTheme) { Surface(Modifier.fillMaxSize()) { App(model) {} } } }
            listOf("List", "PR board", "Conductor").forEach { mode ->
                compose.onNodeWithText(mode).performClick()
                compose.onNodeWithContentDescription("New session").assertIsDisplayed().performClick()
                compose.onNodeWithText("/projects/NewsBlur").assertIsDisplayed()
                listOf("Codex", "Claude", "OpenCode", "Grok", "Start session").forEach { compose.onNodeWithText(it).assertIsDisplayed() }
                val dir = File(context.getExternalFilesDir(null), "conductor-evidence").apply { mkdirs() }
                File(dir, "android-new-session-${mode.replace(' ', '-')}.png").outputStream().use { compose.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
                compose.onNodeWithContentDescription("Close menu").performClick()
            }
        } finally { model.navigate { saved } }
    }

    @Test fun choosesProjectAssistantAndDesktopWithoutSendingToAnotherComputer() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val model = SessionModel(context.applicationContext as Application)
        fun session(id: String, computer: String, path: String) = Session.parse(JSONObject().put("session_id", id).put("device_name", computer).put("cwd", path).put("repo_name", path.substringAfterLast('/')).put("active", true))
        val first = session("desk-one", "Desktop One", "/projects/NewsBlur")
        val second = session("desk-two", "Desktop Two", "/work/Crabigator")
        model.state.value = AppState(paired = true, sessions = listOf(first, second), selected = second)
        var sent: Pair<String, JSONObject>? = null
        var sends = 0
        val finish = CompletableDeferred<Unit>()
        val visible = mutableStateOf(true)
        compose.setContent { MaterialTheme(colorScheme = CrabTheme) {
            Surface(color = Ink) { Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                if (visible.value) NewSessionForm(model) { route, body -> sends++; sent = route to body; finish.await() }
            } }
        } }
        compose.onNodeWithText("/work/Crabigator").assertIsDisplayed()
        listOf("Codex", "Claude", "OpenCode", "Grok").forEach { name ->
            compose.onNodeWithText(name).performScrollTo().performClick().assertIsSelected()
        }
        compose.onNodeWithText("On Desktop Two").performScrollTo().performClick()
        compose.onNodeWithText("Desktop One").performClick()
        compose.onNodeWithText("/projects/NewsBlur").assertIsDisplayed()
        compose.runOnIdle { model.state.value = model.state.value.copy(sessions = listOf(second)) }
        compose.onNodeWithText("Desktop One · Offline").performClick()
        compose.onNodeWithText("Desktop Two").performClick()
        compose.onNodeWithText("Crabigator").performClick()
        compose.onNodeWithText("Other folder…").performClick()
        compose.onNodeWithText("Start session").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithText("Folder").performScrollTo().performTextInput("/projects/new folder")
        compose.onNodeWithText("Start session").performScrollTo().performClick()
        compose.waitUntil(5000) { sent != null }
        assertEquals("/api/sessions/desk-two/spawn", sent!!.first)
        assertEquals("/projects/new folder", sent!!.second.getString("cwd"))
        assertEquals("grok", sent!!.second.getString("platform"))
        compose.runOnIdle { visible.value = false }
        compose.waitForIdle()
        compose.runOnIdle { visible.value = true }
        compose.onNodeWithText("Starting…").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithText("Grok").assertIsSelected()
        compose.onNodeWithText("Folder").assertTextContains("/projects/new folder")
        compose.runOnIdle {
            model.startSession(second, "/should-not-start", "codex") { _, _ -> sends++ }
            assertEquals(1, sends)
            finish.complete(Unit)
        }
        compose.waitUntil(5000) { !model.sessionLaunch.value.busy }
        val dir = File(context.getExternalFilesDir(null), "conductor-evidence").apply { mkdirs() }
        File(dir, "android-new-session.png").outputStream().use { compose.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
    }
}
