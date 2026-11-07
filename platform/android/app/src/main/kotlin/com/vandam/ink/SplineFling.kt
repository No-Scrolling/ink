package com.vandam.ink

import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.ln

/** OverScroller's unbounded spline fling, sampled in the frame's monotonic time domain. */
internal class SplineFling(density: Float, private val friction: Float) {
    private val physicalCoefficient = 9.80665f * 39.37f * (density * 160f) * 0.84f
    private var startedNanos = 0L
    var durationNanos = 0L
        private set
    var distance = 0
        private set
    var position = 0f
        private set
    var velocity = 0f
        private set
    var isFinished = true
        private set

    fun start(velocityY: Int, timeNanos: Long) {
        startedNanos = timeNanos
        position = 0f
        velocity = abs(velocityY.toFloat())
        val deceleration = ln((0.35f * velocity / (friction * physicalCoefficient)).toDouble())
        // Keep Android's millisecond duration and integer final distance; only sampling changes.
        durationNanos = (1000.0 * exp(deceleration / (DECELERATION_RATE - 1.0))).toInt() * 1_000_000L
        distance = (friction * physicalCoefficient *
            exp(DECELERATION_RATE / (DECELERATION_RATE - 1.0) * deceleration) *
            if (velocityY < 0) -1 else 1).toInt()
        isFinished = false
    }

    fun advance(timeNanos: Long): Boolean {
        if (isFinished) return false
        val elapsed = (timeNanos - startedNanos).coerceAtLeast(0L)
        if (elapsed == 0L && durationNanos > 0L) return true
        if (elapsed >= durationNanos) {
            position = distance.toFloat()
            velocity = 0f
            isFinished = true
            return true
        }
        val time = elapsed.toDouble() / durationNanos
        // Android tabulates this Bezier at 100 points. Invert its time coordinate
        // directly to avoid table-slope changes as well as millisecond clock steps.
        var low = 0.0
        var high = 1.0
        repeat(28) {
            val u = (low + high) / 2.0
            val x = 3.0 * u * (1.0 - u) * ((1.0 - u) * 0.175 + u * 0.35) + u * u * u
            if (x < time) low = u else high = u
        }
        val u = (low + high) / 2.0
        val y = 1.5 * u - 0.5 * u * u * u
        val dy = 1.5 * (1.0 - u * u)
        val dx = 0.525 + 1.425 * u * u
        position = (distance * y).toFloat()
        velocity = (abs(distance.toDouble()) * dy / dx * 1_000_000_000.0 / durationNanos).toFloat()
        return true
    }

    fun stop() {
        isFinished = true
    }

    private companion object {
        val DECELERATION_RATE = (ln(0.78) / ln(0.9)).toFloat()
    }
}
