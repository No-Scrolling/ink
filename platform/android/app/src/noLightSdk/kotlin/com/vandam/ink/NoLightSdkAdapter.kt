package com.vandam.ink

import android.view.HapticFeedbackConstants
import android.view.KeyEvent
import android.view.View

internal fun createLightSdkAdapter(
    _activity: MainActivity,
    _updateController: (Long, String) -> Unit,
): LightSdkAdapter = object : LightSdkAdapter {
    override fun start() = Unit

    override fun refresh() = Unit

    override fun stop() = Unit

    override fun performHaptic(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
    }

    override fun forwardDeviceKey(event: KeyEvent) = false

    override fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        complete(
            NativeResult.Failure(
                NativeErrorKind.UNAVAILABLE,
                "Light SDK is not enabled",
                true,
            ),
        )
    }

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        complete(
            NativeResult.Failure(
                NativeErrorKind.UNAVAILABLE,
                "Light SDK is not enabled",
                true,
            ),
        )
    }

    override fun cancel(requestId: Long) = Unit
}
