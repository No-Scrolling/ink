package com.vandam.ink

import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.ImageProxy
import java.io.File
import java.util.UUID
import java.util.concurrent.Executors
import org.json.JSONObject

internal fun createPhotoCaptureFeature(
    host: CameraSessionHost,
    directory: File,
): CameraSessionFeature? = InkPhotoCaptureFeature(host, directory)

private class InkPhotoCaptureFeature(
    private val host: CameraSessionHost,
    private val directory: File,
) : CameraSessionFeature {
    private val imageCapture = ImageCapture.Builder()
        .setCaptureMode(ImageCapture.CAPTURE_MODE_MINIMIZE_LATENCY)
        .build()
    private val executor = Executors.newSingleThreadExecutor()
    private var staged: CapturedFile? = null
    private var capturing = false
    private var stopped = false

    override val useCases = listOf(imageCapture)

    override fun start() {
        host.preview.contentDescription = "Tap to take photo"
        host.preview.isClickable = true
        host.preview.isFocusable = true
        host.preview.setOnClickListener { capture() }
    }

    override fun execute(operation: String): Boolean = when (operation) {
        RETAKE -> {
            retake()
            true
        }
        USE_PHOTO -> {
            usePhoto()
            true
        }
        else -> false
    }

    override fun stop() {
        stopped = true
        host.preview.setOnClickListener(null)
        executor.shutdownNow()
        staged?.file?.delete()
        staged = null
    }

    private fun capture() {
        if (capturing || staged != null || stopped) {
            return
        }
        capturing = true
        host.preview.isClickable = false
        imageCapture.takePicture(
            executor,
            object : ImageCapture.OnImageCapturedCallback() {
                override fun onCaptureSuccess(image: ImageProxy) {
                    save(image)
                }

                override fun onError(exception: ImageCaptureException) {
                    val (kind, retryable) = when (exception.imageCaptureError) {
                        ImageCapture.ERROR_FILE_IO -> "storage" to true
                        ImageCapture.ERROR_CAMERA_CLOSED -> "unavailable" to true
                        ImageCapture.ERROR_INVALID_CAMERA -> "unavailable" to false
                        else -> "capture" to true
                    }
                    fail(kind, exception.message ?: "Photo capture failed", retryable)
                }
            },
        )
    }

    private fun save(image: ImageProxy) {
        val width = if (image.imageInfo.rotationDegrees % 180 == 0) image.width else image.height
        val height = if (image.imageInfo.rotationDegrees % 180 == 0) image.height else image.width
        val capturedAt = System.currentTimeMillis()
        val id = UUID.randomUUID().toString()
        val output = File(directory, "$id.partial")
        val result = runCatching {
            val plane = image.planes.firstOrNull() ?: error("Camera returned no JPEG data")
            val buffer = plane.buffer
            val bytes = ByteArray(buffer.remaining())
            buffer.get(bytes)
            output.outputStream().buffered().use { it.write(bytes) }
            CapturedFile(id, output, width, height, capturedAt)
        }
        image.close()
        result.onSuccess { captured ->
            host.activity.runOnUiThread {
                if (stopped) {
                    captured.file.delete()
                } else {
                    staged = captured
                    host.review("$REVIEW_PREFIX${captured.id}")
                }
            }
        }.onFailure { error ->
            output.delete()
            fail("storage", error.message ?: "Photo could not be saved", true)
        }
    }

    private fun retake() {
        val captured = staged ?: return
        captured.file.delete()
        staged = null
        capturing = false
        host.rebindCamera()
    }

    private fun usePhoto() {
        val captured = staged ?: return
        val accepted = File(directory, "${captured.id}.jpg")
        if (!captured.file.renameTo(accepted)) {
            fail("storage", "Captured photo could not be committed", true)
            return
        }
        staged = null
        host.finish(
            JSONObject()
                .put("source", "$PHOTO_PREFIX${captured.id}")
                .put("width", captured.width)
                .put("height", captured.height)
                .put("mimeType", "image/jpeg")
                .put("capturedAtMs", captured.capturedAtMs),
        )
    }

    private fun fail(kind: String, message: String, retryable: Boolean) {
        host.activity.runOnUiThread {
            if (!stopped) {
                host.fail(kind, message, retryable)
            }
        }
    }

    private data class CapturedFile(
        val id: String,
        val file: File,
        val width: Int,
        val height: Int,
        val capturedAtMs: Long,
    )

    private companion object {
        private const val RETAKE = "retake"
        private const val USE_PHOTO = "use-photo"
        private const val PHOTO_PREFIX = "ink-camera://"
        private const val REVIEW_PREFIX = "ink-camera-review://"
    }
}
