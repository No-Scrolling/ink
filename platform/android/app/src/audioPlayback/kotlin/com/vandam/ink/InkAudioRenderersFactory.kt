package com.vandam.ink

import android.content.Context
import androidx.media3.common.MimeTypes
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.audio.AudioProcessor
import androidx.media3.common.audio.AudioProcessorChain
import androidx.media3.common.audio.SonicAudioProcessor
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.exoplayer.audio.DefaultAudioSink
import androidx.media3.exoplayer.mediacodec.MediaCodecUtil
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread
import kotlin.math.roundToLong

@UnstableApi
internal object AudioDecoderCache {
    private val started = AtomicBoolean(false)
    fun warm() {
        if (!started.compareAndSet(false, true)) return
        thread(name = "InkAudioCodecs", isDaemon = true) {
            for (mime in arrayOf(MimeTypes.AUDIO_VORBIS, MimeTypes.AUDIO_MPEG, MimeTypes.AUDIO_AAC, MimeTypes.AUDIO_OPUS, MimeTypes.AUDIO_FLAC)) {
                MediaCodecUtil.warmDecoderInfoCache(mime, false, false)
            }
        }
    }
}

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
