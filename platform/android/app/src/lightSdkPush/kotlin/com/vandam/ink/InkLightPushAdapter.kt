package com.vandam.ink

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.util.AtomicFile
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import org.unifiedpush.android.connector.FailedReason
import org.unifiedpush.android.connector.MessagingReceiver
import org.unifiedpush.android.connector.UnifiedPush
import org.unifiedpush.android.connector.data.PushEndpoint
import org.unifiedpush.android.connector.data.PushMessage
import java.io.File
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.util.UUID
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

internal fun createLightPushAdapter(
    activity: MainActivity,
    update: (Long, String) -> Unit,
): LightPushAdapter = InkLightPushAdapter(activity, update)

private class InkLightPushAdapter(
    private val activity: MainActivity,
    private val update: (Long, String) -> Unit,
) : LightPushAdapter {
    private val store = InkLightPushStore(activity)
    private val executor = Executors.newSingleThreadExecutor()
    private var controller: Long? = null
    private var started = false
    private val stateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) = refresh()
    }

    override fun start() {
        if (started) return
        started = true
        activity.registerReceiver(
            stateReceiver,
            IntentFilter(ACTION_LIGHT_PUSH_STATE_CHANGED),
            Context.RECEIVER_NOT_EXPORTED,
        )
    }

    override fun stop() {
        if (started) activity.unregisterReceiver(stateReceiver)
        started = false
        controller = null
        executor.shutdownNow()
    }

    override fun refresh() {
        updateController()
        val state = store.registration()
        if (!state.desired || executor.isShutdown) return
        when {
            state.endpoint.isEmpty() && state.status == "registering" -> registerConnector()
            state.endpoint.isNotEmpty() && state.status == "synchronising" -> synchronise(state)
        }
    }

    override fun handleIntent(intent: Intent) {
        val key = intent.getStringExtra(EXTRA_LIGHT_PUSH_KEY) ?: return
        val opened = runCatching { store.open(key) }
            .onFailure { Log.w(TAG, "Could not record Light push tap", it) }
            .getOrNull()
        if (opened == null) return
        InkNotificationPresenter.cancelPush(activity, key)
        intent.removeExtra(EXTRA_LIGHT_PUSH_KEY)
        notifyLightPushStateChanged(activity)
    }

    override fun executeController(controller: Long, operation: String, payload: String): Boolean {
        if (operation == "activate") {
            if (JSONObject(payload).optString("kind") != "light-push") return false
            this.controller = controller
            refresh()
            return true
        }
        if (operation == "deactivate") {
            if (this.controller != controller) return false
            this.controller = null
            return true
        }
        if (this.controller != controller) return false
        when (operation) {
            "register" -> register(payload)
            "retry" -> retry()
            "unregister" -> unregister()
            "dismiss" -> dismiss(payload)
            "clear" -> clear()
            else -> store.fail("protocol", "Unknown Light push operation: $operation", false)
        }
        updateController()
        return true
    }

    private fun register(payload: String) {
        val request = runCatching { JSONObject(payload) }.getOrElse {
            store.fail("protocol", "Invalid Light push registration", false)
            return
        }
        val baseUrl = request.optString("subscriptionBaseUrl").trimEnd('/')
        val token = request.optString("bearerToken")
        val error = runCatching {
            validateBaseUrl(baseUrl)
            require(token.toByteArray().size <= MAX_TOKEN_BYTES && token.none { Character.isISOControl(it.code) }) {
                "Bearer token is invalid"
            }
        }.exceptionOrNull()
        if (error != null) {
            store.fail("protocol", error.message ?: "Invalid Light push registration", false)
            return
        }
        store.configure(baseUrl, token)
        registerConnector()
    }

    private fun retry() {
        val state = store.retry() ?: return
        if (state.endpoint.isEmpty()) registerConnector() else synchronise(state)
    }

    private fun unregister() {
        val registration = store.beginUnregister()
        runCatching { UnifiedPush.unregister(activity, PUSH_INSTANCE) }
            .onFailure { Log.w(TAG, "Could not unregister UnifiedPush", it) }
        if (registration == null || executor.isShutdown) {
            store.completeUnregister(registration?.generation ?: -1, true, null)
            notifyLightPushStateChanged(activity)
            return
        }
        executor.execute {
            val error = InkLightPushRegistrar.delete(registration).exceptionOrNull()
            store.completeUnregister(registration.generation, error == null, error?.message)
            notifyLightPushStateChanged(activity)
        }
    }

    private fun dismiss(payload: String) {
        val key = runCatching { JSONObject(payload).getString("groupKey") }.getOrDefault("")
        if (!validOpaque(key, MAX_KEY_BYTES)) {
            store.fail("protocol", "Invalid Light push group key", false)
            return
        }
        store.dismiss(key)
        InkNotificationPresenter.cancelPush(activity, key)
    }

    private fun clear() {
        store.clear().forEach { InkNotificationPresenter.cancelPush(activity, it) }
    }

    private fun registerConnector() {
        store.markRegistering()
        runCatching {
            UnifiedPush.saveDistributor(activity, BuildConfig.INK_LIGHT_SERVER_PACKAGE)
            UnifiedPush.register(
                activity,
                instance = PUSH_INSTANCE,
                vapid = null,
                messageForDistributor = PUSH_CHANNEL,
            )
        }.onFailure { error ->
            store.fail("no-distributor", error.message ?: "No UnifiedPush distributor is available", true)
        }
        updateController()
    }

    private fun synchronise(registration: PushRegistration) {
        if (executor.isShutdown) return
        store.markSynchronising(registration.generation)
        executor.execute {
            val error = InkLightPushRegistrar.put(registration).exceptionOrNull()
            store.completeSync(registration.generation, error == null, error?.message)
            notifyLightPushStateChanged(activity)
        }
    }

    private fun updateController() {
        val value = store.controllerJson()
        controller?.let { update(it, value) }
    }
}

