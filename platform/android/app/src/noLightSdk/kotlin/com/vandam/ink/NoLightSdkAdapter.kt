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
}
