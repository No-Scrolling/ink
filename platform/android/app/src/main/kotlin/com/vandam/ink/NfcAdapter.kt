package com.vandam.ink

internal interface NfcAdapter : NativeAdapter {
    fun resume()
    fun pause()
    fun stop()
}
