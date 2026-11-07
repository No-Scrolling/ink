package com.vandam.ink

import android.app.Activity

internal fun createAudioPlayback(
    _activity: Activity,
    _updateController: (Long, String) -> Unit,
    _bundle: InkBundle,
    _clock: PlaybackClock,
): AudioPlayback = object : AudioPlayback {
    override fun activate(controller: Long, config: String) = unavailable()

    override fun execute(controller: Long, operation: String, payload: String, complete: NativeResultHandler) = complete(unavailable())

    override fun reportFailure(
        controller: Long,
        kind: String,
        message: String,
        retryable: Boolean,
    ) = Unit

    override fun deactivate(controller: Long) = Unit

    override fun pause() = Unit

    override fun stop() = Unit

    private fun unavailable() = NativeResult.Failure(
        NativeErrorKind.UNAVAILABLE,
        "Audio playback is not enabled",
        false,
    )
}
