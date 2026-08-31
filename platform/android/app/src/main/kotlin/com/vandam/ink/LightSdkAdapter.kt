package com.vandam.ink

import android.view.KeyEvent
import android.view.View

internal interface LightSdkAdapter : NativeAdapter {
    fun start()
    fun refresh()
    fun stop()
    fun performHaptic(view: View)
    fun forwardDeviceKey(event: KeyEvent): Boolean
    fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    )
}

internal data class KeyboardPreferences(
    val hapticsEnabled: Boolean,
    val emojis: String?,
    val keyAnimationEnabled: Boolean,
)

internal typealias KeyboardPreferencesChangedHandler = (KeyboardPreferences) -> Unit
