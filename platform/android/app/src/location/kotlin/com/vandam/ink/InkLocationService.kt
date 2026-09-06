package com.vandam.ink

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import org.json.JSONObject

internal class InkLocationService : Service() {
    private var updates: LocationUpdates? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == STOP) {
            stopTracking()
            return START_NOT_STICKY
        }
        val request = pending
        if (request == null || intent?.getLongExtra("request", -1) != request.first) {
            if (updates == null) stopSelf()
            return START_NOT_STICKY
        }
        pending = null
        instance = this
        lastError = null
        try {
            val options = JSONObject(intent.getStringExtra("options") ?: error("Missing tracking options"))
            validateLocationUpdates(options)
            val notifications = getSystemService(NotificationManager::class.java)
            notifications.createNotificationChannel(NotificationChannel(CHANNEL, "Location tracking", NotificationManager.IMPORTANCE_LOW))
            val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            val stop = PendingIntent.getService(this, 1, Intent(this, InkLocationService::class.java).setAction(STOP), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            val notification = Notification.Builder(this, CHANNEL)
                .setSmallIcon(android.R.drawable.ic_menu_mylocation)
                .setContentTitle("Location tracking")
                .setContentText("Location updates are active")
                .setContentIntent(open)
                .setOngoing(true)
                .addAction(Notification.Action.Builder(null, "Stop", stop).build())
                .build()
            startForeground(NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION)
            val subscription = LocationUpdates(this, options, { location ->
                getSharedPreferences(PREFERENCES, MODE_PRIVATE).edit().putString("fix", location.toJson().toString()).apply()
            }, { failure ->
                lastError = failure.message
                stopTracking()
            })
            subscription.start()
            updates = subscription
            request.second(NativeResult.Success(status(this).toString()))
        } catch (failure: Exception) {
            val message = failure.message ?: "Could not start location tracking"
            lastError = message
            request.second(NativeResult.Failure(if (failure is SecurityException) NativeErrorKind.PERMISSION_DENIED else NativeErrorKind.UNAVAILABLE, message, false))
            stopTracking()
        }
        return START_NOT_STICKY
    }

    private fun stopTracking() {
        updates?.stop()
        updates = null
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        updates?.stop()
        updates = null
        if (instance === this) instance = null
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "ink-location"
        private const val NOTIFICATION = 0x494E4B4C
        private const val STOP = "com.vandam.ink.STOP_LOCATION"
        private const val PREFERENCES = "ink-location"
        private var instance: InkLocationService? = null
        private var pending: Pair<Long, NativeResultHandler>? = null
        private var lastError: String? = null

        fun start(activity: MainActivity, id: Long, options: JSONObject, complete: NativeResultHandler) {
            check(activity.hasWindowFocus() && !activity.isFinishing) { "Start location tracking while the app is visible" }
            check(instance?.updates == null && pending == null) { "Location tracking is already running or starting" }
            if (Build.VERSION.SDK_INT >= 33 && activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                complete(NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, "Notification permission is required for location tracking", false))
                return
            }
            pending = id to complete
            try {
                activity.startForegroundService(Intent(activity, InkLocationService::class.java).putExtra("request", id).putExtra("options", options.toString()))
            } catch (error: Exception) {
                pending = null
                throw error
            }
        }

        fun cancel(id: Long) {
            if (pending?.first == id) pending = null
        }

        fun stop(context: Context) {
            lastError = null
            pending?.second?.invoke(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Location tracking was stopped", false))
            pending = null
            instance?.stopTracking()
            context.stopService(Intent(context, InkLocationService::class.java))
        }

        fun status(context: Context): JSONObject {
            val saved = context.getSharedPreferences(PREFERENCES, MODE_PRIVATE).getString("fix", null)
            return JSONObject().put("running", instance?.updates != null)
                .put("fix", saved?.let(::JSONObject) ?: JSONObject.NULL)
                .put("error", lastError ?: JSONObject.NULL)
        }
    }
}
