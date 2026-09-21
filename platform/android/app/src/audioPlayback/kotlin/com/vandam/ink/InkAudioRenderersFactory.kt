package com.vandam.ink

import android.content.Context
import kotlin.math.roundToLong
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.audio.AudioProcessor
import androidx.media3.common.audio.AudioProcessorChain
import androidx.media3.common.audio.SonicAudioProcessor
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.exoplayer.audio.DefaultAudioSink

@UnstableApi
internal class InkAudioRenderersFactory(context: Context, private val effects: AudioEffects) : DefaultRenderersFactory(context) {
    // Keep PCM processing and speed adjustment in the same software chain.
    override fun buildAudioSink(context: Context, enableFloatOutput: Boolean,
        enableAudioOutputPlaybackParams: Boolean): AudioSink = DefaultAudioSink.Builder(context)
        .setAudioProcessorChain(InkAudioProcessorChain(effects))
        .build()
}

@UnstableApi
private class InkAudioProcessorChain(effects: AudioEffects) : AudioProcessorChain {
    private var speed = 1f
    private val quiet = QuietAudioProcessor { frames, rate ->
        effects.addSavedTime((frames * 1_000_000.0 / rate / speed).roundToLong())
    }
    private val sonic = SonicAudioProcessor()
    private val voice = VoiceAudioProcessor { effects.voiceBoost }
    private val processors = arrayOf<AudioProcessor>(quiet, voice, sonic)

    override fun getAudioProcessors() = processors

    override fun applyPlaybackParameters(playbackParameters: PlaybackParameters): PlaybackParameters {
        speed = playbackParameters.speed
        sonic.setSpeed(speed)
        sonic.setPitch(playbackParameters.pitch)
        return playbackParameters
    }

    override fun applySkipSilenceEnabled(skipSilenceEnabled: Boolean): Boolean {
        quiet.enabled = skipSilenceEnabled
        return skipSilenceEnabled
    }

    override fun getMediaDuration(playoutDuration: Long): Long =
        if (sonic.isActive) sonic.getMediaDuration(playoutDuration) else playoutDuration

    override fun getSkippedOutputFrameCount() = quiet.skippedFrames
}
