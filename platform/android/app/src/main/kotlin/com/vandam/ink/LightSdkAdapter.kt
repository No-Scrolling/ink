package com.vandam.ink

import android.view.View

internal interface LightSdkAdapter : NativeAdapter {
    fun start()
    fun refresh()
    fun stop()
    fun performHaptic(view: View)
}

internal typealias HapticsChangedHandler = (Boolean) -> Unit
