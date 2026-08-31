package com.vandam.ink

import android.app.Activity
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.Typeface
import android.graphics.fonts.Font
import android.graphics.fonts.FontFamily
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Log
import android.view.Choreographer
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.VelocityTracker
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.widget.FrameLayout
import android.widget.OverScroller
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import java.io.File
import java.nio.ByteBuffer
import java.util.concurrent.Executors
import kotlin.math.abs
import kotlin.math.roundToInt
import org.json.JSONObject

class MainActivity : Activity(), SurfaceHolder.Callback {
    internal val publicSansTypeface: Typeface by lazy(LazyThreadSafetyMode.NONE) {
        Typeface.CustomFallbackBuilder(
            FontFamily.Builder(Font.Builder(nativePublicSans()).build()).build(),
        ).build()
    }
    private lateinit var inkView: InkSurfaceView
    private lateinit var root: FrameLayout
    private lateinit var lightSdkAdapter: LightSdkAdapter
    private lateinit var networkAdapter: NetworkAdapter
    private lateinit var audioAdapter: AudioAdapter
    private lateinit var locationAdapter: LocationAdapter
    private lateinit var nfcAdapter: NfcAdapter
    private lateinit var backgroundAdapter: BackgroundAdapter
    private lateinit var cameraAdapter: CameraAdapter
    private lateinit var textInputAdapter: TextInputAdapter
    private lateinit var notificationsAdapter: NotificationsAdapter
    private var engineHandle = 0L
    private var surfaceAttached = false
    private var resumedOnce = false
    private val usesPersistence = nativeUsesPersistence()
    private val persistenceHandler = Handler(Looper.getMainLooper())
    private val nativeRequestHandler = Handler(Looper.getMainLooper())
    private val nativeTimeouts = mutableMapOf<Long, Runnable>()
    private val nativeRequestStartedAt = mutableMapOf<Long, Long>()
    private val nativeRequestLabels = mutableMapOf<Long, String>()
    private val persistenceExecutor by lazy(LazyThreadSafetyMode.NONE) {
        Executors.newSingleThreadExecutor()
    }
    private val persistState = Runnable(::persistAsync)
    private val backCallback = OnBackInvokedCallback {
        handleBack()
    }

    @Suppress("DEPRECATION")
    override fun onBackPressed() {
        handleBack()
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        onUserInteraction()
        if (window.superDispatchKeyEvent(event)) return true
        if (lightSdkAdapter.forwardDeviceKey(event)) return true
        return event.dispatch(this, window.decorView.keyDispatcherState, this)
    }

