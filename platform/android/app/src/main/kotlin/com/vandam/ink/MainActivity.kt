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
import android.view.ScaleGestureDetector
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
import java.util.concurrent.TimeUnit
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
    private val permissionsAdapter by lazy { PermissionsAdapter(this, lightSdkAdapter) }
    private var usesPermissions = false
    private lateinit var lightSdkAdapter: LightSdkAdapter
    private val assetsAdapter by lazy { AssetsAdapter(this) }
    private var usesAssets = false
    private val barcodeAdapter by lazy { createBarcodeAdapter(this) }
    private var usesBarcode = false
    private lateinit var networkAdapter: NetworkAdapter
    private lateinit var audioAdapter: AudioAdapter
    private lateinit var locationAdapter: LocationAdapter
    private lateinit var nfcAdapter: NfcAdapter
    private lateinit var backgroundAdapter: BackgroundAdapter
    private lateinit var cameraAdapter: CameraAdapter
    private lateinit var mapsAdapter: MapsAdapter
    private val filesAdapter by lazy { createFilesAdapter(this) }
    private val sqliteAdapter by lazy { SqliteAdapter(this) }
    private val secureStoreAdapter by lazy { SecureStoreAdapter(this) }
    private val authAdapter by lazy { AuthAdapter(this) { message ->
        runOnUiThread { if (engineHandle != 0L) nativeJavaScriptReceive(engineHandle, message) }
    } }
    private val externalAdapter by lazy { ExternalAdapter(this) }
    private val downloadsAdapter by lazy { createDownloadsAdapter(this, ::updateController) }
    private val connectivityAdapter by lazy { ConnectivityAdapter(this) { message ->
        runOnUiThread { if (engineHandle != 0L) nativeJavaScriptReceive(engineHandle, message) }
    } }
    private lateinit var textInputAdapter: TextInputAdapter
    private lateinit var notificationsAdapter: NotificationsAdapter
    private val systemGlyphRasterizer by lazy { SystemGlyphRasterizer() }
    private var engineHandle = 0L
    private var surfaceAttached = false
    private val nativeRequestHandler = Handler(Looper.getMainLooper())
    private val nativeTimeouts = mutableMapOf<Long, Runnable>()
    private val nativeRequestStartedAt = mutableMapOf<Long, Long>()
    private val nativeRequestLabels = mutableMapOf<Long, String>()
    private val imageExecutor = Executors.newSingleThreadExecutor()
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
            inkView.requestFrame()
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

        engineHandle = nativeCreate()
        check(engineHandle != 0L) { "Ink could not create the runtime" }
        inkView = InkSurfaceView().apply {
            holder.setFormat(PixelFormat.OPAQUE)
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
            updateController(controller, value)
        }
        networkAdapter = createNetworkAdapter(this)
        mapsAdapter = createMapsAdapter(this, root, ::updateController)
        locationAdapter = createLocationAdapter(this)
        nfcAdapter = createNfcAdapter(this)
        backgroundAdapter = createBackgroundAdapter(this)
        cameraAdapter = createCameraAdapter(
            this,
            root,
            { controller, value ->
                updateController(controller, value)
            },
            { controller, source ->
                updateController(controller, JSONObject()
                    .put("status", if (source == null) "active" else "review")
                    .put("reviewSource", source.orEmpty()).toString())
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
                    if (
                        engineHandle != 0L &&
                        nativeAudioSamples(engineHandle, samples, sampleRate)
                    ) {
                        inkView.requestFrame()
                    }
                }
            },
            { controller, value ->
                updateController(controller, value)
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
                val changed = engineHandle != 0L &&
                    nativeAudioSetEnabled(engineHandle, controller, enabled)
                if (changed) inkView.requestFrame()
                changed
            },
        )
        notificationsAdapter = createNotificationsAdapter(this) { controller, value ->
            updateController(controller, value)
        }
        lightSdkAdapter.start()
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
        startDevelopmentBundle(null)
        if (BuildConfig.DEBUG) intent.getStringExtra("ink.dev.generation")?.let(::activateDevelopmentBundle)
        handleNotificationIntent(intent)
    }

    private var developmentBundle: java.io.File? = null
    private var developmentIcons: ByteArray? = null
    private var refreshCompatibility: String? = null
    private var developmentError: android.app.AlertDialog? = null

    internal fun openBundleAsset(path: String): java.io.InputStream =
        developmentBundle?.let { java.io.File(it, path).inputStream() } ?: assets.open(path)

    private fun activateDevelopmentBundle(generation: String) {
        if (!BuildConfig.DEBUG || !generation.matches(Regex("[0-9]+"))) return
        developmentError?.dismiss()
        val directory = java.io.File(filesDir, "ink-dev/$generation")
        try {
            check(java.io.File(directory, "complete").isFile) { "Bundle transfer is incomplete: ${directory.absolutePath}/complete" }
            val source = java.io.File(directory, "app.js").readText()
            val icons = java.io.File(directory, "ink-icons-v1.json").readBytes()
            val compatibility = JSONObject(java.io.File(directory, "ink-bundle-v1.json").readText())
                .optString("refreshCompatibilityHash")
            if (compatibility.isNotEmpty() && compatibility == refreshCompatibility &&
                developmentIcons?.contentEquals(icons) == true && nativeRefreshJavaScript(engineHandle, source)) {
                developmentBundle = directory
            } else startDevelopmentBundle(directory)
            if (developmentBundle == directory) {
                java.io.File(filesDir, "ink-dev/active-worker").writeText(generation)
                java.io.File(filesDir, "ink-dev").listFiles().orEmpty()
                    .filter { it.isDirectory && it.name.matches(Regex("[0-9]+")) }
                    .sortedByDescending { it.name }.drop(3).filter { it != developmentBundle }
                    .forEach { it.deleteRecursively() }
            }
        } catch (error: Exception) {
            android.util.Log.e("Ink", "Bundle activation failed", error)
            developmentError = android.app.AlertDialog.Builder(this).setTitle("Ink development error")
                .setMessage(error.message).setPositiveButton("Reload") { _, _ -> activateDevelopmentBundle(generation) }
                .setNegativeButton("Close", null).show()
        }
    }

    internal fun bundledAudioUri(source: String): android.net.Uri {
        if (!BuildConfig.DEBUG || developmentBundle == null) return android.net.Uri.parse(source)
        val path = source.removePrefix("asset:///")
        require(path.matches(Regex("ink-assets/[a-f0-9]{64}\\.mp3"))) { "Invalid bundled audio source" }
        val pinned = java.io.File(filesDir, "ink-media/${path.substringAfterLast('/')}")
        pinned.parentFile!!.mkdirs()
        if (!pinned.isFile) {
            val atomic = android.util.AtomicFile(pinned)
            val stream = atomic.startWrite()
            try {
                openBundleAsset(path).use { it.copyTo(stream) }
                atomic.finishWrite(stream)
            } catch (error: Exception) {
                atomic.failWrite(stream)
                throw error
            }
        }

        return android.net.Uri.fromFile(pinned)
    }

    private fun startDevelopmentBundle(directory: java.io.File?) {
        try {
            developmentError?.dismiss()
            if (BuildConfig.DEBUG && directory == null) java.io.File(filesDir, "ink-dev/active-worker").delete()
            val source = if (directory == null) assets.open("app.js").bufferedReader().use { it.readText() }
                else java.io.File(directory, "app.js").readText()
            val icons = if (directory == null) assets.open("ink-icons-v1.json").use { it.readBytes() }
                else java.io.File(directory, "ink-icons-v1.json").readBytes()
            if (BuildConfig.DEBUG) {
                val manifest = if (directory == null) assets.open("ink-bundle-v1.json").bufferedReader().use { it.readText() }
                    else java.io.File(directory, "ink-bundle-v1.json").readText()
                refreshCompatibility = JSONObject(manifest).optString("refreshCompatibilityHash")
                developmentIcons = icons
            }
            stopJavaScriptSession()
            nativeRequestHandler.removeCallbacks(drainJavaScript)
            javascriptPending.set(false)
            check(nativeStartJavaScript(engineHandle, source, icons, this)) { "Ink could not start JavaScript; see ink logs" }
            developmentBundle = directory
            inkView.requestFrame()
        } catch (error: Exception) {
            if (!BuildConfig.DEBUG) throw error
            android.util.Log.e("Ink", "Bundle activation failed", error)
            developmentError = android.app.AlertDialog.Builder(this)
                .setTitle("Ink development error")
                .setMessage("${error.message}\nFix the source and save, or reload this generation.")
                .setPositiveButton("Reload") { _, _ -> startDevelopmentBundle(directory) }
                .setNegativeButton("Close", null).show()
        }
    }

    private val javascriptPending = java.util.concurrent.atomic.AtomicBoolean(false)
    private val storeAdapter by lazy {
        StoreAdapter(this) { key ->
            runOnUiThread {
                if (engineHandle != 0L) nativeJavaScriptReceive(engineHandle,
                    JSONObject().put("type", "store-changed").put("key", key).toString())
            }
        }
    }
    private val clipboardAdapter by lazy { ClipboardAdapter(this) }
    private var usesStore = false
    private val drainJavaScript = Runnable {
        javascriptPending.set(false)
        if (engineHandle != 0L) {
if (nativeDrainJavaScript(engineHandle)) inkView.requestFrame()
            if (BuildConfig.DEBUG) {
                val error = nativeTakeJavaScriptError(engineHandle)
                if (error.isNotEmpty()) {
                    stopJavaScriptSession()
                    val mapped = DevelopmentErrors.map(error) { openBundleAsset("app.js.map").bufferedReader().use { it.readText() } }
                    android.util.Log.e("Ink", mapped)
                    developmentError = android.app.AlertDialog.Builder(this).setTitle("Ink development error")
                        .setMessage(mapped).setPositiveButton("Reload") { _, _ -> startDevelopmentBundle(developmentBundle) }
                        .setNegativeButton("Close", null).show()
                }
            }
            val light = nativeIsLightAppearance(engineHandle)
            textInputAdapter.setLightAppearance(light)
            externalAdapter.setLightAppearance(light)
            window.statusBarColor = if (light) Color.WHITE else Color.BLACK
            window.navigationBarColor = if (light) Color.WHITE else Color.BLACK
            syncTextInput()
            syncCameraPortal()
            drainNativeRequests()
            val calls = org.json.JSONArray(nativeTakeJavaScriptCalls(engineHandle))
            for (index in 0 until calls.length()) executeJavaScriptCall(calls.getJSONObject(index))
        }
    }

    private var javascriptSession: Any? = null
    private var javascriptRequests = newJavaScriptRequests()
    private val javascriptControllers = mutableMapOf<Long, String>()
    private fun newJavaScriptRequests(): NativeRequests {
        val session = Any()
        javascriptSession = session
        return NativeRequests(nativeRequestHandler) { id, result ->
            runOnUiThread {
                if (javascriptSession === session) sendJavaScriptResult(id, result)
                else disposeNativeResult(result)
            }
        }
    }


    private fun stopJavaScriptSession() {
        javascriptSession = null
        javascriptRequests.close()
        javascriptControllers.toMap().forEach { (controller, module) ->
            val complete: NativeResultHandler = { disposeNativeResult(it) }
            when (module) {
                AUDIO_MODULE -> audioAdapter.executeController(-controller, "deactivate", "{}", complete)
                NOTIFICATIONS_MODULE -> notificationsAdapter.executeController(-controller, "deactivate", "{}", complete)
                CAMERA_MODULE -> cameraAdapter.executeController(0, -controller, "deactivate", "{}", complete)
                "maps" -> mapsAdapter.executeController(0, -controller, "deactivate", "{}", complete)
                "downloads" -> downloadsAdapter.executeController(-controller, "deactivate", "{}", complete)
            }
        }
        javascriptControllers.clear()
        connectivityAdapter.reset()
        javascriptRequests = newJavaScriptRequests()
    }

    private fun executeJavaScriptCall(call: JSONObject) {
        val id = call.getLong("id")
        if (call.getString("type") == "cancel") {
            javascriptRequests.cancel(id)
            return
        }
        val module = call.getString("module")
        val adapter = when (module) {
            "permissions" -> { usesPermissions = true; permissionsAdapter }
            "store" -> { usesStore = true; storeAdapter }
            "clipboard" -> clipboardAdapter
            "sqlite" -> sqliteAdapter
            "connectivity" -> connectivityAdapter
            "files" -> filesAdapter
            "secure-store" -> secureStoreAdapter
            "auth" -> authAdapter
            "external" -> externalAdapter
            "downloads" -> downloadsAdapter
            "maps" -> mapsAdapter
            NETWORK_MODULE -> networkAdapter
            LIGHT_SDK_MODULE -> lightSdkAdapter
            AUDIO_MODULE -> audioAdapter
            LOCATION_MODULE -> locationAdapter
            NFC_MODULE -> nfcAdapter
            CAMERA_MODULE -> cameraAdapter
            NOTIFICATIONS_MODULE -> notificationsAdapter
            BACKGROUND_MODULE -> backgroundAdapter
            else -> null
        }
        if (adapter == null) {
            sendJavaScriptResult(id, NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Unknown native module: $module", false))
            return
        }
        javascriptRequests.execute(id, call.optLong("timeoutMs", 30_000), { adapter.cancel(-id) }) { complete ->
            val operation = call.getString("operation")
            val payload = (call.opt("payload") ?: JSONObject.NULL).toString()
            if (call.has("controller")) {
                val controller = call.getLong("controller")
                require(controller in 1..9_007_199_254_740_991L) { "Invalid JavaScript controller ID" }
                if (operation == "activate") javascriptControllers[controller] = module
                if (operation == "deactivate") javascriptControllers.remove(controller)
                val finish: NativeResultHandler = { result ->
                    if (operation == "activate" && result is NativeResult.Failure) {
                        runOnUiThread { javascriptControllers.remove(controller) }
                    }
                    complete(result)
                }
                when (module) {
                    AUDIO_MODULE -> audioAdapter.executeController(-controller, operation, payload, finish)
                    NOTIFICATIONS_MODULE -> notificationsAdapter.executeController(-controller, operation, payload, finish)
                    "maps" -> mapsAdapter.executeController(-id, -controller, operation, payload, finish)
                    "downloads" -> downloadsAdapter.executeController(-controller, operation, payload, finish)
                    CAMERA_MODULE -> {
                        if (operation == OPEN_OPERATION) openCameraController(-id, -controller, payload, finish)
                        else cameraAdapter.executeController(-id, -controller, operation, payload, finish)
                    }
                    else -> finish(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unsupported controller module: $module", false))
                }
            } else {
                adapter.execute(-id, operation, payload, complete)
            }
        }
    }

    private fun updateController(controller: Long, value: String) {
        runOnUiThread {
            if (engineHandle != 0L && javascriptControllers.containsKey(-controller)) {
                val event = JSONObject().put("type", "controller").put("id", -controller)
                    .put("value", JSONObject(value))
                var message = event.toString()
                if (message.toByteArray(Charsets.UTF_8).size > 1024 * 1024) {
                    message = event.put("value", JSONObject().put("status", "error")
                        .put("error", inkError("protocol", "Native controller state exceeds the message limit"))).toString()
                }
                nativeJavaScriptReceive(engineHandle, message)
            }
        }
    }

    private fun sendJavaScriptResult(id: Long, result: NativeResult) {
        if (engineHandle == 0L) return
        nativeJavaScriptReceive(engineHandle, javascriptResult(id, result))
    }

    fun onJavaScriptReady() {
        if (javascriptPending.compareAndSet(false, true)) {
            nativeRequestHandler.post(drainJavaScript)
        }
    }

    fun loadWebRuntime(): String = assets.open("ink-assets/ink-web.js").bufferedReader().use { it.readText() }

    override fun onNewIntent(intent: android.content.Intent) {
        super.onNewIntent(intent)
        externalAdapter.handleIntent(intent)
        setIntent(intent)
        if (BuildConfig.DEBUG) intent.getStringExtra("ink.dev.generation")?.let(::activateDevelopmentBundle)
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
        externalAdapter.onResume()
        connectivityAdapter.start()
        mapsAdapter.resume()
        audioAdapter.resume()
        locationAdapter.resume()
        lightSdkAdapter.refresh()

        cameraAdapter.resume()
        syncCameraPortal()
        nfcAdapter.resume()
        notificationsAdapter.refreshEvents()
        syncTextInput()
    }

    @Deprecated("Android activity result callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: android.content.Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        filesAdapter.onActivityResult(requestCode, resultCode, data)
        if (usesPermissions) permissionsAdapter.result(requestCode)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        filesAdapter.onRequestPermissionsResult(requestCode)
        if (usesPermissions) permissionsAdapter.result(requestCode)

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
        inkView.requestFrame()
        syncCameraPortal()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (engineHandle == 0L || !surfaceAttached) {
            return
        }

        logResource("Surface resized to ${width}x${height}")
        nativeResize(engineHandle, width, height)
        inkView.requestFrame()
        syncCameraPortal()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        detachSurface()
    }

    override fun onDestroy() {

        onBackInvokedDispatcher.unregisterOnBackInvokedCallback(backCallback)
        if (usesPermissions) permissionsAdapter.stop()
        lightSdkAdapter.stop()
        notificationsAdapter.stop()
        stopJavaScriptSession()
        if (usesAssets) assetsAdapter.stop()
        if (usesBarcode) barcodeAdapter.stop()
        networkAdapter.stop()
        audioAdapter.stop()
        locationAdapter.stop()
        nfcAdapter.stop()
        backgroundAdapter.stop()
        cameraAdapter.stop()
        mapsAdapter.stop()
        filesAdapter.stop()
        sqliteAdapter.stop()
        externalAdapter.close()
        authAdapter.close()
        connectivityAdapter.close()
        downloadsAdapter.stop()
        if (usesStore) storeAdapter.stop()
        imageExecutor.shutdown()
        imageExecutor.awaitTermination(Long.MAX_VALUE, TimeUnit.NANOSECONDS)
        nativeTimeouts.values.forEach(nativeRequestHandler::removeCallbacks)
        nativeTimeouts.clear()
        nativeRequestStartedAt.clear()
        nativeRequestLabels.clear()
        detachSurface()
        if (engineHandle != 0L) {
            nativeDestroy(engineHandle)
            engineHandle = 0L
        }
        nativeRequestHandler.removeCallbacks(drainJavaScript)
        super.onDestroy()
    }

    override fun onPause() {
        externalAdapter.onPause()
        connectivityAdapter.stop()
        mapsAdapter.pause()
        inkView.setTextCursorActive(false)
        audioAdapter.pause()
        nfcAdapter.pause()
        cameraAdapter.pause()
        locationAdapter.pause()
        super.onPause()
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
            inkView.requestFrame()
        }
        syncTextInput()
    }

    internal fun setKeyboardInset(height: Int) {
        if (engineHandle == 0L) return
        if (nativeSetKeyboardInset(engineHandle, height)) {
            logResource("Keyboard inset=$height surface=${inkView.width}x${inkView.height}")
            inkView.requestFrame()
        }
    }

    private fun syncTextInput() {
        val active = nativeTextInputActive(engineHandle)
        textInputAdapter.sync(active, nativeTextInputAction(engineHandle), nativeTextInputNumeric(engineHandle))
        inkView.setTextCursorActive(active)
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
                    light = value.getBoolean("light"),
                )
            }.getOrNull()
        }
        cameraAdapter.syncPortal(portal, nativeCameraReviewReady(engineHandle))
        val map = nativeMapPortal(engineHandle)
        mapsAdapter.syncPortal(if (map.isEmpty()) null else {
            val value = JSONObject(map)
            MapPortal(value.getLong("controller"), value.getInt("x"), value.getInt("y"), value.getInt("width"), value.getInt("height"))
        })
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

    private fun automaticCameraRequestId(controller: Long): Long =
        if (controller < 0) Long.MIN_VALUE - controller else -(controller + 1L)

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
                if (usesAssets) assetsAdapter.cancel(requestId)
                if (usesBarcode) barcodeAdapter.cancel(requestId)
                networkAdapter.cancel(requestId)
                audioAdapter.cancel(requestId)
                locationAdapter.cancel(requestId)
                nfcAdapter.cancel(requestId)
                backgroundAdapter.cancel(requestId)
                notificationsAdapter.cancel(requestId)
                cameraAdapter.cancel(requestId)
                filesAdapter.cancel(requestId)
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
                "files" -> filesAdapter
                LIGHT_SDK_MODULE -> lightSdkAdapter
                "assets" -> { usesAssets = true; assetsAdapter }
                "barcode" -> { usesBarcode = true; barcodeAdapter }
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
            val message = JSONObject().put("type", "navigate").put("path", href)
            intent?.getStringExtra(EXTRA_NOTIFICATION_PARAMS)?.let { params ->
                if (params.toByteArray().size <= 8192) {
                    runCatching { JSONObject(params) }.getOrNull()?.let { message.put("params", it) }
                }
            }
            nativeJavaScriptReceive(engineHandle, message.toString())
        }
        intent?.removeExtra(EXTRA_NOTIFICATION_HREF)
        intent?.removeExtra(EXTRA_NOTIFICATION_PARAMS)
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
            "error:${result.kind.wireName}"
        } else {
            "ready"
        }
        logResource("$outcome $requestId $label ${elapsed}ms")
        val changed = when (result) {
            is NativeResult.Success, is NativeResult.Bytes -> nativeCompleteAction(engineHandle, requestId)
            is NativeResult.Pixels -> nativeCompletePixels(engineHandle, requestId, result.width, result.height, result.rgba)
            is NativeResult.File -> {
                completeImage(requestId, result)
                return
            }
            is NativeResult.Failure -> if (
                kind == NATIVE_REQUEST_IMAGE
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
        if (changed) inkView.requestFrame()
        syncCameraPortal()
        drainNativeRequests()
    }

    private fun completeImage(requestId: Long, result: NativeResult.File) {
        val handle = engineHandle
        imageExecutor.execute {
            val changed = try {
                nativeCompleteFile(handle, requestId, result.path)
            } finally {
                if (result.deleteAfterRead) File(result.path).delete()
            }
            runOnUiThread {
                if (engineHandle != handle) return@runOnUiThread
                if (changed) inkView.requestFrame()
                syncCameraPortal()
                drainNativeRequests()
            }
        }
    }

    private fun renderFrame() {
        while (true) {
            val request = nativeRender(engineHandle, inkView.isTextCursorVisible())
            if (request.isEmpty()) return

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
            nativeInstallSystemGlyph(
                engineHandle,
                requestId,
                systemGlyphRasterizer.rasterise(grapheme, size) ?: ByteArray(0),
            )
        }
    }

    private fun logResource(message: String) {
        if (BuildConfig.DEBUG) Log.d(RESOURCE_LOG_TAG, message)
    }

    private inner class InkSurfaceView : SurfaceView(this@MainActivity) {
        private val longPress = Runnable {
            if (engineHandle != 0L && surfaceAttached) {
                if (processPointerResult(nativePointer(engineHandle, POINTER_LONG_PRESS, imageTapDownX, imageTapDownY))) requestFrame()
                postFrame()
            }
        }

        override fun onDetachedFromWindow() {
            removeCallbacks(longPress)
            super.onDetachedFromWindow()
        }

        private val choreographer = Choreographer.getInstance()
        private val viewConfiguration = ViewConfiguration.get(this@MainActivity)
        private val minimumFlingVelocity = viewConfiguration.scaledMinimumFlingVelocity
        private val maximumFlingVelocity = viewConfiguration.scaledMaximumFlingVelocity
        private val touchSlop = viewConfiguration.scaledTouchSlop
        private val doubleTapSlop = viewConfiguration.scaledDoubleTapSlop
        private val doubleTapTimeout = ViewConfiguration.getDoubleTapTimeout().toLong()
        private val scroller = OverScroller(this@MainActivity)
        private val scaleGestureDetector = ScaleGestureDetector(
            this@MainActivity,
            object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
                override fun onScaleBegin(detector: ScaleGestureDetector): Boolean {
                    if (!surfaceAttached || engineHandle == 0L) return false
                    val result = nativeImagePinchBegin(
                        engineHandle,
                        detector.focusX,
                        detector.focusY,
                    )
                    pinchActive = (result and POINTER_CAPTURED) != 0
                    if (pinchActive) {
                        hasPendingMove = false
                        nativePointer(engineHandle, MotionEvent.ACTION_CANCEL, 0f, 0f)
                        capturedGesture = true
                        stopFling()
                    }
                    return pinchActive
                }

                override fun onScale(detector: ScaleGestureDetector): Boolean {
                    if (!pinchActive) return false
                    val changed = processPointerResult(
                        nativeImagePinchUpdate(
                            engineHandle,
                            detector.scaleFactor,
                            detector.focusX,
                            detector.focusY,
                        ),
                    )
                    if (changed) requestFrame()
                    return true
                }

                override fun onScaleEnd(detector: ScaleGestureDetector) {
                    if (pinchActive) nativeImagePinchEnd(engineHandle)
                    pinchActive = false
                }
            },
        ).apply {
            isQuickScaleEnabled = false
            isStylusScaleEnabled = false
        }
        private var downY = 0f
        private var framePosted = false
        private var renderPending = false
        private var pendingMoveX = 0f
        private var pendingMoveY = 0f
        private var hasPendingMove = false
        private var lastFlingY = 0
        private var capturedGesture = false
        private var pinchActive = false
        private var imageTapDownX = 0f
        private var imageTapDownY = 0f
        private var imageTapTarget = 0L
        private var previousImageTapTime = 0L
        private var previousImageTapTarget = 0L
        private var previousImageTapX = 0f
        private var previousImageTapY = 0f
        private var textCursorVisible = true
        private var velocityTracker: VelocityTracker? = null
        private val textCursorBlink = object : Runnable {
            override fun run() {
                textCursorVisible = !textCursorVisible
                requestFrame()
                postDelayed(this, TEXT_CURSOR_BLINK_MS)
            }
        }
        private val frameCallback = Choreographer.FrameCallback {
            // Include completed React work even if its handler is queued behind this frame.
            if (engineHandle != 0L && javascriptPending.get()) {
                nativeRequestHandler.removeCallbacks(drainJavaScript)
                drainJavaScript.run()
            }
            framePosted = false
            if (!surfaceAttached || engineHandle == 0L) {
                return@FrameCallback
            }

            // Present a completed React window before the next thumb movement
            // can move the viewport beyond it again.
            val presentBeforeMove = renderPending && hasPendingMove
            if (presentBeforeMove) renderFrame()
            var changed = renderPending && !presentBeforeMove
            renderPending = false
            if (hasPendingMove) {
                hasPendingMove = false
                changed = processPointerResult(
                    nativePointer(
                        engineHandle,
                        MotionEvent.ACTION_MOVE,
                        pendingMoveX,
                        pendingMoveY,
                    ),
                ) || changed
            }
            if (scroller.computeScrollOffset()) {
                val y = scroller.currY
                val delta = y - lastFlingY
                lastFlingY = y
                if (delta != 0) {
                    if (nativeScrollBy(engineHandle, delta.toFloat())) {
                        changed = true
                    } else {
                        scroller.abortAnimation()
                    }
                }
            }
            if (changed) {
                if (presentBeforeMove) renderPending = true else renderFrame()
            }
            postFrame()
        }

        override fun onTouchEvent(event: MotionEvent): Boolean {
            scaleGestureDetector.onTouchEvent(event)
            if (pinchActive || event.pointerCount > 1) {
                removeCallbacks(longPress)
                resetImageTap()
                velocityTracker?.recycle()
                velocityTracker = null
                postFrame()
                return true
            }
            var flingVelocity = 0
            var imageDoubleTap = 0
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    removeCallbacks(longPress)
                    postDelayed(longPress, ViewConfiguration.getLongPressTimeout().toLong())
                    stopFling()
                    downY = event.y
                    imageTapDownX = event.x
                    imageTapDownY = event.y
                    imageTapTarget = if (engineHandle != 0L && surfaceAttached) {
                        nativeImageZoomTarget(engineHandle, event.x, event.y)
                    } else {
                        0L
                    }
                    if (imageTapTarget == 0L) resetImageTap()
                    velocityTracker?.recycle()
                    velocityTracker = VelocityTracker.obtain().also { it.addMovement(event) }
                }
                MotionEvent.ACTION_MOVE -> {
                    velocityTracker?.addMovement(event)
                    if (movedBeyond(event.x, event.y, imageTapDownX, imageTapDownY, touchSlop)) {
                        removeCallbacks(longPress)
                        resetImageTap()
                    }
                }
                MotionEvent.ACTION_UP -> {
                    removeCallbacks(longPress)
                    if (imageTapTarget != 0L) {
                        val isSecondTap = previousImageTapTime != 0L &&
                            previousImageTapTarget == imageTapTarget &&
                            event.eventTime - previousImageTapTime <= doubleTapTimeout &&
                            !movedBeyond(
                                event.x,
                                event.y,
                                previousImageTapX,
                                previousImageTapY,
                                doubleTapSlop,
                            )
                        if (isSecondTap) {
                            imageDoubleTap = nativeImageDoubleTap(engineHandle, event.x, event.y)
                            resetImageTap()
                        } else {
                            previousImageTapTime = event.eventTime
                            previousImageTapTarget = imageTapTarget
                            previousImageTapX = event.x
                            previousImageTapY = event.y
                            imageTapTarget = 0L
                        }
                    } else {
                        resetImageTap()
                    }
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
                    removeCallbacks(longPress)
                    resetImageTap()
                    velocityTracker?.recycle()
                    velocityTracker = null
                }
            }
            if (engineHandle != 0L && surfaceAttached) {
                var changed = false
                if ((imageDoubleTap and POINTER_CAPTURED) != 0) {
                    hasPendingMove = false
                    changed = processPointerResult(imageDoubleTap)
                } else if (event.actionMasked == MotionEvent.ACTION_MOVE) {
                    pendingMoveX = event.x
                    pendingMoveY = event.y
                    hasPendingMove = true
                    postFrame()
                } else {
                    if (hasPendingMove) {
                        hasPendingMove = false
                        changed = processPointerResult(
                            nativePointer(
                                engineHandle,
                                MotionEvent.ACTION_MOVE,
                                pendingMoveX,
                                pendingMoveY,
                            ),
                        )
                    }
                    changed = processPointerResult(
                        nativePointer(
                            engineHandle,
                            event.actionMasked,
                            event.x,
                            event.y,
                        ),
                    ) || changed
                }
                if (changed) {
                    requestFrame()
                }
                postFrame()
            }
            if (flingVelocity != 0 && !capturedGesture) {
                startFling(flingVelocity)
            }
            if (
                event.actionMasked == MotionEvent.ACTION_UP ||
                event.actionMasked == MotionEvent.ACTION_CANCEL
            ) {
                capturedGesture = false
            }
            if (event.actionMasked == MotionEvent.ACTION_UP) {
                performClick()
            }
            return true
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
            if ((result and POINTER_ACTIVATED) != 0) {
                lightSdkAdapter.performHaptic(this)
                drainNativeRequests()
                syncCameraPortal()
                syncTextInput()
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

        fun isTextCursorVisible(): Boolean = textCursorVisible

        fun setTextCursorActive(active: Boolean) {
            removeCallbacks(textCursorBlink)
            textCursorVisible = true
            if (active) {
                postDelayed(textCursorBlink, TEXT_CURSOR_BLINK_MS)
            }
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
            postFrame()
        }

        private fun postFrame() {
            if (
                !framePosted &&
                (renderPending || hasPendingMove || !scroller.isFinished)
            ) {
                framePosted = true
                choreographer.postFrameCallback(frameCallback)
            }
        }

        private fun stopFling() {
            scroller.abortAnimation()
            hasPendingMove = false
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
        private const val POINTER_LONG_PRESS = 4
        private const val POINTER_CHANGED = 1
        private const val POINTER_ACTIVATED = 1 shl 1
        private const val POINTER_CAPTURED = 1 shl 2
        private const val TEXT_CURSOR_BLINK_MS = 500L
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
        private const val NATIVE_REQUEST_CANCEL = 2
        private const val NATIVE_REQUEST_IMAGE = 3

        init {
            System.loadLibrary("ink_android")
        }

        @JvmStatic
        private external fun nativeCreate(): Long

        @JvmStatic
        private external fun nativeTakeJavaScriptError(handle: Long): String

        @JvmStatic
        private external fun nativeRefreshJavaScript(handle: Long, source: String): Boolean

        @JvmStatic
        private external fun nativeStartJavaScript(handle: Long, source: String, icons: ByteArray, activity: MainActivity): Boolean

        @JvmStatic
        private external fun nativeDrainJavaScript(handle: Long): Boolean

        @JvmStatic
        private external fun nativeIsLightAppearance(handle: Long): Boolean

        @JvmStatic
        private external fun nativeTakeJavaScriptCalls(handle: Long): String

        @JvmStatic
        private external fun nativeJavaScriptReceive(handle: Long, message: String)

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
        private external fun nativeSetKeyboardInset(handle: Long, height: Int): Boolean

        @JvmStatic
        private external fun nativeResize(handle: Long, width: Int, height: Int)

        @JvmStatic
        private external fun nativePointer(
            handle: Long,
            action: Int,
            x: Float,
            y: Float,
        ): Int

        @JvmStatic
        private external fun nativeImagePinchBegin(handle: Long, x: Float, y: Float): Int

        @JvmStatic
        private external fun nativeImagePinchUpdate(
            handle: Long,
            scale: Float,
            x: Float,
            y: Float,
        ): Int

        @JvmStatic
        private external fun nativeImagePinchEnd(handle: Long)

        @JvmStatic
        private external fun nativeImageZoomTarget(handle: Long, x: Float, y: Float): Long

        @JvmStatic
        private external fun nativeImageDoubleTap(
            handle: Long,
            x: Float,
            y: Float,
        ): Int

        @JvmStatic
        private external fun nativeScrollBy(handle: Long, delta: Float): Boolean

        @JvmStatic
        private external fun nativeScrollOffset(handle: Long): Float

        @JvmStatic
        private external fun nativeScrollMaximum(handle: Long): Float

        @JvmStatic
        private external fun nativeRender(handle: Long, textCursorVisible: Boolean): String

        @JvmStatic
        private external fun nativeInstallSystemGlyph(
            handle: Long,
            requestId: Long,
            pixels: ByteArray,
        )

        @JvmStatic
        private external fun nativeBack(handle: Long): Boolean

        @JvmStatic
        private external fun nativeCameraPortal(handle: Long): String

        @JvmStatic
        private external fun nativeCompletePixels(handle: Long, requestId: Long, width: Int, height: Int, rgba: ByteArray): Boolean

        @JvmStatic
        private external fun nativeCameraReviewReady(handle: Long): Boolean

        @JvmStatic
        private external fun nativeMapPortal(handle: Long): String

        @JvmStatic
        private external fun nativeTextInputActive(handle: Long): Boolean

        @JvmStatic
        private external fun nativeTextInputAction(handle: Long): Int

        @JvmStatic
        private external fun nativeTextInputNumeric(handle: Long): Boolean

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
