package com.vandam.ink

import android.view.ViewGroup

internal fun createTextInputAdapter(
    _activity: MainActivity,
    _container: ViewGroup,
    _onEdit: TextEditHandler,
): TextInputAdapter = object : TextInputAdapter {
    override fun sync(active: Boolean, action: Int) = Unit

    override fun setLightAppearance(light: Boolean) = Unit

    override fun dismiss() = false

    override fun applyPreferences(preferences: KeyboardPreferences) = Unit
}
