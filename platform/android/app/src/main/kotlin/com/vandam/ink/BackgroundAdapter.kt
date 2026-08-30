package com.vandam.ink

internal interface BackgroundAdapter : NativeAdapter {
    fun reconcile()
    fun stop()
}
