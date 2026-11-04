package com.vandam.ink

import android.app.AppOpsManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.provider.Settings
import android.view.Display
import android.view.Gravity
import android.view.WindowManager
import org.json.JSONObject
import java.util.concurrent.atomic.AtomicLong
import kotlin.math.roundToInt

class InkOverlayService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var windows: WindowManager
    private lateinit var windowContext: Context
    private var runtime: OverlayRuntime? = null
    private var overlayId: String? = null
    private var options = JSONObject()
    private var scale = 1f
    private val appOps by lazy { getSystemService(AppOpsManager::class.java) }
    private val permissionChanged = AppOpsManager.OnOpChangedListener { _, _ ->
        handler.post { if (!Settings.canDrawOverlays(this)) stopSelf() }
    }

    override fun onCreate() {
        super.onCreate()
        active = this
        val display = getSystemService(DisplayManager::class.java).getDisplay(Display.DEFAULT_DISPLAY)
        windowContext = createDisplayContext(display).createWindowContext(WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY, null)
        windows = windowContext.getSystemService(WindowManager::class.java)
        scale = windows.maximumWindowMetrics.bounds.width() / 1080f * 2.55f
        val notifications = getSystemService(NotificationManager::class.java)
        notifications.createNotificationChannel(NotificationChannel(CHANNEL, "Overlays", NotificationManager.IMPORTANCE_LOW))
        val close = PendingIntent.getService(this, 0, Intent(this, InkOverlayService::class.java).setAction(CLOSE), PendingIntent.FLAG_IMMUTABLE)
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        val notification = Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.ic_menu_view)
            .setContentTitle(applicationInfo.loadLabel(packageManager))
            .setContentText("Overlay displayed")
            .setContentIntent(open).setOngoing(true)
            .addAction(Notification.Action.Builder(null, "Close overlay", close).build()).build()
        startForeground(NOTIFICATION, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        appOps.startWatchingMode(AppOpsManager.OPSTR_SYSTEM_ALERT_WINDOW, packageName, permissionChanged)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == CLOSE) { stopSelf(); return START_NOT_STICKY }
        val command = intent?.getLongExtra("command", 0) ?: 0
        val request = commands.remove(command) ?: run {
            if (runtime == null) stopSelf(startId)
            return START_NOT_STICKY
        }
        try {
            val payload = JSONObject(request.payload)
            val id = payload.getString("id")
            require(id.matches(Regex("[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}"))) { "Invalid overlay ID" }
            if (request.operation != "show" && overlayId != id) {
                if (request.operation == "hide") request.complete(NativeResult.Success("null"))
                else request.complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "This overlay is not visible", false))
                if (runtime == null) stopSelf(startId)
                return START_NOT_STICKY
            }
            if (request.operation == "hide") {
                request.complete(NativeResult.Success("null"))
                stopSelf()
                return START_NOT_STICKY
            }
            check(Settings.canDrawOverlays(this)) { "Display over other apps permission is required" }
            val nextOptions = payload.optJSONObject("options") ?: if (request.operation == "show") JSONObject() else options
            val layout = layout(nextOptions)
            val props = payload.getJSONObject("props")
            val current = runtime ?: OverlayRuntime(windowContext, scale, ::stopSelf).also { candidate ->
                try { windows.addView(candidate.host.view, layout) }
                catch (error: Exception) { candidate.close(); throw error }
                runtime = candidate
            }
            if (overlayId != null) windows.updateViewLayout(current.host.view, layout)
            options = nextOptions
            overlayId = id
            current.setContent(id, props)
            publishState()
            request.complete(NativeResult.Success("null"))
        } catch (error: Exception) {
            request.complete(NativeResult.Failure(
                if (!Settings.canDrawOverlays(this)) NativeErrorKind.PERMISSION_DENIED else NativeErrorKind.PROTOCOL,
                error.message ?: "Could not show overlay", false))
            if (runtime == null) stopSelf(startId)
        }
        return START_NOT_STICKY
    }

    private fun layout(options: JSONObject): WindowManager.LayoutParams {
        val display = windows.maximumWindowMetrics.bounds
        fun dimension(name: String, fallback: Double): Int {
            val value = options.optDouble(name, fallback)
            require(value.isFinite() && value > 0) { "$name must be positive" }
            val pixels = (value * scale).roundToInt()
            require(pixels in 1..if (name == "width") display.width() else display.height()) { "$name exceeds the display" }
            return pixels
        }
        val width = dimension("width", display.width() / scale.toDouble())
        val height = dimension("height", Double.NaN)
        val position = options.optString("position", "bottom")
        require(position in setOf("top", "center", "bottom")) { "Invalid overlay position" }
        val offset = options.optDouble("offset", 0.0)
        val maxOffset = (display.height() - height) / if (position == "center") 2 else 1
        require(offset.isFinite() && offset >= 0 && offset * scale <= maxOffset) { "Invalid overlay offset" }
        return WindowManager.LayoutParams(width, height, WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
            PixelFormat.OPAQUE).apply {
            gravity = Gravity.CENTER_HORIZONTAL or when (position) {
                "top" -> Gravity.TOP
                "bottom" -> Gravity.BOTTOM
                else -> Gravity.CENTER_VERTICAL
            }
            y = (offset * scale).roundToInt()
            title = "Ink overlay"
        }
    }

    override fun onDestroy() {
        appOps.stopWatchingMode(permissionChanged)
        handler.removeCallbacksAndMessages(null)
        runtime?.let {
            try { windows.removeViewImmediate(it.host.view) }
            finally { it.close() }
        }
        runtime = null
        overlayId = null
        if (active === this) active = null
        publishState()
        commands.values.toList().forEach { it.complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Overlay service stopped", false)) }
        commands.clear()
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "ink-overlays"
        private const val NOTIFICATION = 0x494e4b
        private const val CLOSE = "com.vandam.ink.CLOSE_OVERLAY"
        private data class Command(val operation: String, val payload: String, val complete: NativeResultHandler)
        private val commands = mutableMapOf<Long, Command>()
        private val nextId = AtomicLong()
        private var active: InkOverlayService? = null
        private val listeners = mutableSetOf<(String) -> Unit>()

        internal fun state(): JSONObject = JSONObject().put("visible", active?.overlayId != null)
            .put("id", active?.overlayId ?: JSONObject.NULL)

        internal fun watch(emit: (String) -> Unit): () -> Unit {
            listeners.add(emit)
            return { listeners.remove(emit) }
        }

        private fun publishState() {
            val message = JSONObject().put("type", "overlays-changed").put("data", state()).toString()
            listeners.toList().forEach { it(message) }
        }

        internal fun command(context: Context, operation: String, payload: String, complete: NativeResultHandler): Long? {
            if (operation !in setOf("show", "update", "hide")) {
                complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown overlay operation", false)); return null
            }
            if (payload.toByteArray().size > 8192) {
                complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Overlay content exceeds 8 KiB", false)); return null
            }
            if (operation == "hide" && active == null) { complete(NativeResult.Success("null")); return null }
            if (operation == "update" && active == null) {
                complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "No overlay is visible", false)); return null
            }
            if (operation != "hide" && !Settings.canDrawOverlays(context)) {
                complete(NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, "Display over other apps permission is required", false)); return null
            }
            val id = nextId.incrementAndGet()
            commands[id] = Command(operation, payload, complete)
            try { context.startForegroundService(Intent(context, InkOverlayService::class.java).putExtra("command", id)) }
            catch (error: Exception) {
                commands.remove(id)
                complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, error.message ?: "Could not start overlay", false))
                return null
            }
            return id
        }

        internal fun cancel(id: Long) { commands.remove(id) }
    }
}
