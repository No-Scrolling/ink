package com.vandam.ink

import android.content.Context
import android.graphics.PixelFormat
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Choreographer
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.VelocityTracker
import android.view.View
import android.view.ViewConfiguration
import android.widget.OverScroller
import kotlin.math.abs

internal class InkHost(context: Context, private val callbacks: Callbacks, private val scale: Float = 0f) : SurfaceHolder.Callback, AutoCloseable {
    interface Callbacks {
        fun beforeFrame() {}
        fun changed() {}
        fun beforeRender() {}
        fun presented() {}
        fun haptic(view: View) {}
    }

    var handle = InkNative.nativeCreate(context.codeCacheDir.absolutePath)
        private set
    var surfaceAttached = false
        private set
    private val handler = Handler(Looper.getMainLooper())
    private val systemGlyphRasterizer = SystemGlyphRasterizer()
    val view = InkSurfaceView(context).apply {
        holder.setFormat(PixelFormat.OPAQUE)
        holder.addCallback(this@InkHost)
        setOnApplyWindowInsetsListener { _, insets ->
            val bounds = insets.displayCutout?.boundingRects.orEmpty()
                .flatMap { listOf(it.left, it.top, it.right, it.bottom) }.toIntArray()
            if (handle != 0L && InkNative.nativeSetDisplayCutouts(handle, bounds)) {
                requestFrame()
                callbacks.changed()
            }
            insets
        }
    }

