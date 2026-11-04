package com.crabigator.app

import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.mutableStateOf
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

class QuestionTransitionTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()
    @Test fun previewAnswersAdvanceWithoutFlashingAndCursorUpdatesKeepDrafts() {
        val first = JSONObject("""{"prompt_type":"question","current_question":0,"cursor_row":1,"questions":[{"question":"Choose a layout","options":[{"label":"List","preview":"┌────────────┐\n│ Story list │\n└────────────┘"},{"label":"Cards","preview":"[ Story ] [ Story ]"}]},{"question":"What happens at the end?","options":[{"label":"Repeat"},{"label":"Stop"}]},{"question":"Start automatically?","options":[{"label":"Yes"},{"label":"No"}]}]}""")
        val prompt = mutableStateOf<JSONObject?>(first)
        var action: PromptActions.Action? = null
        compose.setContent { MaterialTheme(colorScheme = CrabTheme) {
            Surface { Column(Modifier.fillMaxSize()) {
                Text("Interview transition test")
                Spacer(Modifier.weight(1f))
                PromptTransition(prompt.value, false) { action = it }
                Text("Message composer")
            } }
        } }
        compose.onNodeWithText("Choose a layout").assertIsDisplayed()
        capture("preview")
        compose.onNodeWithText("List").performClick()
        assertEquals("enter", action!!.body.getJSONArray("steps").getJSONObject(1).getString("key"))
        compose.runOnIdle { prompt.value = null }
        compose.onNodeWithText("List").assertIsNotEnabled()
        val second = JSONObject(first.toString()).put("current_question", 1)
        compose.runOnIdle { prompt.value = second }
        compose.onNodeWithText("What happens at the end?").assertIsDisplayed()
        compose.onNodeWithText("Your answer").performTextInput("Keep my draft")
        compose.runOnIdle { prompt.value = JSONObject(second.toString()).put("cursor_row", 2) }
        compose.onNodeWithText("Keep my draft").assertIsDisplayed()
        capture("second")
        compose.onNodeWithText("Repeat").performClick()
        assertEquals(1, action!!.body.getJSONArray("steps").length())
        compose.runOnIdle { prompt.value = JSONObject(first.toString()).put("current_question", 2) }
        compose.onNodeWithText("Start automatically?").assertIsDisplayed()
        compose.onNodeWithText("Keep my draft").assertDoesNotExist()
        capture("third")
        compose.runOnIdle { prompt.value = null }
        compose.waitUntil(3000) { compose.onAllNodesWithText("Start automatically?").fetchSemanticsNodes().isEmpty() }
    }
    private fun capture(name: String) {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dir = File(context.getExternalFilesDir(null), "question-evidence").apply { mkdirs() }
        File(dir, "$name.png").outputStream().use { compose.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it) }
    }
}
