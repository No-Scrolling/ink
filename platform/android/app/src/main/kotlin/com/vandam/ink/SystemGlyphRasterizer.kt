package com.vandam.ink

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.Typeface
import kotlin.math.min

internal class SystemGlyphRasterizer {
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.SUBPIXEL_TEXT_FLAG).apply {
        textAlign = Paint.Align.LEFT
        typeface = Typeface.DEFAULT
    }
    private val bounds = Rect()

    fun rasterise(grapheme: String, size: Int): ByteArray? {
        if (size !in 1..MAX_GLYPH_SIZE || !paint.hasGlyph(grapheme)) return null

        paint.textSize = size.toFloat()
        paint.getTextBounds(grapheme, 0, grapheme.length, bounds)
        val available = (size - GLYPH_INSET * 2).coerceAtLeast(1)
        val scale = min(
            1f,
            min(
                available.toFloat() / bounds.width().coerceAtLeast(1),
                available.toFloat() / bounds.height().coerceAtLeast(1),
            ),
        )
        if (scale < 1f) {
            paint.textSize *= scale
            paint.getTextBounds(grapheme, 0, grapheme.length, bounds)
        }
        val bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
        Canvas(bitmap).drawText(
            grapheme,
            size / 2f - bounds.exactCenterX(),
            size / 2f - bounds.exactCenterY(),
            paint,
        )

        val argb = IntArray(size * size)
        bitmap.getPixels(argb, 0, size, 0, 0, size, size)
        bitmap.recycle()

        val rgba = ByteArray(argb.size * 4)
        argb.forEachIndexed { index, pixel ->
            val alpha = Color.alpha(pixel)
            val offset = index * 4
            if (alpha != 0) {
                rgba[offset] = unpremultiply(Color.red(pixel), alpha).toByte()
                rgba[offset + 1] = unpremultiply(Color.green(pixel), alpha).toByte()
                rgba[offset + 2] = unpremultiply(Color.blue(pixel), alpha).toByte()
                rgba[offset + 3] = alpha.toByte()
            }
        }
        return rgba
    }

    private fun unpremultiply(channel: Int, alpha: Int): Int =
        min(255, (channel * 255 + alpha / 2) / alpha)

    private companion object {
        private const val GLYPH_INSET = 1
        private const val MAX_GLYPH_SIZE = 256
    }
}
