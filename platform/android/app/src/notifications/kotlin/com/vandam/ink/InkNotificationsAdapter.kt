package com.vandam.ink

import android.Manifest
import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.util.AtomicFile
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

internal fun createNotificationsAdapter(
    activity: MainActivity,
    update: (Long, String) -> Unit,
): NotificationsAdapter = InkNotificationsAdapter(activity, update)

private class InkNotificationsAdapter(
    private val activity: MainActivity,
    private val update: (Long, String) -> Unit,
) : NotificationsAdapter {
    private var tapController: Long? = null
    private val lightPush = createLightPushAdapter(activity, update)

    override fun start() = lightPush.start()

    override fun stop() = lightPush.stop()

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (operation) {
            "permission-status" -> complete(NativeResult.Success(permissionStatus()))
            "request-permission" -> {
                activity
                    .getSharedPreferences(PERMISSION_PREFS, Context.MODE_PRIVATE)
                    .edit()
                    .putBoolean(PERMISSION_REQUESTED, true)
                    .apply()
                activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), PERMISSION_REQUEST_CODE)
                complete(NativeResult.Success(""))
            }
            else -> complete(
                NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown notifications operation: $operation", false),
            )
        }
    }

    override fun executeController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (lightPush.executeController(controller, operation, payload)) {
            complete(NativeResult.Success(""))
            return
        }
        if (operation == "activate") {
            when (JSONObject(payload).optString("kind")) {
                "local-notifications" -> {
                    updateOperation(controller, "", "", null)
                }
                "notification-tap" -> {
                    tapController = controller
                    updateTap()
                }
            }
            complete(NativeResult.Success(""))
            return
        }
        if (operation == "deactivate") {
            if (tapController == controller) tapController = null
            complete(NativeResult.Success(""))
            return
        }
        when (operation) {
            "schedule" -> schedule(controller, payload)
            "cancel" -> cancel(controller, payload)
            "consume" -> {
                runCatching { InkNotificationStore(activity).consumeEvent() }
                updateTap()
            }
            else -> updateOperation(
                controller,
                operation,
                "",
                OperationError("protocol", "Unknown notifications controller operation: $operation", false),
            )
        }
        complete(NativeResult.Success(""))
    }

    override fun cancel(requestId: Long) = Unit

    override fun refreshEvents() {
        updateTap()
        lightPush.refresh()
    }

    override fun handleIntent(intent: Intent) {
        intent.getStringExtra(EXTRA_NOTIFICATION_ID)?.let { id ->
            runCatching { InkNotificationStore(activity).recordTap(id) }
            InkNotificationPresenter.cancel(activity, id)
            intent.removeExtra(EXTRA_NOTIFICATION_ID)
        }
        lightPush.handleIntent(intent)
    }

    private fun schedule(controller: Long, payload: String) {
        val request = runCatching { LocalNotification.parse(JSONObject(payload)) }.getOrElse {
            updateOperation(
                controller,
                "schedule",
                runCatching { JSONObject(payload).optString("id") }.getOrDefault(""),
                OperationError("invalid-request", it.message ?: "Invalid notification", false),
            )
            return
        }
        val permission = permissionStatus()
        if (permission != "granted") {
            updateOperation(
                controller,
                "schedule",
                request.id,
                OperationError("permission-$permission", "Notification permission is $permission", false),
            )
            return
        }
        val error = runCatching {
            InkNotificationStore(activity).upsert(request)
            InkNotificationScheduler.schedule(activity, request)
        }.exceptionOrNull()
        updateOperation(
            controller,
            "schedule",
            request.id,
            error?.let {
                val kind = if (it is NotificationCapacityException) "capacity" else "storage"
                OperationError(kind, it.message ?: "Could not schedule notification", kind == "storage")
            },
        )
    }

    private fun cancel(controller: Long, payload: String) {
        val id = runCatching { JSONObject(payload).getString("id") }.getOrDefault("")
        val validation = runCatching { LocalNotification.validateId(id) }.exceptionOrNull()
        if (validation != null) {
            updateOperation(
                controller,
                "cancel",
                id,
                OperationError("invalid-request", validation.message ?: "Invalid notification ID", false),
            )
            return
        }
        val error = runCatching {
            InkNotificationStore(activity).remove(id)
            InkNotificationScheduler.cancel(activity, id)
            InkNotificationPresenter.cancel(activity, id)
        }.exceptionOrNull()
        updateOperation(
            controller,
            "cancel",
            id,
            error?.let { OperationError("storage", it.message ?: "Could not cancel notification", true) },
        )
    }

    private fun updateOperation(controller: Long, operation: String, id: String, error: OperationError?) {
        update(
            controller,
            JSONObject()
                .put("status", if (error == null) "idle" else "error")
                .put("operation", operation)
                .put("id", id)
                .put(
                    "error",
                    inkError(
                        error?.kind ?: "unexpected",
                        error?.message.orEmpty(),
                        error?.retryable ?: false,
                    ),
                )
                .toString(),
        )
    }

    private fun updateTap() {
        val controller = tapController ?: return
        val event = runCatching { InkNotificationStore(activity).firstEvent() }.getOrNull()
        val value = JSONObject()
            .put("id", event?.id.orEmpty())
            .put("data", event?.data.orEmpty())
        update(
            controller,
            JSONObject()
                .put("status", if (event == null) "empty" else "ready")
                .put("value", value)
                .toString(),
        )
    }

    private fun permissionStatus(): String {
        if (activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED) {
            return "granted"
        }
        val requested = activity
            .getSharedPreferences(PERMISSION_PREFS, Context.MODE_PRIVATE)
            .getBoolean(PERMISSION_REQUESTED, false)
        return if (requested && !activity.shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS)) {
            "blocked"
        } else {
            "denied"
        }
    }

    private data class OperationError(val kind: String, val message: String, val retryable: Boolean)

    private companion object {
        const val PERMISSION_PREFS = "ink-notification-permission"
        const val PERMISSION_REQUESTED = "requested"
        const val PERMISSION_REQUEST_CODE = 4102
    }
}

