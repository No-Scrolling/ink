package com.vandam.ink

import android.app.Activity
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.fonts.Font
import android.graphics.fonts.FontFamily
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import org.json.JSONObject

internal class InkActivityBindings(
    private val activity: Activity,
    private val runtime: InkUiRuntime,
    createAppAdapters: () -> Map<String, NativeAdapter>,
) : InkUiModules {
    private val publicSansTypeface by lazy(LazyThreadSafetyMode.NONE) {
        Typeface.CustomFallbackBuilder(FontFamily.Builder(Font.Builder(InkNative.nativePublicSans()).build()).build()).build()
    }
    private val engineHandle get() = runtime.handle
    private val inkView get() = runtime.view
    val root = FrameLayout(activity).apply {
        addView(inkView, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
    }
    private val clipboardAdapter by lazy { ClipboardAdapter(activity) }
    private val permissionsAdapter by lazy { PermissionsAdapter(activity, lightSdkAdapter) }
    private var usesPermissions = false
    private val lightSdkAdapter: LightSdkAdapter
    private val assetsAdapter by lazy { AssetsAdapter(runtime.bundle) }
    private val barcodeAdapter by lazy { createBarcodeAdapter(activity) }
    private var usesBarcode = false
    private val networkAdapter: NetworkAdapter
    private val audioAdapter: AudioAdapter
    private val locationAdapter: LocationAdapter
    private val nfcAdapter: NfcAdapter
    private val backgroundAdapter: BackgroundAdapter
    private val cameraAdapter: CameraAdapter
    private val mapsAdapter: MapsAdapter
    private val videoAdapter: VideoAdapter
    private var cameraPortal: CameraPortal? = null
    private var videoPortal: VideoPortal? = null
    private var mapPortal: MapPortal? = null
    private val filesAdapter by lazy { createFilesAdapter(activity, runtime::send) }
    private val appNativeAdapters by lazy(createAppAdapters)
    private val secureStoreAdapter by lazy { SecureStoreAdapter(activity) }
    private val authAdapter by lazy { AuthAdapter(activity, runtime::send) }
    private val externalAdapter by lazy { ExternalAdapter(activity) }
    private val overlaysAdapter by lazy { createOverlaysAdapter(activity, runtime::send) }
    private val downloadsAdapter by lazy { createDownloadsAdapter(activity, runtime::updateController) }
    private val connectivityAdapter by lazy { ConnectivityAdapter(activity, runtime::send) }
    private val textInputAdapter: TextInputAdapter
    private val notificationsAdapter: NotificationsAdapter
    private var adaptersWarmed = false
    override val presentation = object : InkHost.Callbacks {
        override fun changed() { syncNativePortals(); syncTextInput() }
        override fun beforeRender() { if (textInputAdapter.hasEditor) syncTextInput() }
        override fun presented() {
            textInputAdapter.framePresented()
            if (!adaptersWarmed) {
                adaptersWarmed = true
                networkAdapter.warm()
                audioAdapter.warm()
            }
        }
        override fun haptic(view: View) = lightSdkAdapter.performHaptic(view)
    }

    init {
        textInputAdapter = createTextInputAdapter(activity, root, ::handleTextEdit, { publicSansTypeface }, ::setKeyboardInset)
        lightSdkAdapter = createLightSdkAdapter(activity, runtime::updateController)
        networkAdapter = createNetworkAdapter(activity)
        mapsAdapter = createMapsAdapter(activity, root, runtime::updateController)
        videoAdapter = createVideoAdapter(activity, root, runtime::updateController, runtime.bundle, runtime)
        locationAdapter = createLocationAdapter(activity)
        nfcAdapter = createNfcAdapter(activity)
        backgroundAdapter = createBackgroundAdapter(activity)
        cameraAdapter = createCameraAdapter(
            activity,
            root,
            runtime::updateController,
            { controller, source ->
                runtime.updateController(controller, JSONObject()
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
        audioAdapter = createAudioAdapter(activity, runtime.bundle, runtime, runtime, runtime::updateController)
        notificationsAdapter = createNotificationsAdapter(activity, { lightSdkAdapter.performHaptic(inkView) }, runtime::updateController)
        lightSdkAdapter.start()
        notificationsAdapter.start()
    }

    override fun adapter(module: String): NativeAdapter? = when (module) {
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

    override fun controller(requestId: Long, controller: Long, module: String, operation: String, payload: String, complete: NativeResultHandler) {
        when (module) {
            AUDIO_MODULE -> audioAdapter.executeController(controller, operation, payload, complete)
            NOTIFICATIONS_MODULE -> notificationsAdapter.executeController(controller, operation, payload, complete)
            "maps" -> mapsAdapter.executeController(requestId, controller, operation, payload, complete)
            "video" -> videoAdapter.executeController(requestId, controller, operation, payload, complete)
            "downloads" -> downloadsAdapter.executeController(controller, operation, payload, complete)
            CAMERA_MODULE -> if (operation == OPEN_OPERATION) openCameraController(requestId, controller, payload, complete)
                else cameraAdapter.executeController(requestId, controller, operation, payload, complete)
            else -> super.controller(requestId, controller, module, operation, payload, complete)
        }
    }

    override fun resetSession() {
        networkAdapter.reset()
        connectivityAdapter.reset()
        overlaysAdapter.stop()
    }

    override fun appearanceChanged(light: Boolean) {
        textInputAdapter.setLightAppearance(light)
        externalAdapter.setLightAppearance(light)
        val barColour = if (light) Color.WHITE else Color.BLACK
        if (activity.window.statusBarColor != barColour) activity.window.statusBarColor = barColour
        if (activity.window.navigationBarColor != barColour) activity.window.navigationBarColor = barColour
    }

    fun forwardDeviceKey(event: KeyEvent) = lightSdkAdapter.forwardDeviceKey(event)
    fun back(): Boolean {
        inkView.stopScrolling()
        return textInputAdapter.dismiss() || runtime.back()
    }

    fun resume() {
        overlaysAdapter.resume()
        externalAdapter.onResume()
        connectivityAdapter.start()
        filesAdapter.resume()
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
        runtime.send("{\"type\":\"resume\"}")
    }

    fun activityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        filesAdapter.onActivityResult(requestCode, resultCode, data)
        permissionResult(requestCode)
    }

    fun permissionResult(requestCode: Int) { if (usesPermissions) permissionsAdapter.result(requestCode) }

    fun pause() {
        runtime.send("{\"type\":\"pause\"}")
        textInputAdapter.pause()
        externalAdapter.onPause()
        connectivityAdapter.stop()
        mapsAdapter.pause()
        videoAdapter.pause()
        audioAdapter.pause()
        nfcAdapter.pause()
        cameraAdapter.pause()
        locationAdapter.pause()
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
            if (BuildConfig.DEBUG) android.util.Log.d("InkResource", "Keyboard inset=$height surface=${inkView.width}x${inkView.height}")
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
                VideoPortal(value.getLong("controller"), value.getInt("x"), value.getInt("y"), value.getInt("width"), value.getInt("height"),
                    android.graphics.Rect(value.getInt("clipX"), value.getInt("clipY"),
                        value.getInt("clipX") + value.getInt("clipWidth"), value.getInt("clipY") + value.getInt("clipHeight")))
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
            activity.runOnUiThread {
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

    override fun cancelScene(requestId: Long) {
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

    override fun scene(request: InkSceneRequest, complete: NativeResultHandler) {
        val (requestId, _, module, operation, payload, controller) = request
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
            complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown native module: $module", false))
            return
        }
        if (lightCameraPermission) {
            lightSdkAdapter.execute(requestId, operation, CAMERA_PERMISSION) { result ->
                if (result is NativeResult.Failure &&
                    result.kind == NativeErrorKind.UNAVAILABLE
                ) {
                    activity.runOnUiThread {
                        cameraAdapter.execute(requestId, operation, payload, complete)
                    }
                } else {
                    complete(result)
                }
            }
        } else if (lightAudioPermission) {
            lightSdkAdapter.execute(requestId, operation, MICROPHONE_PERMISSION) { result ->
                if (result is NativeResult.Failure &&
                    result.kind == NativeErrorKind.UNAVAILABLE
                ) {
                    activity.runOnUiThread {
                        audioAdapter.execute(requestId, operation, payload, complete)
                    }
                } else {
                    complete(result)
                }
            }
        } else if (controller >= 0L && adapter === audioAdapter) {
            audioAdapter.executeController(controller, operation, payload, complete)
        } else if (controller >= 0L && adapter === lightSdkAdapter) {
            lightSdkAdapter.executeController(
                requestId,
                controller,
                operation,
                payload,
                complete,
            )
        } else if (controller >= 0L && adapter === notificationsAdapter) {
            notificationsAdapter.executeController(controller, operation, payload, complete)
        } else if (controller >= 0L && adapter === cameraAdapter) {
            if (operation != OPEN_OPERATION) {
                cameraAdapter.executeController(
                    requestId,
                    controller,
                    operation,
                    payload,
                    complete,
                )
            } else {
                openCameraController(
                    requestId,
                    controller,
                    payload,
                    complete,
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
                    complete(result)
                } else {
                    lightSdkAdapter.execute(
                        requestId,
                        PERMISSION_STATUS_OPERATION,
                        permission,
                    ) { permissionResult ->
                        complete(
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
            adapter.execute(requestId, operation, payload, complete)
        }
    }

    fun intent(intent: Intent?) {
        if (intent != null) externalAdapter.handleIntent(intent)
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
            runtime.send(message.toString())
        }
        intent?.removeExtra(EXTRA_NOTIFICATION_HREF)
        intent?.removeExtra(EXTRA_NOTIFICATION_PARAMS)
    }

    override fun close() {
        textInputAdapter.close()
        if (usesPermissions) permissionsAdapter.stop()
        lightSdkAdapter.stop()
        notificationsAdapter.stop()
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
    }

    private companion object {
        private const val TEXT_INPUT_SUBMIT = 2
        private const val TEXT_INPUT_DISMISS = 3
        private const val TEXT_INPUT_UPDATE = 4
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

    }
}
