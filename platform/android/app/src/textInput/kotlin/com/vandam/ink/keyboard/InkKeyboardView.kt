package com.vandam.ink.keyboard

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
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
    fun onDismiss()
}

internal class InkKeyboardView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
) : View(context, attrs) {
    var listener: KeyboardListener? = null
    var action: KeyboardAction = KeyboardAction.Return
        set(value) {
            field = value
            invalidate()
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
    private val icons: Map<KeyboardIcon, Drawable> = KeyboardIcon.entries.associateWith { icon ->
        checkNotNull(context.getDrawable(icon.resource)).mutate().apply {
            setTint(Color.WHITE)
        }
    }
    private val atlas: Bitmap = BitmapFactory.decodeResource(resources, R.drawable.ink_keyboard_emoji)
    private val emojiSource = Rect()
    private val keys = mutableListOf<PlacedKey>()
    private var emojiKeys = KeyboardLayouts.defaultEmojis
    private val repeatHandler = Handler(Looper.getMainLooper())
    private var mode = KeyboardMode.Letters
    private var shifted = false
    private var pressed: PlacedKey? = null
    private val repeatBackspace = object : Runnable {
        override fun run() {
            if (pressed?.key?.command != KeyboardCommand.Backspace) {
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
        val desiredHeight = dp(KEYBOARD_HEIGHT_DP + DISMISS_HEIGHT_DP).toInt()
        setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), resolveSize(desiredHeight, heightMeasureSpec))
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        keys.clear()
        drawLayout(canvas, KeyboardLayouts.forMode(mode, emojiKeys))
        drawDismiss(canvas)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                pressed = keys.lastOrNull { it.bounds.contains(event.x, event.y) }
                pressed?.let { key ->
                    performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    if (key.key.command == KeyboardCommand.Backspace) {
                        listener?.onBackspace()
                        repeatHandler.postDelayed(repeatBackspace, BACKSPACE_DELAY_MS)
                    }
                }
                invalidate()
            }
            MotionEvent.ACTION_MOVE -> {
                val current = keys.lastOrNull { it.bounds.contains(event.x, event.y) }
                if (current != pressed) {
                    cancelPress()
                }
            }
            MotionEvent.ACTION_UP -> {
                val released = pressed?.takeIf { it.bounds.contains(event.x, event.y) }
                repeatHandler.removeCallbacks(repeatBackspace)
                pressed = null
                released?.key?.let(::release)
                invalidate()
                performClick()
            }
            MotionEvent.ACTION_CANCEL -> cancelPress()
        }
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
        repeatHandler.removeCallbacks(repeatBackspace)
        super.onDetachedFromWindow()
    }

    private fun drawLayout(canvas: Canvas, layout: KeyboardLayout) {
        var top = dp(KEYBOARD_TOP_PADDING_DP)
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
            is KeyboardKey.Emoji -> drawEmoji(canvas, placed, key.atlasIndex)
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
                KeyboardCommand.Shift -> drawIcon(
                    canvas,
                    if (shifted) KeyboardIcon.KeyboardArrowDown else KeyboardIcon.KeyboardArrowUp,
                    x,
                    y,
                )
                KeyboardCommand.Backspace -> drawIcon(canvas, KeyboardIcon.ChevronLeft, x, y)
                KeyboardCommand.Emoji -> drawIcon(canvas, KeyboardIcon.Mood, x, y)
                KeyboardCommand.Submit -> drawIcon(canvas, action.icon, x, y)
                KeyboardCommand.Space -> drawSpace(canvas, placed.bounds)
                KeyboardCommand.Dismiss -> drawIcon(canvas, KeyboardIcon.Dismiss, x, y)
                KeyboardCommand.Letters -> drawIcon(canvas, KeyboardIcon.MatchCase, x, y)
                KeyboardCommand.Numbers, KeyboardCommand.Symbols -> Unit
            }
        }
    }

    private inline fun drawPressed(
        canvas: Canvas,
        placed: PlacedKey,
        isSpace: Boolean = false,
        draw: (x: Float, y: Float) -> Unit,
    ) {
        val isPressed = keyAnimationEnabled && pressed == placed
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

    private fun drawEmoji(canvas: Canvas, placed: PlacedKey, index: Int) {
        val column = index % 8
        val row = index / 8
        emojiSource.set(column * 64, row * 64, (column + 1) * 64, (row + 1) * 64)
        val animated = keyAnimationEnabled && pressed == placed
        val scale = if (animated) 1.25f else 1f
        val size = dp(25f) * scale
        val lift = if (animated) dp(-12f) else 0f
        val destination = RectF(
            placed.contentX() - size / 2f,
            placed.bounds.centerY() - size / 2f + lift,
            placed.contentX() + size / 2f,
            placed.bounds.centerY() + size / 2f + lift,
        )
        canvas.drawBitmap(atlas, emojiSource, destination, paint)
    }

    private fun drawIcon(canvas: Canvas, icon: KeyboardIcon, x: Float, y: Float) {
        val size = dp(icon.sizeDp).toInt()
        val halfSize = size / 2
        icons.getValue(icon).apply {
            setBounds(x.toInt() - halfSize, y.toInt() - halfSize, x.toInt() + halfSize, y.toInt() + halfSize)
            draw(canvas)
        }
    }

    private fun drawSpace(canvas: Canvas, bounds: RectF) {
        val y = bounds.bottom - dp(7f)
        canvas.drawLine(bounds.left, y, bounds.right, y, linePaint)
    }

    private fun drawDismiss(canvas: Canvas) {
        val bounds = RectF(0f, dp(KEYBOARD_HEIGHT_DP), width.toFloat(), height.toFloat())
        val key = KeyboardKey.Command(KeyboardCommand.Dismiss, widthDp = width / resources.displayMetrics.density)
        val placed = PlacedKey(key, bounds)
        keys += placed
        drawCommand(canvas, placed, key)
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
            KeyboardCommand.Dismiss -> listener?.onDismiss()
        }
    }

    private fun show(newMode: KeyboardMode) {
        mode = newMode
        shifted = false
    }

    private fun cancelPress() {
        repeatHandler.removeCallbacks(repeatBackspace)
        pressed = null
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

    private val KeyboardAction.icon: KeyboardIcon
        get() = when (this) {
            KeyboardAction.Return -> KeyboardIcon.KeyboardReturn
            KeyboardAction.Search -> KeyboardIcon.Search
            KeyboardAction.Done -> KeyboardIcon.Done
        }

    private data class PlacedKey(
        val key: KeyboardKey,
        val bounds: RectF,
    )

    private companion object {
        private const val KEYBOARD_TOP_PADDING_DP = 4f
        private const val CONTENT_EDGE_OFFSET_DP = 18f
        private const val BACKSPACE_DELAY_MS = 350L
        private const val BACKSPACE_REPEAT_MS = 65L
    }
}
