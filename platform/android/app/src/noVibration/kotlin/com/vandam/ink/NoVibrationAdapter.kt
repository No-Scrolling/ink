package com.vandam.ink

import android.content.Context

internal fun createVibrationAdapter(_context: Context, _hapticsEnabled: () -> Boolean): VibrationAdapter =
    object : VibrationAdapter {
        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Vibration is not enabled for this app", false))
        }
        override fun cancel(requestId: Long) = Unit
        override fun stop() = Unit
    }
