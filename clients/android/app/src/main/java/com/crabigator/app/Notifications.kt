package com.crabigator.app

import android.app.*
import android.content.*
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.app.RemoteInput
import androidx.work.*
import com.google.firebase.FirebaseApp
import com.google.firebase.messaging.FirebaseMessaging
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.util.concurrent.ConcurrentHashMap
import org.json.JSONObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

class CrabigatorApplication : Application() {
    override fun onCreate() { super.onCreate(); Notifications.channel(this) }
}
object Notifications {
    const val CHANNEL = "questions"
    fun channel(context: Context) { context.getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel(CHANNEL, "Questions and permissions", NotificationManager.IMPORTANCE_HIGH).apply { description = "Answer your Crabigator sessions"; lockscreenVisibility = Notification.VISIBILITY_PRIVATE }) }
    fun register(context: Context, api: Api) {
        if (FirebaseApp.getApps(context).isEmpty() || api.credentials.token.isBlank()) return
        val request = OneTimeWorkRequestBuilder<PushRegistrationWorker>()
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork("push-registration", ExistingWorkPolicy.KEEP, request)
    }
    fun status(context: Context): String = when {
        FirebaseApp.getApps(context).isEmpty() -> "Push not configured"
        !NotificationManagerCompat.from(context).areNotificationsEnabled() -> "Notifications off"
        context.getSystemService(NotificationManager::class.java).getNotificationChannel(CHANNEL)?.importance == NotificationManager.IMPORTANCE_NONE -> "Notifications off"
        else -> "Notifications on"
    }

    fun reconcile(context: Context, id: String? = null) {
        val request = OneTimeWorkRequestBuilder<NotificationWorker>().setInputData(workDataOf("session_id" to id)).setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()).setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST).build()
        WorkManager.getInstance(context).enqueueUniqueWork("notification-${id ?: "all"}", ExistingWorkPolicy.APPEND_OR_REPLACE, request)
    }
    private val locks = ConcurrentHashMap<String, Mutex>()
    suspend fun sync(context: Context, id: String, api: Api) = locks.getOrPut(id) { Mutex() }.withLock {
        syncLocked(context, id, api)
    }
    suspend fun answered(context: Context, id: String, api: Api) = locks.getOrPut(id) { Mutex() }.withLock {
        cancel(context, id)
        try { syncLocked(context, id, api) }
        catch (e: Exception) {
            if (e is CancellationException) throw e
            reconcile(context, id)
        }
    }
    private suspend fun syncLocked(context: Context, id: String, api: Api) {
        val snap = try { api.snapshot(id) } catch (e: ApiException) { if (e.status in listOf(401, 403, 404)) { cancel(context, id); return }; throw e }
        if (BuildConfig.DEBUG) android.util.Log.d("CrabigatorPush", "session=$id pending=${snap.optBoolean("attention_pending")} revision=${snap.optLong("prompt_revision")} structured=${snap.optJSONObject("prompt") != null}")
        if (!snap.optBoolean("attention_pending", snap.optJSONObject("prompt") != null) || !snap.optBoolean("desktop_connected")) { cancel(context, id); return }
        val prompt = snap.optJSONObject("prompt") ?: JSONObject().put("prompt_type", "text").put("state", snap.text("state"))
        show(context, id, snap.text("title", "Crabigator"), prompt, snap.optLong("prompt_revision"))
    }
    fun cancel(context: Context, id: String) { NotificationManagerCompat.from(context).cancel(id, 1) }
    @Suppress("MissingPermission")
    fun show(context: Context, id: String, title: String, prompt: JSONObject, revision: Long) {
        val manager = NotificationManagerCompat.from(context)
        if (!manager.areNotificationsEnabled()) return
        val open = PendingIntent.getActivity(context, id.hashCode(), Intent(context, MainActivity::class.java).putExtra("session_id", id).setData(android.net.Uri.parse("crabigator://session/$id")), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val details = PromptActions.details(prompt)
        val text = listOf(PromptActions.title(prompt), details.take(2000) + if (details.length > 2000) "…" else "").filter { it.isNotBlank() }.joinToString("\n")
        val builder = NotificationCompat.Builder(context, CHANNEL).setSmallIcon(R.drawable.ic_crabigator).setContentTitle(title).setContentText(text).setStyle(NotificationCompat.BigTextStyle().bigText(text)).setContentIntent(open).setOnlyAlertOnce(true).setAutoCancel(false).setCategory(NotificationCompat.CATEGORY_MESSAGE).setVisibility(NotificationCompat.VISIBILITY_PRIVATE).setTimeoutAfter(3600000).setExtras(android.os.Bundle().apply { putString("session_id", id); putLong("revision", revision) })
        fun pending(action: String, option: Int = -1, mutable: Boolean = false): PendingIntent {
            val intent = Intent(context, ReplyReceiver::class.java).setAction(action).setData(android.net.Uri.parse("crabigator://reply/$id/$revision/$option")).putExtra("session_id", id).putExtra("revision", revision).putExtra("option", option)
            return PendingIntent.getBroadcast(context, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT or if (mutable) PendingIntent.FLAG_MUTABLE else PendingIntent.FLAG_IMMUTABLE)
        }
        val q = PromptActions.question(prompt)
        val freeText = (q != null && !prompt.has("review") && q.optBoolean("allows_other", true)) || (prompt.text("prompt_type") == "text" && prompt.text("state") == "question")
        (if (details.length > 2000) emptyList() else PromptActions.options(prompt)).take(if (freeText) 2 else 3).forEachIndexed { i, option -> builder.addAction(NotificationCompat.Action.Builder(0, option.text("label"), pending("choose", i)).setAuthenticationRequired(true).build()) }
        if (freeText) builder.addAction(NotificationCompat.Action.Builder(0, "Reply", pending("reply", mutable = true)).addRemoteInput(RemoteInput.Builder("answer").setLabel("Your answer").build()).setAllowGeneratedReplies(false).setAuthenticationRequired(true).build())
        manager.notify(id, 1, builder.build())
    }
    @Suppress("MissingPermission")
    fun failed(context: Context, id: String, message: String = "Open the session to try again.") {
        val open = PendingIntent.getActivity(context, id.hashCode(), Intent(context, MainActivity::class.java).putExtra("session_id", id), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        NotificationManagerCompat.from(context).notify(id, 1, NotificationCompat.Builder(context, CHANNEL).setSmallIcon(R.drawable.ic_crabigator).setContentTitle("Answer was not sent").setContentText(message).setContentIntent(open).setAutoCancel(true).build())
    }
}
class PushService : FirebaseMessagingService() {
    override fun onMessageReceived(message: RemoteMessage) { if (BuildConfig.DEBUG) android.util.Log.d("CrabigatorPush", "FCM message received"); message.data["session_id"]?.takeIf { it.matches(Regex("[A-Za-z0-9-]{1,100}")) }?.let { Notifications.reconcile(this, it) } }
    override fun onNewToken(token: String) { Notifications.register(this, Api(Credentials(this))) }
    override fun onDeletedMessages() { Notifications.reconcile(this) }
}
class ReplyReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getStringExtra("session_id") ?: return
        val text = RemoteInput.getResultsFromIntent(intent)?.getCharSequence("answer")?.toString()
        if (intent.action == "reply" && text.isNullOrBlank()) {
            Notifications.failed(context, id)
            return
        }
        val data = try {
            workDataOf("session_id" to id, "revision" to intent.getLongExtra("revision", -1), "option" to intent.getIntExtra("option", -1), "answer" to text)
        } catch (_: IllegalStateException) {
            Notifications.failed(context, id, "Reply is too long. Open the session to send it.")
            return
        }
        val request = OneTimeWorkRequestBuilder<ReplyWorker>().setInputData(data).setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST).build()
        WorkManager.getInstance(context).enqueueUniqueWork("reply-$id-${intent.getLongExtra("revision", -1)}", ExistingWorkPolicy.KEEP, request)
    }
}
class PushRegistrationWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val api = Api(Credentials(applicationContext))
        if (api.credentials.token.isBlank() || FirebaseApp.getApps(applicationContext).isEmpty()) return Result.success()
        return try {
            val token = suspendCancellableCoroutine<String> { continuation ->
                FirebaseMessaging.getInstance().token
                    .addOnSuccessListener { if (continuation.isActive) continuation.resume(it) }
                    .addOnFailureListener { if (continuation.isActive) continuation.resumeWithException(it) }
            }
            api.call("/api/mobile/push", JSONObject().put("token", token).put("platform", "android"))
            Result.success()
        } catch (e: Exception) {
            if (e is CancellationException) throw e
            if (e is ApiException && e.status in listOf(401, 403)) Result.failure()
            else if (runAttemptCount < 5) Result.retry() else Result.failure()
        }
    }
}
class NotificationWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val api = Api(Credentials(applicationContext))
        if (api.credentials.token.isBlank()) return Result.success()
        return try {
            val id = inputData.getString("session_id")
            val ids = if (id != null) listOf(id) else {
                val board = api.call("/api/prs/board")
                val live = (board.array("sessions") + board.array("prs").flatMap { it.array("sessions") + it.array("touching") }).filter { it.text("state") in listOf("question", "permission") }.map { it.text("session_id") }
                val shown = applicationContext.getSystemService(NotificationManager::class.java).activeNotifications.mapNotNull { it.tag }
                (live + shown).distinct()
            }
            ids.forEach { Notifications.sync(applicationContext, it, api) }; Result.success()
        } catch (e: Exception) { if (e is CancellationException) throw e; if (runAttemptCount < 3) Result.retry() else Result.failure() }
    }
}
class ReplyWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val id = inputData.getString("session_id") ?: return Result.failure()
        val api = Api(Credentials(applicationContext))
        return try {
            val snap = api.snapshot(id)
            val revision = inputData.getLong("revision", -1)
            val prompt = snap.optJSONObject("prompt") ?: JSONObject().put("prompt_type", "text").put("state", snap.text("state"))
            if (!snap.optBoolean("attention_pending", snap.optJSONObject("prompt") != null) || snap.optLong("prompt_revision") != revision) { Notifications.sync(applicationContext, id, api); return Result.success() }
            val action = PromptActions.action(prompt, option = inputData.getInt("option", -1).takeIf { it >= 0 }, text = inputData.getString("answer"))
            api.action(id, action.route, action.body, revision)
            Notifications.answered(applicationContext, id, api)
            Result.success()
        } catch (e: Exception) {
            if (e is CancellationException) throw e
            if (e is ApiException && e.status == 409) { Notifications.reconcile(applicationContext, id); Result.success() }
            else { Notifications.failed(applicationContext, id); Result.failure() }
        }
    }
}
