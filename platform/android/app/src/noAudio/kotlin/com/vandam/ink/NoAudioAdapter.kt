package com.vandam.ink

internal fun createAudioAdapter(
    _activity: MainActivity,
    _processSamples: (ShortArray, Int) -> Unit,
    _updateController: (Long, String) -> Unit,
    _activateProcessor: (Long, String, String) -> Boolean,
    _deactivateProcessor: (Long) -> Unit,
    _setProcessorEnabled: (Long, Boolean) -> Boolean,
): AudioAdapter = object : AudioAdapter {
    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        complete(unavailable())
    }

    override fun executeController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        complete(unavailable())
    }

    override fun cancel(requestId: Long) = Unit

    override fun pause() = Unit

    override fun stop() = Unit

    private fun unavailable() = NativeResult.Failure(
        NativeErrorKind.UNAVAILABLE,
        "Audio is not enabled",
        false,
    )
}
