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
    private val spelling = lazy { createSpelling(activity, onEdit, keyboard) }
    private val inputActions = lazy { InputActions(activity, keyboard, onEdit) }

    override fun syncContext(context: String) {
        if (context.isNotEmpty() || spelling.isInitialized()) spelling.value.sync(context)
        if (context.isNotEmpty() || inputActions.isInitialized()) inputActions.value.sync(context)
    }

    override fun close() {
        if (spelling.isInitialized()) spelling.value.close()
        if (inputActions.isInitialized()) inputActions.value.close()
    }

    private fun resizeContent() {
        val inset = if (keyboard.visibility == View.VISIBLE) keyboard.height else 0
        activity.setKeyboardInset(inset)
    }

    override fun sync(active: Boolean, action: Int, numeric: Boolean) {
        if (!active && !keyboardView.isInitialized()) return
        keyboard.numeric = numeric
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
        if (inputActions.isInitialized() && inputActions.value.dismiss()) return true
        if (spelling.isInitialized() && spelling.value.dismiss()) return true
        if (!keyboardView.isInitialized() || keyboard.visibility != View.VISIBLE) {
            return false
        }
        onEdit(TextEdit.Dismiss)
        sync(false, 0, false)
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
        if (inputActions.isInitialized()) inputActions.value.dismiss()
        spelling.value.insert(text)
    }

    override fun onBackspace() {
        if (inputActions.isInitialized()) inputActions.value.dismiss()
        spelling.value.backspace()
    }

    override fun onAction() {
        if (inputActions.isInitialized()) inputActions.value.dismiss()
        if (keyboard.action == KeyboardAction.Return) {
            spelling.value.insert("\n")
        } else {
            onEdit(TextEdit.Submit)
        }
    }
}