class InkLightPushReceiver : MessagingReceiver() {
    override fun onNewEndpoint(context: Context, endpoint: PushEndpoint, instance: String) {
        if (instance != PUSH_INSTANCE) return
        val pending = goAsync()
        LIGHT_PUSH_EXECUTOR.execute {
            try {
                val store = InkLightPushStore(context)
                runCatching {
                    store.setEndpoint(endpoint.url)?.let { registration ->
                        val error = InkLightPushRegistrar.put(registration).exceptionOrNull()
                        store.completeSync(
                            registration.generation,
                            error == null,
                            error?.message,
                        )
                    }
                }.onFailure { error ->
                    Log.w(TAG, "Could not store UnifiedPush endpoint", error)
                    runCatching {
                        store.fail("storage", error.message ?: "Could not store push endpoint", true)
                    }
                }
                notifyLightPushStateChanged(context)
            } finally {
                pending.finish()
            }
        }
    }

    override fun onRegistrationFailed(context: Context, reason: FailedReason, instance: String) {
        if (instance != PUSH_INSTANCE) return
        val kind = if (reason == FailedReason.ACTION_REQUIRED) "no-distributor" else "registration"
        InkLightPushStore(context).fail(kind, "UnifiedPush registration failed: $reason", true)
        notifyLightPushStateChanged(context)
    }

    override fun onUnregistered(context: Context, instance: String) {
        if (instance != PUSH_INSTANCE) return
        val pending = goAsync()
        LIGHT_PUSH_EXECUTOR.execute {
            try {
                val store = InkLightPushStore(context)
                val registration = store.pendingUnregister()
                val error = registration?.let { InkLightPushRegistrar.delete(it).exceptionOrNull() }
                store.completeUnregister(
                    registration?.generation ?: -1,
                    error == null,
                    error?.message,
                )
                notifyLightPushStateChanged(context)
            } finally {
                pending.finish()
            }
        }
    }

    override fun onTempUnavailable(context: Context, instance: String) {
        if (instance != PUSH_INSTANCE) return
        InkLightPushStore(context).fail("registration", "UnifiedPush is temporarily unavailable", true)
        notifyLightPushStateChanged(context)
    }

    override fun onMessage(context: Context, message: PushMessage, instance: String) {
        if (instance != PUSH_INSTANCE) return
        val pending = goAsync()
        LIGHT_PUSH_EXECUTOR.execute {
            try {
                val store = InkLightPushStore(context)
                val effects = runCatching {
                    store.apply(parseEnvelope(message.content))
                }.getOrElse { error ->
                    Log.w(TAG, "Rejected Light push payload: ${error.message}")
                    runCatching {
                        store.fail(
                            "protocol",
                            error.message ?: "Rejected Light push payload",
                            false,
                        )
                    }
                    notifyLightPushStateChanged(context)
                    return@execute
                }
                effects.cancelled.forEach { InkNotificationPresenter.cancelPush(context, it) }
                effects.shown.forEach { presentPush(context, it) }
                notifyLightPushStateChanged(context)
            } finally {
                pending.finish()
            }
        }
    }
}

class InkLightPushDismissReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val key = intent.getStringExtra(EXTRA_LIGHT_PUSH_KEY) ?: return
        InkLightPushStore(context).dismiss(key)
        notifyLightPushStateChanged(context)
    }
}

private fun presentPush(context: Context, message: LightPushRecord) {
    val encodedKey = Uri.encode(message.groupKey)
    val content = PendingIntent.getActivity(
        context,
        message.groupKey.hashCode(),
        Intent(context, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
            .setData(Uri.parse("ink-light-push://tap/$encodedKey"))
            .putExtra(EXTRA_LIGHT_PUSH_KEY, message.groupKey)
            .putExtra(EXTRA_NOTIFICATION_HREF, message.route),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
    val dismiss = PendingIntent.getBroadcast(
        context,
        message.groupKey.hashCode(),
        Intent(context, InkLightPushDismissReceiver::class.java)
            .setData(Uri.parse("ink-light-push://dismiss/$encodedKey"))
            .putExtra(EXTRA_LIGHT_PUSH_KEY, message.groupKey),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
    runCatching {
        InkNotificationPresenter.presentPush(
            context,
            message.groupKey,
            message.title,
            message.body,
            content,
            dismiss,
        )
    }.onFailure { Log.w(TAG, "Could not present Light push notification", it) }
}

private object InkLightPushRegistrar {
    fun put(registration: PushRegistration): Result<Unit> = request(registration, "PUT")
    fun delete(registration: PushRegistration): Result<Unit> = request(registration, "DELETE")

    private fun request(registration: PushRegistration, method: String): Result<Unit> = runCatching {
        val connection = URL("${registration.baseUrl}/${registration.installationId}")
            .openConnection() as HttpURLConnection
        try {
            connection.requestMethod = method
            connection.connectTimeout = REGISTRATION_TIMEOUT_MS
            connection.readTimeout = REGISTRATION_TIMEOUT_MS
            connection.setRequestProperty("Accept", "application/json")
            if (registration.bearerToken.isNotEmpty()) {
                connection.setRequestProperty("Authorization", "Bearer ${registration.bearerToken}")
            }
            if (method == "PUT") {
                connection.doOutput = true
                connection.setRequestProperty("Content-Type", "application/json")
                val bytes = JSONObject().put("endpoint", registration.endpoint).toString().toByteArray()
                connection.setFixedLengthStreamingMode(bytes.size)
                connection.outputStream.use { it.write(bytes) }
            }
            val code = connection.responseCode
            require(code in 200..299) { "Subscription server returned HTTP $code" }
        } finally {
            connection.disconnect()
        }
    }
}

private data class PushRegistration(
    val installationId: String,
    val baseUrl: String,
    val bearerToken: String,
    val endpoint: String,
    val generation: Long,
    val desired: Boolean,
    val status: String,
)

private data class LightPushRecord(
    val id: String,
    val groupKey: String,
    val title: String,
    val body: String,
    val route: String,
    val receivedAtMs: Long,
) {
    fun json(): JSONObject = JSONObject()
        .put("id", id)
        .put("groupKey", groupKey)
        .put("title", title)
        .put("body", body)
        .put("route", route)
        .put("receivedAtMs", receivedAtMs)

    companion object {
        fun fromJson(value: JSONObject) = LightPushRecord(
            id = value.getString("id"),
            groupKey = value.getString("groupKey"),
            title = value.getString("title"),
            body = value.getString("body"),
            route = value.optString("route"),
            receivedAtMs = value.getLong("receivedAtMs"),
        )
    }
}

private data class LightPushEvent(
    val operation: String,
    val id: String,
    val groupKey: String,
    val title: String,
    val body: String,
    val route: String,
)

private data class PushEffects(
    val shown: List<LightPushRecord>,
    val cancelled: Set<String>,
)

private class InkLightPushStore(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, "ink-light-push-v1.json"))

    fun controllerJson(): String = locked { read().controllerJson().toString() }

    fun registration(): PushRegistration = locked { read().registration() }

    fun configure(baseUrl: String, token: String) = mutate { state ->
        state.generation += 1
        state.desired = true
        state.baseUrl = baseUrl
        state.bearerToken = token
        state.status = "registering"
        state.clearError()
    }

    fun retry(): PushRegistration? = mutateReturning { state ->
        if (!state.desired || state.baseUrl.isEmpty()) return@mutateReturning null
        state.status = if (state.endpoint.isEmpty()) "registering" else "synchronising"
        state.clearError()
        state.registration()
    }

    fun markRegistering() = mutate { state ->
        if (state.desired) state.status = "registering"
    }

    fun markSynchronising(generation: Long) = mutate { state ->
        if (state.desired && state.generation == generation) state.status = "synchronising"
    }

    fun setEndpoint(endpoint: String): PushRegistration? = mutateReturning { state ->
        if (!state.desired || state.baseUrl.isEmpty()) return@mutateReturning null
        require(endpoint.toByteArray().size <= MAX_ENDPOINT_BYTES) { "Push endpoint is too long" }
        state.endpoint = endpoint
        state.status = "synchronising"
        state.clearError()
        state.registration()
    }

    fun completeSync(generation: Long, success: Boolean, message: String?) = mutate { state ->
        if (!state.desired || state.generation != generation) return@mutate
        if (success) {
            state.status = "ready"
            state.registeredAtMs = System.currentTimeMillis()
            state.clearError()
        } else {
            state.setError("subscription", message ?: "Could not synchronise push endpoint", true)
        }
    }

    fun beginUnregister(): PushRegistration? = mutateReturning { state ->
        if (state.baseUrl.isEmpty()) {
            state.resetRegistration()
            return@mutateReturning null
        }
        state.generation += 1
        state.desired = false
        state.status = "synchronising"
        state.clearError()
        state.registration()
    }

    fun pendingUnregister(): PushRegistration? = locked {
        read().takeIf { !it.desired && it.baseUrl.isNotEmpty() }?.registration()
    }

    fun completeUnregister(generation: Long, success: Boolean, message: String?) = mutate { state ->
        if (state.desired || (generation >= 0 && state.generation != generation)) return@mutate
        if (state.baseUrl.isEmpty()) return@mutate
        if (success) {
            state.resetRegistration()
        } else {
            state.setError("subscription", message ?: "Could not remove push subscription", true)
        }
    }

    fun fail(kind: String, message: String, retryable: Boolean) = mutate { state ->
        state.setError(kind, message, retryable)
    }

    fun dismiss(key: String) = mutate { state ->
        state.messages.removeAll { it.groupKey == key }
    }

    fun clear(): List<String> = mutateReturning { state ->
        val keys = state.messages.map { it.groupKey }
        state.messages.clear()
        state.openedKey = ""
        keys
    }

    fun open(key: String): LightPushRecord? = mutateReturning { state ->
        val message = state.messages.find { it.groupKey == key } ?: return@mutateReturning null
        state.messages.removeAll { it.groupKey == key }
        state.openedKey = key
        message
    }

    fun apply(events: List<LightPushEvent>): PushEffects = mutateReturning { state ->
        val shown = mutableListOf<LightPushRecord>()
        val cancelled = mutableSetOf<String>()
        events.forEach { event ->
            if (event.id in state.recentIds) return@forEach
            state.recentIds += event.id
            if (event.operation == "clear") {
                if (state.messages.removeAll { it.groupKey == event.groupKey }) {
                    cancelled += event.groupKey
                }
                return@forEach
            }
            val record = LightPushRecord(
                event.id,
                event.groupKey,
                event.title,
                event.body,
                event.route,
                System.currentTimeMillis(),
            )
            state.messages.removeAll { it.groupKey == event.groupKey }
            state.messages += record
            shown += record
            while (state.messages.size > MAX_MESSAGES) {
                cancelled += state.messages.removeAt(0).groupKey
            }
        }
        while (state.recentIds.size > MAX_RECENT_IDS) state.recentIds.removeAt(0)
        PushEffects(shown, cancelled)
    }

    private fun read(): PushState {
        if (!file.baseFile.exists()) return PushState()
        return runCatching {
            val root = JSONObject(file.readFully().toString(Charsets.UTF_8))
            require(root.optInt("version") == STORE_VERSION) { "Unsupported push store version" }
            PushState.fromJson(root)
        }.getOrElse { error ->
            Log.w(TAG, "Could not read Light push state", error)
            PushState()
        }
    }

    private fun write(state: PushState) {
        val output = file.startWrite()
        try {
            output.write(state.json().toString().toByteArray())
            output.fd.sync()
            file.finishWrite(output)
        } catch (error: Throwable) {
            file.failWrite(output)
            throw error
        }
    }

    private inline fun mutate(action: (PushState) -> Unit) = locked {
        val state = read()
        action(state)
        write(state)
    }

    private inline fun <T> mutateReturning(action: (PushState) -> T): T = locked {
        val state = read()
        val result = action(state)
        write(state)
        result
    }

    private inline fun <T> locked(action: () -> T): T = synchronized(LOCK) { action() }

    private companion object {
        val LOCK = Any()
    }
}

private data class PushState(
    var installationId: String = UUID.randomUUID().toString(),
    var generation: Long = 0,
    var desired: Boolean = false,
    var baseUrl: String = "",
    var bearerToken: String = "",
    var endpoint: String = "",
    var registeredAtMs: Long = 0,
    var status: String = "idle",
    var errorKind: String = "",
    var errorMessage: String = "",
    var errorRetryable: Boolean = false,
    var openedKey: String = "",
    val messages: MutableList<LightPushRecord> = mutableListOf(),
    val recentIds: MutableList<String> = mutableListOf(),
) {
    fun registration() = PushRegistration(
        installationId,
        baseUrl,
        bearerToken,
        endpoint,
        generation,
        desired,
        status,
    )

    fun clearError() {
        errorKind = ""
        errorMessage = ""
        errorRetryable = false
    }

    fun setError(kind: String, message: String, retryable: Boolean) {
        status = "error"
        errorKind = kind
        errorMessage = message.take(512)
        errorRetryable = retryable
    }

    fun resetRegistration() {
        desired = false
        baseUrl = ""
        bearerToken = ""
        endpoint = ""
        registeredAtMs = 0
        status = "idle"
        clearError()
    }

    fun controllerJson(): JSONObject = JSONObject()
        .put("status", status)
        .put("endpoint", endpoint)
        .put("registeredAtMs", registeredAtMs)
        .put("openedKey", openedKey)
        .put("messages", JSONArray().also { values -> messages.forEach { values.put(it.json()) } })
        .put("error", inkError(errorKind.ifEmpty { "unexpected" }, errorMessage, errorRetryable))

    fun json(): JSONObject = controllerJson()
        .put("version", STORE_VERSION)
        .put("installationId", installationId)
        .put("generation", generation)
        .put("desired", desired)
        .put("baseUrl", baseUrl)
        .put("bearerToken", bearerToken)
        .put("recentIds", JSONArray(recentIds))

    companion object {
        fun fromJson(root: JSONObject): PushState {
            val messages = root.optJSONArray("messages") ?: JSONArray()
            val recent = root.optJSONArray("recentIds") ?: JSONArray()
            val error = root.optJSONObject("error")
            return PushState(
                installationId = root.getString("installationId"),
                generation = root.optLong("generation"),
                desired = root.optBoolean("desired"),
                baseUrl = root.optString("baseUrl"),
                bearerToken = root.optString("bearerToken"),
                endpoint = root.optString("endpoint"),
                registeredAtMs = root.optLong("registeredAtMs"),
                status = root.optString("status", "idle"),
                errorKind = error?.optString("kind") ?: root.optString("errorKind"),
                errorMessage = error?.optString("message") ?: root.optString("errorMessage"),
                errorRetryable = error?.optBoolean("retryable")
                    ?: root.optBoolean("errorRetryable"),
                openedKey = root.optString("openedKey"),
                messages = MutableList(messages.length().coerceAtMost(MAX_MESSAGES)) {
                    LightPushRecord.fromJson(messages.getJSONObject(it))
                },
                recentIds = MutableList(recent.length().coerceAtMost(MAX_RECENT_IDS)) {
                    recent.getString(it)
                },
            )
        }
    }
}

private fun parseEnvelope(bytes: ByteArray): List<LightPushEvent> {
    require(bytes.size <= MAX_PAYLOAD_BYTES) { "Push payload exceeds $MAX_PAYLOAD_BYTES bytes" }
    val text = bytes.toString(Charsets.UTF_8)
    require(text.toByteArray(Charsets.UTF_8).contentEquals(bytes)) { "Push payload is not valid UTF-8" }
    val root = JSONObject(text)
    require(root.getInt("version") == WIRE_VERSION) { "Unsupported push envelope version" }
    val events = root.getJSONArray("events")
    require(events.length() in 1..MAX_EVENTS) { "Push envelope has an invalid event count" }
    return List(events.length()) { index ->
        val value = events.getJSONObject(index)
        val operation = value.getString("operation")
        require(operation == "show" || operation == "clear") { "Unknown push operation" }
        val id = value.getString("id")
        val groupKey = value.getString("groupKey")
        require(validOpaque(id, MAX_ID_BYTES)) { "Invalid push event ID" }
        require(validOpaque(groupKey, MAX_KEY_BYTES)) { "Invalid push group key" }
        value.opt("sentAtMs")?.let {
            require(it is Number && it.toDouble().isFinite() && it.toLong() >= 0) {
                "Invalid push event timestamp"
            }
        }
        if (operation == "clear") {
            LightPushEvent(operation, id, groupKey, "", "", "")
        } else {
            val title = value.getString("title")
            val body = value.getString("body")
            val route = value.optString("route")
            require(title.isNotBlank() && title.toByteArray().size <= MAX_TITLE_BYTES) {
                "Invalid push title"
            }
            require(body.isNotBlank() && body.toByteArray().size <= MAX_BODY_BYTES) {
                "Invalid push body"
            }
            require(route.isEmpty() || validNotificationRoute(route)) { "Invalid push route" }
            require(route.toByteArray().size <= MAX_ROUTE_BYTES) { "Push route is too long" }
            LightPushEvent(operation, id, groupKey, title, body, route)
        }
    }
}

private fun validateBaseUrl(value: String) {
    require(value.toByteArray().size in 1..MAX_BASE_URL_BYTES) { "Subscription base URL is invalid" }
    val uri = URI(value)
    require(uri.host != null && uri.userInfo == null && uri.query == null && uri.fragment == null) {
        "Subscription base URL is invalid"
    }
    val loopback = uri.host == "localhost" || uri.host == "127.0.0.1" ||
        uri.host == "::1" || uri.host == "10.0.2.2"
    require(uri.scheme == "https" || (uri.scheme == "http" && loopback)) {
        "Subscription base URL must use HTTPS (HTTP is allowed only for an emulator loopback)"
    }
}

private fun validOpaque(value: String, maximumBytes: Int): Boolean =
    value.isNotEmpty() && value.toByteArray().size <= maximumBytes &&
        value.none { Character.isISOControl(it.code) }

internal fun validNotificationRoute(route: String): Boolean =
    route.startsWith('/') &&
        !route.contains("//") &&
        (route == "/" || !route.endsWith('/')) &&
        !route.contains('?') &&
        !route.contains('#')

private fun notifyLightPushStateChanged(context: Context) {
    context.sendBroadcast(
        Intent(ACTION_LIGHT_PUSH_STATE_CHANGED).setPackage(context.packageName),
    )
}

private const val TAG = "InkLightPush"
private const val PUSH_INSTANCE = "light-push"
private const val PUSH_CHANNEL = "remote"
private const val ACTION_LIGHT_PUSH_STATE_CHANGED = "com.vandam.ink.LIGHT_PUSH_STATE_CHANGED"
private const val EXTRA_LIGHT_PUSH_KEY = "com.vandam.ink.lightpush.KEY"
private const val WIRE_VERSION = 1
private const val STORE_VERSION = 1
private const val MAX_PAYLOAD_BYTES = 4096
private const val MAX_EVENTS = 16
private const val MAX_MESSAGES = 64
private const val MAX_RECENT_IDS = 512
private const val MAX_ID_BYTES = 128
private const val MAX_KEY_BYTES = 128
private const val MAX_TITLE_BYTES = 120
private const val MAX_BODY_BYTES = 512
private const val MAX_ROUTE_BYTES = 256
private const val MAX_ENDPOINT_BYTES = 4096
private const val MAX_BASE_URL_BYTES = 2048
private const val MAX_TOKEN_BYTES = 2048
private const val REGISTRATION_TIMEOUT_MS = 10_000
private val LIGHT_PUSH_EXECUTOR: ExecutorService = Executors.newSingleThreadExecutor()
