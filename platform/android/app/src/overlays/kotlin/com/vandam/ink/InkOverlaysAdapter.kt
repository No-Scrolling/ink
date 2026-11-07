package com.vandam.ink

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.Settings

internal fun createOverlaysAdapter(activity: Activity, emit: (String) -> Unit): OverlaysAdapter = InkOverlaysAdapter(activity, emit)

private class InkOverlaysAdapter(private val activity: Activity, private val emit: (String) -> Unit) : OverlaysAdapter {
    private var permission: Pair<Long, NativeResultHandler>? = null
    private val pending = mutableMapOf<Long, Long>()
    private var unwatch: (() -> Unit)? = null

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        when (operation) {
            "permission" -> complete(NativeResult.Success(Settings.canDrawOverlays(activity).toString()))
            "state" -> complete(NativeResult.Success(InkOverlayService.state().toString()))
            "watch" -> {
                if (unwatch == null) unwatch = InkOverlayService.watch(emit)
                complete(NativeResult.Success(InkOverlayService.state().toString()))
            }
            "unwatch" -> {
                unwatch?.invoke()
                unwatch = null
                complete(NativeResult.Success("null"))
            }
            "request-permission" -> {
                if (Settings.canDrawOverlays(activity)) { complete(NativeResult.Success("null")); return }
                if (permission != null) { complete(NativeResult.Failure(NativeErrorKind.BUSY, "Overlay permission is already open", true)); return }
                try {
                    permission = requestId to complete
                    activity.startActivity(Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:${activity.packageName}")))
                } catch (error: Exception) {
                    permission = null
                    complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, error.message ?: "Overlay settings are unavailable", false))
                }
            }
            else -> {
                val command = InkOverlayService.command(activity, operation, payload) { result ->
                    pending.remove(requestId)
                    complete(result)
                }
                if (command != null) pending[requestId] = command
            }
        }
    }

    override fun resume() {
        val current = permission ?: return
        permission = null
        current.second(NativeResult.Success("null"))
    }

    override fun cancel(requestId: Long) {
        if (permission?.first == requestId) permission = null
        pending.remove(requestId)?.let(InkOverlayService::cancel)
    }

    override fun stop() {
        unwatch?.invoke()
        unwatch = null
        permission = null
        pending.values.forEach(InkOverlayService::cancel)
        pending.clear()
    }
}
