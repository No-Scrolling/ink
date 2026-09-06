package com.vandam.ink.keyboard

import com.vandam.ink.R

internal enum class KeyboardIcon(
    val resource: Int,
    val sizeDp: Float,
) {
    ChevronLeft(R.drawable.ink_keyboard_chevron_left, 32f),
    KeyboardArrowUp(R.drawable.ink_keyboard_arrow_up, 30f),
    KeyboardArrowDown(R.drawable.ink_keyboard_arrow_down, 30f),
    MatchCase(R.drawable.ink_keyboard_match_case, 30f),
    Mood(R.drawable.ink_keyboard_mood, 24f),
    KeyboardReturn(R.drawable.ink_keyboard_return, 30f),
    Search(R.drawable.ink_keyboard_search, 30f),
    Done(R.drawable.ink_keyboard_done, 30f),
}