internal data class LocalNotification(
    val id: String,
    val title: String,
    val body: String,
    val href: String,
    val data: String,
    val triggerAtMs: Long,
    val displayed: Boolean = false,
) {
    fun json(): JSONObject = JSONObject()
        .put("id", id)
        .put("title", title)
        .put("body", body)
        .put("href", href)
        .put("data", data)
        .put("triggerAtMs", triggerAtMs)
        .put("displayed", displayed)

    companion object {
        private val idPattern = Regex("[A-Za-z0-9][A-Za-z0-9._-]{0,63}")

        fun parse(json: JSONObject): LocalNotification {
            val id = json.getString("id")
            validateId(id)
            val title = json.getString("title")
            val body = json.getString("body")
            val href = json.optString("href")
            val data = json.optString("data")
            require(title.isNotBlank() && title.toByteArray().size <= 120) { "title must be 1–120 bytes" }
            require(body.isNotBlank() && body.toByteArray().size <= 512) { "body must be 1–512 bytes" }
            require(href.toByteArray().size <= 256 && (href.isEmpty() || validRoute(href))) {
                "href must be an Ink route of at most 256 bytes"
            }
            require(data.toByteArray().size <= 1_800) { "data must be at most 1800 bytes" }
            require(json.toString().toByteArray().size <= 2_800) { "notification payload must be at most 2800 bytes" }
            val hasDelay = json.has("delayMs")
            val hasTrigger = json.has("triggerAtMs")
            require(hasDelay.xor(hasTrigger)) { "exactly one of delayMs or triggerAtMs is required" }
            val now = System.currentTimeMillis()
            val trigger = if (hasDelay) {
                val delay = numberAsLong(json, "delayMs")
                require(delay in 0..MAX_FUTURE_MS) { "delayMs must be between 0 and $MAX_FUTURE_MS" }
                now + delay
            } else {
                numberAsLong(json, "triggerAtMs")
            }
            require(trigger in 0..(now + MAX_FUTURE_MS)) { "triggerAtMs is outside the supported range" }
            return LocalNotification(id, title, body, href, data, trigger)
        }

        fun fromJson(json: JSONObject): LocalNotification = LocalNotification(
            json.getString("id"),
            json.getString("title"),
            json.getString("body"),
            json.optString("href"),
            json.optString("data"),
            json.getLong("triggerAtMs"),
            json.optBoolean("displayed"),
        )

        fun validateId(id: String) {
            require(idPattern.matches(id)) { "notification IDs must match [A-Za-z0-9][A-Za-z0-9._-]{0,63}" }
        }

        private fun validRoute(href: String): Boolean =
            href.startsWith('/') &&
                !href.contains("//") &&
                (href == "/" || !href.endsWith('/')) &&
                !href.contains('?') &&
                !href.contains('#')

        private fun numberAsLong(json: JSONObject, name: String): Long {
            val number = json.getDouble(name)
            require(number.isFinite() && number >= 0.0 && number % 1.0 == 0.0) { "$name must be a non-negative integer" }
            return number.toLong()
        }

        private const val MAX_FUTURE_MS = 366L * 24 * 60 * 60 * 1000
    }
}

