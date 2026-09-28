package com.vandam.ink

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Color
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.camera.core.CameraSelector
import androidx.camera.core.Preview
import androidx.camera.core.UseCase
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import java.io.File
import org.json.JSONObject

internal fun createCameraAdapter(
    activity: MainActivity,
    root: FrameLayout,
    updateController: (Long, String) -> Unit,
    updateReview: (Long, String?) -> Unit,
    requestOpen: (Long) -> Unit,
): CameraAdapter = InkCameraAdapter(
    activity,
    root,
    updateController,
    updateReview,
    requestOpen,
)

internal interface CameraSessionFeature {
    val useCases: List<UseCase>
    val capturePreview: View? get() = null

    fun image(source: String): NativeResult.Pixels? = null

    fun openCamera(onReady: () -> Unit) = Unit

    fun releaseCamera() = Unit

    fun start()

    fun execute(operation: String): Boolean

    fun stop()
}

internal interface CameraSessionHost {
    val activity: MainActivity
    val preview: PreviewView
    val facing: String
    val flashColour: Int

    fun releaseCamera()

    fun rebindCamera()

    fun review(source: String, saving: Boolean = false)

    fun clearReview()

    fun emit(value: JSONObject)

    fun finish(value: JSONObject)

    fun fail(kind: String, message: String, retryable: Boolean)
}

