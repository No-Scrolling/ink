package com.vandam.ink

import android.view.HapticFeedbackConstants
import android.view.View

internal fun createLightSdkAdapter(
    _activity: MainActivity,
    onHapticsChanged: HapticsChangedHandler,
): LightSdkAdapter = object : LightSdkAdapter {
    override fun start() {
        onHapticsChanged(true)
    }

    override fun refresh() = Unit

    override fun stop() = Unit

    override fun performHaptic(view: View) {
        view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
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
