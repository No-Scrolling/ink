package com.vandam.ink

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.Typeface
import android.graphics.Color
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.MultiFormatWriter
import org.json.JSONObject
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.FutureTask

internal fun createBarcodeAdapter(activity: MainActivity): BarcodeAdapter = InkBarcodeAdapter(activity)

private class InkBarcodeAdapter(private val activity: MainActivity) : BarcodeAdapter {
    private val executor = Executors.newSingleThreadExecutor()
    private val requests = ConcurrentHashMap<Long, FutureTask<Unit>>()

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val task = FutureTask<Unit> {
            var file: File? = null
            val result = try {
                require(operation == "image") { "Unknown barcode operation" }
                val request = JSONObject(payload)
                val source = JSONObject(request.getString("source"))
                val value = source.getString("value")
                require(value.isNotEmpty() && value.length <= 8192) { "Invalid barcode payload length" }
                val size = source.getDouble("size")
                require(size > 0 && size <= 1080) { "Invalid barcode size" }
                val pixels = request.getInt("pixelSize")
                require(pixels in 1..4096) { "Barcode image is too large" }
                val format = when (source.getString("format")) {
                    "qr" -> BarcodeFormat.QR_CODE
                    "aztec" -> BarcodeFormat.AZTEC
                    "data-matrix" -> BarcodeFormat.DATA_MATRIX
                    "pdf417" -> BarcodeFormat.PDF_417
                    "ean-13" -> BarcodeFormat.EAN_13
                    "ean-8" -> BarcodeFormat.EAN_8
                    "upc-a" -> BarcodeFormat.UPC_A
                    "upc-e" -> BarcodeFormat.UPC_E
                    "code-39" -> BarcodeFormat.CODE_39
                    "code-93" -> BarcodeFormat.CODE_93
                    "code-128" -> BarcodeFormat.CODE_128
                    "itf" -> BarcodeFormat.ITF
                    "codabar" -> BarcodeFormat.CODABAR
                    else -> error("Unsupported barcode format")
                }
                val hints = mutableMapOf<EncodeHintType, Any>(EncodeHintType.CHARACTER_SET to "UTF-8")
                if (format == BarcodeFormat.QR_CODE || format == BarcodeFormat.PDF_417) {
                    hints[EncodeHintType.MARGIN] = 2
                }
                val matrix = MultiFormatWriter().encode(value, format, 0, 0, hints)
                val linear = matrix.height == 1
                val horizontalBorder = if (format == BarcodeFormat.AZTEC || format == BarcodeFormat.DATA_MATRIX) 2 else 0
                val verticalBorder = if (linear) 2 else horizontalBorder
                val scale = pixels / (matrix.width + horizontalBorder * 2)
                require(scale > 0) { "Barcode does not fit the requested width" }
                val width = matrix.width * scale
                val height = if (linear) maxOf(1, width / 3) else matrix.height * scale
                // Pad to the requested pixel width so the renderer never stretches barcode modules.
                val left = (pixels - width) / 2
                val verticalPadding = if (linear) verticalBorder * scale * 2 else pixels - width
                val top = verticalPadding / 2
                val bitmapWidth = pixels
                val label = if (linear && source.optBoolean("showValue")) Paint(Paint.ANTI_ALIAS_FLAG).apply {
                    color = Color.BLACK
                    typeface = Typeface.MONOSPACE
                    textAlign = Paint.Align.CENTER
                    textSize = pixels * 0.05f
                    val available = bitmapWidth - pixels * 0.04f
                    val measured = measureText(value)
                    if (measured > available) textSize *= available / measured
                } else null
                val labelGap = (pixels * 0.02f).toInt()
                val labelBounds = Rect()
                label?.getTextBounds(value, 0, value.length, labelBounds)
                val labelHeight = if (label != null) labelBounds.height() + labelGap * 2 else 0
                val barcodeHeight = height + if (label != null) top else verticalPadding
                val bitmapHeight = barcodeHeight + labelHeight
                val bitmap = Bitmap.createBitmap(bitmapWidth, bitmapHeight, Bitmap.Config.ARGB_8888)
                try {
                    val row = IntArray(bitmapWidth)
                    for (y in 0 until bitmapHeight) {
                        if (Thread.currentThread().isInterrupted) throw InterruptedException()
                        for (x in 0 until bitmapWidth) {
                            val inside = x in left until left + width && y in top until top + height
                            val black = inside && matrix[(x - left) / scale, if (linear) 0 else (y - top) / scale]
                            row[x] = if (black) Color.BLACK else Color.WHITE
                        }
                        bitmap.setPixels(row, 0, bitmapWidth, 0, y, bitmapWidth, 1)
                    }
                    label?.let { paint ->
                        Canvas(bitmap).drawText(value, bitmapWidth / 2f, (barcodeHeight + labelGap - labelBounds.top).toFloat(), paint)
                    }
                    val target = File.createTempFile("ink-barcode-", ".png", activity.cacheDir)
                    file = target
                    target.outputStream().use { check(bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)) }
                    NativeResult.File(target.absolutePath)
                } finally {
                    bitmap.recycle()
                }
            } catch (error: Exception) {
                file?.delete()
                NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Could not generate barcode", false)
            }
            activity.runOnUiThread {
                if (requests.remove(requestId) != null) complete(result) else file?.delete()
            }
        }
        requests[requestId] = task
        executor.execute(task)
    }

    override fun cancel(requestId: Long) { requests.remove(requestId)?.cancel(true) }
    override fun stop() {
        requests.keys.toList().forEach(::cancel)
        executor.shutdown()
    }
}
