package com.vandam.ink

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.View
import java.io.File
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import org.json.JSONArray
import org.json.JSONObject

internal data class InkSceneRequest(
    val id: Long,
    val kind: Int,
    val module: String,
    val operation: String,
    val payload: String,
    val controller: Long,
)

internal interface InkUiModules : AutoCloseable {
    val presentation: InkHost.Callbacks? get() = null
    fun adapter(module: String): NativeAdapter?
    fun controller(requestId: Long, controller: Long, module: String, operation: String, payload: String, complete: NativeResultHandler) {
        val adapter = adapter(module)
        if (adapter is NativeControllerAdapter) adapter.executeController(controller, operation, payload, complete)
        else complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unsupported controller module: $module", false))
    }
    fun scene(request: InkSceneRequest, complete: NativeResultHandler)
    fun cancelScene(requestId: Long)
    fun resetSession() {}
    fun appearanceChanged(light: Boolean) {}
}

internal class InkUiRuntime(
    private val context: Context,
    val bundle: InkBundle,
    createModules: (InkUiRuntime) -> InkUiModules,
    scale: Float = 0f,
    private val onFailure: (String) -> Unit,
) : InkJavaScriptHost, AudioProcessing, PlaybackClock, AutoCloseable {
    var handle = InkNative.nativeCreate(context.codeCacheDir.absolutePath, BuildConfig.INK_MEDIA_LIBRARY_ENABLED)
        private set
    private val handler = Handler(Looper.getMainLooper())
    @Volatile private var closed = false
    private var running = false
    private val host: InkHost = InkHost(context, handle, object : InkHost.Callbacks {
        override fun beforeFrame() {
            if (javascriptPending.get()) { handler.removeCallbacks(drainJavaScript); drainJavaScript.run() }
        }
        override fun changed() {
            drainNativeRequests()
            modules.presentation?.changed()
        }
        override fun beforeRender() {
            drainNativeRequests()
            modules.presentation?.beforeRender()
        }
        override fun presented() { modules.presentation?.presented() }
        override fun haptic(view: View) = performHaptic()
    }, scale)
    val view: InkHost.InkSurfaceView get() = host.view
    private val imageExecutor = Executors.newFixedThreadPool(4)
    private var sceneRequests = NativeRequests(handler, NativeRequests.Policy.SCENE)
    private var refreshCompatibility: String? = null
    private var developmentIcons: ByteArray? = null

    fun start(directory: File? = bundle.directory) {
        check(!closed) { "Ink runtime is closed" }
        val source = bundle.source(directory)
        val icons = bundle.icons(directory)
        if (BuildConfig.DEBUG && bundle.name == "app") {
            refreshCompatibility = bundle.compatibility(directory)
            developmentIcons = icons ?: context.assets.open("ink-icons-v1.bin").use { it.readBytes() }
        }
        stopJavaScriptSession()
        handler.removeCallbacks(drainJavaScript)
        javascriptPending.set(false)
        javascriptRequests = newJavaScriptRequests()
        sceneRequests = NativeRequests(handler, NativeRequests.Policy.SCENE)
        check(InkNative.nativeStartJavaScript(handle, source, icons, context.assets,
            context.getDatabasePath("ink-store.db").path, context.filesDir.path, this, bundle.name)) {
            "Ink could not start JavaScript; see ink logs"
        }
        bundle.select(directory)
        running = true
        view.requestFrame()
    }

    fun activate(generation: String) {
        val directory = bundle.development(generation)
        val source = File(directory, "${bundle.name}.js").readText()
        val icons = File(directory, "ink-icons-v1.bin").readBytes()
        val compatibility = bundle.compatibility(directory)
        if (compatibility.isNotEmpty() && compatibility == refreshCompatibility &&
            developmentIcons?.contentEquals(icons) == true && InkNative.nativeRefreshJavaScript(handle, source)) {
            bundle.select(directory)
        } else start(directory)
        bundle.markActive(generation)
    }

    fun send(message: String) {
        val session = javascriptSession
        onMain {
            if (running && !closed && javascriptSession === session) InkNative.nativeJavaScriptReceive(handle, message)
        }
    }

    fun back(): Boolean {
        view.stopScrolling()
        if (handle == 0L || !InkNative.nativeBack(handle)) return false
        view.requestFrame()
        modules.presentation?.changed()
        drainNativeRequests()
        return true
    }

    private fun performHaptic() {
        val presentation = modules.presentation
        if (presentation == null) view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
        else presentation.haptic(view)
    }

    private fun onMain(action: () -> Unit) {
        if (Looper.myLooper() == Looper.getMainLooper()) action() else handler.post(action)
    }

    private val javascriptPending = AtomicBoolean(false)
    private var pendingJavaScriptCalls: JSONArray? = null
    private var javascriptCallIndex = 0
    private fun drainJavaScriptCalls(): Boolean {
        val calls = pendingJavaScriptCalls ?: return true
        val deadline = SystemClock.uptimeMillis() + 4
        repeat(32) {
            if (javascriptCallIndex == calls.length()) { pendingJavaScriptCalls = null; return true }
            executeJavaScriptCall(calls.getJSONObject(javascriptCallIndex++))
            if (SystemClock.uptimeMillis() >= deadline) { onJavaScriptReady(); return false }
        }
        onJavaScriptReady()
        return false
    }
    private val drainJavaScript = Runnable {
        javascriptPending.set(false)
        if (running && !closed) {
            if (!drainJavaScriptCalls()) return@Runnable
            if (InkNative.nativeDrainJavaScript(handle)) view.requestCommitFrame()
            val error = InkNative.nativeTakeJavaScriptError(handle)
            if (!error.isNullOrEmpty()) {
                stopJavaScriptSession()
                onFailure(error)
                return@Runnable
            }
            modules.appearanceChanged(InkNative.nativeIsLightAppearance(handle))
            modules.presentation?.changed()
            drainNativeRequests()
            InkNative.nativeTakeJavaScriptCalls(handle)?.let { encoded ->
                pendingJavaScriptCalls = JSONArray(encoded)
                javascriptCallIndex = 0
                drainJavaScriptCalls()
            }
        }
    }

    @Volatile private var javascriptSession: Any? = null
    private var javascriptRequests = newJavaScriptRequests()
    private val javascriptControllers = mutableMapOf<Long, String>()
    private fun newJavaScriptRequests(): NativeRequests {
        val session = Any()
        javascriptSession = session
        return NativeRequests(handler) { id, result ->
            onMain {
                if (!closed && running && javascriptSession === session) sendJavaScriptResult(id, result)
                else disposeNativeResult(result)
            }
        }
    }

    private fun stopJavaScriptSession() {
        running = false
        javascriptSession = null
        javascriptRequests.close()
        pendingJavaScriptCalls = null
        sceneRequests.close()
        handler.removeCallbacks(drainNativeRequestsLater)
        javascriptControllers.toMap().forEach { (controller, module) ->
            modules.controller(0, -controller, module, "deactivate", "{}") { disposeNativeResult(it) }
        }
        javascriptControllers.clear()
        modules.resetSession()
    }

    private fun executeJavaScriptCall(call: JSONObject) {
        val id = call.getLong("id")
        if (call.getString("type") == "cancel") {
            javascriptRequests.cancel(id)
            return
        }
        val bytes = if (call.optBoolean("binary")) InkNative.nativeTakeJavaScriptBytes(handle, id) else null
        if (call.optBoolean("binary") && bytes == null) {
            sendJavaScriptResult(id, NativeResult.Failure(NativeErrorKind.PROTOCOL, "Missing binary request data", false))
            return
        }
        val module = call.getString("module")
        if (module == "interaction" && call.getString("operation") == "haptic") {
            performHaptic()
            sendJavaScriptResult(id, NativeResult.Success(""))
            return
        }
        val adapter = modules.adapter(module)
        if (adapter == null) {
            sendJavaScriptResult(id, NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Unknown native module: $module", false))
            return
        }
        javascriptRequests.execute(id, call.optLong("timeoutMs", 30_000), { adapter.cancel(-id) }) { complete ->
            val session = javascriptSession
            val operation = call.getString("operation")
            val payload = (call.opt("payload") ?: JSONObject.NULL).toString()
            if (call.has("controller")) {
                val controller = call.getLong("controller")
                require(controller in 1..9_007_199_254_740_991L) { "Invalid JavaScript controller ID" }
                if (operation == "activate") javascriptControllers[controller] = module
                if (operation == "deactivate") javascriptControllers.remove(controller)
                val finish: NativeResultHandler = { result ->
                    if (operation == "activate" && result is NativeResult.Failure) {
                        onMain { if (javascriptSession === session) javascriptControllers.remove(controller) }
                    }
                    complete(result)
                }
                modules.controller(-id, -controller, module, operation, payload, finish)
            } else {
                if (bytes != null) adapter.executeBytes(-id, operation, payload, bytes, complete)
                else adapter.execute(-id, operation, payload, complete)
            }
        }
    }

    override fun processAudioSamples(samples: ShortArray, sampleRate: Int) = onMain {
        if (handle != 0L && InkNative.nativeAudioSamples(handle, samples, sampleRate)) view.requestFrame()
    }

    override fun activateAudioProcessor(controller: Long, kind: String, config: String): Boolean =
        handle != 0L && InkNative.nativeAudioActivate(handle, controller, kind, config)

    override fun deactivateAudioProcessor(controller: Long) {
        if (handle != 0L) InkNative.nativeAudioDeactivate(handle, controller)
    }

    override fun setAudioProcessorEnabled(controller: Long, enabled: Boolean): Boolean {
        val changed = handle != 0L && InkNative.nativeAudioSetEnabled(handle, controller, enabled)
        if (changed) view.requestFrame()
        return changed
    }

    override fun updatePlaybackClock(controller: Long, position: Long, duration: Long, playing: Boolean, speed: Float) {
        if (handle != 0L && InkNative.nativePlaybackClock(handle, controller, position / 1000f, duration / 1000f, playing, speed)) view.requestFrame()
    }

    override fun removePlaybackClock(controller: Long) {
        if (handle != 0L) InkNative.nativeRemovePlaybackClock(handle, controller)
    }

    override fun observeAudioMeasurements(controller: Long, enabled: Boolean): Boolean =
        handle != 0L && InkNative.nativeAudioObserve(handle, controller, enabled)

    override fun updateCaptureState(controller: Long, value: String) {
        if (handle != 0L && InkNative.nativeCaptureState(handle, -controller, value)) view.requestFrame()
    }

    override fun removeCaptureState(controller: Long) {
        if (handle != 0L) InkNative.nativeCaptureState(handle, -controller, "")
    }

    internal fun updateController(controller: Long, value: String) {
        val session = javascriptSession
        onMain {
            if (handle != 0L && javascriptSession === session && javascriptControllers.containsKey(-controller)) {
                InkNative.nativeJavaScriptReceive(handle, javascriptControllerMessage(-controller, value))
            }
        }
    }

    private fun sendJavaScriptResult(id: Long, result: NativeResult) {
        if (handle == 0L) return
        val message = javascriptResult(id, result)
        if (result is NativeResult.Binary) {
            if (!InkNative.nativeJavaScriptReceiveBytes(handle, message, result.bytes)) Log.e("Ink", "Could not deliver native binary result")
        } else InkNative.nativeJavaScriptReceive(handle, message)
    }

    override fun onJavaScriptReady() {
        if (!closed && javascriptPending.compareAndSet(false, true)) {
            handler.post(drainJavaScript)
        }
    }

    private val drainNativeRequestsLater = Runnable { drainNativeRequests() }
    private var drainingNativeRequests = false
    private val modules = try { createModules(this) } catch (error: Exception) {
        host.close()
        InkNative.nativeDestroy(handle)
        throw error
    }
    private fun cancelNativeRequest(requestId: Long) {
        if (!sceneRequests.cancel(requestId)) {
            logResource("cancel $requestId  0ms")
            modules.cancelScene(requestId)
        }
    }

    private fun drainNativeRequests() {
        if (!running || closed || drainingNativeRequests) return
        drainingNativeRequests = true
        try {
            handler.removeCallbacks(drainNativeRequestsLater)
            val deadline = SystemClock.uptimeMillis() + 4
            var handled = 0
            while (handle != 0L && handled++ < 32 && SystemClock.uptimeMillis() < deadline) {
                val request = InkNative.nativeNextRequest(handle)?.let(::JSONObject) ?: return
                if (request.optBoolean("yield")) break
                val requestId = request.getLong("id")
                val kind = request.getInt("kind")
                if (kind == NATIVE_REQUEST_CANCEL) {
                    cancelNativeRequest(requestId)
                    continue
                }
                val module = request.getString("module")
                val operation = request.getString("operation")
                val payload = request.getString("payload")
                val controller = request.getLong("controller")
                val label = "$module/$operation"
                val startedAt = SystemClock.elapsedRealtime()
                logResource("start $requestId $label")
                if (BuildConfig.INK_PRESENTATION_TIMING && kind == NATIVE_REQUEST_IMAGE) {
                    val provenance = JSONObject(payload)
                    Log.i("Ink", "InkImageRequest " + JSONObject().put("version", 1)
                        .put("request_id", requestId).put("module", module).put("operation", operation)
                        .put("source", provenance.optString("url", provenance.optString("source")))
                        .put("ns", System.nanoTime()))
                }
                val session = javascriptSession
                sceneRequests.execute(
                    requestId,
                    request.getLong("timeout"),
                    { modules.cancelScene(requestId) },
                    reply = { result ->
                        if (!closed && running && javascriptSession === session) {
                            completeNativeRequest(requestId, kind, label, startedAt, result)
                        } else disposeNativeResult(result)
                    },
                    cancelled = {
                        logResource("cancel $requestId $label ${SystemClock.elapsedRealtime() - startedAt}ms")
                    },
                ) { complete ->
                    modules.scene(InkSceneRequest(requestId, kind, module, operation, payload, controller)) { result ->
                        onMain { complete(result) }
                    }
                }
            }
            handler.post(drainNativeRequestsLater)
        } finally { drainingNativeRequests = false }
    }

    private fun completeNativeRequest(requestId: Long, kind: Int, label: String, startedAt: Long, result: NativeResult) {
        if (handle == 0L) {
            disposeNativeResult(result)
            return
        }
        val elapsed = SystemClock.elapsedRealtime() - startedAt
        val outcome = if (result is NativeResult.Failure) {
            "error:${result.kind.wireName}"
        } else {
            "ready"
        }
        logResource("$outcome $requestId $label ${elapsed}ms")
        val changed = when (result) {
            is NativeResult.Success, is NativeResult.Bytes, is NativeResult.Binary -> InkNative.nativeCompleteAction(handle, requestId)
            is NativeResult.Pixels -> InkNative.nativeCompletePixels(handle, requestId, result.width, result.height, result.rgba)
            is NativeResult.File, is NativeResult.Asset, is NativeResult.EncodedImage -> {
                completeImage(requestId, result)
                return
            }
            is NativeResult.Failure -> if (
                kind == NATIVE_REQUEST_IMAGE
            ) {
                InkNative.nativeFailRequest(
                    handle,
                    requestId,
                    result.kind.code,
                    result.message,
                    result.retryable,
                )
            } else {
                InkNative.nativeCompleteAction(handle, requestId)
            }
        }
        if (changed) view.requestFrame()
        modules.presentation?.changed()
        drainNativeRequests()
    }

    private fun completeImage(requestId: Long, result: NativeResult) {
        val engine = handle
        val session = javascriptSession
        imageExecutor.execute {
            if (javascriptSession !== session) { disposeNativeResult(result); return@execute }
            val update = when (result) {
                is NativeResult.Asset -> InkNative.nativeCompleteAsset(engine, requestId, context.assets, result.path)
                is NativeResult.EncodedImage -> InkNative.nativeCompleteEncoded(engine, requestId, result.bytes, result.size)
                is NativeResult.File -> try {
                    InkNative.nativeCompleteFile(engine, requestId, result.path)
                } finally {
                    if (result.deleteAfterRead) File(result.path).delete()
                }
                else -> IMAGE_UNCHANGED
            }
            onMain {
                if (handle != engine || javascriptSession !== session) return@onMain
                // Offscreen images share the next vsync frame so their uploads happen before scrolling.
                if (update == IMAGE_VISIBLE) view.requestCommitFrame()
                else if (update == IMAGE_OFFSCREEN) view.requestFrame()
                modules.presentation?.changed()
                drainNativeRequests()
            }
        }
    }

    private fun logResource(message: String) {
        if (BuildConfig.DEBUG) Log.d(RESOURCE_LOG_TAG, message)
    }

    override fun close() {
        if (closed) return
        closed = true
        try {
            stopJavaScriptSession()
            modules.close()
        } finally {
            // Queued result callbacks must still run to dispose resources from the retired session.
            handler.removeCallbacks(drainJavaScript)
            handler.removeCallbacks(drainNativeRequestsLater)
            host.close()
            imageExecutor.shutdown()
            InkNative.nativeDestroy(handle)
            handle = 0L
        }
    }

    private companion object {
        private const val RESOURCE_LOG_TAG = "InkResource"
        private const val NATIVE_REQUEST_CANCEL = 2
        private const val NATIVE_REQUEST_IMAGE = 3
        private const val IMAGE_UNCHANGED = 0
        private const val IMAGE_VISIBLE = 1
        private const val IMAGE_OFFSCREEN = 2

    }
}