internal data class NotificationEvent(val id: String, val data: String) {
    fun json(): JSONObject = JSONObject().put("id", id).put("data", data)
}

private class NotificationCapacityException(message: String) : IllegalStateException(message)

internal class InkNotificationStore(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, "ink-notifications-v1.json"))

    fun upsert(notification: LocalNotification) = locked {
        val state = read()
        val replacing = state.notifications.any { it.id == notification.id }
        if (!replacing && state.notifications.size + state.events.size >= MAX_RECORDS) {
            throw NotificationCapacityException("At most $MAX_RECORDS notifications and tap events may be retained")
        }
        state.notifications.removeAll { it.id == notification.id }
        state.notifications.add(notification)
        write(state)
    }

    fun remove(id: String) = locked {
        val state = read()
        if (state.notifications.removeAll { it.id == id }) write(state)
    }

    fun markDisplayed(id: String): LocalNotification? = locked {
        val state = read()
        val index = state.notifications.indexOfFirst { it.id == id }
        if (index < 0) return@locked null
        val value = state.notifications[index].copy(displayed = true)
        state.notifications[index] = value
        write(state)
        value
    }

    fun recordTap(id: String): NotificationEvent? = locked {
        val state = read()
        val notification = state.notifications.find { it.id == id } ?: return@locked null
        state.notifications.removeAll { it.id == id }
        val event = NotificationEvent(notification.id, notification.data)
        state.events.add(event)
        write(state)
        event
    }

    fun firstEvent(): NotificationEvent? = locked { read().events.firstOrNull() }

    fun consumeEvent() = locked {
        val state = read()
        if (state.events.isNotEmpty()) {
            state.events.removeAt(0)
            write(state)
        }
    }

    fun scheduled(): List<LocalNotification> = locked { read().notifications.filterNot { it.displayed } }

    private fun read(): StoreState {
        if (!file.baseFile.exists()) return StoreState(mutableListOf(), mutableListOf())
        val root = JSONObject(file.readFully().toString(Charsets.UTF_8))
        val notifications = root.optJSONArray("notifications") ?: JSONArray()
        val events = root.optJSONArray("events") ?: JSONArray()
        return StoreState(
            MutableList(notifications.length()) { LocalNotification.fromJson(notifications.getJSONObject(it)) },
            MutableList(events.length()) {
                val event = events.getJSONObject(it)
                NotificationEvent(event.getString("id"), event.optString("data"))
            },
        )
    }

    private fun write(state: StoreState) {
        val notifications = JSONArray().also { array -> state.notifications.forEach { array.put(it.json()) } }
        val events = JSONArray().also { array -> state.events.forEach { array.put(it.json()) } }
        val bytes = JSONObject().put("notifications", notifications).put("events", events).toString().toByteArray()
        val output = file.startWrite()
        try {
            output.write(bytes)
            output.fd.sync()
            file.finishWrite(output)
        } catch (error: Throwable) {
            file.failWrite(output)
            throw error
        }
    }

    private data class StoreState(
        val notifications: MutableList<LocalNotification>,
        val events: MutableList<NotificationEvent>,
    )

    private inline fun <T> locked(action: () -> T): T = synchronized(lock) { action() }

    companion object {
        private const val MAX_RECORDS = 128
        private val lock = Any()
    }
}

private object InkNotificationScheduler {
    fun schedule(context: Context, notification: LocalNotification) {
        cancel(context, notification.id)
        InkNotificationPresenter.cancel(context, notification.id)
        if (notification.triggerAtMs <= System.currentTimeMillis()) {
            InkNotificationPresenter.present(context, InkNotificationStore(context).markDisplayed(notification.id) ?: return)
            return
        }
        context.getSystemService(AlarmManager::class.java).setAndAllowWhileIdle(
            AlarmManager.RTC_WAKEUP,
            notification.triggerAtMs,
            alarmIntent(context, notification.id),
        )
    }

