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
