package com.vandam.ink

internal interface AudioAdapter : NativeAdapter {
    fun executeController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    )

    fun pause()

    fun resume() = Unit

    fun stop()
}
