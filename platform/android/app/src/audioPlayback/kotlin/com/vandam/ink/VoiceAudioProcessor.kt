package com.vandam.ink

import androidx.media3.common.C
import androidx.media3.common.audio.AudioProcessor.AudioFormat
import androidx.media3.common.audio.AudioProcessor.StreamMetadata
import androidx.media3.common.audio.BaseAudioProcessor
import androidx.media3.common.util.UnstableApi
import java.nio.ByteBuffer
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.log10
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.tan

/** Streaming loudness levelling with linked-channel compression and peak protection. */
@UnstableApi
internal class VoiceAudioProcessor(private val enabled: () -> Boolean) : BaseAudioProcessor() {
    private var ending = false
    private var channels = 0
    private var rate = 0
    private var delay = 0
    private var write = 0L
    private var read = 0L
    private var audio = FloatArray(0)
    private var peaks = FloatArray(0)
    private var deque = LongArray(0)
    private var head = 0L
    private var tail = 0L
    private var filters = emptyArray<Array<Filter>>()
    private var energies = DoubleArray(0)
    private var energyIndex = 0
    private var energyCount = 0
    private var energySum = 0.0
    private var measureFrames = 0
    private var gain = 1.0
    private var targetGain = 1.0
    private var envelope = 0.0
    private var limiter = 1.0
    private var mix = 0.0
    private var gainRise = 0.0
    private var gainFall = 0.0
    private var attack = 0.0
    private var release = 0.0
    private var limiterRelease = 0.0

    override fun onConfigure(inputAudioFormat: AudioFormat): AudioFormat {
        if (inputAudioFormat.encoding != C.ENCODING_PCM_16BIT || inputAudioFormat.channelCount !in 1..2) {
            return AudioFormat.NOT_SET
        }
        return inputAudioFormat
    }

    override fun onFlush(streamMetadata: StreamMetadata) {
        ending = false
        channels = inputAudioFormat.channelCount
        rate = inputAudioFormat.sampleRate
        if (rate <= 0 || channels !in 1..2) return
        delay = (rate * .01).roundToInt()
        audio = FloatArray((delay + 1) * channels)
        peaks = FloatArray(delay + 1)
        deque = LongArray(delay + 1)
        filters = Array(channels) { arrayOf(Filter.shelf(rate), Filter.highPass(rate)) }
        energies = DoubleArray((rate * .4).roundToInt())
        energyIndex = 0
        energyCount = 0
        energySum = 0.0
        measureFrames = 0
        write = 0
        read = 0
        head = 0
        tail = 0
        gain = 1.0
        targetGain = 1.0
        envelope = 0.0
        limiter = 1.0
        mix = 0.0
        gainRise = coefficient(3.0)
        gainFall = coefficient(.3)
        attack = coefficient(.02)
        release = coefficient(.05)
        limiterRelease = coefficient(.08)
    }

    private fun coefficient(seconds: Double) = 1 - exp(-1 / (seconds * rate))

