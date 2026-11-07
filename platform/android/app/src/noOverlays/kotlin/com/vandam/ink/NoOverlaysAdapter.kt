package com.vandam.ink

import android.app.Activity

internal fun createOverlaysAdapter(activity: Activity, emit: (String) -> Unit): OverlaysAdapter = object : OverlaysAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Overlay support is not included in this app", false))
    }
    override fun cancel(requestId: Long) {}
}
