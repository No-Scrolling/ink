package com.vandam.ink

import android.content.Context
import android.text.Editable
import android.text.InputFilter
import android.text.InputType
import android.text.TextWatcher
import android.text.StaticLayout
import android.text.Layout
import android.text.Spanned
import android.text.style.SuggestionSpan
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputConnectionWrapper
import android.widget.EditText
import android.widget.FrameLayout
import org.json.JSONObject
import android.graphics.Color
import android.util.TypedValue
import android.view.Gravity
import android.view.ContextThemeWrapper
import android.view.MotionEvent
import kotlin.math.roundToInt

private const val SPELLING_ANNOTATE = "ing.noscroll.ink.spelling.annotate.v1"
private const val MAX_EDITOR_LINES = 3

internal fun createTextInputAdapter(
    activity: MainActivity,
    container: ViewGroup,
    onEdit: TextEditHandler,
): TextInputAdapter = SystemKeyboardAdapter(activity, container, onEdit)

private class SystemKeyboardAdapter(
    private val activity: MainActivity,
    private val container: ViewGroup,
    private val onEdit: TextEditHandler,
) : TextInputAdapter {
    private val inputMethod = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
    private var active = false
    private var action = -1
    private var numeric = false
    private var inputId = -1
    private var nativeText = ""
    private var nativeCursor = 0
    private var nativeBaseline = 0f
    private var syncing = false
    private var pending = false
    private var imeVisible = false
    private var lightAppearance: Boolean? = null
    private var lineMeasurement: Triple<String, Int, Float>? = null
    private var measuredLineCount = 1
    private var awaitingEditorDraw = false
    private var hideAfterFrame = false
    private val publish = Runnable { publishEdit() }
    private val editorDrawn = Runnable {
        if (!active || !awaitingEditorDraw || inputId < 0) return@Runnable
        awaitingEditorDraw = false
        onEdit(TextEdit.Update(JSONObject().put("id", inputId).put("text", nativeText)
            .put("nativeEditor", true).put("lines", measuredLines()).toString()))
    }
    private val hideEditor = Runnable {
        if (hideAfterFrame) {
            editor.visibility = View.GONE
            inputId = -1
            hideAfterFrame = false
        }
    }
    private val editor: EditText = object : EditText(ContextThemeWrapper(activity, R.style.Theme_Ink_TextEditor)) {
        override fun onDraw(canvas: android.graphics.Canvas) {
            super.onDraw(canvas)
            if (active && awaitingEditorDraw) {
                removeCallbacks(editorDrawn)
                post(editorDrawn)
            }
        }

        override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
            super.onLayout(changed, left, top, right, bottom)
            if (action != 0) translationY = nativeBaseline - baseline
        }

        override fun onTouchEvent(event: MotionEvent): Boolean = active && super.onTouchEvent(event)
        override fun onCreateInputConnection(info: EditorInfo): InputConnection? {
            val connection = super.onCreateInputConnection(info) ?: return null
            if (info.extras == null) info.extras = Bundle()
            info.extras.putBoolean(SPELLING_ANNOTATE, true)
            return object : InputConnectionWrapper(connection, false) {
                override fun performPrivateCommand(action: String?, data: Bundle?): Boolean {
                    if (action == SPELLING_ANNOTATE) {
                        val annotated = data?.getCharSequence("annotations") as? Spanned ?: return false
                        val editable = editor.text
                        if (!active || annotated.toString() != editable.toString()) return false
                        val incoming = annotated.getSpans(0, annotated.length, SuggestionSpan::class.java)
                        val existing = editable.getSpans(0, editable.length, SuggestionSpan::class.java)
                        fun matches(old: SuggestionSpan, next: SuggestionSpan) =
                            editable.getSpanStart(old) == annotated.getSpanStart(next) &&
                            editable.getSpanEnd(old) == annotated.getSpanEnd(next) &&
                            old.flags == next.flags && old.suggestions.contentEquals(next.suggestions)
                        existing.filter { old -> incoming.none { matches(old, it) } }.forEach(editable::removeSpan)
                        incoming.filter { next -> existing.none { editable.getSpanStart(it) >= 0 && matches(it, next) } }.forEach {
                            editable.setSpan(it, annotated.getSpanStart(it), annotated.getSpanEnd(it), Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                        }
                        return true
                    }
                    return super.performPrivateCommand(action, data)
                }
            }
        }
        override fun onSelectionChanged(start: Int, end: Int) {
            super.onSelectionChanged(start, end)
            scheduleEdit()
        }

        override fun onKeyPreIme(keyCode: Int, event: KeyEvent): Boolean {
            if (keyCode == KeyEvent.KEYCODE_BACK && event.action == KeyEvent.ACTION_UP && dismiss()) return true
            return super.onKeyPreIme(keyCode, event)
        }
    }

    init {
        activity.window.addFlags(WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED)
        editor.visibility = View.GONE
        editor.background = null
        editor.typeface = activity.publicSansTypeface
        // Match Ink's unhinted glyph metrics rather than Android's font shaping defaults.
        editor.fontFeatureSettings = "'kern' 0, 'liga' 0"
        editor.paint.hinting = android.graphics.Paint.HINTING_OFF
        editor.paint.isSubpixelText = true
        editor.paint.isLinearText = true
        editor.includeFontPadding = false
        editor.gravity = Gravity.TOP or Gravity.START
        editor.breakStrategy = Layout.BREAK_STRATEGY_SIMPLE
        editor.hyphenationFrequency = Layout.HYPHENATION_FREQUENCY_NONE
        editor.setTextColor(Color.WHITE)
        editor.setHighlightColor(0x66888888)
        editor.isFocusableInTouchMode = true
        editor.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        editor.maxLines = MAX_EDITOR_LINES
        editor.setPadding(0, 0, 0, 0)
        editor.filters = arrayOf(InputFilter { source, start, end, _, _, _ ->
            val value = source.subSequence(start, end).toString()
            val filtered = (if (action == 0) value else value.replace('\n', ' ')).filter { char ->
                if (numeric) char in '0'..'9' else !char.isISOControl() || (char == '\n' && action == 0)
            }
            if (filtered == value) null else filtered
        })
        editor.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(text: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(text: CharSequence?, start: Int, before: Int, count: Int) = Unit
            override fun afterTextChanged(text: Editable?) {
                if (!syncing) resizeEditor()
                scheduleEdit()
            }
        })
        editor.setOnEditorActionListener { _, _, event ->
            if (action == 0) false else {
                if (event == null || event.action == KeyEvent.ACTION_UP) {
                    publishEdit()
                    onEdit(TextEdit.Submit)
                }
                true
            }
        }
        container.addView(editor, FrameLayout.LayoutParams(1, 1))
        activity.window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
        container.setOnApplyWindowInsetsListener { _, insets ->
            val visible = insets.isVisible(WindowInsets.Type.ime())
            val dismissed = imeVisible && !visible && active
            imeVisible = visible
            if (dismissed) onEdit(TextEdit.Dismiss)
            activity.setKeyboardInset(if (visible && active) insets.getInsets(WindowInsets.Type.ime()).bottom else 0)
            insets
        }
    }

    private fun scheduleEdit() {
        if (syncing || !active || pending) return
        pending = true
        container.post(publish)
    }

    private fun measuredLines(): Int {
        if (action != 0) return 1
        val key = Triple(editor.text.toString(), editor.layoutParams.width.coerceAtLeast(1), editor.textSize)
        if (key != lineMeasurement) {
            measuredLineCount = StaticLayout.Builder.obtain(editor.text, 0, editor.length(), editor.paint, key.second)
                .setIncludePad(false).setBreakStrategy(Layout.BREAK_STRATEGY_SIMPLE)
                .setHyphenationFrequency(Layout.HYPHENATION_FREQUENCY_NONE)
                .build().lineCount
            lineMeasurement = key
        }
        return measuredLineCount.coerceIn(1, MAX_EDITOR_LINES)
    }

    private fun resizeEditor() {
        val lines = measuredLines()
        // Android omits line spacing after the final line. Match its content height so
        // the unscrolled editor can draw underlines into its bottom padding too.
        val finalLineSpacing = if (action == 0 && measuredLineCount <= MAX_EDITOR_LINES) {
            editor.lineHeight - editor.paint.fontMetricsInt.run { descent - ascent }
        } else 0
        if (action == 0) editor.translationY = finalLineSpacing.toFloat()
        val height = lines * editor.lineHeight - finalLineSpacing + editor.paddingBottom
        if (editor.layoutParams.height != height) {
            editor.layoutParams = (editor.layoutParams as FrameLayout.LayoutParams).apply { this.height = height }
        }
    }

    private fun publishEdit() {
        container.removeCallbacks(publish)
        pending = false
        if (!active || inputId < 0) return
        val value = editor.text.toString()
        val cursor = editor.selectionEnd.coerceIn(0, value.length)
        if (value == nativeText && cursor == nativeCursor) return
        val payload = JSONObject().put("id", inputId).put("text", nativeText)
            .put("value", value).put("selection", cursor).put("lines", measuredLines())
        nativeText = value
        nativeCursor = cursor
        onEdit(TextEdit.Update(payload.toString()))
    }

    override fun sync(active: Boolean, action: Int, numeric: Boolean) {
        if (!active && !this.active) return
        val opening = active && !this.active
        val changed = action != this.action || numeric != this.numeric
        this.active = active
        if (!active) {
            container.removeCallbacks(publish)
            pending = false
            inputMethod.hideSoftInputFromWindow(editor.windowToken, 0)
            editor.clearFocus()
            imeVisible = false
            activity.setKeyboardInset(0)
            return
        }
        this.action = action
        this.numeric = numeric
        if (changed) {
            syncing = true
            editor.inputType = inputType()
            editor.setSingleLine(action != 0)
            editor.typeface = activity.publicSansTypeface
            editor.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI or when (action) {
                0 -> EditorInfo.IME_FLAG_NO_ENTER_ACTION
                2 -> EditorInfo.IME_ACTION_DONE
                else -> EditorInfo.IME_ACTION_SEARCH
            }
            syncing = false
        }
        if (opening || changed) editor.post {
            if (this.active) {
                editor.requestFocus()
                inputMethod.restartInput(editor)
                inputMethod.showSoftInput(editor, InputMethodManager.SHOW_IMPLICIT)
            }
        }
    }

    override val hasEditor: Boolean get() = editor.visibility == View.VISIBLE

    override fun syncContext(context: String) {
        if (context.isEmpty()) {
            awaitingEditorDraw = false
            hideAfterFrame = editor.visibility == View.VISIBLE
            return
        }
        hideAfterFrame = false
        editor.removeCallbacks(hideEditor)
        if (pending) return
        val data = JSONObject(context)
        val bounds = data.optJSONObject("editor") ?: return
        nativeBaseline = bounds.getDouble("baseline").toFloat()
        val width = bounds.getDouble("width").roundToInt().coerceAtLeast(1)
        val bottomPadding = bounds.getDouble("bottomPadding").roundToInt().coerceAtLeast(0)
        val height = bounds.getDouble("height").roundToInt().coerceAtLeast(1) + bottomPadding
        val params = editor.layoutParams as FrameLayout.LayoutParams
        val left = bounds.getDouble("x").roundToInt()
        val top = bounds.getDouble("y").roundToInt()
        if (params.width != width || params.height != height || params.leftMargin != left || params.topMargin != top) {
            editor.layoutParams = FrameLayout.LayoutParams(width, height).apply { leftMargin = left; topMargin = top }
        }
        val fontSize = bounds.getDouble("fontSize").toFloat()
        if (editor.textSize != fontSize) editor.setTextSize(TypedValue.COMPLEX_UNIT_PX, fontSize)
        val lineHeight = bounds.getDouble("lineHeight").roundToInt()
        if (editor.lineHeight != lineHeight) editor.setLineHeight(lineHeight)
        if (editor.paddingBottom != bottomPadding) {
            editor.setPadding(0, 0, 0, bottomPadding)
        }
        editor.visibility = View.VISIBLE
        val id = data.getInt("id")
        val text = data.getString("text")
        val placeholder = data.optString("placeholder")
        if (editor.hint?.toString() != placeholder) editor.hint = placeholder
        val hintColour = data.getInt("hintColour")
        if (editor.currentHintTextColor != hintColour) editor.setHintTextColor(hintColour)
        val cursor = data.getInt("cursor").coerceIn(0, text.length)
        syncing = true
        val changed = inputId != id
        val textChanged = editor.text.toString() != text
        val type = inputType()
        val typeChanged = editor.inputType != type
        if (typeChanged) {
            editor.inputType = type
            editor.typeface = activity.publicSansTypeface
        }
        if (changed || textChanged) editor.setText(text)
        if (changed || textChanged) editor.setSelection(cursor)
        resizeEditor()
        inputId = id
        nativeText = text
        nativeCursor = cursor
        syncing = false
        if ((changed || typeChanged) && editor.hasFocus()) inputMethod.restartInput(editor)
        if (active && !data.optBoolean("nativeEditor")) {
            awaitingEditorDraw = true
            editor.invalidate()
        }
    }

    override fun framePresented() {
        if (hideAfterFrame) {
            // Keep the editor until the replacement Vulkan frame reaches the compositor.
            editor.removeCallbacks(hideEditor)
            editor.postOnAnimation(hideEditor)
        }
    }

    private fun inputType(): Int = if (numeric) InputType.TYPE_CLASS_NUMBER else InputType.TYPE_CLASS_TEXT or
        (if (action == 0) InputType.TYPE_TEXT_FLAG_MULTI_LINE else 0) or
        InputType.TYPE_TEXT_FLAG_AUTO_CORRECT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES

    override fun dismiss(): Boolean {
        if (!active) return false
        publishEdit()
        onEdit(TextEdit.Dismiss)
        return true
    }

    override fun setLightAppearance(light: Boolean) {
        if (lightAppearance == light) return
        lightAppearance = light
        val colour = if (light) Color.BLACK else Color.WHITE
        editor.setTextColor(colour)
        editor.textCursorDrawable?.mutate()?.setTint(colour)
        editor.textSelectHandle?.mutate()?.setTint(colour)
        editor.textSelectHandleLeft?.mutate()?.setTint(colour)
        editor.textSelectHandleRight?.mutate()?.setTint(colour)
    }

    override fun pause() {
        publishEdit()
        sync(false, action, numeric)
        container.removeCallbacks(publish)
        editor.removeCallbacks(editorDrawn)
        editor.removeCallbacks(hideEditor)
        awaitingEditorDraw = false
        hideAfterFrame = false
        editor.visibility = View.GONE
        imeVisible = false
        activity.setKeyboardInset(0)
    }

    override fun close() {
        pause()
        container.setOnApplyWindowInsetsListener(null)
        container.removeView(editor)
    }
}
