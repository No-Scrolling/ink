package com.vandam.ink

internal interface NetworkAdapter : NativeAdapter {
    fun warm() {}
    fun reset()
    fun stop()
}
