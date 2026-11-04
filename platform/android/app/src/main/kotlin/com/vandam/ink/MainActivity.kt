package com.vandam.ink

import android.app.Activity
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.fonts.Font
import android.graphics.fonts.FontFamily
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Log
import android.view.KeyEvent
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.widget.FrameLayout
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import java.io.File
import java.util.concurrent.Executors
import org.json.JSONObject

class MainActivity : Activity(), InkJavaScriptHost {
    internal val publicSansTypeface: Typeface by lazy(LazyThreadSafetyMode.NONE) {
        Typeface.CustomFallbackBuilder(
            FontFamily.Builder(Font.Builder(InkNative.nativePublicSans()).build()).build(),
        ).build()
    }
    private lateinit var host: InkHost
    private val inkView get() = host.view
    private lateinit var root: FrameLayout
    private val permissionsAdapter by lazy { PermissionsAdapter(this, lightSdkAdapter) }
    private var usesPermissions = false
    private lateinit var lightSdkAdapter: LightSdkAdapter
    private val assetsAdapter by lazy { AssetsAdapter(this) }
    private val barcodeAdapter by lazy { createBarcodeAdapter(this) }
    private var usesBarcode = false
    private lateinit var networkAdapter: NetworkAdapter
    private lateinit var audioAdapter: AudioAdapter
    private lateinit var locationAdapter: LocationAdapter
    private lateinit var nfcAdapter: NfcAdapter
    private lateinit var backgroundAdapter: BackgroundAdapter
    private lateinit var cameraAdapter: CameraAdapter
    private lateinit var mapsAdapter: MapsAdapter
    private lateinit var videoAdapter: VideoAdapter
    private var cameraPortal: CameraPortal? = null
    private var videoPortal: VideoPortal? = null
    private var mapPortal: MapPortal? = null
    private val filesAdapter by lazy { createFilesAdapter(this) }
    private val appNativeAdapters by lazy { createAppNativeAdapters(this) }
    private val secureStoreAdapter by lazy { SecureStoreAdapter(this) }
    private val authAdapter by lazy { AuthAdapter(this) { message ->
        runOnUiThread { if (engineHandle != 0L) InkNative.nativeJavaScriptReceive(engineHandle, message) }
    } }
    private val externalAdapter by lazy { ExternalAdapter(this) }
    private val overlaysAdapter by lazy { createOverlaysAdapter(this) { message ->
        runOnUiThread { if (engineHandle != 0L) InkNative.nativeJavaScriptReceive(engineHandle, message) }
    } }
    private val downloadsAdapter by lazy { createDownloadsAdapter(this, ::updateController) }
    private val connectivityAdapter by lazy { ConnectivityAdapter(this) { message ->
        runOnUiThread { if (engineHandle != 0L) InkNative.nativeJavaScriptReceive(engineHandle, message) }
    } }
    private lateinit var textInputAdapter: TextInputAdapter
    private lateinit var notificationsAdapter: NotificationsAdapter
    private val engineHandle get() = host.handle
    private val surfaceAttached get() = host.surfaceAttached
    private val nativeRequestHandler = Handler(Looper.getMainLooper())
    private val nativeTimeouts = mutableMapOf<Long, Runnable>()
    private val nativeRequestStartedAt = mutableMapOf<Long, Long>()
    private val nativeRequestLabels = mutableMapOf<Long, String>()
    private val imageExecutor = Executors.newFixedThreadPool(2)
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
        if (engineHandle == 0L || !InkNative.nativeBack(engineHandle)) {
            finish()
        } else {
            inkView.requestFrame()
            syncNativePortals()
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

        host = InkHost(this, object : InkHost.Callbacks {
            override fun beforeFrame() {
                if (javascriptPending.get()) {
                    nativeRequestHandler.removeCallbacks(drainJavaScript)
                    drainJavaScript.run()
                }
            }
            override fun changed() {
                drainNativeRequests()
                syncNativePortals()
                syncTextInput()
            }
            override fun beforeRender() {
                drainNativeRequests()
                if (textInputAdapter.hasEditor) syncTextInput()
            }
            override fun presented() = textInputAdapter.framePresented()
            override fun haptic(view: android.view.View) = lightSdkAdapter.performHaptic(view)
        })
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
        lightSdkAdapter = createLightSdkAdapter(this, ::updateController)
        networkAdapter = createNetworkAdapter(this)
        mapsAdapter = createMapsAdapter(this, root, ::updateController)
        videoAdapter = createVideoAdapter(this, root, ::updateController)
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
                        InkNative.nativeAudioSamples(engineHandle, samples, sampleRate)
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
                    InkNative.nativeAudioActivate(engineHandle, controller, kind, config)
            },
            { controller ->
                if (engineHandle != 0L) {
                    InkNative.nativeAudioDeactivate(engineHandle, controller)
                }
            },
            { controller, enabled ->
                val changed = engineHandle != 0L &&
                    InkNative.nativeAudioSetEnabled(engineHandle, controller, enabled)
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

    internal fun bundleImage(path: String): NativeResult =
        developmentBundle?.let { NativeResult.File(java.io.File(it, path).path, deleteAfterRead = false) } ?: NativeResult.Asset(path)

    private fun activateDevelopmentBundle(generation: String) {
        if (!BuildConfig.DEBUG || !generation.matches(Regex("[0-9]+"))) return
        developmentError?.dismiss()
        val directory = java.io.File(filesDir, "ink-dev/$generation")
        try {
            check(java.io.File(directory, "complete").isFile) { "Bundle transfer is incomplete: ${directory.absolutePath}/complete" }
            val source = java.io.File(directory, "app.js").readText()
            val icons = java.io.File(directory, "ink-icons-v1.bin").readBytes()
            val compatibility = JSONObject(java.io.File(directory, "ink-bundle-v1.json").readText())
                .optString("refreshCompatibilityHash")
            if (compatibility.isNotEmpty() && compatibility == refreshCompatibility &&
                developmentIcons?.contentEquals(icons) == true && InkNative.nativeRefreshJavaScript(engineHandle, source)) {
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
        return android.net.Uri.fromFile(bundledMediaFile(source))
    }

    internal fun bundledMediaFile(source: String): java.io.File {
        val path = source.removePrefix("asset:///")
        require(path.matches(Regex("ink-assets/[a-f0-9]{64}\\.(mp3|mp4|mov|webm)"))) { "Invalid bundled media source" }
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

        return pinned
    }

    private fun startDevelopmentBundle(directory: java.io.File?) {
        try {
            developmentError?.dismiss()
            if (BuildConfig.DEBUG && directory == null) java.io.File(filesDir, "ink-dev/active-worker").delete()
            // Native code reads installed bundles from the APK off the UI thread.
            val source = directory?.let { java.io.File(it, "app.js").readText() }
            val icons = directory?.let { java.io.File(it, "ink-icons-v1.bin").readBytes() }
            if (BuildConfig.DEBUG) {
                val manifest = if (directory == null) assets.open("ink-bundle-v1.json").bufferedReader().use { it.readText() }
                    else java.io.File(directory, "ink-bundle-v1.json").readText()
                refreshCompatibility = JSONObject(manifest).optString("refreshCompatibilityHash")
                developmentIcons = icons ?: assets.open("ink-icons-v1.bin").use { it.readBytes() }
            }
            stopJavaScriptSession()
            nativeRequestHandler.removeCallbacks(drainJavaScript)
            javascriptPending.set(false)
            check(InkNative.nativeStartJavaScript(engineHandle, source, icons, assets,
                getDatabasePath("ink-store.db").path, filesDir.path, this, "app")) { "Ink could not start JavaScript; see ink logs" }
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
    private val clipboardAdapter by lazy { ClipboardAdapter(this) }
    private var pendingJavaScriptCalls: org.json.JSONArray? = null
    private var javascriptCallIndex = 0
    private fun drainJavaScriptCalls(): Boolean {
        val calls = pendingJavaScriptCalls ?: return true
        val deadline = SystemClock.uptimeMillis() + 4
        repeat(32) {
            if (javascriptCallIndex == calls.length()) { pendingJavaScriptCalls = null; return true }
            executeJavaScriptCall(calls.getJSONObject(javascriptCallIndex++))
            if (SystemClock.uptimeMillis() >= deadline) { onJavaScriptReady(); return false }
        }
        onJavaScriptReady()
        return false
    }
    private val drainJavaScript = Runnable {
        javascriptPending.set(false)
        if (engineHandle != 0L) {
            if (!drainJavaScriptCalls()) return@Runnable
            if (InkNative.nativeDrainJavaScript(engineHandle)) inkView.requestCommitFrame()
            val error = InkNative.nativeTakeJavaScriptError(engineHandle)
            if (!error.isNullOrEmpty()) {
                stopJavaScriptSession()
                if (BuildConfig.DEBUG) {
                    val mapped = DevelopmentErrors.map(error) { openBundleAsset("app.js.map").bufferedReader().use { it.readText() } }
                    android.util.Log.e("Ink", mapped)
                    developmentError = android.app.AlertDialog.Builder(this).setTitle("Ink development error")
                        .setMessage(mapped).setPositiveButton("Reload") { _, _ -> startDevelopmentBundle(developmentBundle) }
                        .setNegativeButton("Close", null).show()
                } else {
                    android.util.Log.e("Ink", error)
                    android.app.AlertDialog.Builder(this)
                        .setTitle("App stopped")
                        .setMessage("Close the app and open it again to try again.")
                        .setPositiveButton("Close") { _, _ -> finish() }
                        .setCancelable(false)
                        .show()
                }
                return@Runnable
            }
            val light = InkNative.nativeIsLightAppearance(engineHandle)
            textInputAdapter.setLightAppearance(light)
            externalAdapter.setLightAppearance(light)
            val barColour = if (light) Color.WHITE else Color.BLACK
            if (window.statusBarColor != barColour) window.statusBarColor = barColour
            if (window.navigationBarColor != barColour) window.navigationBarColor = barColour
            syncTextInput()
            syncNativePortals()
            drainNativeRequests()
            InkNative.nativeTakeJavaScriptCalls(engineHandle)?.let { encoded ->
                pendingJavaScriptCalls = org.json.JSONArray(encoded)
                javascriptCallIndex = 0
                drainJavaScriptCalls()
            }
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
        pendingJavaScriptCalls = null
        nativeTimeouts.keys.toList().forEach(::cancelNativeRequest)
        nativeRequestHandler.removeCallbacks(drainNativeRequestsLater)
        javascriptControllers.toMap().forEach { (controller, module) ->
            val complete: NativeResultHandler = { disposeNativeResult(it) }
            when (module) {
                AUDIO_MODULE -> audioAdapter.executeController(-controller, "deactivate", "{}", complete)
                NOTIFICATIONS_MODULE -> notificationsAdapter.executeController(-controller, "deactivate", "{}", complete)
                CAMERA_MODULE -> cameraAdapter.executeController(0, -controller, "deactivate", "{}", complete)
                "maps" -> mapsAdapter.executeController(0, -controller, "deactivate", "{}", complete)
                "video" -> videoAdapter.executeController(0, -controller, "deactivate", "{}", complete)
                "downloads" -> downloadsAdapter.executeController(-controller, "deactivate", "{}", complete)
                else -> (appNativeAdapters[module] as? NativeControllerAdapter)?.executeController(-controller, "deactivate", "{}", complete)
            }
        }
        javascriptControllers.clear()
        networkAdapter.reset()
        connectivityAdapter.reset()
        overlaysAdapter.stop()
        javascriptRequests = newJavaScriptRequests()
    }

    internal fun performInteractionHaptic() {
        lightSdkAdapter.performHaptic(inkView)
    }

    private fun executeJavaScriptCall(call: JSONObject) {
        val id = call.getLong("id")
        if (call.getString("type") == "cancel") {
            javascriptRequests.cancel(id)
            return
        }
        val bytes = if (call.optBoolean("binary")) InkNative.nativeTakeJavaScriptBytes(engineHandle, id) else null
        if (call.optBoolean("binary") && bytes == null) {
            sendJavaScriptResult(id, NativeResult.Failure(NativeErrorKind.PROTOCOL, "Missing binary request data", false))
            return
        }
        val module = call.getString("module")
        if (module == "interaction" && call.getString("operation") == "haptic") {
            performInteractionHaptic()
            sendJavaScriptResult(id, NativeResult.Success(""))
            return
        }
        val adapter = when (module) {
            "permissions" -> { usesPermissions = true; permissionsAdapter }
            "clipboard" -> clipboardAdapter
            "connectivity" -> connectivityAdapter
            "files" -> filesAdapter
            "secure-store" -> secureStoreAdapter
            "auth" -> authAdapter
            "external" -> externalAdapter
            "overlays" -> overlaysAdapter
            "downloads" -> downloadsAdapter
            "video" -> videoAdapter
            "maps" -> mapsAdapter
            NETWORK_MODULE -> networkAdapter
            LIGHT_SDK_MODULE -> lightSdkAdapter
            AUDIO_MODULE -> audioAdapter
            LOCATION_MODULE -> locationAdapter
            NFC_MODULE -> nfcAdapter
            CAMERA_MODULE -> cameraAdapter
            NOTIFICATIONS_MODULE -> notificationsAdapter
            BACKGROUND_MODULE -> backgroundAdapter
            else -> appNativeAdapters[module]
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
                    "video" -> videoAdapter.executeController(-id, -controller, operation, payload, finish)
                    "downloads" -> downloadsAdapter.executeController(-controller, operation, payload, finish)
                    CAMERA_MODULE -> {
                        if (operation == OPEN_OPERATION) openCameraController(-id, -controller, payload, finish)
                        else cameraAdapter.executeController(-id, -controller, operation, payload, finish)
                    }
                    else -> if (adapter is NativeControllerAdapter) adapter.executeController(-controller, operation, payload, finish)
                        else finish(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unsupported controller module: $module", false))
                }
            } else {
                if (bytes != null) adapter.executeBytes(-id, operation, payload, bytes, complete)
                else adapter.execute(-id, operation, payload, complete)
            }
        }
    }

    internal fun updatePlaybackClock(controller: Long, position: Long, duration: Long, playing: Boolean, speed: Float) {
        if (engineHandle != 0L && InkNative.nativePlaybackClock(engineHandle, controller, position / 1000f, duration / 1000f, playing, speed)) inkView.requestFrame()
    }

    internal fun removePlaybackClock(controller: Long) {
        if (engineHandle != 0L) InkNative.nativeRemovePlaybackClock(engineHandle, controller)
    }

    internal fun observeAudioMeasurements(controller: Long, enabled: Boolean): Boolean =
        engineHandle != 0L && InkNative.nativeAudioObserve(engineHandle, controller, enabled)

    internal fun updateCaptureState(controller: Long, value: String) {
        if (engineHandle != 0L && InkNative.nativeCaptureState(engineHandle, -controller, value)) inkView.requestFrame()
    }

    internal fun removeCaptureState(controller: Long) {
        if (engineHandle != 0L) InkNative.nativeCaptureState(engineHandle, -controller, "")
    }

    internal fun updateController(controller: Long, value: String) {
        runOnUiThread {
            if (engineHandle != 0L && javascriptControllers.containsKey(-controller)) {
                val event = JSONObject().put("type", "controller").put("id", -controller)
                    .put("value", JSONObject(value))
                var message = event.toString()
                if (message.toByteArray(Charsets.UTF_8).size > 1024 * 1024) {
                    message = event.put("value", JSONObject().put("status", "error")
                        .put("error", inkError("protocol", "Native controller state exceeds the message limit"))).toString()
                }
                InkNative.nativeJavaScriptReceive(engineHandle, message)
            }
        }
    }

    private fun sendJavaScriptResult(id: Long, result: NativeResult) {
        if (engineHandle == 0L) return
        val message = javascriptResult(id, result)
        if (result is NativeResult.Binary) {
            if (!InkNative.nativeJavaScriptReceiveBytes(engineHandle, message, result.bytes)) Log.e("Ink", "Could not deliver native binary result")
        } else InkNative.nativeJavaScriptReceive(engineHandle, message)
    }

    override fun onJavaScriptReady() {
        if (javascriptPending.compareAndSet(false, true)) {
            nativeRequestHandler.post(drainJavaScript)
        }
    }

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
        overlaysAdapter.resume()
        externalAdapter.onResume()
        connectivityAdapter.start()
        mapsAdapter.resume()
        videoAdapter.resume()
        audioAdapter.resume()
        locationAdapter.resume()
        lightSdkAdapter.refresh()

        cameraAdapter.resume()
        syncNativePortals()
        nfcAdapter.resume()
        notificationsAdapter.refreshEvents()
        syncTextInput()
        if (engineHandle != 0L) InkNative.nativeJavaScriptReceive(engineHandle, "{\"type\":\"resume\"}")
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
        if (usesPermissions) permissionsAdapter.result(requestCode)

    }

    override fun onDestroy() {
        nativeRequestHandler.removeCallbacks(drainNativeRequestsLater)
        textInputAdapter.close()
        onBackInvokedDispatcher.unregisterOnBackInvokedCallback(backCallback)
        if (usesPermissions) permissionsAdapter.stop()
        lightSdkAdapter.stop()
        notificationsAdapter.stop()
        stopJavaScriptSession()
        if (usesBarcode) barcodeAdapter.stop()
        networkAdapter.stop()
        audioAdapter.stop()
        locationAdapter.stop()
        nfcAdapter.stop()
        backgroundAdapter.stop()
        cameraAdapter.stop()
        mapsAdapter.stop()
        videoAdapter.stop()
        filesAdapter.stop()
        externalAdapter.close()
        authAdapter.close()
        connectivityAdapter.close()
        downloadsAdapter.stop()
        imageExecutor.shutdown()
        nativeTimeouts.values.forEach(nativeRequestHandler::removeCallbacks)
        nativeTimeouts.clear()
        nativeRequestStartedAt.clear()
        nativeRequestLabels.clear()
        host.close()
        nativeRequestHandler.removeCallbacks(drainJavaScript)
        super.onDestroy()
    }

    override fun onPause() {
        if (engineHandle != 0L) InkNative.nativeJavaScriptReceive(engineHandle, "{\"type\":\"pause\"}")
        textInputAdapter.pause()
        externalAdapter.onPause()
        connectivityAdapter.stop()
        mapsAdapter.pause()
        videoAdapter.pause()
        audioAdapter.pause()
        nfcAdapter.pause()
        cameraAdapter.pause()
        locationAdapter.pause()
        super.onPause()
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
            is TextEdit.Update -> TEXT_INPUT_UPDATE to edit.payload
            TextEdit.Submit -> TEXT_INPUT_SUBMIT to null
            TextEdit.Dismiss -> TEXT_INPUT_DISMISS to null
        }
        if (InkNative.nativeTextInput(engineHandle, action, value)) {
            inkView.requestFrame()
        }
        syncTextInput()
    }

    internal fun setKeyboardInset(height: Int) {
        if (engineHandle == 0L) return
        if (InkNative.nativeSetKeyboardInset(engineHandle, height)) {
            logResource("Keyboard inset=$height surface=${inkView.width}x${inkView.height}")
            syncTextInput()
            inkView.requestFrame()
        }
    }

    private fun syncTextInput() {
        if (!BuildConfig.INK_TEXT_INPUT_ENABLED || engineHandle == 0L) return
        val state = InkNative.nativeTextInputState(engineHandle)
        textInputAdapter.sync(state and 1 != 0, state ushr 2, state and 2 != 0)
        textInputAdapter.syncContext(InkNative.nativeTextInputContext(engineHandle))
    }

    private fun syncNativePortals() {
        if (engineHandle == 0L) {
            return
        }
        // Null means unchanged; still sync adapters so recreated views can mount.
        if (BuildConfig.INK_CAMERA_ENABLED) {
            val encoded = InkNative.nativeCameraPortal(engineHandle)
            if (encoded != null) cameraPortal = if (encoded.isEmpty()) {
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
            cameraAdapter.syncPortal(cameraPortal, InkNative.nativeCameraReviewReady(engineHandle))
        }
        if (BuildConfig.INK_VIDEO_ENABLED) {
            val video = InkNative.nativeVideoPortal(engineHandle)
            if (video != null) videoPortal = if (video.isEmpty()) null else {
                val value = JSONObject(video)
                VideoPortal(value.getLong("controller"), value.getInt("x"), value.getInt("y"), value.getInt("width"), value.getInt("height"))
            }
            videoAdapter.syncPortal(videoPortal)
        }
        if (BuildConfig.INK_MAPS_ENABLED) {
            val map = InkNative.nativeMapPortal(engineHandle)
            if (map != null) mapPortal = if (map.isEmpty()) null else {
                val value = JSONObject(map)
                MapPortal(value.getLong("controller"), value.getInt("x"), value.getInt("y"), value.getInt("width"), value.getInt("height"))
            }
            mapsAdapter.syncPortal(mapPortal)
        }
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

    private val drainNativeRequestsLater = Runnable { drainNativeRequests() }
    private var drainingNativeRequests = false
    private fun cancelNativeRequest(requestId: Long) {
        nativeTimeouts.remove(requestId)?.let(nativeRequestHandler::removeCallbacks)
        nativeRequestStartedAt.remove(requestId)
        nativeRequestLabels.remove(requestId)
        lightSdkAdapter.cancel(requestId)
        if (usesBarcode) barcodeAdapter.cancel(requestId)
        networkAdapter.cancel(requestId)
        audioAdapter.cancel(requestId)
        locationAdapter.cancel(requestId)
        nfcAdapter.cancel(requestId)
        backgroundAdapter.cancel(requestId)
        notificationsAdapter.cancel(requestId)
        cameraAdapter.cancel(requestId)
        filesAdapter.cancel(requestId)
    }

    private fun drainNativeRequests() {
        if (engineHandle == 0L || drainingNativeRequests) return
        drainingNativeRequests = true
        try {
            nativeRequestHandler.removeCallbacks(drainNativeRequestsLater)
            val deadline = SystemClock.uptimeMillis() + 4
            var handled = 0
            while (engineHandle != 0L && handled++ < 32 && SystemClock.uptimeMillis() < deadline) {
                val request = InkNative.nativeNextRequest(engineHandle)?.let(::JSONObject) ?: return
                if (request.optBoolean("yield")) break
                val requestId = request.getLong("id")
                val kind = request.getInt("kind")
                if (kind == NATIVE_REQUEST_CANCEL) {
                    val label = nativeRequestLabels.remove(requestId).orEmpty()
                    val elapsed = nativeRequestStartedAt.remove(requestId)?.let {
                        SystemClock.elapsedRealtime() - it
                    }
                    logResource("cancel $requestId $label ${elapsed ?: 0}ms")
                    cancelNativeRequest(requestId)
                    continue
                }
                val module = request.getString("module")
                val operation = request.getString("operation")
                val payload = request.getString("payload")
                val controller = request.getLong("controller")
                val label = "$module/$operation"
                nativeRequestStartedAt[requestId] = SystemClock.elapsedRealtime()
                nativeRequestLabels[requestId] = label
                logResource("start $requestId $label")
                if (BuildConfig.INK_PRESENTATION_TIMING && kind == NATIVE_REQUEST_IMAGE) {
                    val provenance = JSONObject(payload)
                    Log.i("Ink", "InkImageRequest " + JSONObject().put("version", 1)
                        .put("request_id", requestId).put("module", module).put("operation", operation)
                        .put("source", provenance.optString("url", provenance.optString("source")))
                        .put("ns", System.nanoTime()))
                }
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
                    "assets" -> assetsAdapter
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
                    request.getLong("timeout"),
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
            nativeRequestHandler.post(drainNativeRequestsLater)
        } finally { drainingNativeRequests = false }
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
            InkNative.nativeJavaScriptReceive(engineHandle, message.toString())
        }
        intent?.removeExtra(EXTRA_NOTIFICATION_HREF)
        intent?.removeExtra(EXTRA_NOTIFICATION_PARAMS)
    }

    private fun completeNativeRequest(requestId: Long, kind: Int, result: NativeResult) {
        if (engineHandle == 0L) {
            disposeNativeResult(result)
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
            is NativeResult.Success, is NativeResult.Bytes, is NativeResult.Binary -> InkNative.nativeCompleteAction(engineHandle, requestId)
            is NativeResult.Pixels -> InkNative.nativeCompletePixels(engineHandle, requestId, result.width, result.height, result.rgba)
            is NativeResult.File, is NativeResult.Asset, is NativeResult.EncodedImage -> {
                completeImage(requestId, result)
                return
            }
            is NativeResult.Failure -> if (
                kind == NATIVE_REQUEST_IMAGE
            ) {
                InkNative.nativeFailRequest(
                    engineHandle,
                    requestId,
                    result.kind.code,
                    result.message,
                    result.retryable,
                )
            } else {
                InkNative.nativeCompleteAction(engineHandle, requestId)
            }
        }
        if (changed) inkView.requestFrame()
        syncNativePortals()
        drainNativeRequests()
    }

    private fun completeImage(requestId: Long, result: NativeResult) {
        val handle = engineHandle
        imageExecutor.execute {
            val update = when (result) {
                is NativeResult.Asset -> InkNative.nativeCompleteAsset(handle, requestId, assets, result.path)
                is NativeResult.EncodedImage -> InkNative.nativeCompleteEncoded(handle, requestId, result.bytes, result.size)
                is NativeResult.File -> try {
                    InkNative.nativeCompleteFile(handle, requestId, result.path)
                } finally {
                    if (result.deleteAfterRead) File(result.path).delete()
                }
                else -> IMAGE_UNCHANGED
            }
            runOnUiThread {
                if (engineHandle != handle) return@runOnUiThread
                // Offscreen images share the next vsync frame so their uploads happen before scrolling.
                if (update == IMAGE_VISIBLE) inkView.requestCommitFrame()
                else if (update == IMAGE_OFFSCREEN) inkView.requestFrame()
                syncNativePortals()
                drainNativeRequests()
            }
        }
    }

    private fun logResource(message: String) {
        if (BuildConfig.DEBUG) Log.d(RESOURCE_LOG_TAG, message)
    }

    private companion object {
        private const val TEXT_INPUT_SUBMIT = 2
        private const val TEXT_INPUT_DISMISS = 3
        private const val TEXT_INPUT_UPDATE = 4
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
        private const val IMAGE_UNCHANGED = 0
        private const val IMAGE_VISIBLE = 1
        private const val IMAGE_OFFSCREEN = 2

    }
}
