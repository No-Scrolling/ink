package com.vandam.ink

import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import com.vandam.ink.keyboard.InkKeyboardView
import com.vandam.ink.keyboard.KeyboardAction
import com.vandam.ink.keyboard.KeyboardListener

internal fun createTextInputAdapter(
    activity: MainActivity,
    container: ViewGroup,
    onEdit: TextEditHandler,
): TextInputAdapter = InkKeyboardAdapter(activity, container, onEdit)

private class InkKeyboardAdapter(
    private val activity: MainActivity,
    container: ViewGroup,
    private val onEdit: TextEditHandler,
) : TextInputAdapter, KeyboardListener {
    private val keyboard = InkKeyboardView(activity).apply {
        action = KeyboardAction.Search
        listener = this@InkKeyboardAdapter
        typeface = activity.publicSansTypeface
        visibility = View.GONE
    }

    init {
        container.addView(
            keyboard,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
                Gravity.BOTTOM,
            ),
        )
        keyboard.addOnLayoutChangeListener { _, _, _, _, _, _, _, _, _ ->
            resizeContent()
        }
    }

    private fun resizeContent() {
        val inset = if (keyboard.visibility == View.VISIBLE) keyboard.height else 0
        activity.setKeyboardInset(inset)
    }

    override fun sync(active: Boolean, action: Int) {
        keyboard.action = when (action) {
            0 -> KeyboardAction.Return
            2 -> KeyboardAction.Done
            else -> KeyboardAction.Search
        }
        if (active && keyboard.visibility != View.VISIBLE) {
            keyboard.reset()
        }
        keyboard.visibility = if (active) View.VISIBLE else View.GONE
        resizeContent()
        if (active) {
            keyboard.requestFocus()
        }
    }

    override fun setLightAppearance(light: Boolean) {
        keyboard.lightAppearance = light
    }

    override fun dismiss(): Boolean {
        if (keyboard.visibility != View.VISIBLE) {
            return false
        }
        onDismiss()
        return true
    }

    override fun applyPreferences(preferences: KeyboardPreferences) {
        keyboard.isHapticFeedbackEnabled = preferences.hapticsEnabled
        keyboard.emojis = preferences.emojis
        keyboard.keyAnimationEnabled = preferences.keyAnimationEnabled
    }

    override fun onText(text: String) {
        onEdit(TextEdit.Insert(text))
    }

    override fun onBackspace() {
        onEdit(TextEdit.Backspace)
    }

    override fun onAction() {
        onEdit(TextEdit.Submit)
        sync(false, 0)
    }

    override fun onDismiss() {
        onEdit(TextEdit.Dismiss)
        sync(false, 0)
    }
}
