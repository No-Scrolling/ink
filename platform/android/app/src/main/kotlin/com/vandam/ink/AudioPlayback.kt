package com.vandam.ink

internal interface AudioPlayback {
    fun activate(controller: Long, config: String): NativeResult

    fun execute(
        controller: Long,
        operation: String,
        payload: String,
    ): NativeResult

    fun reportFailure(
        controller: Long,
        kind: String,
        message: String,
        retryable: Boolean,
    )

    fun deactivate(controller: Long)

    fun pause()

    fun stop()
}
