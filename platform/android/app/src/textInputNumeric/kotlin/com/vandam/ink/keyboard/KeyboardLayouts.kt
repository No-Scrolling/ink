package com.vandam.ink.keyboard

internal object KeyboardLayouts {
    val defaultEmojis: List<KeyboardKey.Emoji> = emptyList()
    fun parseEmojis(value: String?): List<KeyboardKey.Emoji> = emptyList()
    fun forMode(mode: KeyboardMode, emojis: List<KeyboardKey.Emoji>): KeyboardLayout = numericLayout
    fun actionIcon(action: KeyboardAction): KeyboardIcon = when (action) {
        KeyboardAction.Search -> KeyboardIcon.Search
        else -> KeyboardIcon.Done
    }
    fun commandIcon(command: KeyboardCommand, shifted: Boolean): KeyboardIcon? = null
}
