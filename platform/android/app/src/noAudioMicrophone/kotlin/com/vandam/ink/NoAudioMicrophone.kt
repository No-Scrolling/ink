package com.vandam.ink

internal fun createAudioMicrophone(
    _activity: MainActivity,
    _processSamples: (ShortArray, Int) -> Unit,
    _updateController: (Long, String) -> Unit,
    _activateProcessor: (Long, String, String) -> Boolean,
    _deactivateProcessor: (Long) -> Unit,
    _setProcessorEnabled: (Long, Boolean) -> Boolean,
    _recordingActive: () -> Boolean,
): AudioMicrophone = object : AudioMicrophone {
    override val active = false
    override fun activate(controller: Long, kind: String, config: String) = unavailable()
    override fun execute(controller: Long, operation: String, complete: NativeResultHandler) = complete(unavailable())
    override fun deactivate(controller: Long) = Unit
    override fun pause() = Unit
    override fun resume() = Unit
    override fun stop() = Unit

    private fun unavailable() = NativeResult.Failure(
        NativeErrorKind.UNAVAILABLE, "Microphone analysis is not enabled", false,
    )
}
