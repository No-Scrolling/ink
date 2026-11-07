package com.vandam.ink

import android.app.Activity

internal fun createAudioRecording(
    _activity: Activity,
    _updateController: (Long, String) -> Unit,
    _analysisActive: () -> Boolean,
): AudioRecording = object : AudioRecording {
    override val active = false
    override fun activate(controller: Long) = unavailable()
    override fun execute(operation: String) = unavailable()
    override fun deactivate() = Unit
    override fun pause() = Unit
    override fun stop() = Unit

    private fun unavailable() = NativeResult.Failure(
        NativeErrorKind.UNAVAILABLE, "Audio recording is not enabled", false,
    )
}
