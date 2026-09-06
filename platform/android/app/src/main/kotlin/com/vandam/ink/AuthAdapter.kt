package com.vandam.ink

import android.content.Context
import java.net.HttpURLConnection
import java.net.URI
import java.net.URLEncoder
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import org.json.JSONObject

internal class AuthAdapter(context: Context, private val emit: (String) -> Unit = {}) : NativeAdapter, AutoCloseable {
    init { observers[this] = emit }
    private val values = SecureValues(context.applicationContext)
    private class Request {
        @Volatile var cancelled = false
        @Volatile var connection: HttpURLConnection? = null
        var shared = false
        fun cancel() { cancelled = true; if (!shared) connection?.disconnect() }
    }
    private val requests = ConcurrentHashMap<Long, Request>()
    @Volatile private var closed = false
    override fun cancel(requestId: Long) { requests.remove(requestId)?.cancel() }
    override fun close() { closed = true; observers.remove(this); requests.values.forEach { it.cancel() }; requests.clear() }
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        if (closed) { complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Account runtime has stopped", false)); return }
        val request = Request().apply { shared = operation == "token" }
        requests[requestId] = request
        executor.execute {
            try {
                val input = JSONObject(payload)
                val config = input.getJSONObject("config")
                val id = config.getString("id")
                require(id.isNotBlank() && id.length <= 180) { "Invalid account ID" }
                val session = sessions.computeIfAbsent(id) { Session() }
                val result = synchronized(session) {
                    if (request.cancelled && !request.shared) throw IllegalStateException("Account request cancelled")
                    val key = "auth.$id"
                    val saved = if (operation == "sign-out") null else values.get(key)?.let { JSONObject(it) }
                    if (saved != null) require(saved.getString("clientId") == config.getString("clientId") && saved.getString("endpoint") == config.getString("tokenEndpoint")) { "Account configuration has changed; sign out first" }
                    when (operation) {
                        "status" -> JSONObject().put("status", if (saved == null) "signed-out" else "signed-in")
                        "begin" -> JSONObject().put("generation", session.generation)
                        "sign-out" -> { session.generation++; values.remove(key); JSONObject.NULL }
                        "device-start" -> post(config.getString("deviceAuthorizationEndpoint"), mapOf("client_id" to config.getString("clientId"), "scope" to scopes(config)), request)
                        "token", "exchange" -> {
                            if (operation == "token" && saved != null && saved.getLong("expiresAt") > System.currentTimeMillis() + 30_000) {
                                saved.getString("access_token")
                            } else {
                                val fields = mutableMapOf("client_id" to config.getString("clientId"))
                                if (operation == "token") {
                                    check(saved != null && saved.optString("refresh_token").isNotEmpty()) { "Interactive sign-in required" }
                                    fields["grant_type"] = "refresh_token"
                                    fields["refresh_token"] = saved.getString("refresh_token")
                                } else {
                                    check(input.getLong("generation") == session.generation) { "Sign-in cancelled by session change" }
                                    val supplied = input.getJSONObject("fields")
                                    supplied.keys().forEach { fields[it] = supplied.getString(it) }
                                }
                                val token = post(config.getString("tokenEndpoint"), fields, request)
                                if (token.has("error")) token else {
                                    require(token.optString("access_token").isNotEmpty()) { "Provider returned no access token" }
                                    require(token.optString("token_type", "Bearer").equals("Bearer", true)) { "Unsupported token type" }
                                    if (!token.has("refresh_token") && operation == "token" && saved != null) token.put("refresh_token", saved.optString("refresh_token"))
                                    token.put("expiresAt", System.currentTimeMillis() + token.optLong("expires_in", 3600).coerceIn(0, 31536000) * 1000)
                                    token.put("clientId", config.getString("clientId")).put("endpoint", config.getString("tokenEndpoint"))
                                    if (!request.cancelled || operation == "token") {
                                        values.set(key, token.toString())
                                        if (operation == "exchange") session.generation++
                                    }
                                    if (operation == "token") token.getString("access_token") else JSONObject().put("complete", true)
                                }
                            }
                        }
                        else -> throw IllegalArgumentException("Unknown auth operation")
                    }
                }
                if (operation == "sign-out" || (operation == "exchange" && result is JSONObject && result.optBoolean("complete"))) {
                    val event = JSONObject().put("type", "auth-changed").put("id", id).toString()
                    observers.values.forEach { observer -> observer(event) }
                }
                if (!request.cancelled) complete(NativeResult.Success(if (result is String) JSONObject.quote(result) else result.toString()))
            } catch (error: Exception) {
                if (!request.cancelled) complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED,
                    if (error is IllegalArgumentException || error is IllegalStateException) error.message ?: "Account operation failed" else "Account operation failed: ${error.javaClass.simpleName}", false))
            } finally { requests.remove(requestId, request) }
        }
    }
    private fun scopes(config: JSONObject): String = config.optJSONArray("scopes")?.let { array -> (0 until array.length()).joinToString(" ") { array.getString(it) } } ?: ""
    private fun post(endpoint: String, fields: Map<String, String>, request: Request): JSONObject {
        val uri = URI(endpoint)
        val localDebug = BuildConfig.DEBUG && uri.scheme == "http" && uri.host in setOf("localhost", "127.0.0.1", "10.0.2.2")
        require((uri.scheme == "https" || localDebug) && uri.host != null && uri.userInfo == null) { "OAuth endpoints must use HTTPS" }
        val connection = uri.toURL().openConnection() as HttpURLConnection
        request.connection = connection
        try {
            check(!request.cancelled || request.shared) { "Account request cancelled" }
            connection.instanceFollowRedirects = false
            connection.requestMethod = "POST"
            connection.connectTimeout = 15_000; connection.readTimeout = 20_000
            connection.doOutput = true
            connection.setRequestProperty("Content-Type", "application/x-www-form-urlencoded")
            connection.outputStream.use { it.write(fields.entries.joinToString("&") { "${URLEncoder.encode(it.key, "UTF-8")}=${URLEncoder.encode(it.value, "UTF-8")}" }.toByteArray()) }
            val stream = if (connection.responseCode in 200..299) connection.inputStream else connection.errorStream
            check(stream != null) { "Provider request failed (${connection.responseCode})" }
            val bytes = stream.use { input ->
                val output = java.io.ByteArrayOutputStream()
                val buffer = ByteArray(8192)
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) break
                    require(output.size() + count <= 1024 * 1024) { "Provider response too large" }
                    output.write(buffer, 0, count)
                }
                output.toByteArray()
            }
            val result = JSONObject(bytes.toString(Charsets.UTF_8))
            check(connection.responseCode in 200..299 || result.has("error")) { "Provider request failed (${connection.responseCode})" }
            return result
        } finally { request.connection = null; connection.disconnect() }
    }
    private class Session { var generation = 0L }
    companion object {
        private val executor = Executors.newCachedThreadPool()
        private val sessions = ConcurrentHashMap<String, Session>()
        private val observers = ConcurrentHashMap<AuthAdapter, (String) -> Unit>()
    }
}
