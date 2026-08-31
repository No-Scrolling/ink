package com.vandam.ink

import android.os.Handler
import android.os.Looper
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.MultiFormatReader
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.ReaderException
import com.google.zxing.Result
import com.google.zxing.common.HybridBinarizer
import java.util.EnumMap
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import org.json.JSONObject

internal fun createCodeScannerFeature(
    host: CameraSessionHost,
    config: JSONObject,
): CameraSessionFeature? = InkCodeScannerFeature(host, config)

private class InkCodeScannerFeature(
    private val host: CameraSessionHost,
    config: JSONObject,
) : CameraSessionFeature {
    private val formats = config.getJSONArray("formats").let { values ->
        (0 until values.length()).map { values.getString(it) }
    }
    private val reader = MultiFormatReader().apply {
        setHints(
            EnumMap<DecodeHintType, Any>(DecodeHintType::class.java).apply {
                put(DecodeHintType.POSSIBLE_FORMATS, formats.map(FORMATS::getValue))
                put(DecodeHintType.TRY_HARDER, true)
                put(DecodeHintType.CHARACTER_SET, "UTF-8")
            },
        )
    }
    private val executor = Executors.newSingleThreadExecutor()
    private val handler = Handler(Looper.getMainLooper())
    private val completed = AtomicBoolean(false)
    private val timeout = Runnable {
        if (completed.compareAndSet(false, true)) {
            host.fail("timeout", "No code was found before the scanner timed out", true)
        }
    }
    private val analysis = ImageAnalysis.Builder()
        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
        .build()

    override val useCases = listOf(analysis)

    override fun start() {
        host.preview.contentDescription = "Scanning for a code"
        analysis.setAnalyzer(executor, ::analyse)
        handler.postDelayed(timeout, SCAN_TIMEOUT_MS)
    }

    override fun execute(operation: String): Boolean = false

    override fun stop() {
        completed.set(true)
        handler.removeCallbacks(timeout)
        analysis.clearAnalyzer()
        executor.shutdownNow()
    }

    private fun analyse(image: ImageProxy) {
        if (completed.get()) {
            image.close()
            return
        }
        val decoded = runCatching {
            val bytes = luminance(image)
            decode(
                PlanarYUVLuminanceSource(
                    bytes,
                    image.width,
                    image.height,
                    0,
                    0,
                    image.width,
                    image.height,
                    false,
                ),
            )
        }
        image.close()
        decoded.onSuccess { result ->
            if (result != null && completed.compareAndSet(false, true)) {
                val format = FORMAT_NAMES[result.barcodeFormat]
                if (format == null) {
                    fail("decoder", "Scanner returned an unsupported code format", false)
                } else {
                    host.activity.runOnUiThread {
                        handler.removeCallbacks(timeout)
                        host.finish(
                            JSONObject()
                                .put("text", result.text)
                                .put("format", format),
                        )
                    }
                }
            }
        }.onFailure { error ->
            if (completed.compareAndSet(false, true)) {
                fail("decoder", error.message ?: "Code decoder failed", true)
            }
        }
    }

    private fun luminance(image: ImageProxy): ByteArray {
        val plane = image.planes.firstOrNull() ?: error("Camera returned no luminance plane")
        val buffer = plane.buffer
        val start = buffer.position()
        val limit = buffer.limit()
        val output = ByteArray(image.width * image.height)
        var outputIndex = 0
        for (row in 0 until image.height) {
            for (column in 0 until image.width) {
                val index = start + row * plane.rowStride + column * plane.pixelStride
                if (index >= limit) {
                    error("Camera returned an invalid luminance plane")
                }
                output[outputIndex++] = buffer.get(index)
            }
        }
        return output
    }

    private fun decode(source: PlanarYUVLuminanceSource): Result? {
        var candidate = source
        repeat(4) {
            try {
                return reader.decodeWithState(BinaryBitmap(HybridBinarizer(candidate)))
            } catch (_: ReaderException) {
                reader.reset()
            }
            if (!candidate.isRotateSupported) {
                return null
            }
            candidate = candidate.rotateCounterClockwise() as PlanarYUVLuminanceSource
        }
        return null
    }

    private fun fail(kind: String, message: String, retryable: Boolean) {
        host.activity.runOnUiThread {
            handler.removeCallbacks(timeout)
            host.fail(kind, message, retryable)
        }
    }

    private companion object {
        private const val SCAN_TIMEOUT_MS = 60_000L
        private val FORMATS = mapOf(
            "qr" to BarcodeFormat.QR_CODE,
            "aztec" to BarcodeFormat.AZTEC,
            "data-matrix" to BarcodeFormat.DATA_MATRIX,
            "pdf417" to BarcodeFormat.PDF_417,
            "codabar" to BarcodeFormat.CODABAR,
            "code-39" to BarcodeFormat.CODE_39,
            "code-93" to BarcodeFormat.CODE_93,
            "code-128" to BarcodeFormat.CODE_128,
            "ean-8" to BarcodeFormat.EAN_8,
            "ean-13" to BarcodeFormat.EAN_13,
            "itf" to BarcodeFormat.ITF,
            "upc-a" to BarcodeFormat.UPC_A,
            "upc-e" to BarcodeFormat.UPC_E,
        )
        private val FORMAT_NAMES = FORMATS.entries.associate { (name, format) -> format to name }
    }
}
