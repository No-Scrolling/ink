package com.vandam.ink

internal interface AudioRecording {
    val active: Boolean
    fun activate(controller: Long): NativeResult
    fun execute(operation: String): NativeResult
    fun deactivate()
    fun pause()
    fun stop()
}
