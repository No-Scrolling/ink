package com.vandam.ink

import android.content.Context
import android.content.res.Configuration
import android.widget.OverScroller
import kotlin.math.abs
import kotlin.math.roundToLong

internal class SplineFlingContracts(private val context: Context) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("fling retains Android distance, duration and spline shape", ::referenceCurve)
        test("fling decays smoothly at 60, 90 and 120 Hz", ::frameSteps)
        test("fling stops, restarts and delivers its final sub-pixel step once", ::lifetime)
    }

    private fun near(expected: Double, actual: Float, tolerance: Double = 0.001) {
        check(abs(expected - actual) <= tolerance) { "Expected $expected ± $tolerance, got $actual" }
    }

    private fun referenceCurve() {
        val configured = context.createConfigurationContext(Configuration(context.resources.configuration).apply {
            densityDpi = 480
        })
        val android = OverScroller(configured)
        val fling = SplineFling(3f, 0.015f)
        for (speed in listOf(1000, 12000, -12000)) {
            android.forceFinished(true)
            android.fling(0, 0, 0, speed, 0, 0, Int.MIN_VALUE, Int.MAX_VALUE)
            fling.start(speed, 0L)
            check(fling.distance == android.finalY)
            // Android 14 reference values at density 3 and the default 0.015 friction.
            check(fling.durationNanos == if (speed == 1000) 247_000_000L else 1_540_000_000L)
            check(fling.distance == when (speed) { 1000 -> 86; 12000 -> 6470; else -> -6470 })
        }
        fling.start(12000, 0L)
        // Independent points on the cubic: parameters 1/4, 1/2 and 3/4 give
        // these exact (time, distance) fractions, without solving the production inverse.
        for ((time, position, speed) in listOf(
            Triple(0.138671875, 2375.703125, 9621.294735799),
            Triple(0.321875, 4448.125, 5363.360044211),
            Triple(0.594140625, 5913.984375, 2078.380982975),
        )) {
            fling.advance((time * 1_540_000_000L).roundToLong())
            near(position, fling.position)
            near(speed, fling.velocity)
        }
    }

    private fun frameSteps() {
        for (hz in listOf(60, 90, 120)) {
            val fling = SplineFling(3f, 0.015f)
            val origin = 9_000_000_123L
            fling.start(12000, origin)
            var previousPosition = 0f
            var previousStep = Float.POSITIVE_INFINITY
            for (frame in 1..(hz * 1.54).toInt()) {
                check(fling.advance(origin + (frame * 1_000_000_000.0 / hz).roundToLong()))
                val step = fling.position - previousPosition
                check(step > 0f)
                // Float position subtraction can contribute up to 0.001 px near the end.
                check(step <= previousStep + 0.001f) { "$hz Hz frame $frame: $previousStep -> $step" }
                previousStep = step
                previousPosition = fling.position
            }
            fling.advance(origin + 1_540_000_000L)
            near(6470.0, fling.position, 0.0)
            check(fling.isFinished)
        }
    }

    private fun lifetime() {
        val fling = SplineFling(3f, 0.015f)
        check(!fling.advance(0L))
        fling.start(-12000, 100L)
        fling.advance(99L)
        near(0.0, fling.position, 0.0)
        fling.advance(495_687_600L)
        near(-4448.125, fling.position)
        check(fling.velocity > 0f) // OverScroller.currVelocity is a magnitude, including row-mount waits.
        fling.stop()
        check(!fling.advance(600_000_000L))
        fling.start(12000, 1_000_000_000L)
        near(0.0, fling.position, 0.0)
        check(fling.advance(2_539_999_999L))
        check(!fling.isFinished)
        check(fling.advance(2_540_000_000L))
        near(6470.0, fling.position, 0.0)
        check(fling.isFinished && !fling.advance(3_000_000_000L))
        fling.start(0, 0L)
        check(fling.advance(0L) && fling.isFinished)
        near(0.0, fling.position, 0.0)
    }
}
