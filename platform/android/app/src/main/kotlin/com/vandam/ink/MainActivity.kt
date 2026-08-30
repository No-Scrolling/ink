package com.vandam.ink

import android.app.Activity
import android.graphics.Color
import android.graphics.PixelFormat
import android.os.Bundle
import android.view.Choreographer
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.VelocityTracker
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.widget.OverScroller
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import kotlin.math.abs

class MainActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var inkView: InkSurfaceView
    private var engineHandle = 0L
    private var surfaceAttached = false
    private val backCallback = OnBackInvokedCallback {
        handleBack()
    }

    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        handleBack()
    }

    private fun handleBack() {
        inkView.stopScrolling()
        if (engineHandle == 0L || !nativeBack(engineHandle)) {
            finish()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen().setOnExitAnimationListener { splashScreenView ->
            splashScreenView.remove()
        }
        super.onCreate(savedInstanceState)

        window.setWindowAnimations(0)
        window.setDecorFitsSystemWindows(false)
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK

        engineHandle = nativeCreate()
        inkView = InkSurfaceView().apply {
            holder.setFormat(PixelFormat.RGBA_8888)
            holder.addCallback(this@MainActivity)
        }

        setContentView(
            inkView,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        onBackInvokedDispatcher.registerOnBackInvokedCallback(
            OnBackInvokedDispatcher.PRIORITY_DEFAULT,
            backCallback,
        )
        enterFullscreen()
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            enterFullscreen()
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        if (engineHandle == 0L || surfaceAttached) {
            return
        }

        val frame = holder.surfaceFrame
        nativeAttachSurface(
            engineHandle,
            holder.surface,
            frame.width().coerceAtLeast(1),
            frame.height().coerceAtLeast(1),
        )
        surfaceAttached = true
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (engineHandle == 0L || !surfaceAttached) {
            return
        }

        nativeResize(engineHandle, width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        detachSurface()
    }

    override fun onDestroy() {
        onBackInvokedDispatcher.unregisterOnBackInvokedCallback(backCallback)
        detachSurface()
        if (engineHandle != 0L) {
            nativeDestroy(engineHandle)
            engineHandle = 0L
        }
        super.onDestroy()
    }

    private fun detachSurface() {
        inkView.stopScrolling()
        if (engineHandle != 0L && surfaceAttached) {
            nativeDetachSurface(engineHandle)
            surfaceAttached = false
        }
    }

    private fun enterFullscreen() {
        window.insetsController?.apply {
            hide(WindowInsets.Type.systemBars())
            systemBarsBehavior =
                WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }

    private inner class InkSurfaceView : SurfaceView(this@MainActivity) {
        private val choreographer = Choreographer.getInstance()
        private val minimumFlingVelocity =
            ViewConfiguration.get(this@MainActivity).scaledMinimumFlingVelocity
        private val maximumFlingVelocity =
            ViewConfiguration.get(this@MainActivity).scaledMaximumFlingVelocity
        private val touchSlop = ViewConfiguration.get(this@MainActivity).scaledTouchSlop
        private val scroller = OverScroller(this@MainActivity)
        private var downY = 0f
        private var framePosted = false
        private var lastFlingY = 0
        private var velocityTracker: VelocityTracker? = null
        private val frameCallback = Choreographer.FrameCallback {
            framePosted = false
            if (!surfaceAttached || engineHandle == 0L || !scroller.computeScrollOffset()) {
                return@FrameCallback
            }

            val y = scroller.currY
            val delta = y - lastFlingY
            lastFlingY = y
            if (delta != 0 && !nativeScrollBy(engineHandle, delta.toFloat())) {
                scroller.abortAnimation()
                return@FrameCallback
            }
            postFlingFrame()
        }

        override fun onTouchEvent(event: MotionEvent): Boolean {
            var flingVelocity = 0
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    stopFling()
                    downY = event.y
                    velocityTracker?.recycle()
                    velocityTracker = VelocityTracker.obtain().also { it.addMovement(event) }
                }
                MotionEvent.ACTION_MOVE -> velocityTracker?.addMovement(event)
                MotionEvent.ACTION_UP -> {
                    velocityTracker?.apply {
                        addMovement(event)
                        computeCurrentVelocity(1000, maximumFlingVelocity.toFloat())
                        if (
                            abs(event.y - downY) > touchSlop &&
                            abs(yVelocity) >= minimumFlingVelocity
                        ) {
                            flingVelocity = -yVelocity.toInt()
                        }
                        recycle()
                    }
                    velocityTracker = null
                }
                MotionEvent.ACTION_CANCEL -> {
                    velocityTracker?.recycle()
                    velocityTracker = null
                }
            }
            if (engineHandle != 0L && surfaceAttached) {
                nativePointer(engineHandle, event.actionMasked, event.x, event.y)
            }
            if (flingVelocity != 0) {
                startFling(flingVelocity)
            }
            if (event.actionMasked == MotionEvent.ACTION_UP) {
                performClick()
            }
            return true
        }

        fun stopScrolling() {
            stopFling()
            velocityTracker?.recycle()
            velocityTracker = null
        }

        private fun startFling(velocityY: Int) {
            lastFlingY = 0
            scroller.fling(0, 0, 0, velocityY, 0, 0, Int.MIN_VALUE, Int.MAX_VALUE)
            postFlingFrame()
        }

        private fun postFlingFrame() {
            if (!framePosted && !scroller.isFinished) {
                framePosted = true
                choreographer.postFrameCallback(frameCallback)
            }
        }

        private fun stopFling() {
            scroller.abortAnimation()
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
        init {
            System.loadLibrary("ink_android")
        }

        @JvmStatic
        private external fun nativeCreate(): Long

        @JvmStatic
        private external fun nativeAttachSurface(
            handle: Long,
            surface: Surface,
            width: Int,
            height: Int,
        )

        @JvmStatic
        private external fun nativeResize(handle: Long, width: Int, height: Int)

        @JvmStatic
        private external fun nativePointer(
            handle: Long,
            action: Int,
            x: Float,
            y: Float,
        ): Boolean

        @JvmStatic
        private external fun nativeScrollBy(handle: Long, delta: Float): Boolean

        @JvmStatic
        private external fun nativeBack(handle: Long): Boolean

        @JvmStatic
        private external fun nativeDetachSurface(handle: Long)

        @JvmStatic
        private external fun nativeDestroy(handle: Long)
    }
}
