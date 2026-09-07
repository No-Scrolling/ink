package com.vandam.ink

internal interface AudioMicrophone {
    val active: Boolean
    fun activate(controller: Long, kind: String, config: String): NativeResult
    fun execute(controller: Long, operation: String, complete: NativeResultHandler)
    fun deactivate(controller: Long)
    fun pause()
    fun resume()
    fun stop()
}
