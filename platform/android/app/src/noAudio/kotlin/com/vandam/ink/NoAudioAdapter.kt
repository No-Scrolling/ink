package com.vandam.ink

import android.app.Activity

internal fun createAudioAdapter(
    _activity: Activity,
    _bundle: InkBundle,
    _clock: PlaybackClock,
    _processing: AudioProcessing,
    _updateController: (Long, String) -> Unit,
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
