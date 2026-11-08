package com.vandam.ink

import android.content.Context
import android.os.VibrationAttributes
import android.os.VibrationEffect
import android.os.VibratorManager
import org.json.JSONObject
import kotlin.math.roundToInt

internal fun createVibrationAdapter(context: Context, hapticsEnabled: () -> Boolean): VibrationAdapter =
    InkVibrationAdapter(context, hapticsEnabled)

private class InkVibrationAdapter(context: Context, private val hapticsEnabled: () -> Boolean) : VibrationAdapter {
    private val vibrator = context.applicationContext.getSystemService(VibratorManager::class.java)
        ?.defaultVibrator?.takeIf { it.hasVibrator() }

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val result = try {
            when (operation) {
                "vibrate" -> vibrate(JSONObject(payload))
                "stop" -> stop()
                else -> error("Unknown vibration operation: $operation")
            }
            NativeResult.Success("")
        } catch (error: Exception) {
            NativeResult.Failure(NativeErrorKind.PROTOCOL, error.message ?: "Invalid vibration", false)
        }
        complete(result)
    }

    private fun vibrate(body: JSONObject) {
        val alarm = body.optBoolean("alarm", false)
        if (!alarm && !hapticsEnabled()) return
        val vibrator = vibrator ?: return
        val segments = body.getJSONArray("segments")
        val durations = LongArray(segments.length()) { index -> segments.getJSONObject(index).getLong("ms") }
        val amplitudes = IntArray(segments.length()) { index ->
            val segment = segments.getJSONObject(index)
            if (segment.has("strength")) (segment.getDouble("strength") * 255).roundToInt()
            else VibrationEffect.DEFAULT_AMPLITUDE
        }
        if (durations.all { it == 0L } || amplitudes.all { it == 0 }) return
        val effect = when {
            durations.size == 1 && amplitudes[0] == VibrationEffect.DEFAULT_AMPLITUDE ->
                VibrationEffect.createOneShot(durations[0], VibrationEffect.DEFAULT_AMPLITUDE)
            vibrator.hasAmplitudeControl() -> VibrationEffect.createWaveform(durations, amplitudes, -1)
            else -> {
                // Match LightVibrator: each segment is on for its amplitude fraction, then off.
                val timings = mutableListOf(0L)
                durations.forEachIndexed { index, duration ->
                    val on = duration * amplitudes[index] / 255
                    timings += on
                    timings += duration - on
                }
                VibrationEffect.createWaveform(timings.toLongArray(), -1)
            }
        }
        vibrator.vibrate(effect, VibrationAttributes.createForUsage(
            if (alarm) VibrationAttributes.USAGE_ALARM else VibrationAttributes.USAGE_TOUCH,
        ))
    }

    override fun cancel(requestId: Long) = Unit
    override fun stop() { vibrator?.cancel() }
}
