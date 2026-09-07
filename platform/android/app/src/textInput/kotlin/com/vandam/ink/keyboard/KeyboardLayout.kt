package com.vandam.ink.keyboard

internal const val KEYBOARD_HEIGHT_DP = 164f

internal enum class KeyboardMode {
    Letters,
    Numeric,
    Numbers,
    Symbols,
    Emoji,
}

internal enum class KeyboardCommand {
    Shift,
    Backspace,
    Letters,
    Numbers,
    Symbols,
    Emoji,
    Space,
    Submit,
}

internal enum class KeyContentAlignment {
    Start,
    Centre,
    End,
}

internal sealed interface KeyboardKey {
    val widthDp: Float
    val alignment: KeyContentAlignment

    data class Text(
        val value: String,
        override val widthDp: Float = 35f,
        override val alignment: KeyContentAlignment = KeyContentAlignment.Centre,
    ) : KeyboardKey

    data class Emoji(
        val value: String,
        override val widthDp: Float = 43f,
        override val alignment: KeyContentAlignment = KeyContentAlignment.Centre,
    ) : KeyboardKey

    data class Command(
        val command: KeyboardCommand,
        val label: String? = null,
        override val widthDp: Float = 49f,
        override val alignment: KeyContentAlignment = KeyContentAlignment.Centre,
    ) : KeyboardKey

    data class Gap(
        override val widthDp: Float,
    ) : KeyboardKey {
        override val alignment = KeyContentAlignment.Centre
    }
}

internal data class KeyboardRow(
    val keys: List<KeyboardKey>,
    val heightDp: Float = 44f,
)

internal data class KeyboardLayout(
    val rows: List<KeyboardRow>,
)
