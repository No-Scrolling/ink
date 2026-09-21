package com.vandam.ink

import androidx.media3.common.C
import androidx.media3.common.audio.AudioProcessor.AudioFormat
import androidx.media3.common.audio.AudioProcessor.StreamMetadata
import androidx.media3.common.audio.AudioProcessor.UnhandledAudioFormatException
import androidx.media3.common.audio.BaseAudioProcessor
import androidx.media3.common.util.UnstableApi
import java.nio.ByteBuffer
import kotlin.math.abs
import kotlin.math.roundToInt

/** Shortens quiet regions with channel-linked, crossfaded cuts. */
@UnstableApi
internal class QuietAudioProcessor(private val onSkipped: (Long, Int) -> Unit) : BaseAudioProcessor() {
    var enabled = false
    var skippedFrames = 0L
        private set
    private var frames = 0
    private var channels = 0
    private var blockFrames = 0
    private var windowFrames = 0
    private var cutFrames = 0
    private var samples = ShortArray(0)
    private var quiet = BooleanArray(0)

    override fun isActive() = enabled && super.isActive()

    override fun onConfigure(inputAudioFormat: AudioFormat): AudioFormat {
        if (inputAudioFormat.encoding != C.ENCODING_PCM_16BIT) {
            throw UnhandledAudioFormatException(inputAudioFormat)
        }
        return inputAudioFormat
    }

    override fun onFlush(streamMetadata: StreamMetadata) {
        frames = 0
        skippedFrames = 0
        channels = inputAudioFormat.channelCount
        if (!enabled || channels <= 0) return
        // Preserve the same time windows at other decoder sample rates.
        val scale = inputAudioFormat.sampleRate / 44100.0
        blockFrames = (4096 * scale).roundToInt()
        windowFrames = (148 * scale).roundToInt().coerceAtLeast(3)
        cutFrames = (102 * scale).roundToInt().coerceIn(1, windowFrames - 2)
        samples = ShortArray(blockFrames * channels)
        quiet = BooleanArray(blockFrames)
    }

    override fun queueInput(inputBuffer: ByteBuffer) {
        val count = minOf(inputBuffer.remaining() / (channels * 2), blockFrames - frames)
        for (i in frames * channels until (frames + count) * channels) {
            samples[i] = inputBuffer.short
        }
        frames += count
        if (frames == blockFrames) outputBlock()
    }

    override fun onQueueEndOfStream() {
        if (frames > 0) outputBlock()
    }

    private fun outputBlock() {
        val previousSkipped = skippedFrames
        val output = replaceOutputBuffer(frames * channels * 2)
        for (frame in 0 until frames) {
            var peak = 0
            for (channel in 0 until channels) {
                peak = maxOf(peak, abs(samples[frame * channels + channel].toInt()))
            }
            // −48 dBFS; compare against the unrounded amplitude threshold.
            quiet[frame] = peak <= 130.45177
        }
        var read = 0
        var loudFrames = 0
        for (i in 0 until minOf(windowFrames, frames)) {
            if (!quiet[i]) loudFrames++
        }
        val fadeFrames = windowFrames - cutFrames
        while (read + windowFrames < frames) {
            if (loudFrames == 0) {
                for (i in 0 until fadeFrames) {
                    val mix = i.toFloat() / (fadeFrames - 1)
                    for (channel in 0 until channels) {
                        val start = samples[(read + i) * channels + channel].toInt()
                        val end = samples[(read + cutFrames + i) * channels + channel].toInt()
                        output.putShort((start + (end - start) * mix).roundToInt().toShort())
                    }
                }
                skippedFrames += cutFrames
                read += windowFrames
                loudFrames = 0
                for (i in read until minOf(read + windowFrames, frames)) {
                    if (!quiet[i]) loudFrames++
                }
            } else {
                for (channel in 0 until channels) output.putShort(samples[read * channels + channel])
                if (!quiet[read]) loudFrames--
                if (!quiet[read + windowFrames]) loudFrames++
                read++
            }
        }
        for (i in read * channels until frames * channels) output.putShort(samples[i])
        val removed = skippedFrames - previousSkipped
        if (removed > 0) onSkipped(removed, inputAudioFormat.sampleRate)
        frames = 0
        output.flip()
    }

    override fun onReset() {
        enabled = false
        frames = 0
        skippedFrames = 0
        samples = ShortArray(0)
        quiet = BooleanArray(0)
    }
}
