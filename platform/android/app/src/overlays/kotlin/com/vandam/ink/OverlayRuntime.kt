package com.vandam.ink

import android.content.Context
import android.provider.Settings
import android.util.Log
import org.json.JSONObject

internal class OverlayRuntime(context: Context, scale: Float, stop: () -> Unit) : AutoCloseable {
    private val runtime = InkUiRuntime(context, InkBundle.overlay(context), { InkOverlayModules(context, it) }, scale) { error ->
        Log.e("InkOverlay", error)
        stop()
    }
    val view get() = runtime.view

    init {
        try { runtime.start() }
        catch (error: Exception) { runtime.close(); throw error }
    }

    fun setContent(id: String, props: JSONObject) {
        runtime.send(JSONObject().put("type", "overlay-content").put("id", id).put("props", props).toString())
    }

    override fun close() = runtime.close()
}

private class InkOverlayModules(private val context: Context, private val runtime: InkUiRuntime) : InkUiModules {
    private val connectivity = ConnectivityAdapter(context) { message ->
        runtime.send(message)
    }
    private val network = createNetworkAdapter(context)
    private val auth = AuthAdapter(context) { message ->
        runtime.send(message)
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
                        runtime.send(message)
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

    private val assets = AssetsAdapter(runtime.bundle)

    override fun adapter(module: String): NativeAdapter? = when (module) {
        "overlays" -> overlayAdapter
        "clipboard" -> clipboard
        "secure-store" -> secureStore
        "auth" -> auth
        "connectivity" -> connectivity
        "network" -> network
        else -> null
    }

    override fun scene(request: InkSceneRequest, complete: NativeResultHandler) {
        if (request.kind == 3 && request.module == "assets") assets.execute(request.id, request.operation, request.payload, complete)
        else complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "This native feature is unavailable in an overlay", false))
    }

    override fun cancelScene(requestId: Long) = assets.cancel(requestId)

    override fun resetSession() {
        network.reset()
        connectivity.reset()
    }

    override fun close() {
        connectivity.close()
        network.stop()
        auth.close()
        unwatchOverlayState?.invoke()
        unwatchOverlayState = null
        overlayCommands.values.forEach(InkOverlayService::cancel)
        overlayCommands.clear()
    }
}
