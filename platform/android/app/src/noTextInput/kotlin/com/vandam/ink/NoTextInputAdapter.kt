package com.vandam.ink

import android.view.ViewGroup

internal fun createTextInputAdapter(
    _activity: MainActivity,
    _container: ViewGroup,
    _onEdit: TextEditHandler,
): TextInputAdapter = object : TextInputAdapter {
    override fun sync(active: Boolean, action: Int) = Unit

    override fun dismiss() = false
}
