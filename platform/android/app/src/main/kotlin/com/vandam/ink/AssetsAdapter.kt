package com.vandam.ink

import org.json.JSONObject

internal class AssetsAdapter(private val activity: MainActivity) : NativeAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val path = runCatching { JSONObject(payload).getString("source") }.getOrNull()
        if (operation != "image" || path == null || !path.matches(Regex("ink-assets/[a-f0-9]{64}\\.(png|jpe?g|gif|webp)"))) {
            complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid bundled image", false))
            return
        }
        complete(activity.bundleImage(path))
    }

    override fun cancel(requestId: Long) {}
}
