package com.crabigator.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.security.KeyStore
import java.util.UUID
import java.util.concurrent.TimeUnit
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Credentials are encrypted with a non-exportable Android Keystore key. */
class Credentials(context: Context) {
    companion object { private val lock = Any() }
    private val prefs = context.getSharedPreferences("connection", Context.MODE_PRIVATE)
    val mobileId: String = synchronized(lock) {
        prefs.getString("mobile_id", null) ?: UUID.randomUUID().toString().also { prefs.edit().putString("mobile_id", it).apply() }
    }
    val origin: String get() = prefs.getString("origin", "https://drinkcrabigator.com")!!
    private fun key(): SecretKey = synchronized(lock) {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey("crabigator", null) as? SecretKey) ?: KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder("crabigator", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    val token: String
        get() = runCatching {
            val saved = prefs.getString("token", null) ?: return ""
            val bytes = Base64.decode(saved, Base64.NO_WRAP)
            Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes.take(12).toByteArray())) }.doFinal(bytes.drop(12).toByteArray()).toString(Charsets.UTF_8)
        }.getOrDefault("")
    fun saveConnection(origin: String, token: String) = synchronized(lock) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val encrypted = Base64.encodeToString(cipher.iv + cipher.doFinal(token.toByteArray()), Base64.NO_WRAP)
        prefs.edit().putString("origin", origin.trimEnd('/')).putString("token", encrypted).apply()
    }
    fun connection(): Pair<String, String> = synchronized(lock) { origin to token }
    fun clear() { prefs.edit().remove("token").apply() }
}

internal fun pairingRequest(origin: String, code: String, mobileId: String, deviceName: String): Request {
    val uri = java.net.URI(origin)
    require(uri.scheme == "https" && uri.host != null && uri.userInfo == null && uri.rawQuery == null
        && uri.rawFragment == null && uri.path.orEmpty() in listOf("", "/")) { "Use an HTTPS server address." }
    val normalized = code.trim().filter { it.isLetterOrDigit() }
        .let { if (it.length == 9) it.uppercase().chunked(3).joinToString("-") else code.trim() }
    val body = JSONObject().put("pairing_token", normalized).put("mobile_id", mobileId).put("mobile_name", deviceName)
    // Pairing must never send an existing server's bearer token to the new host.
    return Request.Builder().url(origin.trimEnd('/') + "/api/pairing/claim")
        .post(body.toString().toRequestBody("application/json".toMediaType())).build()
}

class Api(val credentials: Credentials) {
    // Terminal actions must not be replayed automatically after an ambiguous failure.
    val client = OkHttpClient.Builder().connectTimeout(15, TimeUnit.SECONDS).readTimeout(25, TimeUnit.SECONDS)
        .pingInterval(20, TimeUnit.SECONDS).retryOnConnectionFailure(false).followRedirects(false).build()
    fun request(path: String): Request.Builder {
        val (origin, token) = credentials.connection()
        return Request.Builder().url(origin + path).header("Authorization", "Bearer $token")
    }
    private suspend fun execute(request: Request): JSONObject = withContext(Dispatchers.IO) {
        client.newCall(request).execute().use { response ->
            val json = runCatching { JSONObject(response.body?.string().orEmpty()) }.getOrNull()
            if (!response.isSuccessful) throw ApiException(response.code, json?.optString("error")
                ?.takeIf { it.isNotBlank() } ?: "Request failed (${response.code})")
            json ?: throw java.io.IOException("The server returned an unreadable response.")
        }
    }
    suspend fun call(path: String, body: JSONObject? = null): JSONObject = execute(request(path)
        .apply { if (body != null) post(body.toString().toRequestBody("application/json".toMediaType())) }.build())
    suspend fun pair(code: String, origin: String = credentials.origin) {
        val result = execute(pairingRequest(origin.trim(), code, credentials.mobileId, android.os.Build.MODEL))
        val token = result.getString("mobile_token")
        require(token.isNotBlank()) { "The server did not return a pairing credential." }
        credentials.saveConnection(origin.trim(), token)
    }
    suspend fun snapshot(id: String) = call("/api/mobile/sessions/$id")
    suspend fun action(id: String, route: String, body: JSONObject, revision: Long? = null): JSONObject {
        val payload = JSONObject(body.toString())
        if (revision != null) payload.put("expected_prompt_revision", revision)
        else payload.remove("expected_prompt_revision")
        return call("/api/sessions/$id/$route", payload)
    }
}
class ApiException(val status: Int, message: String) : Exception(message)
fun JSONArray.objects(): List<JSONObject> = (0 until length()).mapNotNull { optJSONObject(it) }
fun JSONObject.array(key: String): List<JSONObject> = optJSONArray(key)?.objects().orEmpty()
fun JSONObject.text(key: String, fallback: String = ""): String = if (isNull(key)) fallback else optString(key, fallback)

data class Session(val id: String, val title: String, val repo: String, val machine: String, val state: String, val active: Boolean, val glyph: String, val color: String, val recap: String, val platform: String, val branch: String, val background: String, val stats: SessionStats = SessionStats()) {
    val attention get() = state == "question" || state == "permission"
    companion object {
        fun parse(j: JSONObject) = Session(j.text("session_id", j.text("id")), j.text("title").ifBlank { j.text("dir_name", "Session") }, listOf(j.text("repo_owner"), j.text("repo_name")).filter { it.isNotBlank() }.joinToString("/"), j.text("device_name"), j.text("state", "ready"), j.optBoolean("active", true), j.optJSONObject("session_mark")?.text("glyph", "◈") ?: "◈", j.optJSONObject("session_mark")?.text("fg_hex", "#FFAA77") ?: "#FFAA77", j.optJSONObject("recap")?.text("headline") ?: "", j.text("platform"), j.text("branch"), j.optJSONObject("session_mark")?.text("bg_hex", "#352B25") ?: "#352B25", SessionStats.parse(j))
    }
}
data class PullRequest(val key: String, val title: String, val repo: String, val number: Int, val state: String, val checks: String, val url: String, val sessions: List<Session>, val watched: Boolean = false) {
    companion object {
        fun parse(j: JSONObject): PullRequest {
            val p = j.optJSONObject("pr") ?: j
            val repo = "${j.text("owner")}/${j.text("repo")}"
            return PullRequest("$repo#${j.optInt("number")}", p.text("title", "Pull request"), repo, j.optInt("number"), p.text("state", "OPEN"), when { p.optInt("checks_failed") > 0 -> "${p.optInt("checks_failed")} failed"; p.optInt("checks_pending") > 0 -> "Checks running"; p.optInt("checks_total") > 0 -> "Checks passed"; else -> "" }, p.text("url"), (j.array("sessions") + j.array("touching")).map(Session::parse).distinctBy { it.id }, p.optBoolean("watched"))
        }
    }
}
