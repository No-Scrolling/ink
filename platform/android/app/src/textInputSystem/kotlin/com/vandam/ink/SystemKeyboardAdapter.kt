package com.vandam.ink

import android.content.Context
import android.text.Editable
import android.text.InputFilter
import android.text.InputType
import android.text.TextWatcher
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import org.json.JSONObject

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
    private var suggestions = false
    private var autoCorrect = false
    private var inputId = -1
    private var nativeText = ""
    private var nativeCursor = 0
    private var syncing = false
    private var pending = false
    private var imeVisible = false
    private val publish = Runnable { publishEdit() }
    private val editor = object : EditText(activity) {
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
        editor.alpha = 0f
        editor.isFocusableInTouchMode = true
        editor.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
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
            override fun afterTextChanged(text: Editable?) = scheduleEdit()
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
            activity.setKeyboardInset(if (visible) insets.getInsets(WindowInsets.Type.ime()).bottom else 0)
            val dismissed = imeVisible && !visible && active
            imeVisible = visible
            if (dismissed) onEdit(TextEdit.Dismiss)
            insets
        }
    }

    private fun scheduleEdit() {
        if (syncing || !active || pending) return
        pending = true
        container.post(publish)
    }

    private fun publishEdit() {
        container.removeCallbacks(publish)
        pending = false
        if (!active || inputId < 0) return
        val value = editor.text.toString()
        val cursor = editor.selectionEnd.coerceIn(0, value.length)
        if (value == nativeText && cursor == nativeCursor) return
        val payload = JSONObject().put("id", inputId).put("text", nativeText)
            .put("value", value).put("selection", cursor)
        nativeText = value
        nativeCursor = cursor
        onEdit(TextEdit.Assistance(payload.toString()))
    }

    override fun sync(active: Boolean, action: Int, numeric: Boolean) {
        if (!active && !this.active) return
        val opening = active && !this.active
        val changed = action != this.action || numeric != this.numeric
        this.active = active
        this.action = action
        this.numeric = numeric
        if (!active) {
            container.removeCallbacks(publish)
            pending = false
            inputId = -1
            inputMethod.hideSoftInputFromWindow(editor.windowToken, 0)
            editor.clearFocus()
            return
        }
        if (changed) {
            syncing = true
            editor.inputType = inputType()
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

    override fun syncContext(context: String) {
        if (!active || context.isEmpty() || pending) return
        val data = JSONObject(context)
        val id = data.getInt("id")
        val text = data.getString("text")
        val cursor = data.getInt("cursor").coerceIn(0, text.length)
        syncing = true
        val changed = inputId != id
        autoCorrect = data.getBoolean("autoCorrect")
        suggestions = data.getBoolean("spellCheck") || autoCorrect
        val type = inputType()
        val typeChanged = editor.inputType != type
        if (typeChanged) editor.inputType = type
        if (changed || editor.text.toString() != text) editor.setText(text)
        if (changed || editor.selectionEnd != cursor) editor.setSelection(cursor)
        inputId = id
        nativeText = text
        nativeCursor = cursor
        syncing = false
        if ((changed || typeChanged) && editor.hasFocus()) inputMethod.restartInput(editor)
    }

    private fun inputType(): Int = if (numeric) InputType.TYPE_CLASS_NUMBER else InputType.TYPE_CLASS_TEXT or
        (if (action == 0) InputType.TYPE_TEXT_FLAG_MULTI_LINE else 0) or
        (if (autoCorrect) InputType.TYPE_TEXT_FLAG_AUTO_CORRECT else 0) or
        (if (suggestions) 0 else InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS)

    override fun dismiss(): Boolean {
        if (!active) return false
        publishEdit()
        onEdit(TextEdit.Dismiss)
        return true
    }

    override fun setLightAppearance(light: Boolean) = Unit
    override fun applyPreferences(preferences: KeyboardPreferences) = Unit

    override fun close() {
        active = false
        container.removeCallbacks(publish)
        container.setOnApplyWindowInsetsListener(null)
        inputMethod.hideSoftInputFromWindow(editor.windowToken, 0)
        container.removeView(editor)
    }
}
