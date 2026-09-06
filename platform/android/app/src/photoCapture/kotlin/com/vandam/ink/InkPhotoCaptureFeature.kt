package com.vandam.ink

import android.graphics.Canvas
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.camera.core.UseCase
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
    private val executor = Executors.newSingleThreadExecutor()
    private var staged: CapturedFile? = null
    private var capturing = false
    private var stopped = false
    private var attempt: String? = null
    private var reviewPixels: NativeResult.Pixels? = null

    private val camera = InkDirectPhotoCamera(
        host.activity,
        host.facing,
        onReview = { pixels ->
            val current = attempt
            if (!stopped && current != null) {
                reviewPixels = pixels
                host.review("$REVIEW_PREFIX$current", saving = true)
            }
        },
        onPhoto = { bytes, width, height, capturedAt ->
            val current = attempt
            if (!stopped && current != null) {
                executor.execute { save(bytes, width, height, capturedAt, current) }
            }
        },
        onError = { kind, message, retryable -> fail(kind, message, retryable) },
    )
    private var flashAwaitingDraw = false
    private val flashView: View = object : View(host.activity) {
        init {
            visibility = GONE
            importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_NO
        }

        override fun onDraw(canvas: Canvas) {
            super.onDraw(canvas)
            if (flashAwaitingDraw) {
                flashAwaitingDraw = false
                postDelayed(clearFlash, 100)
            }
        }
    }
    private val clearFlash: Runnable = Runnable {
        flashAwaitingDraw = false
        flashView.visibility = View.GONE
    }

    override fun image(source: String): NativeResult.Pixels? =
        if (source == "$REVIEW_PREFIX$attempt") reviewPixels else null

    override val useCases = emptyList<UseCase>()
    override val capturePreview = FrameLayout(host.activity).apply {
        addView(camera.preview, FrameLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT,
        ))
        addView(flashView, FrameLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT,
        ))
    }
    override fun openCamera(onReady: () -> Unit) = camera.open(onReady)
    override fun releaseCamera() = camera.release()

    override fun start() {
        camera.preview.contentDescription = "Tap to take photo"
        camera.preview.isClickable = true
        camera.preview.isFocusable = true
        camera.preview.setOnClickListener { capture() }
    }

    override fun execute(operation: String): Boolean = when (operation) {
        "capture" -> {
            if (capturing || staged != null || stopped) false else {
                capture()
                true
            }
        }
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
        flashView.removeCallbacks(clearFlash)
        clearFlash.run()
        camera.stop()
        camera.preview.setOnClickListener(null)
        attempt = null
        reviewPixels = null
        executor.shutdown()
        staged?.file?.delete()
        staged = null
    }

    private fun capture() {
        if (capturing || staged != null || stopped) {
            return
        }
        capturing = true
        val current = UUID.randomUUID().toString()
        attempt = current
        camera.preview.isClickable = false
        flashView.removeCallbacks(clearFlash)
        flashView.setBackgroundColor(host.flashColour)
        flashAwaitingDraw = true
        flashView.visibility = View.VISIBLE
        if (!camera.capture()) {
            fail("unavailable", "Camera is not ready to capture", true, current)
        }
    }

    private fun save(bytes: ByteArray, width: Int, height: Int, capturedAt: Long, current: String) {
        val id = current
        val output = File(directory, "$id.partial")
        val result = runCatching {
            output.outputStream().buffered().use { it.write(bytes) }
            CapturedFile(id, output, width, height, capturedAt)
        }
        result.onSuccess { captured ->
            host.activity.runOnUiThread {
                if (stopped || attempt != current) {
                    captured.file.delete()
                } else {
                    staged = captured
                    host.review("$REVIEW_PREFIX${captured.id}")
                }
            }
        }.onFailure { error ->
            output.delete()
            fail("storage", error.message ?: "Photo could not be saved", true, current)
        }
    }

    private fun retake() {
        if (!capturing) return
        attempt = null
        reviewPixels = null
        staged?.file?.delete()
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
        val managed = try {
            InkManagedFiles(host.activity).adopt(accepted, "image/jpeg", id = captured.id, width = captured.width, height = captured.height)
        } catch (error: Exception) {
            accepted.delete()
            fail("storage", error.message ?: "Captured photo could not be saved", true)
            return
        }
        host.finish(
            JSONObject()
                .put("source", "$PHOTO_PREFIX${captured.id}")
                .put("width", captured.width)
                .put("height", captured.height)
                .put("mimeType", "image/jpeg")
                .put("capturedAtMs", captured.capturedAtMs)
                .put("file", managed
                    .put("uri", android.net.Uri.fromFile(accepted).toString())
                    .put("source", "$PHOTO_PREFIX${captured.id}")
                    .put("name", accepted.name)
                    .put("size", accepted.length())
                    .put("mimeType", "image/jpeg")),
        )
    }

    private fun fail(kind: String, message: String, retryable: Boolean, current: String? = attempt) {
        host.activity.runOnUiThread {
            if (!stopped && attempt == current) {
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
