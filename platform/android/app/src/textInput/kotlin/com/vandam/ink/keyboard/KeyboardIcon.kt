package com.vandam.ink.keyboard

import com.vandam.ink.R

internal data class KeyboardIcon(val resource: Int, val sizeDp: Float) {
    companion object {
        val ChevronLeft = KeyboardIcon(R.drawable.ink_keyboard_chevron_left, 32f)
        val Search = KeyboardIcon(R.drawable.ink_keyboard_search, 30f)
        val Done = KeyboardIcon(R.drawable.ink_keyboard_done, 30f)
    }
}
