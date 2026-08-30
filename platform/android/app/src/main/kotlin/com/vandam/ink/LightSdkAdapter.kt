package com.vandam.ink

import android.view.View

internal interface LightSdkAdapter {
    fun start()
    fun refresh()
    fun stop()
    fun performHaptic(view: View)
}

internal typealias HapticsChangedHandler = (Boolean) -> Unit