    private fun handleBack() {
        inkView.stopScrolling()
        if (textInputAdapter.dismiss()) {
            return
        }
        if (engineHandle == 0L || !nativeBack(engineHandle)) {
            finish()
        } else {
            syncCameraPortal()
            drainNativeRequests()
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

        engineHandle = nativeCreate(File(noBackupFilesDir, "ink-state-v1").absolutePath)
        inkView = InkSurfaceView().apply {
            holder.setFormat(PixelFormat.RGBA_8888)
            holder.addCallback(this@MainActivity)
        }
        root = FrameLayout(this).apply {
            addView(
                inkView,
                FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
        }
        textInputAdapter = createTextInputAdapter(this, root, ::handleTextEdit)
        lightSdkAdapter = createLightSdkAdapter(
            this,
            textInputAdapter::applyPreferences,
        ) { controller, value ->
            if (engineHandle != 0L) {
                nativeUpdateController(engineHandle, controller, value)
            }
        }
        networkAdapter = createNetworkAdapter(this)
        locationAdapter = createLocationAdapter(this)
        nfcAdapter = createNfcAdapter(this)
        backgroundAdapter = createBackgroundAdapter(this)
        cameraAdapter = createCameraAdapter(
            this,
            root,
            { controller, value ->
                if (
                    engineHandle != 0L &&
                    nativeUpdateController(engineHandle, controller, value)
                ) {
                    syncCameraPortal()
                    drainNativeRequests()
                }
            },
            { controller, source ->
                if (
                    engineHandle != 0L &&
                    nativeSetCameraReview(engineHandle, controller, source.orEmpty())
                ) {
                    syncCameraPortal()
                    drainNativeRequests()
                }
            },
            { controller ->
                openCameraController(
                    automaticCameraRequestId(controller),
                    controller,
                    "{}",
                ) {}
            },
        )
        audioAdapter = createAudioAdapter(
            this,
            { samples, sampleRate ->
                runOnUiThread {
                    if (engineHandle != 0L) {
                        nativeAudioSamples(engineHandle, samples, sampleRate)
                    }
                }
            },
            { controller, value ->
                if (engineHandle != 0L) {
                    nativeUpdateController(engineHandle, controller, value)
                }
            },
            { controller, kind, config ->
                engineHandle != 0L &&
                    nativeAudioActivate(engineHandle, controller, kind, config)
            },
            { controller ->
                if (engineHandle != 0L) {
                    nativeAudioDeactivate(engineHandle, controller)
                }
            },
            { controller, enabled ->
                engineHandle != 0L &&
                    nativeAudioSetEnabled(engineHandle, controller, enabled)
            },
        )
        notificationsAdapter = createNotificationsAdapter(this) { controller, value ->
            if (engineHandle != 0L) {
                nativeUpdateController(engineHandle, controller, value)
            }
        }
        lightSdkAdapter.start()
        backgroundAdapter.reconcile()
        notificationsAdapter.start()
        drainNativeRequests()
        setContentView(
            root,
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
        handleNotificationIntent(intent)
    }

    override fun onNewIntent(intent: android.content.Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleNotificationIntent(intent)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            enterFullscreen()
        }
    }

    override fun onResume() {
        super.onResume()
        lightSdkAdapter.refresh()
        backgroundAdapter.reconcile()
        if (resumedOnce && engineHandle != 0L && nativeResume(engineHandle)) {
            syncCameraPortal()
            drainNativeRequests()
        }
        cameraAdapter.resume()
        syncCameraPortal()
        resumedOnce = true
        nfcAdapter.resume()
        notificationsAdapter.refreshEvents()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (engineHandle != 0L && nativeResume(engineHandle)) {
            syncCameraPortal()
            drainNativeRequests()
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
        syncCameraPortal()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (engineHandle == 0L || !surfaceAttached) {
            return
        }

        nativeResize(engineHandle, width, height)
        syncCameraPortal()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        detachSurface()
    }

    override fun onDestroy() {
        if (usesPersistence) {
            persistNow()
            persistenceExecutor.shutdown()
        }
        onBackInvokedDispatcher.unregisterOnBackInvokedCallback(backCallback)
        lightSdkAdapter.stop()
        notificationsAdapter.stop()
        networkAdapter.stop()
        audioAdapter.stop()
        locationAdapter.stop()
        nfcAdapter.stop()
        backgroundAdapter.stop()
        cameraAdapter.stop()
        nativeTimeouts.values.forEach(nativeRequestHandler::removeCallbacks)
        nativeTimeouts.clear()
        nativeRequestStartedAt.clear()
        nativeRequestLabels.clear()
        detachSurface()
        if (engineHandle != 0L) {
            nativeDestroy(engineHandle)
            engineHandle = 0L
        }
        super.onDestroy()
    }

    override fun onPause() {
        persistNow()
        audioAdapter.pause()
        nfcAdapter.pause()
        cameraAdapter.pause()
        super.onPause()
    }

    private fun schedulePersistence() {
        if (!usesPersistence) {
            return
        }
        persistenceHandler.removeCallbacks(persistState)
        persistenceHandler.postDelayed(persistState, PERSISTENCE_DELAY_MS)
    }

    private fun persistNow() {
        if (!usesPersistence) {
            return
        }
        persistenceHandler.removeCallbacks(persistState)
        if (engineHandle != 0L) {
            val handle = engineHandle
            persistenceExecutor.submit { nativePersist(handle) }.get()
        }
    }

    private fun persistAsync() {
        if (usesPersistence && engineHandle != 0L) {
            val handle = engineHandle
            persistenceExecutor.execute { nativePersist(handle) }
        }
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

    private fun handleTextEdit(edit: TextEdit) {
        if (engineHandle == 0L) {
            return
        }
        val (action, value) = when (edit) {
            is TextEdit.Insert -> TEXT_INPUT_INSERT to edit.text
            TextEdit.Backspace -> TEXT_INPUT_BACKSPACE to null
            TextEdit.Submit -> TEXT_INPUT_SUBMIT to null
            TextEdit.Dismiss -> TEXT_INPUT_DISMISS to null
        }
        if (nativeTextInput(engineHandle, action, value)) {
            schedulePersistence()
        }
        syncTextInput()
    }

    private fun syncTextInput() {
        textInputAdapter.sync(
            nativeTextInputActive(engineHandle),
            nativeTextInputAction(engineHandle),
        )
    }

    private fun syncCameraPortal() {
        if (engineHandle == 0L) {
            return
        }
        val encoded = nativeCameraPortal(engineHandle)
        val portal = if (encoded.isEmpty()) {
            null
        } else {
            runCatching {
                val value = JSONObject(encoded)
                CameraPortal(
                    controller = value.getLong("controller"),
                    kind = value.getString("kind"),
                    x = value.getInt("x"),
                    y = value.getInt("y"),
                    width = value.getInt("width"),
                    height = value.getInt("height"),
                )
            }.getOrNull()
        }
        cameraAdapter.syncPortal(portal)
    }

    private fun openCameraController(
        requestId: Long,
        controller: Long,
        payload: String,
        complete: NativeResultHandler,
    ) {
        lightSdkAdapter.execute(
            requestId,
            PERMISSION_STATUS_OPERATION,
            CAMERA_PERMISSION,
        ) { result ->
            runOnUiThread {
                when {
                    result is NativeResult.Failure &&
                        result.kind == NativeErrorKind.UNAVAILABLE ->
                        cameraAdapter.executeController(
                            requestId,
                            controller,
                            OPEN_OPERATION,
                            payload,
                            complete,
                        )
                    result is NativeResult.Success && result.value == GRANTED_PERMISSION ->
                        cameraAdapter.executeController(
                            requestId,
                            controller,
                            OPEN_OPERATION,
                            payload,
                            complete,
                        )
                    result is NativeResult.Success -> cameraAdapter.permissionDenied(
                        requestId,
                        controller,
                        result.value == BLOCKED_PERMISSION,
                        complete,
                    )
                    else -> complete(result)
                }
            }
        }
    }

    private fun automaticCameraRequestId(controller: Long): Long = -(controller + 1L)

    private fun drainNativeRequests() {
        while (engineHandle != 0L) {
            val requestId = nativeNextRequest(engineHandle)
            if (requestId == 0L) {
                return
            }
            val kind = nativeRequestKind(engineHandle, requestId)
            if (kind == NATIVE_REQUEST_CANCEL) {
                val label = nativeRequestLabels.remove(requestId).orEmpty()
                val elapsed = nativeRequestStartedAt.remove(requestId)?.let {
                    SystemClock.elapsedRealtime() - it
                }
                logResource("cancel $requestId $label ${elapsed ?: 0}ms")
                nativeTimeouts.remove(requestId)?.let(nativeRequestHandler::removeCallbacks)
                lightSdkAdapter.cancel(requestId)
                networkAdapter.cancel(requestId)
                audioAdapter.cancel(requestId)
                locationAdapter.cancel(requestId)
                nfcAdapter.cancel(requestId)
                backgroundAdapter.cancel(requestId)
                notificationsAdapter.cancel(requestId)
                cameraAdapter.cancel(requestId)
                continue
            }
            val module = nativeRequestModule(engineHandle, requestId)
            val operation = nativeRequestOperation(engineHandle, requestId)
            val payload = nativeRequestPayload(engineHandle, requestId)
            val controller = nativeRequestController(engineHandle, requestId)
            val label = "$module/$operation"
            nativeRequestStartedAt[requestId] = SystemClock.elapsedRealtime()
            nativeRequestLabels[requestId] = label
            logResource("start $requestId $label")
            val lightAudioPermission = module == AUDIO_MODULE &&
                controller < 0L &&
                (operation == PERMISSION_STATUS_OPERATION ||
                    operation == REQUEST_PERMISSION_OPERATION)
            val lightCameraPermission = module == CAMERA_MODULE &&
                controller < 0L &&
                (operation == PERMISSION_STATUS_OPERATION ||
                    operation == REQUEST_PERMISSION_OPERATION)
            val adapter = when (module) {
                LIGHT_SDK_MODULE -> lightSdkAdapter
                NETWORK_MODULE -> networkAdapter
                AUDIO_MODULE -> if (lightAudioPermission) lightSdkAdapter else audioAdapter
                LOCATION_MODULE -> locationAdapter
                NFC_MODULE -> nfcAdapter
                BACKGROUND_MODULE -> backgroundAdapter
                NOTIFICATIONS_MODULE -> notificationsAdapter
                CAMERA_MODULE -> cameraAdapter
                else -> null
            }
            if (adapter == null) {
                completeNativeRequest(
                    requestId,
                    kind,
                    NativeResult.Failure(
                        NativeErrorKind.PROTOCOL,
                        "Unknown native module: $module",
                        false,
                    ),
                )
                continue
            }
            val timeout = Runnable {
                adapter.cancel(requestId)
                if (module == CAMERA_MODULE) {
                    lightSdkAdapter.cancel(requestId)
                }
                completeNativeRequest(
                    requestId,
                    kind,
                    NativeResult.Failure(
                        NativeErrorKind.TIMEOUT,
                        "Native request timed out",
                        true,
                    ),
                )
            }
            nativeTimeouts[requestId] = timeout
            nativeRequestHandler.postDelayed(
                timeout,
                nativeRequestTimeoutMs(engineHandle, requestId),
            )
            val execute = { result: NativeResult ->
                runOnUiThread { completeNativeRequest(requestId, kind, result) }
            }
            if (lightCameraPermission) {
                lightSdkAdapter.execute(requestId, operation, CAMERA_PERMISSION) { result ->
                    if (result is NativeResult.Failure &&
                        result.kind == NativeErrorKind.UNAVAILABLE
                    ) {
                        runOnUiThread {
                            cameraAdapter.execute(requestId, operation, payload, execute)
                        }
                    } else {
                        execute(result)
                    }
                }
            } else if (lightAudioPermission) {
                lightSdkAdapter.execute(requestId, operation, MICROPHONE_PERMISSION) { result ->
                    if (result is NativeResult.Failure &&
                        result.kind == NativeErrorKind.UNAVAILABLE
                    ) {
                        runOnUiThread {
                            audioAdapter.execute(requestId, operation, payload, execute)
                        }
                    } else {
                        execute(result)
                    }
                }
            } else if (controller >= 0L && adapter === audioAdapter) {
                audioAdapter.executeController(controller, operation, payload, execute)
            } else if (controller >= 0L && adapter === lightSdkAdapter) {
                lightSdkAdapter.executeController(
                    requestId,
                    controller,
                    operation,
                    payload,
                    execute,
                )
            } else if (controller >= 0L && adapter === notificationsAdapter) {
                notificationsAdapter.executeController(controller, operation, payload, execute)
            } else if (controller >= 0L && adapter === cameraAdapter) {
                if (operation != OPEN_OPERATION) {
                    cameraAdapter.executeController(
                        requestId,
                        controller,
                        operation,
                        payload,
                        execute,
                    )
                } else {
                    openCameraController(
                        requestId,
                        controller,
                        payload,
                        execute,
                    )
                }
            } else if (adapter === locationAdapter) {
                locationAdapter.execute(requestId, operation, payload) { result ->
                    val permission = if (
                        result is NativeResult.Failure &&
                        result.kind == NativeErrorKind.PERMISSION_DENIED
                    ) {
                        locationAdapter.requiredPermission(payload)
                    } else {
                        null
                    }
                    if (permission == null) {
                        execute(result)
                    } else {
                        lightSdkAdapter.execute(
                            requestId,
                            PERMISSION_STATUS_OPERATION,
                            permission,
                        ) { permissionResult ->
                            execute(
                                if (
                                    permissionResult is NativeResult.Success &&
                                    permissionResult.value == BLOCKED_PERMISSION
                                ) {
                                    NativeResult.Failure(
                                        NativeErrorKind.PERMISSION_BLOCKED,
                                        "Location permission is blocked by LightOS",
                                        false,
                                    )
                                } else {
                                    result
                                },
                            )
                        }
                    }
                }
            } else {
                adapter.execute(requestId, operation, payload, execute)
            }
        }
    }

    private fun handleNotificationIntent(intent: android.content.Intent?) {
        intent?.let(notificationsAdapter::handleIntent)
        notificationsAdapter.refreshEvents()
        val href = intent?.getStringExtra(EXTRA_NOTIFICATION_HREF).orEmpty()
        if (href.isNotEmpty() && engineHandle != 0L) {
            if (nativeNavigate(engineHandle, href)) {
                inkView.stopScrolling()
            }
        }
        intent?.removeExtra(EXTRA_NOTIFICATION_HREF)
    }

    private fun completeNativeRequest(requestId: Long, kind: Int, result: NativeResult) {
        if (engineHandle == 0L) {
            return
        }
        nativeTimeouts.remove(requestId)?.let(nativeRequestHandler::removeCallbacks)
        val label = nativeRequestLabels.remove(requestId).orEmpty()
        val elapsed = nativeRequestStartedAt.remove(requestId)?.let {
            SystemClock.elapsedRealtime() - it
        } ?: 0L
        val outcome = if (result is NativeResult.Failure) {
            "error:${result.kind.name.lowercase()}"
        } else {
            "ready"
        }
        logResource("$outcome $requestId $label ${elapsed}ms")
        when (result) {
            is NativeResult.Success -> if (kind == NATIVE_REQUEST_RESOURCE) {
                nativeCompleteString(engineHandle, requestId, result.value)
            } else {
                nativeCompleteAction(engineHandle, requestId)
            }
            is NativeResult.Bytes -> nativeCompleteBytes(engineHandle, requestId, result.value)
            is NativeResult.File -> try {
                nativeCompleteFile(engineHandle, requestId, result.path)
            } finally {
                if (result.deleteAfterRead) {
                    File(result.path).delete()
                }
            }
            is NativeResult.Failure -> if (
                kind == NATIVE_REQUEST_RESOURCE || kind == NATIVE_REQUEST_IMAGE
            ) {
                nativeFailRequest(
                    engineHandle,
                    requestId,
                    result.kind.code,
                    result.message,
                    result.retryable,
                )
            } else {
                nativeCompleteAction(engineHandle, requestId)
            }
        }
        syncCameraPortal()
        drainNativeRequests()
    }

    private fun logResource(message: String) {
        if (BuildConfig.DEBUG) Log.d(RESOURCE_LOG_TAG, message)
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
                val changed = nativePointer(engineHandle, event.actionMasked, event.x, event.y)
                if (changed) {
                    schedulePersistence()
                    syncCameraPortal()
                }
                drainNativeRequests()
                if (event.actionMasked == MotionEvent.ACTION_UP) {
                    if (changed) {
                        lightSdkAdapter.performHaptic(this)
                    }
                    syncTextInput()
                }
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
            val startY = nativeScrollOffset(engineHandle).roundToInt()
            lastFlingY = startY
            scroller.fling(
                0,
                startY,
                0,
                velocityY,
                0,
                0,
                0,
                nativeScrollMaximum(engineHandle).roundToInt(),
            )
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
        private const val TEXT_INPUT_INSERT = 0
        private const val TEXT_INPUT_BACKSPACE = 1
        private const val TEXT_INPUT_SUBMIT = 2
        private const val TEXT_INPUT_DISMISS = 3
        private const val PERSISTENCE_DELAY_MS = 250L
        private const val RESOURCE_LOG_TAG = "InkResource"
        private const val LIGHT_SDK_MODULE = "light-sdk"
        private const val NETWORK_MODULE = "network"
        private const val AUDIO_MODULE = "audio"
        private const val LOCATION_MODULE = "location"
        private const val NFC_MODULE = "nfc"
        private const val BACKGROUND_MODULE = "background"
        private const val NOTIFICATIONS_MODULE = "notifications"
        private const val CAMERA_MODULE = "camera"
        private const val PERMISSION_STATUS_OPERATION = "permission-status"
        private const val REQUEST_PERMISSION_OPERATION = "request-permission"
        private const val MICROPHONE_PERMISSION = "microphone"
        private const val CAMERA_PERMISSION = "camera"
        private const val GRANTED_PERMISSION = "granted"
        private const val BLOCKED_PERMISSION = "blocked"
        private const val OPEN_OPERATION = "open"
        private const val NATIVE_REQUEST_RESOURCE = 0
        private const val NATIVE_REQUEST_CANCEL = 2
        private const val NATIVE_REQUEST_IMAGE = 3

        init {
            System.loadLibrary("ink_android")
        }

        @JvmStatic
        private external fun nativeCreate(statePath: String): Long

        @JvmStatic
        private external fun nativeUsesPersistence(): Boolean

        @JvmStatic
        private external fun nativePersist(handle: Long)

        @JvmStatic
        private external fun nativePublicSans(): ByteBuffer

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
        private external fun nativeScrollOffset(handle: Long): Float

        @JvmStatic
        private external fun nativeScrollMaximum(handle: Long): Float

        @JvmStatic
        private external fun nativeBack(handle: Long): Boolean

        @JvmStatic
        private external fun nativeNavigate(handle: Long, path: String): Boolean

        @JvmStatic
        private external fun nativeCameraPortal(handle: Long): String

        @JvmStatic
        private external fun nativeSetCameraReview(
            handle: Long,
            controller: Long,
            source: String,
        ): Boolean

        @JvmStatic
        private external fun nativeResume(handle: Long): Boolean

        @JvmStatic
        private external fun nativeTextInputActive(handle: Long): Boolean

        @JvmStatic
        private external fun nativeTextInputAction(handle: Long): Int

        @JvmStatic
        private external fun nativeTextInput(handle: Long, action: Int, value: String?): Boolean

        @JvmStatic
        private external fun nativeNextRequest(handle: Long): Long

        @JvmStatic
        private external fun nativeRequestKind(handle: Long, requestId: Long): Int

        @JvmStatic
        private external fun nativeRequestController(handle: Long, requestId: Long): Long

        @JvmStatic
        private external fun nativeRequestTimeoutMs(handle: Long, requestId: Long): Long

        @JvmStatic
        private external fun nativeRequestModule(handle: Long, requestId: Long): String

        @JvmStatic
        private external fun nativeRequestOperation(handle: Long, requestId: Long): String

        @JvmStatic
        private external fun nativeRequestPayload(handle: Long, requestId: Long): String

        @JvmStatic
        private external fun nativeCompleteString(
            handle: Long,
            requestId: Long,
            value: String,
        ): Boolean

        @JvmStatic
        private external fun nativeCompleteBytes(
            handle: Long,
            requestId: Long,
            value: ByteArray,
        ): Boolean

        @JvmStatic
        private external fun nativeCompleteFile(
            handle: Long,
            requestId: Long,
            path: String,
        ): Boolean

        @JvmStatic
        private external fun nativeFailRequest(
            handle: Long,
            requestId: Long,
            kind: Int,
            message: String,
            retryable: Boolean,
        ): Boolean

        @JvmStatic
        private external fun nativeCompleteAction(handle: Long, requestId: Long): Boolean

        @JvmStatic
        private external fun nativeUpdateController(
            handle: Long,
            controller: Long,
            value: String,
        ): Boolean

        @JvmStatic
        private external fun nativeAudioActivate(
            handle: Long,
            controller: Long,
            kind: String,
            config: String,
        ): Boolean

        @JvmStatic
        private external fun nativeAudioDeactivate(handle: Long, controller: Long)

        @JvmStatic
        private external fun nativeAudioSetEnabled(
            handle: Long,
            controller: Long,
            enabled: Boolean,
        ): Boolean

        @JvmStatic
        private external fun nativeAudioSamples(
            handle: Long,
            samples: ShortArray,
            sampleRate: Int,
        ): Boolean

        @JvmStatic
        private external fun nativeDetachSurface(handle: Long)

        @JvmStatic
        private external fun nativeDestroy(handle: Long)
    }
}
