package com.crabigator.app

import android.app.NotificationManager
import android.view.ViewGroup
import android.widget.TextView
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class NotificationCardTest {
    @Test fun refreshingTheCardReplacesChoicesWithoutRepeatingSystemButtons() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val id = "notification-card-test"
        val prompt = JSONObject("""{"prompt_type":"question","current_question":0,"questions":[{"question":"Which layout?","allows_other":false,"options":[{"label":"List","description":"Open only the sessions you choose"},{"label":"Board","description":"Group sessions by pull request"}]}]}""")
        try {
            Notifications.show(context, id, "Crabigator notification test", prompt, 7)
            val notification = context.getSystemService(NotificationManager::class.java).activeNotifications.single { it.tag == id }.notification
            assertTrue("Choices belong in the card only", notification.actions.isNullOrEmpty())
            val card = requireNotNull(notification.bigContentView)
            instrumentation.runOnMainSync {
                val view = card.apply(context, null)
                card.reapply(context, view)
                val choices = view.findViewById<ViewGroup>(R.id.notification_choices)
                assertEquals("Reapplying must replace the old rows", 2, choices.childCount)
                assertEquals("List", choices.getChildAt(0).findViewById<TextView>(R.id.notification_choice_label).text.toString())
                assertEquals("Open only the sessions you choose", choices.getChildAt(0).findViewById<TextView>(R.id.notification_choice_description).text.toString())
                assertTrue("The drawn choice must be tappable", choices.getChildAt(0).hasOnClickListeners())
            }
        } finally { Notifications.cancel(context, id) }
    }
}
