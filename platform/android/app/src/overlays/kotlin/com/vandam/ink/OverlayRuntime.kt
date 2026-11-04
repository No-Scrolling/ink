package com.vandam.ink

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.View
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

internal class OverlayRuntime(private val context: Context, scale: Float, private val stop: () -> Unit) : InkJavaScriptHost, AutoCloseable {
    private val handler = Handler(Looper.getMainLooper())
    private val pending = AtomicBoolean()
    private var closed = false
    private var draining = false
    private val developmentBundle = if (BuildConfig.DEBUG) {
        File(context.filesDir, "ink-dev/active-worker").takeIf { it.isFile }?.readText()
            ?.takeIf { it.matches(Regex("[0-9]+")) }
            ?.let { File(context.filesDir, "ink-dev/$it") }
            ?.takeIf { File(it, "complete").isFile && File(it, "overlay.js").isFile }
    } else null
    val host: InkHost = InkHost(context, object : InkHost.Callbacks {
        override fun beforeFrame() {
            if (pending.get()) { handler.removeCallbacks(drain); drain.run() }
        }
        override fun changed() = drainRequests()
        override fun beforeRender() = drainRequests()
        override fun haptic(view: View) { view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY) }
    }, scale)
    private val images = Executors.newSingleThreadExecutor()
    private val requests = NativeRequests(handler) { id, result ->
        handler.post {
            if (closed) disposeNativeResult(result)
            else {
                val message = javascriptResult(id, result)
                if (result is NativeResult.Binary) InkNative.nativeJavaScriptReceiveBytes(host.handle, message, result.bytes)
                else InkNative.nativeJavaScriptReceive(host.handle, message)
            }
        }
    }
    private val connectivity = ConnectivityAdapter(context) { message ->
        handler.post { if (!closed) InkNative.nativeJavaScriptReceive(host.handle, message) }
    }
    private val network = createNetworkAdapter(context)
    private val auth = AuthAdapter(context) { message ->
        handler.post { if (!closed) InkNative.nativeJavaScriptReceive(host.handle, message) }
    }
    private val clipboard = ClipboardAdapter(context)
    private val secureStore = SecureStoreAdapter(context)
    private val overlayCommands = mutableMapOf<Long, Long>()
    private var unwatchOverlayState: (() -> Unit)? = null
    private val overlayAdapter = object : NativeAdapter {
        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
            when (operation) {
                "permission" -> complete(NativeResult.Success(Settings.canDrawOverlays(context).toString()))
                "state" -> complete(NativeResult.Success(InkOverlayService.state().toString()))
                "watch" -> {
                    if (unwatchOverlayState == null) unwatchOverlayState = InkOverlayService.watch { message ->
                        handler.post { if (!closed) InkNative.nativeJavaScriptReceive(host.handle, message) }
                    }
                    complete(NativeResult.Success(InkOverlayService.state().toString()))
                }
                "unwatch" -> {
                    unwatchOverlayState?.invoke()
                    unwatchOverlayState = null
                    complete(NativeResult.Success("null"))
                }
                else -> {
                    val command = InkOverlayService.command(context, operation, payload) { result ->
                        overlayCommands.remove(requestId)
                        complete(result)
                    }
                    if (command != null) overlayCommands[requestId] = command
                }
            }
        }
        override fun cancel(requestId: Long) { overlayCommands.remove(requestId)?.let(InkOverlayService::cancel) }
    }
    private val drain: Runnable = Runnable {
        pending.set(false)
        if (!closed) {
            if (InkNative.nativeDrainJavaScript(host.handle)) host.view.requestCommitFrame()
            val error = InkNative.nativeTakeJavaScriptError(host.handle)
            if (!error.isNullOrEmpty()) {
                Log.e("InkOverlay", error)
                stop()
                return@Runnable
            }
            InkNative.nativeTakeJavaScriptCalls(host.handle)?.let { source ->
                val calls = JSONArray(source)
                for (index in 0 until calls.length()) execute(calls.getJSONObject(index))
            }
            drainRequests()
        }
    }

    init {
        try {
            check(InkNative.nativeStartJavaScript(host.handle,
                developmentBundle?.let { File(it, "overlay.js").readText() },
                developmentBundle?.let { File(it, "ink-icons-v1.bin").readBytes() }, context.assets,
                context.getDatabasePath("ink-store.db").path, context.filesDir.path, this, "overlay")) {
                "Could not start overlay JavaScript"
            }
        } catch (error: Exception) { close(); throw error }
    }

    override fun onJavaScriptReady() {
        if (pending.compareAndSet(false, true)) handler.post(drain)
    }

    fun setContent(id: String, props: JSONObject) {
        check(!closed)
        InkNative.nativeJavaScriptReceive(host.handle, JSONObject().put("type", "overlay-content")
            .put("id", id).put("props", props).toString())
    }

    private fun execute(call: JSONObject) {
        val id = call.getLong("id")
        if (call.getString("type") == "cancel") { requests.cancel(id); return }
        val module = call.getString("module")
        if (module == "interaction" && call.getString("operation") == "haptic") {
            host.view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
            InkNative.nativeJavaScriptReceive(host.handle, javascriptResult(id, NativeResult.Success("")))
            return
        }
        val adapter = when (module) {
            "overlays" -> overlayAdapter
            "clipboard" -> clipboard
            "secure-store" -> secureStore
            "auth" -> auth
            "connectivity" -> connectivity
            "network" -> network
            else -> null
        }
        if (adapter == null || call.has("controller")) {
            InkNative.nativeJavaScriptReceive(host.handle, javascriptResult(id, NativeResult.Failure(
                NativeErrorKind.UNAVAILABLE, "This operation requires an app screen", false)))
            return
        }
        val bytes = if (call.optBoolean("binary")) InkNative.nativeTakeJavaScriptBytes(host.handle, id) else null
        requests.execute(id, call.optLong("timeoutMs", 30_000), { adapter.cancel(-id) }) { complete ->
            val payload = (call.opt("payload") ?: JSONObject.NULL).toString()
            if (bytes != null) adapter.executeBytes(-id, call.getString("operation"), payload, bytes, complete)
            else adapter.execute(-id, call.getString("operation"), payload, complete)
        }
    }

    private fun drainRequests() {
        if (closed || draining) return
        draining = true
        try {
            repeat(32) {
                val request = InkNative.nativeNextRequest(host.handle)?.let(::JSONObject) ?: return
                if (request.optBoolean("yield")) { handler.post(::drainRequests); return }
                val id = request.getLong("id")
                if (request.getInt("kind") == 2) return@repeat
                val payload = JSONObject(request.getString("payload"))
                val path = payload.optString("source")
                if (request.getInt("kind") == 3 && request.getString("module") == "assets" &&
                    path.matches(Regex("ink-assets/[a-f0-9]{64}\\.(png|jpe?g|gif|webp)"))) {
                    val handle = host.handle
                    images.execute {
                        val update = if (developmentBundle == null) InkNative.nativeCompleteAsset(handle, id, context.assets, path)
                            else InkNative.nativeCompleteFile(handle, id, File(developmentBundle, path).path)
                        handler.post { if (!closed && update != 0) host.view.requestFrame() }
                    }
                } else {
                    InkNative.nativeFailRequest(host.handle, id, NativeErrorKind.UNAVAILABLE.code,
                        "This native feature is unavailable in an overlay", false)
                }
            }
            handler.post(::drainRequests)
        } finally { draining = false }
    }

    override fun close() {
        if (closed) return
        closed = true
        requests.close()
        connectivity.close()
        network.stop()
        auth.close()
        unwatchOverlayState?.invoke()
        unwatchOverlayState = null
        overlayCommands.values.forEach(InkOverlayService::cancel)
        overlayCommands.clear()
        images.shutdown()
        handler.removeCallbacksAndMessages(null)
        host.close()
    }
}
