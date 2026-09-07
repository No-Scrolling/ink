package com.vandam.ink

internal fun createAudioRecording(
    _activity: MainActivity,
    _updateController: (Long, String) -> Unit,
    _analysisActive: () -> Boolean,
): AudioRecording = object : AudioRecording {
    override val active = false
    override val source: String? = null
    override fun activate(controller: Long) = unavailable()
    override fun execute(operation: String) = unavailable()
    override fun deactivate() = Unit
    override fun pause() = Unit
    override fun stop() = Unit

    private fun unavailable() = NativeResult.Failure(
        NativeErrorKind.UNAVAILABLE, "Audio recording is not enabled", false,
    )
}
