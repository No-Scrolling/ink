package com.vandam.ink

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.ImageFormat
import android.graphics.Matrix
import android.graphics.Rect
import android.graphics.SurfaceTexture
import android.graphics.YuvImage
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureFailure
import android.hardware.camera2.CaptureRequest
import android.hardware.camera2.params.OutputConfiguration
import android.hardware.camera2.params.SessionConfiguration
import android.media.Image
import android.media.ImageReader
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Size
import android.view.Surface
import android.view.TextureView
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.util.concurrent.Executors
import kotlin.math.abs
import kotlin.math.max

internal class InkDirectPhotoCamera(
    context: Context,
    private val facing: String,
    private val onReview: (NativeResult.Pixels) -> Unit,
    private val onPhoto: (ByteArray, Int, Int, Long) -> Unit,
    private val onError: (String, String, Boolean) -> Unit,
) {
    val preview = TextureView(context)
    private val manager = context.getSystemService(CameraManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private val workerThread = HandlerThread("InkPhotoEncoder").apply { start() }
    private val worker = Handler(workerThread.looper)
    private var connection: Connection? = null
    private var ready: (() -> Unit)? = null
    private var wanted = false
    private var stopped = false
    private var captureVersion = 0

    init {
        preview.surfaceTextureListener = object : TextureView.SurfaceTextureListener {
            override fun onSurfaceTextureAvailable(texture: SurfaceTexture, width: Int, height: Int) = connect()
            override fun onSurfaceTextureSizeChanged(texture: SurfaceTexture, width: Int, height: Int) {
                connection?.transform()
            }
            override fun onSurfaceTextureDestroyed(texture: SurfaceTexture): Boolean {
                disconnect()
                return true
            }
            override fun onSurfaceTextureUpdated(texture: SurfaceTexture) = Unit
        }
    }

    fun open(onReady: () -> Unit) {
        if (stopped) return
        captureVersion++
        wanted = true
        ready = onReady
        if (preview.isAvailable) connect()
    }

    fun release() {
        wanted = false
        ready = null
        disconnect()
    }

    fun stop() {
        stopped = true
        captureVersion++
        release()
        cameraExecutor.execute { workerThread.quitSafely() }
    }

    fun capture(): Boolean = connection?.capture() ?: false

    private fun disconnect() {
        val previous = connection
        connection = null
        previous?.close()
    }

    private fun fail(kind: String, message: String, retryable: Boolean = true) {
        if (!stopped && wanted) onError(kind, message, retryable)
    }

    @SuppressLint("MissingPermission")
    private fun connect() {
        if (!wanted || stopped || connection != null) return
        try {
            val lens = if (facing == "front") CameraCharacteristics.LENS_FACING_FRONT else CameraCharacteristics.LENS_FACING_BACK
            val id = manager.cameraIdList.firstOrNull {
                manager.getCameraCharacteristics(it).get(CameraCharacteristics.LENS_FACING) == lens
            }
            if (id == null) {
                fail("unavailable", "The requested camera is unavailable", false)
                return
            }
            val current = Connection(manager.getCameraCharacteristics(id))
            connection = current
            current.prepare()
            cameraExecutor.execute {
                try {
                    manager.openCamera(id, current.callback, main)
                } catch (error: Exception) {
                    main.post {
                        if (connection === current) fail("unavailable", error.message ?: "Camera could not be opened")
                    }
                }
            }
        } catch (error: Exception) {
            fail("unavailable", error.message ?: "Camera could not be opened")
        }
    }

    private inner class Connection(private val characteristics: CameraCharacteristics) {
        private var device: CameraDevice? = null
        private var session: CameraCaptureSession? = null
        private var reader: ImageReader? = null
        private var surface: Surface? = null
        private lateinit var previewSize: Size
        private var capturing = false
        @Volatile private var photoVersion = 0
        @Volatile private var captureRotation = 0
        @Volatile private var capturedAt = 0L
        private val current get() = connection === this && wanted && !stopped

        val callback = object : CameraDevice.StateCallback() {
            override fun onOpened(camera: CameraDevice) {
                if (!current) {
                    cameraExecutor.execute { camera.close() }
                    return
                }
                device = camera
                try {
                    val stateCallback = object : CameraCaptureSession.StateCallback() {
                        override fun onConfigured(configured: CameraCaptureSession) {
                            if (!current) {
                                configured.close()
                                return
                            }
                            session = configured
                            try {
                                val request = camera.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW).apply {
                                    addTarget(surface!!)
                                    settings(this)
                                }
                                configured.setRepeatingRequest(request.build(), null, main)
                                ready?.invoke()
                                ready = null
                            } catch (error: Exception) {
                                fail("unavailable", error.message ?: "Camera preview failed")
                            }
                        }
                        override fun onConfigureFailed(failed: CameraCaptureSession) {
                            failed.close()
                            if (current) fail("unavailable", "Camera stream configuration failed")
                        }
                    }
                    camera.createCaptureSession(SessionConfiguration(
                        SessionConfiguration.SESSION_REGULAR,
                        listOf(OutputConfiguration(surface!!), OutputConfiguration(reader!!.surface)),
                        preview.context.mainExecutor,
                        stateCallback,
                    ))
                } catch (error: Exception) {
                    fail("unavailable", error.message ?: "Camera session failed")
                }
            }
            override fun onDisconnected(camera: CameraDevice) {
                cameraExecutor.execute { camera.close() }
                if (current) fail("unavailable", "Camera disconnected")
            }
            override fun onError(camera: CameraDevice, error: Int) {
                cameraExecutor.execute { camera.close() }
                if (current) fail(
                    if (error == ERROR_CAMERA_IN_USE || error == ERROR_MAX_CAMERAS_IN_USE) "busy" else "unavailable",
                    "Camera failed (error $error)",
                    error != ERROR_CAMERA_DISABLED,
                )
            }
        }

        fun prepare() {
            val map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
                ?: error("Camera has no supported streams")
            val photoSize = map.getOutputSizes(ImageFormat.YUV_420_888)?.maxByOrNull { it.width.toLong() * it.height }
                ?: error("Camera has no YUV capture stream")
            val sizes = map.getOutputSizes(SurfaceTexture::class.java).toList()
            val bounded = sizes.filter { max(it.width, it.height) <= 1920 }.ifEmpty { sizes }
            val ratio = photoSize.width.toDouble() / photoSize.height
            previewSize = bounded.minWith(compareBy<Size> { abs(it.width.toDouble() / it.height - ratio) }
                .thenByDescending { it.width.toLong() * it.height })
            val texture = preview.surfaceTexture ?: error("Camera preview surface is unavailable")
            texture.setDefaultBufferSize(previewSize.width, previewSize.height)
            surface = Surface(texture)
            reader = ImageReader.newInstance(photoSize.width, photoSize.height, ImageFormat.YUV_420_888, 2).apply {
                setOnImageAvailableListener({ source ->
                    var image: Image? = null
                    try {
                        image = source.acquireNextImage()
                        if (image != null) {
                            val rotation = captureRotation
                            val time = capturedAt
                            val width = if (rotation % 180 == 0) image.width else image.height
                            val height = if (rotation % 180 == 0) image.height else image.width
                            val version = photoVersion
                            val review = reviewPixels(image, rotation)
                            main.post {
                                if (!stopped && captureVersion == version) onReview(review)
                            }
                            val bytes = encode(image, rotation)
                            image.close()
                            image = null
                            main.post {
                                if (!stopped && captureVersion == version) {
                                    onPhoto(bytes, width, height, time)
                                }
                            }
                        }
                    } catch (error: Exception) {
                        val version = photoVersion
                        main.post {
                            if (!stopped && captureVersion == version) onError("capture", error.message ?: "Photo encoding failed", true)
                        }
                    } finally {
                        image?.close()
                    }
                }, worker)
            }
            transform()
        }

        private fun rotation(): Int {
            val displayDegrees = when (preview.display?.rotation) {
                Surface.ROTATION_90 -> 90
                Surface.ROTATION_180 -> 180
                Surface.ROTATION_270 -> 270
                else -> 0
            }
            val sensor = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
            return (sensor + (if (facing == "front") displayDegrees else -displayDegrees) + 360) % 360
        }

        fun transform() {
            if (preview.width == 0 || preview.height == 0) return
            val swapped = rotation() % 180 != 0
            val sensor = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION) ?: 0
            val width = preview.width.toFloat()
            val height = preview.height.toFloat()
            val swapDimensions = if (sensor == 0) !swapped else swapped
            val scaleX = width / (if (swapDimensions) previewSize.height else previewSize.width)
            val scaleY = height / (if (swapDimensions) previewSize.width else previewSize.height)
            val scale = max(scaleX, scaleY)
            val matrix = Matrix()
            if (swapped) {
                matrix.setScale(scale / scaleX, scale / scaleY, width / 2, height / 2)
            } else {
                matrix.setScale(
                    height / width / scaleY * scale,
                    width / height / scaleX * scale,
                    width / 2,
                    height / 2,
                )
            }
            // SurfaceTexture already applies sensor orientation; compensate for
            // display rotation here.
            matrix.postRotate(-(preview.display?.rotation ?: Surface.ROTATION_0) * 90f, width / 2, height / 2)
            preview.setTransform(matrix)
        }

        private fun settings(builder: CaptureRequest.Builder) {
            builder.set(CaptureRequest.CONTROL_MODE, CaptureRequest.CONTROL_MODE_AUTO)
            builder.set(CaptureRequest.CONTROL_AE_MODE, CaptureRequest.CONTROL_AE_MODE_ON)
            builder.set(CaptureRequest.CONTROL_AWB_MODE, CaptureRequest.CONTROL_AWB_MODE_AUTO)
            builder.set(CaptureRequest.FLASH_MODE, CaptureRequest.FLASH_MODE_OFF)
            builder.set(CaptureRequest.CONTROL_ENABLE_ZSL, false)
            val afModes = characteristics.get(CameraCharacteristics.CONTROL_AF_AVAILABLE_MODES) ?: intArrayOf()
            if (CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE in afModes) {
                builder.set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE)
            }
            if (CaptureRequest.NOISE_REDUCTION_MODE_OFF in (characteristics.get(CameraCharacteristics.NOISE_REDUCTION_AVAILABLE_NOISE_REDUCTION_MODES) ?: intArrayOf())) {
                builder.set(CaptureRequest.NOISE_REDUCTION_MODE, CaptureRequest.NOISE_REDUCTION_MODE_OFF)
            }
            if (CaptureRequest.EDGE_MODE_OFF in (characteristics.get(CameraCharacteristics.EDGE_AVAILABLE_EDGE_MODES) ?: intArrayOf())) {
                builder.set(CaptureRequest.EDGE_MODE, CaptureRequest.EDGE_MODE_OFF)
            }
        }

        fun capture(): Boolean {
            val camera = device ?: return false
            val active = session ?: return false
            if (!current || capturing) return false
            capturing = true
            photoVersion = ++captureVersion
            captureRotation = rotation()
            capturedAt = System.currentTimeMillis()
            try {
                val request = camera.createCaptureRequest(CameraDevice.TEMPLATE_STILL_CAPTURE).apply {
                    addTarget(reader!!.surface)
                    settings(this)
                }
                active.capture(request.build(), object : CameraCaptureSession.CaptureCallback() {
                    override fun onCaptureFailed(session: CameraCaptureSession, request: CaptureRequest, failure: CaptureFailure) {
                        if (current) fail("capture", "Photo capture failed (reason ${failure.reason})")
                    }
                }, main)
            } catch (error: Exception) {
                fail("capture", error.message ?: "Photo capture failed")
            }
            return true
        }

        fun close() {
            val oldSession = session
            val oldDevice = device
            val oldReader = reader
            val oldSurface = surface
            cameraExecutor.execute {
                try {
                    oldSession?.close()
                    oldDevice?.close()
                } finally {
                    worker.post {
                        oldReader?.close()
                        oldSurface?.release()
                    }
                }
            }
        }
    }

    private companion object {
        // Serialise reopening with shutdown, including across camera pages.
        val cameraExecutor = Executors.newSingleThreadExecutor()

        @JvmStatic
        private external fun nativeReviewPixels(
            y: ByteBuffer, yRow: Int, yPixel: Int,
            u: ByteBuffer, uRow: Int, uPixel: Int,
            v: ByteBuffer, vRow: Int, vPixel: Int,
            sourceWidth: Int, sourceHeight: Int, width: Int, height: Int, rotation: Int,
        ): ByteArray
    }

    private fun reviewPixels(image: Image, rotation: Int): NativeResult.Pixels {
        val uprightW = if (rotation % 180 == 0) image.width else image.height
        val uprightH = if (rotation % 180 == 0) image.height else image.width
        val scale = minOf(1.0, 1440.0 / max(uprightW, uprightH))
        val width = maxOf(1, (uprightW * scale).toInt())
        val height = maxOf(1, (uprightH * scale).toInt())
        val planes = image.planes
        val rgba = nativeReviewPixels(
            planes[0].buffer.slice(), planes[0].rowStride, planes[0].pixelStride,
            planes[1].buffer.slice(), planes[1].rowStride, planes[1].pixelStride,
            planes[2].buffer.slice(), planes[2].rowStride, planes[2].pixelStride,
            image.width, image.height, width, height, rotation,
        )
        return NativeResult.Pixels(width, height, rgba)
    }

    // Copy each plane using its own strides, rotating before JPEG encoding so all
    // consumers see upright pixels without relying on EXIF orientation support.
    private fun encode(image: Image, rotation: Int): ByteArray {
        val width = if (rotation % 180 == 0) image.width else image.height
        val height = if (rotation % 180 == 0) image.height else image.width
        val bytes = ByteArray(width * height * 3 / 2)
        image.planes.forEachIndexed { planeIndex, plane ->
            val chroma = planeIndex != 0
            val sourceW = if (chroma) image.width / 2 else image.width
            val sourceH = if (chroma) image.height / 2 else image.height
            val pixelStep = if (chroma) 2 else 1
            val offset = if (chroma) width * height + (if (planeIndex == 1) 1 else 0) else 0
            val targetStep = when (rotation) {
                90 -> width
                180 -> -pixelStep
                270 -> -width
                else -> pixelStep
            }
            val buffer = plane.buffer.duplicate()
            val rowStride = plane.rowStride
            val pixelStride = plane.pixelStride
            // Bulk-read short strips, then rotate in tiles to keep writes local.
            val strip = ByteArray(rowStride * minOf(32, sourceH))
            var top = 0
            while (top < sourceH) {
                val rows = minOf(32, sourceH - top)
                buffer.get(strip, 0, minOf(rowStride * rows, buffer.remaining()))
                var left = 0
                while (left < sourceW) {
                    val columns = minOf(32, sourceW - left)
                    for (row in 0 until rows) {
                        val y = top + row
                        val dx = when (rotation) { 90 -> sourceH - 1 - y; 180 -> sourceW - 1 - left; 270 -> y; else -> left }
                        val dy = when (rotation) { 90 -> left; 180 -> sourceH - 1 - y; 270 -> sourceW - 1 - left; else -> y }
                        var target = offset + dy * width + dx * pixelStep
                        var source = row * rowStride + left * pixelStride
                        repeat(columns) {
                            bytes[target] = strip[source]
                            source += pixelStride
                            target += targetStep
                        }
                    }
                    left += columns
                }
                top += rows
            }
        }
        return ByteArrayOutputStream().use { output ->
            check(YuvImage(bytes, ImageFormat.NV21, width, height, null).compressToJpeg(Rect(0, 0, width, height), 95, output)) {
                "Photo could not be encoded"
            }
            output.toByteArray()
        }
    }
}
