package com.vandam.ink.keyboard

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.os.Handler
import android.os.Looper
import android.util.AttributeSet
import android.view.HapticFeedbackConstants
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import com.vandam.ink.R

internal enum class KeyboardAction {
    Return,
    Search,
    Done,
}

internal interface KeyboardListener {
    fun onText(text: String)
    fun onBackspace()
    fun onAction()
}

internal class InkKeyboardView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : View(context, attrs) {
    var lightAppearance: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            val foreground = if (value) Color.BLACK else Color.WHITE
            setBackgroundColor(if (value) Color.WHITE else Color.BLACK)
            paint.color = foreground
            linePaint.color = foreground
            emojiPaint.color = foreground
            icons.values.forEach { it.setTint(foreground) }
            invalidate()
        }
    var listener: KeyboardListener? = null
    var action: KeyboardAction = KeyboardAction.Return
        set(value) {
            field = value
            invalidate()
        }
    var numeric: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            reset()
        }
    var typeface: Typeface = Typeface.DEFAULT
        set(value) {
            field = value
            paint.typeface = value
            invalidate()
        }
    var emojis: String? = null
        set(value) {
            field = value
            emojiKeys = KeyboardLayouts.parseEmojis(value)
                .filter { emojiPaint.hasGlyph(it.value) }
                .ifEmpty { KeyboardLayouts.defaultEmojis }
            invalidate()
        }
    var keyAnimationEnabled: Boolean = true
        set(value) {
            field = value
            invalidate()
        }

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textAlign = Paint.Align.CENTER
        typeface = this@InkKeyboardView.typeface
    }
    private val linePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
        strokeWidth = dp(1.7f)
    }
    private val emojiPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.SUBPIXEL_TEXT_FLAG).apply {
        color = Color.WHITE
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT
    }
    private val icons = mutableMapOf<KeyboardIcon, Drawable>()
    private val keys = mutableListOf<PlacedKey>()
    private var emojiKeys = KeyboardLayouts.defaultEmojis
    private val repeatHandler = Handler(Looper.getMainLooper())
    private var mode = KeyboardMode.Letters
    private var shifted = false
    private val pressed = mutableMapOf<Int, PlacedKey>()
    private val repeatBackspace = object : Runnable {
        override fun run() {
            if (pressed.values.none { it.key.command == KeyboardCommand.Backspace }) {
                return
            }
            listener?.onBackspace()
            repeatHandler.postDelayed(this, BACKSPACE_REPEAT_MS)
        }
    }

    init {
        setBackgroundColor(Color.BLACK)
        isFocusable = true
        isFocusableInTouchMode = true
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        // Match Ink's 20-unit content inset and its viewport scale.
        val bottomInset = 20f * width / 1080f * 2.55f
        val desiredHeight = (dp(KEYBOARD_HEIGHT_DP) + bottomInset).toInt()
        setMeasuredDimension(width, resolveSize(desiredHeight, heightMeasureSpec))
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        keys.clear()
        drawLayout(canvas, KeyboardLayouts.forMode(mode, emojiKeys))
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val index = event.actionIndex
        val id = event.getPointerId(index)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                if (event.actionMasked == MotionEvent.ACTION_DOWN) cancelPress()
                keys.lastOrNull { it.hitTest(event.getX(index), event.getY(index)) }?.let { key ->
                    val repeating = pressed.values.any { it.key.command == KeyboardCommand.Backspace }
                    pressed[id] = key
                    performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    if (key.key.command == KeyboardCommand.Backspace) {
                        listener?.onBackspace()
                        if (!repeating) repeatHandler.postDelayed(repeatBackspace, BACKSPACE_DELAY_MS)
                    }
                }
            }
            MotionEvent.ACTION_MOVE -> {
                for (pointerIndex in 0 until event.pointerCount) {
                    val pointerId = event.getPointerId(pointerIndex)
                    val key = pressed[pointerId] ?: continue
                    if (!key.hitTest(event.getX(pointerIndex), event.getY(pointerIndex))) {
                        pressed.remove(pointerId)
                    }
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val released = pressed.remove(id)?.takeIf { it.hitTest(event.getX(index), event.getY(index)) }
                released?.key?.let(::release)
                performClick()
            }
            MotionEvent.ACTION_CANCEL -> cancelPress()
        }
        if (pressed.values.none { it.key.command == KeyboardCommand.Backspace }) {
            repeatHandler.removeCallbacks(repeatBackspace)
        }
        invalidate()
        return true
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean = when (keyCode) {
        KeyEvent.KEYCODE_DEL -> {
            listener?.onBackspace()
            true
        }
        KeyEvent.KEYCODE_ENTER -> {
            listener?.onAction()
            true
        }
        else -> {
            val unicode = event.unicodeChar
            if (unicode == 0 || Character.isISOControl(unicode)) {
                super.onKeyDown(keyCode, event)
            } else {
                listener?.onText(String(Character.toChars(unicode)))
                true
            }
        }
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    override fun onDetachedFromWindow() {
        cancelPress()
        super.onDetachedFromWindow()
    }

    fun reset() {
        show(if (numeric) KeyboardMode.Numeric else KeyboardMode.Letters)
    }

    private fun drawLayout(canvas: Canvas, layout: KeyboardLayout) {
        var top = dp(KEYBOARD_TOP_PADDING_DP + if (numeric) 6f else 0f)
        layout.rows.forEach { row ->
            val rowWidth = row.keys.sumOf { it.widthDp.toDouble() }.toFloat()
            var left = (width - dp(rowWidth)) / 2f
            val height = dp(row.heightDp)
            row.keys.forEach { key ->
                val keyWidth = dp(key.widthDp)
                if (key !is KeyboardKey.Gap) {
                    val placed = PlacedKey(key, RectF(left, top, left + keyWidth, top + height))
                    keys += placed
                    drawKey(canvas, placed)
                }
                left += keyWidth
            }
            top += height
        }
    }

    private fun drawKey(canvas: Canvas, placed: PlacedKey) {
        when (val key = placed.key) {
            is KeyboardKey.Text -> drawTextKey(canvas, placed, displayedText(key.value))
            is KeyboardKey.Emoji -> drawEmoji(canvas, placed, key.value)
            is KeyboardKey.Command -> drawCommand(canvas, placed, key)
            is KeyboardKey.Gap -> Unit
        }
    }

    private fun drawTextKey(canvas: Canvas, placed: PlacedKey, label: String) {
        drawPressed(canvas, placed) { x, y ->
            drawLabel(canvas, label, x, y, 25f)
        }
    }

    private fun drawCommand(canvas: Canvas, placed: PlacedKey, key: KeyboardKey.Command) {
        drawPressed(canvas, placed, isSpace = key.command == KeyboardCommand.Space) { x, y ->
            if (key.label != null) {
                drawLabel(canvas, key.label, x, y, 16f)
                return@drawPressed
            }

            when (key.command) {
                KeyboardCommand.Backspace -> drawIcon(canvas, KeyboardIcon.ChevronLeft, x, y)
                KeyboardCommand.Submit -> drawIcon(canvas, KeyboardLayouts.actionIcon(action), x, y)
                KeyboardCommand.Space -> drawSpace(canvas, placed.bounds)
                else -> KeyboardLayouts.commandIcon(key.command, shifted)?.let { drawIcon(canvas, it, x, y) }
            }
        }
    }

    private inline fun drawPressed(
        canvas: Canvas,
        placed: PlacedKey,
        isSpace: Boolean = false,
        draw: (x: Float, y: Float) -> Unit,
    ) {
        val isPressed = keyAnimationEnabled && pressed.containsValue(placed)
        val lift = if (isPressed) dp(if (isSpace) -8f else -12f) else 0f
        val scale = if (isPressed) if (isSpace) 1.1f else 1.25f else 1f
        val centreX = placed.contentX()
        val centreY = placed.bounds.centerY() + lift

        canvas.save()
        canvas.scale(scale, scale, centreX, centreY)
        draw(centreX, centreY)
        canvas.restore()
    }

    private fun drawLabel(canvas: Canvas, label: String, x: Float, y: Float, sizeDp: Float) {
        paint.textSize = dp(sizeDp)
        val baseline = y - (paint.ascent() + paint.descent()) / 2f
        canvas.drawText(label, x, baseline, paint)
    }

    private fun drawEmoji(canvas: Canvas, placed: PlacedKey, emoji: String) {
        drawPressed(canvas, placed) { x, y ->
            emojiPaint.textSize = dp(25f)
            val baseline = y - (emojiPaint.ascent() + emojiPaint.descent()) / 2f
            canvas.drawText(emoji, x, baseline, emojiPaint)
        }
    }

    private fun drawIcon(canvas: Canvas, icon: KeyboardIcon, x: Float, y: Float) {
        val size = dp(icon.sizeDp).toInt()
        val halfSize = size / 2
        icons.getOrPut(icon) {
            checkNotNull(context.getDrawable(icon.resource)).mutate().apply {
                setTint(if (lightAppearance) Color.BLACK else Color.WHITE)
            }
        }.apply {
            setBounds(x.toInt() - halfSize, y.toInt() - halfSize, x.toInt() + halfSize, y.toInt() + halfSize)
            draw(canvas)
        }
    }

    private fun drawSpace(canvas: Canvas, bounds: RectF) {
        val y = bounds.bottom - dp(7f)
        canvas.drawLine(bounds.left, y, bounds.right, y, linePaint)
    }

    private fun displayedText(value: String): String =
        if (mode == KeyboardMode.Letters && shifted) value.uppercase() else value

    private fun release(key: KeyboardKey) {
        when (key) {
            is KeyboardKey.Text -> {
                listener?.onText(displayedText(key.value))
                shifted = false
            }
            is KeyboardKey.Emoji -> listener?.onText(key.value)
            is KeyboardKey.Command -> release(key.command)
            is KeyboardKey.Gap -> Unit
        }
    }

    private fun release(command: KeyboardCommand) {
        when (command) {
            KeyboardCommand.Shift -> shifted = !shifted
            KeyboardCommand.Backspace -> Unit
            KeyboardCommand.Letters -> show(KeyboardMode.Letters)
            KeyboardCommand.Numbers -> show(KeyboardMode.Numbers)
            KeyboardCommand.Symbols -> show(KeyboardMode.Symbols)
            KeyboardCommand.Emoji -> show(KeyboardMode.Emoji)
            KeyboardCommand.Space -> listener?.onText(" ")
            KeyboardCommand.Submit -> listener?.onAction()
        }
    }

    private fun show(newMode: KeyboardMode) {
        cancelPress()
        mode = newMode
        shifted = false
    }

    private fun cancelPress() {
        repeatHandler.removeCallbacks(repeatBackspace)
        pressed.clear()
        invalidate()
    }

    private fun dp(value: Float): Float = value * resources.displayMetrics.density

    private fun PlacedKey.contentX(): Float = when (key.alignment) {
        KeyContentAlignment.Start -> bounds.left + dp(CONTENT_EDGE_OFFSET_DP)
        KeyContentAlignment.Centre -> bounds.centerX()
        KeyContentAlignment.End -> bounds.right - dp(CONTENT_EDGE_OFFSET_DP)
    }

    private val KeyboardKey.command: KeyboardCommand?
        get() = (this as? KeyboardKey.Command)?.command

    private fun PlacedKey.hitTest(x: Float, y: Float): Boolean =
        bounds.contains(x, y) ||
            key.command == KeyboardCommand.Space &&
            x >= bounds.left && x < bounds.right &&
            y >= bounds.bottom && y < bounds.bottom + dp(SPACE_HIT_EXTENSION_DP)

    private data class PlacedKey(
        val key: KeyboardKey,
        val bounds: RectF,
    )

    private companion object {
        private const val KEYBOARD_TOP_PADDING_DP = 4f
        private const val CONTENT_EDGE_OFFSET_DP = 18f
        private const val SPACE_HIT_EXTENSION_DP = 8f
        private const val BACKSPACE_DELAY_MS = 350L
        private const val BACKSPACE_REPEAT_MS = 65L
    }
}
