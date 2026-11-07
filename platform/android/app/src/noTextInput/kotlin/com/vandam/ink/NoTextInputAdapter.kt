package com.vandam.ink

import android.app.Activity
import android.view.ViewGroup

internal fun createTextInputAdapter(
    _activity: Activity,
    _container: ViewGroup,
    _onEdit: TextEditHandler,
    _typeface: () -> android.graphics.Typeface,
    _onInset: (Int) -> Unit,
): TextInputAdapter = object : TextInputAdapter {
    override fun sync(active: Boolean, action: Int, numeric: Boolean) = Unit

    override fun setLightAppearance(light: Boolean) = Unit

    override fun dismiss() = false

}