    fun cancel(context: Context, id: String) {
        context.getSystemService(AlarmManager::class.java).cancel(alarmIntent(context, id))
    }

    private fun alarmIntent(context: Context, id: String): PendingIntent = PendingIntent.getBroadcast(
        context,
        id.hashCode(),
        Intent(context, InkNotificationAlarmReceiver::class.java)
            .setData(Uri.parse("ink-notification://alarm/${Uri.encode(id)}"))
            .putExtra(EXTRA_NOTIFICATION_ID, id),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
}

internal object InkNotificationPresenter {
    private const val CHANNEL_ID = "ink-reminders"
    private const val PUSH_CHANNEL_ID = "ink-messages"

    fun present(context: Context, notification: LocalNotification) {
        ensureChannel(context)
        val tapIntent = PendingIntent.getActivity(
            context,
            notification.id.hashCode(),
            Intent(context, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)
                .setData(Uri.parse("ink-notification://tap/${Uri.encode(notification.id)}"))
                .putExtra(EXTRA_NOTIFICATION_ID, notification.id)
                .putExtra(EXTRA_NOTIFICATION_HREF, notification.href),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val value = Notification.Builder(context, CHANNEL_ID)
            .setContentTitle(notification.title)
            .setContentText(notification.body)
            .setSmallIcon(android.R.drawable.ic_popup_reminder)
            .setCategory(Notification.CATEGORY_REMINDER)
            .setAutoCancel(true)
            .setContentIntent(tapIntent)
            .setDeleteIntent(
                PendingIntent.getBroadcast(
                    context,
                    notification.id.hashCode(),
                    Intent(context, InkNotificationDismissReceiver::class.java)
                        .setData(Uri.parse("ink-notification://dismiss/${Uri.encode(notification.id)}"))
                        .putExtra(EXTRA_NOTIFICATION_ID, notification.id),
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
                ),
            )
            .build()
        context.getSystemService(NotificationManager::class.java).notify(notification.id, 0, value)
    }

    fun presentPush(
        context: Context,
        key: String,
        title: String,
        body: String,
        contentIntent: PendingIntent,
        deleteIntent: PendingIntent,
    ) {
        ensureChannel(context, PUSH_CHANNEL_ID, "Messages")
        val value = Notification.Builder(context, PUSH_CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(body)
            .setSmallIcon(android.R.drawable.ic_dialog_email)
            .setCategory(Notification.CATEGORY_MESSAGE)
            .setAutoCancel(true)
            .setContentIntent(contentIntent)
            .setDeleteIntent(deleteIntent)
            .build()
        context.getSystemService(NotificationManager::class.java)
            .notify("ink-push:$key", 0, value)
    }

    fun cancelPush(context: Context, key: String) {
        context.getSystemService(NotificationManager::class.java).cancel("ink-push:$key", 0)
    }

    fun cancel(context: Context, id: String) {
        context.getSystemService(NotificationManager::class.java).cancel(id, 0)
    }

    private fun ensureChannel(context: Context) {
        ensureChannel(context, CHANNEL_ID, "Reminders")
    }

    private fun ensureChannel(context: Context, id: String, name: String) {
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(
            NotificationChannel(id, name, NotificationManager.IMPORTANCE_HIGH).apply {
                enableVibration(true)
                setShowBadge(true)
            },
        )
    }
}

class InkNotificationAlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_NOTIFICATION_ID) ?: return
        val notification = runCatching { InkNotificationStore(context).markDisplayed(id) }.getOrNull() ?: return
        InkNotificationPresenter.present(context, notification)
    }
}

class InkNotificationDismissReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_NOTIFICATION_ID) ?: return
        runCatching { InkNotificationStore(context).remove(id) }
    }
}

class InkNotificationBootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        runCatching { InkNotificationStore(context).scheduled() }
            .getOrDefault(emptyList())
            .forEach { InkNotificationScheduler.schedule(context, it) }
    }
}

internal const val EXTRA_NOTIFICATION_ID = "com.vandam.ink.notification.ID"
