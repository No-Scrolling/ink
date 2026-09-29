package com.vandam.ink

internal interface NetworkAdapter : NativeAdapter {
    fun reset()
    fun stop()
}