    override fun queueInput(inputBuffer: ByteBuffer) {
        val count = inputBuffer.remaining() / (channels * 2)
        if (count == 0) return
        val output = replaceOutputBuffer(count * channels * 2)
        val active = enabled()
        repeat(count) {
            mix = (mix + (if (active) 1 else -1) / (rate * .02)).coerceIn(0.0, 1.0)
            val slot = (write % peaks.size).toInt()
            var energy = 0.0
            var peak = 0.0
            for (channel in 0 until channels) {
                val sample = inputBuffer.short / 32768.0
                audio[slot * channels + channel] = sample.toFloat()
                val weighted = filters[channel][1].process(filters[channel][0].process(sample))
                energy += weighted * weighted
                peak = max(peak, abs(sample))
            }
            energySum += energy - energies[energyIndex]
            energies[energyIndex] = energy
            energyIndex = (energyIndex + 1) % energies.size
            energyCount = min(energyCount + 1, energies.size)
            if (++measureFrames >= rate / 10) {
                measureFrames = 0
                val loudness = -.691 + 10 * log10(max(energySum / energyCount, 1e-12))
                // Gate silence so pauses do not turn room noise into a gain target.
                if (loudness > -60 && energyCount == energies.size) {
                    targetGain = 10.0.pow(((-14 - loudness).coerceIn(-24.0, 12.0)) / 20)
                }
            }
            gain += (targetGain - gain) * if (targetGain > gain) gainRise else gainFall
            val level = peak * gain
            envelope += (level - envelope) * if (level > envelope) attack else release
            val db = 20 * log10(max(envelope, 1e-9))
            // Speech compression: −18 dBFS threshold, 2.25:1 ratio and a 6 dB soft knee.
            val above = db + 18
            val slope = 1 - 1 / 2.25
            val reduction = when {
                above <= -3 -> 0.0
                above < 3 -> (above + 3).pow(2) * slope / 12
                else -> above * slope
            }
            val boost = gain * 10.0.pow((4 - reduction) / 20)
            val applied = 1 + mix * (boost - 1)
            peaks[slot] = (peak * applied).toFloat()
            for (channel in 0 until channels) audio[slot * channels + channel] *= applied.toFloat()
            while (tail > head && peaks[(deque[((tail - 1) % deque.size).toInt()] % peaks.size).toInt()] <= peaks[slot]) tail--
            deque[(tail++ % deque.size).toInt()] = write++
            if (write - read > delay) emit(output)
        }
        output.flip()
    }

    private fun emit(output: ByteBuffer) {
        val peak = peaks[(deque[(head % deque.size).toInt()] % peaks.size).toInt()]
        val ceiling = 1 - mix * (1 - .89125094)
        val target = if (peak > ceiling) ceiling / peak else 1.0
        limiter = min(target, limiter + (1 - limiter) * limiterRelease)
        val slot = (read % peaks.size).toInt()
        for (channel in 0 until channels) {
            val sample = audio[slot * channels + channel] * limiter
            output.putShort((sample * 32768).roundToInt().coerceIn(-32768, 32767).toShort())
        }
        read++
        while (tail > head && deque[(head % deque.size).toInt()] < read) head++
    }

    override fun onQueueEndOfStream() { ending = true }

    override fun getOutput(): ByteBuffer {
        val pending = super.getOutput()
        if (pending.hasRemaining() || !ending || write == read) return pending
        val output = replaceOutputBuffer(((write - read) * channels * 2).toInt())
        while (read < write) emit(output)
        output.flip()
        return super.getOutput()
    }

    override fun isEnded() = super.isEnded() && write == read

    override fun onReset() {
        audio = FloatArray(0)
        peaks = FloatArray(0)
        deque = LongArray(0)
        energies = DoubleArray(0)
        filters = emptyArray()
        write = 0
        read = 0
    }

    private class Filter(val b0: Double, val b1: Double, val b2: Double, val a1: Double, val a2: Double) {
        private var z1 = 0.0
        private var z2 = 0.0
        fun process(x: Double): Double {
            val y = b0 * x + z1
            z1 = b1 * x - a1 * y + z2
            z2 = b2 * x - a2 * y
            return y
        }
        companion object {
            // BS.1770 K-weighting filters, recalculated for the decoder sample rate.
            fun shelf(rate: Int): Filter {
                val k = tan(PI * 1681.974450955533 / rate)
                val vh = 10.0.pow(3.999843853973347 / 20)
                val vb = vh.pow(.4996667741545416)
                val q = .7071752369554196
                val a = 1 + k / q + k * k
                return Filter((vh + vb * k / q + k * k) / a, 2 * (k * k - vh) / a,
                    (vh - vb * k / q + k * k) / a, 2 * (k * k - 1) / a, (1 - k / q + k * k) / a)
            }
            fun highPass(rate: Int): Filter {
                val k = tan(PI * 38.13547087602444 / rate)
                val q = .5003270373238773
                val a = 1 + k / q + k * k
                return Filter(1.0, -2.0, 1.0, 2 * (k * k - 1) / a, (1 - k / q + k * k) / a)
            }
        }
    }
}
