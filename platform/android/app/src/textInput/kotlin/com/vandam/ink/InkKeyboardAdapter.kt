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
    private var lightAppearance = false
    private var preferences: KeyboardPreferences? = null
    private val keyboardView = lazy {
        InkKeyboardView(activity).apply {
            action = KeyboardAction.Search
            listener = this@InkKeyboardAdapter
            typeface = activity.publicSansTypeface
            visibility = View.GONE
            lightAppearance = this@InkKeyboardAdapter.lightAppearance
            preferences?.let {
                isHapticFeedbackEnabled = it.hapticsEnabled
                emojis = it.emojis
                keyAnimationEnabled = it.keyAnimationEnabled
            }
        }.also { keyboard ->
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
    }
    private val keyboard by keyboardView

    private fun resizeContent() {
        val inset = if (keyboard.visibility == View.VISIBLE) keyboard.height else 0
        activity.setKeyboardInset(inset)
    }

    override fun sync(active: Boolean, action: Int) {
        if (!active && !keyboardView.isInitialized()) return
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
        lightAppearance = light
        if (keyboardView.isInitialized()) keyboard.lightAppearance = light
    }

    override fun dismiss(): Boolean {
        if (!keyboardView.isInitialized() || keyboard.visibility != View.VISIBLE) {
            return false
        }
        onEdit(TextEdit.Dismiss)
        sync(false, 0)
        return true
    }

    override fun applyPreferences(preferences: KeyboardPreferences) {
        this.preferences = preferences
        if (!keyboardView.isInitialized()) return
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
        if (keyboard.action == KeyboardAction.Return) {
            onEdit(TextEdit.Insert("\n"))
        } else {
            onEdit(TextEdit.Submit)
            sync(false, 0)
        }
    }
}