private class InkCameraAdapter(
    private val activity: MainActivity,
    private val root: FrameLayout,
    private val updateController: (Long, String) -> Unit,
    private val updateReview: (Long, String?) -> Unit,
    private val requestOpen: (Long) -> Unit,
) : CameraAdapter {
    private val controllers = mutableMapOf<Long, ControllerRecipe>()
    private val readyStates = mutableMapOf<Long, String>()
    private val pendingRequests = mutableMapOf<Long, Long>()
    private val autoOpenRequested = mutableSetOf<Long>()
    private val photoDirectory = File(activity.noBackupFilesDir, PHOTO_DIRECTORY).apply {
        mkdirs()
        listFiles().orEmpty().filter { it.extension == "partial" }.forEach(File::delete)
    }
    private var portal: CameraPortal? = null
    private var session: CameraSession? = null
    private var paused = false

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (operation) {
            PERMISSION_STATUS -> complete(NativeResult.Success(permissionStatus()))
            REQUEST_PERMISSION -> requestPermission(complete)
            IMAGE -> loadImage(payload, complete)
            "remove-photo" -> removePhoto(payload, complete)
            else -> complete(protocol("Unknown camera operation: $operation"))
        }
    }

    override fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (operation) {
            ACTIVATE -> activate(controller, payload, complete)
            DEACTIVATE -> deactivate(controller, complete)
            OPEN -> open(requestId, controller, complete)
            RETAKE, USE_PHOTO, "capture" -> {
                val active = session?.takeIf { it.controller == controller }
                if (active == null || !active.execute(operation)) {
                    complete(protocol("Camera session cannot $operation now"))
                } else {
                    complete(NativeResult.Success(""))
                }
            }
            else -> complete(protocol("Unknown camera session operation: $operation"))
        }
    }

    override fun permissionDenied(
        requestId: Long,
        controller: Long,
        blocked: Boolean,
        complete: NativeResultHandler,
    ) {
        autoOpenRequested.remove(controller)
        publishError(
            controller,
            if (blocked) "permission-blocked" else "permission-denied",
            if (blocked) {
                "Camera permission is blocked by LightOS"
            } else {
                "Camera permission is not granted"
            },
            !blocked,
        )
        complete(
            NativeResult.Failure(
                if (blocked) {
                    NativeErrorKind.PERMISSION_BLOCKED
                } else {
                    NativeErrorKind.PERMISSION_DENIED
                },
                "Camera permission is not available",
                !blocked,
            ),
        )
    }

    override fun cancel(requestId: Long) {
        val controller = pendingRequests.remove(requestId) ?: return
        session?.takeIf { it.controller == controller }?.fail(
            "timeout",
            "Camera took too long to open",
            true,
        )
    }

    override fun syncPortal(portal: CameraPortal?, reviewReady: Boolean) {
        this.portal = portal
        val active = session
        if (portal == null) {
            if (active?.reviewing == true) {
                if (reviewReady) active.revealReview()
                return
            }
            if (active != null && !active.awaitingPreview) {
                active.close(restore = false)
            } else {
                active?.mountPreview(null)
            }
            return
        }
        if (active != null && active.controller != portal.controller) {
            active.close(restore = true)
        }
        session?.takeIf { it.controller == portal.controller }?.mountPreview(portal)
        requestAutomaticOpen(portal.controller)
    }

    override fun pause() {
        paused = true
        session?.close(restore = true)
    }

    override fun resume() {
        paused = false
        portal?.let { requestAutomaticOpen(it.controller) }
    }

    override fun stop() {
        session?.close(restore = false)
        photoDirectory.listFiles().orEmpty().filter { it.extension == "partial" }.forEach(File::delete)
        controllers.clear()
        readyStates.clear()
        pendingRequests.clear()
        autoOpenRequested.clear()
        portal = null
    }

    private fun activate(
        controller: Long,
        payload: String,
        complete: NativeResultHandler,
    ) {
        val recipe = runCatching { JSONObject(payload) }.getOrElse {
            complete(protocol("Ink produced invalid camera session configuration"))
            return
        }
        val kind = recipe.optString(KIND)
        if (kind != PHOTO && kind != SCANNER) {
            complete(protocol("Unknown camera session: $kind"))
            return
        }
        controllers[controller] = ControllerRecipe(
            kind,
            recipe.optJSONObject(CONFIG) ?: JSONObject(),
        )
        complete(NativeResult.Success(""))
        if (portal?.controller == controller) {
            requestAutomaticOpen(controller)
        }
    }

    private fun deactivate(controller: Long, complete: NativeResultHandler) {
        session?.takeIf { it.controller == controller }?.close(restore = false)
        controllers.remove(controller)
        readyStates.remove(controller)
        autoOpenRequested.remove(controller)
        updateReview(controller, null)
        complete(NativeResult.Success(""))
    }

    private fun requestAutomaticOpen(controller: Long) {
        if (
            paused ||
            session != null ||
            controllers[controller] == null ||
            !autoOpenRequested.add(controller)
        ) {
            return
        }
        requestOpen(controller)
    }

    private fun open(
        requestId: Long,
        controller: Long,
        complete: NativeResultHandler,
    ) {
        autoOpenRequested.remove(controller)
        val recipe = controllers[controller]
        if (recipe == null) {
            complete(protocol("Camera session is not active"))
            return
        }
        val active = session
        if (active != null) {
            if (active.controller == controller) {
                complete(NativeResult.Success(""))
            } else {
                publishError(controller, "busy", "Another camera session is open", true)
                complete(
                    NativeResult.Failure(
                        NativeErrorKind.UNAVAILABLE,
                        "Another camera session is open",
                        true,
                    ),
                )
            }
            return
        }
        if (paused) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Camera is paused", true))
            return
        }
        if (activity.checkSelfPermission(Manifest.permission.CAMERA) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            permissionDenied(requestId, controller, false, complete)
            return
        }
        publish(controller, state(controller, "opening"))
        if (requestId >= 0L) {
            pendingRequests[requestId] = controller
        }
        CameraSession(requestId, controller, recipe, complete).also {
            session = it
            it.mountPreview(portal?.takeIf { value -> value.controller == controller })
            it.open()
        }
    }

    private fun requestPermission(complete: NativeResultHandler) {
        activity.getSharedPreferences(PREFERENCES, 0)
            .edit()
            .putBoolean(REQUESTED, true)
            .apply()
        activity.requestPermissions(arrayOf(Manifest.permission.CAMERA), REQUEST_CODE)
        complete(NativeResult.Success(""))
    }

    private fun permissionStatus(): String {
        if (activity.checkSelfPermission(Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return "granted"
        }
        val requested = activity.getSharedPreferences(PREFERENCES, 0)
            .getBoolean(REQUESTED, false)
        return when {
            !requested -> "unknown"
            activity.shouldShowRequestPermissionRationale(Manifest.permission.CAMERA) -> "denied"
            else -> "blocked"
        }
    }

    private fun loadImage(payload: String, complete: NativeResultHandler) {
        val source = runCatching { JSONObject(payload).getString(SOURCE) }.getOrNull()
        val (id, suffix) = when {
            source?.startsWith(PHOTO_PREFIX) == true ->
                source.removePrefix(PHOTO_PREFIX) to ".jpg"
            source?.startsWith(REVIEW_PREFIX) == true ->
                source.removePrefix(REVIEW_PREFIX) to ".partial"
            else -> null to null
        }
        if (source == null || id == null || suffix == null || !PHOTO_ID.matches(id)) {
            complete(protocol("Ink produced an invalid captured photo source"))
            return
        }
        session?.image(source)?.let {
            complete(it)
            return
        }
        val file = File(photoDirectory, "$id$suffix")
        if (!file.isFile) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    "Captured photo is no longer available",
                    false,
                ),
            )
            return
        }
        complete(NativeResult.File(file.absolutePath, deleteAfterRead = false))
    }

    private fun publish(controller: Long, value: String) {
        updateController(controller, value)
    }

    private fun publishReady(controller: Long, value: JSONObject) {
        val state = state(controller, "ready", value)
        readyStates[controller] = state
        publish(controller, state)
    }

    private fun removePhoto(payload: String, complete: NativeResultHandler) {
        val source = runCatching { JSONObject(payload).getString(SOURCE) }.getOrNull()
        val id = source?.takeIf { it.startsWith(PHOTO_PREFIX) }?.removePrefix(PHOTO_PREFIX)
        if (id == null || !PHOTO_ID.matches(id)) {
            complete(protocol("Invalid captured photo source"))
            return
        }
        val file = File(photoDirectory, "$id.jpg")
        if (file.exists() && !file.delete()) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Photo could not be removed", true))
        } else {
            runCatching { InkManagedFiles(activity).remove(id) }
                .onSuccess { complete(NativeResult.Success("")) }
                .onFailure { complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, it.message ?: "Photo metadata could not be removed", true)) }
        }
    }

    private fun publishError(
        controller: Long,
        kind: String,
        message: String,
        retryable: Boolean,
    ) {
        publish(
            controller,
            state(
                controller,
                "error",
                error = JSONObject()
                    .put(KIND, kind)
                    .put(MESSAGE, message)
                    .put(RETRYABLE, retryable),
            ),
        )
    }

    private fun restore(controller: Long) {
        publish(controller, readyStates[controller] ?: state(controller, "idle"))
    }

    private fun state(
        controller: Long,
        status: String,
        value: JSONObject = emptyValue(controller),
        error: JSONObject = emptyError(),
    ): String = JSONObject()
        .put(STATUS, status)
        .put(VALUE, value)
        .put(ERROR, error)
        .toString()

    private fun emptyValue(controller: Long): JSONObject =
        if (controllers[controller]?.kind == SCANNER) {
            JSONObject()
                .put(TEXT, "")
                .put(FORMAT, "qr")
        } else {
            JSONObject()
                .put(SOURCE, "")
                .put(WIDTH, 0)
                .put(HEIGHT, 0)
                .put(MIME_TYPE, "image/jpeg")
                .put(CAPTURED_AT_MS, 0)
        }

    private fun emptyError() = JSONObject()
        .put(KIND, "unexpected")
        .put(MESSAGE, "")
        .put(RETRYABLE, false)

    private fun protocol(message: String) = NativeResult.Failure(
        NativeErrorKind.PROTOCOL,
        message,
        false,
    )

    private inner class CameraSession(
        val requestId: Long,
        val controller: Long,
        private val recipe: ControllerRecipe,
        private val complete: NativeResultHandler,
    ) : CameraSessionHost {
        override val activity = this@InkCameraAdapter.activity
        override val facing = recipe.config.optString("facing", "back")
        override val flashColour get() = if (portal?.light == true) Color.WHITE else Color.BLACK
        override var preview = createPreviewView()
            private set
        private val previewHost = FrameLayout(activity).apply {
            clipChildren = true
            clipToPadding = true
            addView(
                preview,
                FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
        }
        private val lifecycleOwner = CameraLifecycleOwner()
        private val cameraPreview = Preview.Builder().build()
        private val selector = if (facing == "front") {
            CameraSelector.DEFAULT_FRONT_CAMERA
        } else CameraSelector.DEFAULT_BACK_CAMERA
        private var provider: ProcessCameraProvider? = null
        private var feature: CameraSessionFeature? = null
        private var requestCompleted = false
        private var closed = false
        var awaitingPreview = true
            private set
        var reviewing = false
            private set

        fun open() {
            cameraPreview.surfaceProvider = preview.surfaceProvider
            val future = ProcessCameraProvider.getInstance(activity)
            future.addListener(
                {
                    if (closed || session !== this) {
                        return@addListener
                    }
                    val cameraProvider = runCatching { future.get() }.getOrElse { error ->
                        fail("unavailable", error.message ?: "Camera is unavailable", true)
                        return@addListener
                    }
                    provider = cameraProvider
                    if (!runCatching {
                            cameraProvider.hasCamera(selector)
                        }.getOrDefault(false)
                    ) {
                        fail("unavailable", "The requested camera is unavailable", false)
                        return@addListener
                    }
                    val selected = if (recipe.kind == PHOTO) {
                        createPhotoCaptureFeature(this, photoDirectory)
                    } else {
                        createCodeScannerFeature(this, recipe.config)
                    }
                    if (selected == null) {
                        fail("unavailable", "This camera capability is not packaged", false)
                        return@addListener
                    }
                    feature = selected
                    selected.capturePreview?.let { view ->
                        previewHost.removeAllViews()
                        previewHost.addView(view, FrameLayout.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.MATCH_PARENT,
                        ))
                    }
                    lifecycleOwner.start()
                    selected.start()
                    bindCameraAfterLayout {
                        publish(controller, state(controller, "active"))
                        completeRequest(NativeResult.Success(""))
                    }
                },
                ContextCompat.getMainExecutor(activity),
            )
        }

        fun mountPreview(portal: CameraPortal?) {
            if (reviewing && !closed) return
            if (portal == null || closed) {
                (previewHost.parent as? ViewGroup)?.removeView(previewHost)
                return
            }
            awaitingPreview = false
            val params = FrameLayout.LayoutParams(portal.width, portal.height).apply {
                leftMargin = portal.x
                topMargin = portal.y
            }
            if (previewHost.parent == null) {
                root.addView(previewHost, params)
            } else {
                previewHost.layoutParams = params
            }
        }

        fun image(source: String): NativeResult.Pixels? = feature?.image(source)

        fun execute(operation: String): Boolean = feature?.execute(operation) == true

        override fun releaseCamera() {
            feature?.releaseCamera()
            val cameraProvider = provider ?: return
            val uses = feature?.useCases.orEmpty()
            cameraProvider.unbind(cameraPreview, *uses.toTypedArray())
        }

        override fun rebindCamera() {
            awaitingPreview = true
            releaseCamera()
            reviewing = false
            if (feature?.capturePreview != null) {
                feature?.start()
                updateReview(controller, null)
                mountPreview(portal?.takeIf { it.controller == controller })
                bindCameraAfterLayout {}
                return
            }
            (previewHost.parent as? ViewGroup)?.removeView(previewHost)
            previewHost.removeAllViews()
            preview = createPreviewView()
            previewHost.addView(
                preview,
                FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
            cameraPreview.surfaceProvider = preview.surfaceProvider
            feature?.start()
            updateReview(controller, null)
            mountPreview(portal?.takeIf { it.controller == controller })
            bindCameraAfterLayout {}
        }

        override fun review(source: String, saving: Boolean) {
            reviewing = true
            publish(controller, JSONObject()
                .put("status", "review")
                .put("reviewSource", source)
                .put("saving", saving).toString())
        }

        fun revealReview() {
            if (previewHost.parent == null) return
            releaseCamera()
            (previewHost.parent as? ViewGroup)?.removeView(previewHost)
        }

        override fun clearReview() {
            reviewing = false
            updateReview(controller, null)
        }

        override fun emit(value: JSONObject) {
            if (!closed) publish(controller, state(controller, "active", value))
        }

        override fun finish(value: JSONObject) {
            if (closed) {
                return
            }
            publishReady(controller, value)
            clearReview()
            close(restore = false)
        }

        override fun fail(kind: String, message: String, retryable: Boolean) {
            if (closed) {
                return
            }
            publishError(controller, kind, message, retryable)
            clearReview()
            completeRequest(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    message,
                    retryable,
                ),
            )
            close(restore = false)
        }

        fun close(restore: Boolean) {
            if (closed) {
                return
            }
            closed = true
            feature?.stop()
            releaseCamera()
            lifecycleOwner.stop()
            (previewHost.parent as? ViewGroup)?.removeView(previewHost)
            updateReview(controller, null)
            completeRequest(NativeResult.Success(""))
            if (restore) {
                restore(controller)
            }
            if (session === this) {
                session = null
            }
        }

        private fun bindCamera(): Boolean {
            val cameraProvider = provider ?: return false
            val useCases = feature?.useCases.orEmpty()
            return runCatching {
                cameraProvider.bindToLifecycle(
                    lifecycleOwner,
                    selector,
                    cameraPreview,
                    *useCases.toTypedArray(),
                )
            }.isSuccess
        }

        private fun bindCameraAfterLayout(onBound: () -> Unit) {
            val target = feature?.capturePreview ?: preview
            if (target.isAttachedToWindow && target.isLaidOut && target.width > 0 && target.height > 0) {
                finishBinding(onBound)
                return
            }
            target.addOnLayoutChangeListener(
                object : View.OnLayoutChangeListener {
                    override fun onLayoutChange(
                        view: View,
                        left: Int,
                        top: Int,
                        right: Int,
                        bottom: Int,
                        oldLeft: Int,
                        oldTop: Int,
                        oldRight: Int,
                        oldBottom: Int,
                    ) {
                        if (!target.isAttachedToWindow || right <= left || bottom <= top) return
                        target.removeOnLayoutChangeListener(this)
                        if (target === (feature?.capturePreview ?: preview)) {
                            finishBinding(onBound)
                        }
                    }
                },
            )
        }

        private fun finishBinding(onBound: () -> Unit) {
            if (closed || reviewing) {
                return
            }
            val active = feature
            if (active?.capturePreview != null) {
                active.openCamera(onBound)
                return
            }
            if (!bindCamera()) {
                fail("unavailable", "Camera could not be opened", true)
                return
            }
            onBound()
        }

        private fun createPreviewView() = PreviewView(activity).apply {
            implementationMode = PreviewView.ImplementationMode.COMPATIBLE
            scaleType = PreviewView.ScaleType.FILL_CENTER
        }

        private fun completeRequest(result: NativeResult) {
            if (requestCompleted) {
                return
            }
            requestCompleted = true
            pendingRequests.remove(requestId)
            complete(result)
        }
    }

    private data class ControllerRecipe(
        val kind: String,
        val config: JSONObject,
    )

    private companion object {
        private const val PERMISSION_STATUS = "permission-status"
        private const val REQUEST_PERMISSION = "request-permission"
        private const val IMAGE = "image"
        private const val ACTIVATE = "activate"
        private const val DEACTIVATE = "deactivate"
        private const val OPEN = "open"
        private const val RETAKE = "retake"
        private const val USE_PHOTO = "use-photo"
        private const val PHOTO = "photo"
        private const val SCANNER = "scanner"
        private const val STATUS = "status"
        private const val VALUE = "value"
        private const val ERROR = "error"
        private const val KIND = "kind"
        private const val CONFIG = "config"
        private const val MESSAGE = "message"
        private const val RETRYABLE = "retryable"
        private const val SOURCE = "source"
        private const val WIDTH = "width"
        private const val HEIGHT = "height"
        private const val MIME_TYPE = "mimeType"
        private const val CAPTURED_AT_MS = "capturedAtMs"
        private const val TEXT = "text"
        private const val FORMAT = "format"
        private const val PHOTO_DIRECTORY = "ink-camera"
        private const val PHOTO_PREFIX = "ink-camera://"
        private const val REVIEW_PREFIX = "ink-camera-review://"
        private const val PREFERENCES = "ink-camera"
        private const val REQUESTED = "camera-permission-requested"
        private const val REQUEST_CODE = 0xCA
        private val PHOTO_ID =
            Regex("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
    }
}

private class CameraLifecycleOwner : LifecycleOwner {
    private val registry = LifecycleRegistry(this)

    override val lifecycle: Lifecycle get() = registry

    fun start() {
        registry.handleLifecycleEvent(Lifecycle.Event.ON_CREATE)
        registry.handleLifecycleEvent(Lifecycle.Event.ON_START)
        registry.handleLifecycleEvent(Lifecycle.Event.ON_RESUME)
    }

    fun stop() {
        if (registry.currentState.isAtLeast(Lifecycle.State.RESUMED)) {
            registry.handleLifecycleEvent(Lifecycle.Event.ON_PAUSE)
        }
        if (registry.currentState.isAtLeast(Lifecycle.State.STARTED)) {
            registry.handleLifecycleEvent(Lifecycle.Event.ON_STOP)
        }
        if (registry.currentState.isAtLeast(Lifecycle.State.CREATED)) {
            registry.handleLifecycleEvent(Lifecycle.Event.ON_DESTROY)
        }
    }
}
