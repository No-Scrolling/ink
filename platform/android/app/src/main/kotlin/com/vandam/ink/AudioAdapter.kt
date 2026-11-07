package com.vandam.ink

internal interface PlaybackClock {
    fun updatePlaybackClock(controller: Long, position: Long, duration: Long, playing: Boolean, speed: Float)
    fun removePlaybackClock(controller: Long)
}

internal interface AudioProcessing {
    fun processAudioSamples(samples: ShortArray, sampleRate: Int)
    fun activateAudioProcessor(controller: Long, kind: String, config: String): Boolean
    fun deactivateAudioProcessor(controller: Long)
    fun setAudioProcessorEnabled(controller: Long, enabled: Boolean): Boolean
    fun observeAudioMeasurements(controller: Long, enabled: Boolean): Boolean
    fun updateCaptureState(controller: Long, value: String)
    fun removeCaptureState(controller: Long)
}

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