    init { check(handle != 0L) { "Ink could not create the runtime" } }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (handle == 0L || surfaceAttached) return
        val frame = holder.surfaceFrame
        surfaceAttached = InkNative.nativeAttachSurface(handle, holder.surface, frame.width().coerceAtLeast(1), frame.height().coerceAtLeast(1))
        if (!surfaceAttached) { scheduleSurfaceRecovery(); return }
        if (scale > 0f) InkNative.nativeSetViewportScale(handle, scale)
        surfaceRetryDelay = 16L
        handler.removeCallbacks(recoverSurface)
        view.requestFrame()
        callbacks.changed()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (handle == 0L || !surfaceAttached) return
        InkNative.nativeResize(handle, width, height)
        if (scale > 0f) InkNative.nativeSetViewportScale(handle, scale)
        view.requestFrame()
        callbacks.changed()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) = detachSurface()

    private fun detachSurface() {
        handler.removeCallbacksAndMessages(null)
        view.stopScrolling()
        if (handle != 0L && surfaceAttached) InkNative.nativeDetachSurface(handle)
        surfaceAttached = false
    }

    override fun close() {
        detachSurface()
        view.release()
        view.holder.removeCallback(this)
        if (handle != 0L) InkNative.nativeDestroy(handle)
        handle = 0L
    }

    private val revealImages = Runnable {
        if (handle != 0L && surfaceAttached) view.requestFrame()
    }

    private var surfaceRetryDelay = 16L
    private val recoverSurface = Runnable {
        if (handle != 0L && view.holder.surface.isValid && !surfaceAttached) surfaceCreated(view.holder)
    }
    private fun scheduleSurfaceRecovery() {
        handler.removeCallbacks(recoverSurface)
        handler.postDelayed(recoverSurface, surfaceRetryDelay)
        surfaceRetryDelay = (surfaceRetryDelay * 2).coerceAtMost(250L)
    }
    private val retryRender = Runnable {
        if (handle != 0L && surfaceAttached) view.requestFrame()
    }

    private fun renderFrame() {
        handler.removeCallbacks(revealImages)
        val imageWait = InkNative.nativeImageWaitRemaining(handle)
        if (imageWait > 0) handler.postDelayed(revealImages, imageWait)
        val glyphStarted = System.nanoTime()
        var glyphs = 0
        while (true) {
            if (glyphs >= 4 || (glyphs > 0 && System.nanoTime() - glyphStarted >= 4_000_000L)) {
                handler.post(retryRender)
                return
            }
            val request = InkNative.nativeRender(handle)
            if (request == "presented") {
                callbacks.presented()
                return
            }
            if (request.isEmpty()) return
            if (request == "retry") {
                handler.removeCallbacks(retryRender)
                handler.postDelayed(retryRender, 16L)
                return
            }
            if (request == "surface-lost") {
                surfaceAttached = false
                scheduleSurfaceRecovery()
                return
            }

            val firstBreak = request.indexOf('\n')
            val secondBreak = request.indexOf('\n', firstBreak + 1)
            if (firstBreak < 1 || secondBreak <= firstBreak) {
                Log.e(RESOURCE_LOG_TAG, "Invalid system glyph request")
                return
            }
            val requestId = request.take(firstBreak).toLongOrNull()
            val size = request.substring(firstBreak + 1, secondBreak).toIntOrNull()
            if (requestId == null || size == null) {
                Log.e(RESOURCE_LOG_TAG, "Invalid system glyph request")
                return
            }
            val grapheme = request.substring(secondBreak + 1)
            if (!InkNative.nativeInstallSystemGlyph(
                handle,
                requestId,
                systemGlyphRasterizer.rasterise(grapheme, size) ?: IntArray(0),
            )) return
            glyphs++
        }
    }

    inner class InkSurfaceView(context: Context) : SurfaceView(context) {
        private val longPresses = mutableMapOf<Int, Runnable>()
        private val touchStarts = mutableMapOf<Int, Pair<Float, Float>>()

        override fun onDetachedFromWindow() {
            cancelLongPresses()
            removeCallbacks(commitFrame)
            commitFramePosted = false
            if (handle != 0L) InkNative.nativePointer(handle, MotionEvent.ACTION_CANCEL, 0, 0f, 0f)
            stopScrolling()
            super.onDetachedFromWindow()
        }

        private val choreographer = Choreographer.getInstance()
        private val viewConfiguration = ViewConfiguration.get(context)
        private val minimumFlingVelocity = viewConfiguration.scaledMinimumFlingVelocity
        private val maximumFlingVelocity = viewConfiguration.scaledMaximumFlingVelocity
        private val touchSlop = viewConfiguration.scaledTouchSlop
        private val doubleTapSlop = viewConfiguration.scaledDoubleTapSlop
        private val doubleTapTimeout = ViewConfiguration.getDoubleTapTimeout().toLong()
        private val scroller = OverScroller(context)
        private val scaleGestureDetector = ScaleGestureDetector(
            context,
            object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
                override fun onScaleBegin(detector: ScaleGestureDetector): Boolean {
                    if (!surfaceAttached || handle == 0L) return false
                    val result = InkNative.nativeImagePinchBegin(
                        handle,
                        detector.focusX,
                        detector.focusY,
                    )
                    pinchActive = (result and POINTER_CAPTURED) != 0
                    if (pinchActive) {
                        pendingMoves.clear()
                        InkNative.nativePointer(handle, MotionEvent.ACTION_CANCEL, 0, 0f, 0f)
                        capturedGesture = true
                        stopFling()
                    }
                    return pinchActive
                }

                override fun onScale(detector: ScaleGestureDetector): Boolean {
                    if (!pinchActive) return false
                    val changed = processPointerResult(
                        InkNative.nativeImagePinchUpdate(
                            handle,
                            detector.scaleFactor,
                            detector.focusX,
                            detector.focusY,
                        ),
                    )
                    if (changed) requestFrame()
                    return true
                }

                override fun onScaleEnd(detector: ScaleGestureDetector) {
                    if (pinchActive) InkNative.nativeImagePinchEnd(handle)
                    pinchActive = false
                }
            },
        ).apply {
            isQuickScaleEnabled = false
            isStylusScaleEnabled = false
        }
        private var downY = 0f
        private var framePosted = false
        private var frameDelayed = false
        private var renderPending = false
        private var commitFramePosted = false
        private var touchActive = false
        private val commitFrame = Runnable {
            commitFramePosted = false
            if (handle != 0L && surfaceAttached && renderPending) {
                if (canPresentCommit() && InkNative.nativeFrameReady(handle)) {
                    if (framePosted) {
                        choreographer.removeFrameCallback(frameCallback)
                        framePosted = false
                    }
                    renderPending = false
                    presentFrame()
                } else {
                    postFrame()
                }
            }
        }
        private val pendingMoves = mutableMapOf<Int, Pair<Float, Float>>()
        private val hasPendingMove: Boolean get() = pendingMoves.isNotEmpty()
        private var lastFlingY = 0
        private var waitingFlingVelocity = 0
        private var capturedGesture = false
        private var pinchActive = false
        private var imageTapDownX = 0f
        private var imageTapDownY = 0f
        private var imageTapTarget = 0L
        private var previousImageTapTime = 0L
        private var previousImageTapTarget = 0L
        private var previousImageTapX = 0f
        private var previousImageTapY = 0f
        private var velocityTracker: VelocityTracker? = null
        private val frameCallback = Choreographer.FrameCallback {
            if (BuildConfig.INK_PRESENTATION_TIMING) Log.i("Ink", "FrameCallback ns=${System.nanoTime()}")
            callbacks.beforeFrame()
            framePosted = false
            if (!surfaceAttached || handle == 0L) {
                return@FrameCallback
            }
            // A second callback can arrive before the GPU finishes the previous frame.
            // Keep pending work for the next vsync instead of blocking input dispatch.
            if (!InkNative.nativeFrameReady(handle)) {
                postFrame()
                return@FrameCallback
            }

            // Use the latest movement when its rows are mounted. Otherwise show
            // the completed window before moving beyond it and requesting more rows.
            val pending = renderPending
            renderPending = false
            val presentBeforeMove = pending && hasPendingMove && pendingMoves.any { (id, position) ->
                !InkNative.nativeCanPresentPointerMove(handle, id, position.second)
            }
            if (presentBeforeMove) presentFrame()
            var changed = InkNative.nativeAnimateScene(handle) || (pending && !presentBeforeMove)
            if (hasPendingMove) changed = flushPointerMoves() || changed
            if (pending && waitingFlingVelocity != 0) {
                when (InkNative.nativeScrollBy(handle, 1f)) {
                    SCROLL_CHANGED -> {
                        startFling(waitingFlingVelocity)
                        changed = true
                    }
                    0 -> waitingFlingVelocity = 0
                }
            }
            if (scroller.computeScrollOffset()) {
                val y = scroller.currY
                val delta = y - lastFlingY
                lastFlingY = y
                if (delta != 0) {
                    when (InkNative.nativeScrollBy(handle, delta.toFloat())) {
                        SCROLL_CHANGED -> changed = true
                        SCROLL_PENDING -> {
                            waitingFlingVelocity = scroller.currVelocity.toInt()
                            scroller.forceFinished(true)
                        }
                        else -> scroller.abortAnimation()
                    }
                }
            }
            if (changed) {
                if (presentBeforeMove) renderPending = true else presentFrame()
            }
            postFrame()
        }

        override fun onTouchEvent(event: MotionEvent): Boolean {
            val action = event.actionMasked
            val index = event.actionIndex
            val id = event.getPointerId(index)
            val x = event.getX(index)
            val y = event.getY(index)
            when (action) {
                MotionEvent.ACTION_DOWN -> touchActive = true
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> touchActive = false
            }
            val wasPinching = pinchActive
            scaleGestureDetector.onTouchEvent(event)
            if (event.pointerCount > 1) {
                resetImageTap()
                velocityTracker?.recycle()
                velocityTracker = null
            }
            if (wasPinching || pinchActive) {
                cancelLongPresses()
                pendingMoves.clear()
                if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) capturedGesture = false
                postFrame()
                return true
            }
            var flingVelocity = 0
            var imageDoubleTap = 0
            when (action) {
                MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                    if (action == MotionEvent.ACTION_DOWN) {
                        cancelLongPresses()
                        stopFling()
                        downY = y
                        imageTapDownX = x
                        imageTapDownY = y
                        imageTapTarget = if (handle != 0L && surfaceAttached) {
                            InkNative.nativeImageZoomTarget(handle, x, y)
                        } else {
                            0L
                        }
                        if (imageTapTarget == 0L) resetImageTap()
                        velocityTracker?.recycle()
                        velocityTracker = VelocityTracker.obtain().also { it.addMovement(event) }
                    }
                    touchStarts[id] = x to y
                    val longPress = Runnable {
                        if (handle != 0L && surfaceAttached) {
                            if (processPointerResult(InkNative.nativePointer(handle, POINTER_LONG_PRESS, id, x, y))) requestFrame()
                            postFrame()
                        }
                    }
                    longPresses[id] = longPress
                    postDelayed(longPress, ViewConfiguration.getLongPressTimeout().toLong())
                }
                MotionEvent.ACTION_MOVE -> {
                    velocityTracker?.addMovement(event)
                    for (pointerIndex in 0 until event.pointerCount) {
                        val pointerId = event.getPointerId(pointerIndex)
                        val start = touchStarts[pointerId] ?: continue
                        if (movedBeyond(event.getX(pointerIndex), event.getY(pointerIndex), start.first, start.second, touchSlop)) {
                            longPresses.remove(pointerId)?.let(::removeCallbacks)
                            resetImageTap()
                        }
                    }
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                    longPresses.remove(id)?.let(::removeCallbacks)
                    touchStarts.remove(id)
                    if (action == MotionEvent.ACTION_UP) {
                        if (imageTapTarget != 0L) {
                            val isSecondTap = previousImageTapTime != 0L &&
                                previousImageTapTarget == imageTapTarget &&
                                event.eventTime - previousImageTapTime <= doubleTapTimeout &&
                                !movedBeyond(x, y, previousImageTapX, previousImageTapY, doubleTapSlop)
                            if (isSecondTap) {
                                imageDoubleTap = InkNative.nativeImageDoubleTap(handle, x, y)
                                resetImageTap()
                            } else {
                                previousImageTapTime = event.eventTime
                                previousImageTapTarget = imageTapTarget
                                previousImageTapX = x
                                previousImageTapY = y
                                imageTapTarget = 0L
                            }
                        } else {
                            resetImageTap()
                        }
                        velocityTracker?.apply {
                            addMovement(event)
                            computeCurrentVelocity(1000, maximumFlingVelocity.toFloat())
                            if (abs(y - downY) > touchSlop && abs(yVelocity) >= minimumFlingVelocity) {
                                flingVelocity = -yVelocity.toInt()
                            }
                            recycle()
                        }
                        velocityTracker = null
                    }
                }
                MotionEvent.ACTION_CANCEL -> {
                    cancelLongPresses()
                    resetImageTap()
                    velocityTracker?.recycle()
                    velocityTracker = null
                }
            }
            if (handle != 0L && surfaceAttached) {
                val changed = if ((imageDoubleTap and POINTER_CAPTURED) != 0) {
                    pendingMoves.clear()
                    processPointerResult(imageDoubleTap)
                } else if (action == MotionEvent.ACTION_MOVE) {
                    for (pointerIndex in 0 until event.pointerCount) {
                        pendingMoves[event.getPointerId(pointerIndex)] = event.getX(pointerIndex) to event.getY(pointerIndex)
                    }
                    false
                } else {
                    val moved = flushPointerMoves()
                    val nativeAction = when (action) {
                        MotionEvent.ACTION_POINTER_DOWN -> MotionEvent.ACTION_DOWN
                        MotionEvent.ACTION_POINTER_UP -> MotionEvent.ACTION_UP
                        else -> action
                    }
                    processPointerResult(InkNative.nativePointer(handle, nativeAction, id, x, y)) || moved
                }
                if (changed) requestFrame()
                postFrame()
            }
            if (flingVelocity != 0 && !capturedGesture) startFling(flingVelocity)
            if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_CANCEL) {
                capturedGesture = false
            }
            if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP) performClick()
            return true
        }

        private fun flushPointerMoves(): Boolean {
            var changed = false
            val moves = pendingMoves.toList()
            pendingMoves.clear()
            for ((id, position) in moves) {
                changed = processPointerResult(InkNative.nativePointer(
                    handle, MotionEvent.ACTION_MOVE, id, position.first, position.second,
                )) || changed
            }
            return changed
        }

        private fun cancelLongPresses() {
            longPresses.values.forEach(::removeCallbacks)
            longPresses.clear()
            touchStarts.clear()
        }

        private fun movedBeyond(
            x: Float,
            y: Float,
            originX: Float,
            originY: Float,
            slop: Int,
        ): Boolean {
            val deltaX = x - originX
            val deltaY = y - originY
            return deltaX * deltaX + deltaY * deltaY > slop * slop
        }

        private fun resetImageTap() {
            imageTapTarget = 0L
            previousImageTapTime = 0L
            previousImageTapTarget = 0L
        }

        private fun processPointerResult(result: Int): Boolean {
            capturedGesture = capturedGesture || (result and POINTER_CAPTURED) != 0
            if ((result and POINTER_HAPTIC) != 0) {
                callbacks.haptic(this)
            }
            if ((result and POINTER_ACTIVATED) != 0 ||
                ((result and POINTER_CHANGED) != 0 && !capturedGesture)) {
                callbacks.changed()
            }
            return (result and POINTER_CHANGED) != 0
        }

        fun requestFrame() {
            if (Looper.myLooper() != Looper.getMainLooper()) {
                post(::requestFrame)
                return
            }
            renderPending = true
            postFrame()
        }

        fun requestCommitFrame() {
            if (BuildConfig.INK_PRESENTATION_TIMING) {
                Log.i("Ink", "CommitRequested ns=${System.nanoTime()} frame_posted=${if (framePosted) 1 else 0} touch_active=${if (touchActive) 1 else 0}")
            }
            renderPending = true
            if (commitFramePosted) return
            // A completed commit can replace the queued frame once the GPU is ready.
            if (surfaceAttached && canPresentCommit()) {
                commitFramePosted = true
                post(commitFrame)
            } else {
                postFrame()
            }
        }

        private fun canPresentCommit(): Boolean =
            !touchActive && !pinchActive && !hasPendingMove && scroller.isFinished && waitingFlingVelocity == 0 &&
                !InkNative.nativeHasSceneAnimations(handle)

        private fun presentFrame() {
            // Native-only scrolling can queue image loads without a touch or React commit.
            callbacks.beforeRender()
            if (BuildConfig.INK_PRESENTATION_TIMING) Log.i("Ink", "FrameStart ns=${System.nanoTime()}")
            renderFrame()
            postFrame()
        }

        fun stopScrolling() {
            stopFling()
            velocityTracker?.recycle()
            velocityTracker = null
        }

        fun release() {
            cancelLongPresses()
            stopScrolling()
            removeCallbacks(commitFrame)
            commitFramePosted = false
            renderPending = false
        }

        private fun startFling(velocityY: Int) {
            // Prepending rows shifts content coordinates; clamp travel against the live layout.
            lastFlingY = 0
            waitingFlingVelocity = 0
            scroller.fling(
                0,
                0,
                0,
                velocityY,
                0,
                0,
                Int.MIN_VALUE,
                Int.MAX_VALUE,
            )
            postFrame()
        }

        private fun postFrame() {
            val delay = when {
                renderPending || hasPendingMove || !scroller.isFinished -> 0L
                surfaceAttached && handle != 0L -> InkNative.nativeNextSceneFrame(handle)
                else -> -1L
            }
            if (delay < 0 || (framePosted && (!frameDelayed || delay > 0))) return
            // Slow animations such as playback progress wait for their next visible step,
            // but urgent work replaces that wait with the next vsync.
            if (framePosted) choreographer.removeFrameCallback(frameCallback)
            framePosted = true
            frameDelayed = delay > 0
            if (frameDelayed) {
                choreographer.postFrameCallbackDelayed(frameCallback, delay)
            } else {
                choreographer.postFrameCallback(frameCallback)
            }
        }

        private fun stopFling() {
            waitingFlingVelocity = 0
            scroller.abortAnimation()
            pendingMoves.clear()
            if (framePosted) {
                choreographer.removeFrameCallback(frameCallback)
                framePosted = false
            }
        }

        override fun performClick(): Boolean {
            super.performClick()
            return true
        }
    }


    private companion object {
        const val SCROLL_CHANGED = 1
        const val SCROLL_PENDING = 2
        const val POINTER_LONG_PRESS = 4
        const val POINTER_CHANGED = 1
        const val POINTER_ACTIVATED = 1 shl 1
        const val POINTER_CAPTURED = 1 shl 2
        const val POINTER_HAPTIC = 1 shl 3
        const val RESOURCE_LOG_TAG = "InkResource"
    }
}
