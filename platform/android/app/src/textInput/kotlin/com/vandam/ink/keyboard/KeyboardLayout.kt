package com.vandam.ink.keyboard

internal const val KEYBOARD_HEIGHT_DP = 164f
internal const val DISMISS_HEIGHT_DP = 56f

internal enum class KeyboardMode {
    Letters,
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
    Dismiss,
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
        val atlasIndex: Int,
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

internal object KeyboardLayouts {
    fun forMode(mode: KeyboardMode): KeyboardLayout = when (mode) {
        KeyboardMode.Letters -> letters
        KeyboardMode.Numbers -> numbers
        KeyboardMode.Symbols -> symbols
        KeyboardMode.Emoji -> emoji
    }

    private val emojiValues = listOf(
        "😅", "☺️", "🙃", "😍", "😜", "😂", "😭", "😎",
        "🙌", "👍", "👎", "🤞", "✌️", "👌", "👋", "🙏",
        "✨", "🔥", "❤️", "💔", "🏆", "🎯", "👑", "👀",
    )

    private val letters = KeyboardLayout(
        listOf(
            textRow("qwertyuiop"),
            textRow("asdfghjkl"),
            KeyboardRow(
                listOf(command(KeyboardCommand.Shift)) +
                    textKeys("zxcvbnm") +
                    command(KeyboardCommand.Backspace),
            ),
            standardBottomRow(KeyboardCommand.Numbers, "123"),
        ),
    )

    private val numbers = KeyboardLayout(
        listOf(
            textRow("1234567890"),
            textRow(listOf("-", "/", ":", ";", "(", ")", "$", "&", "@", "\"")),
            shortThirdRow(".,?!'", KeyboardCommand.Symbols, "#+="),
            standardBottomRow(KeyboardCommand.Letters),
        ),
    )

    private val symbols = KeyboardLayout(
        listOf(
            textRow(listOf("[", "]", "{", "}", "#", "%", "^", "*", "+", "=")),
            textRow(listOf("_", "\\", "|", "~", "<", ">", "€", "£", "¥")),
            shortThirdRow(".,?!'", KeyboardCommand.Numbers, "123"),
            standardBottomRow(KeyboardCommand.Letters),
        ),
    )

    private val emoji = KeyboardLayout(
        emojiValues.chunked(8).mapIndexed { row, values ->
            KeyboardRow(
                values.mapIndexed { column, value ->
                    KeyboardKey.Emoji(value, row * 8 + column)
                },
            )
        } + KeyboardRow(
            listOf(
                command(KeyboardCommand.Letters),
                KeyboardKey.Gap(245f),
                command(KeyboardCommand.Backspace),
            ),
            heightDp = 28f,
        ),
    )

    private fun textRow(characters: String): KeyboardRow = textRow(
        characters.map { it.toString() },
    )

    private fun textRow(values: List<String>): KeyboardRow = KeyboardRow(
        values.map { KeyboardKey.Text(it) },
    )

    private fun textKeys(characters: String): List<KeyboardKey> =
        characters.map { KeyboardKey.Text(it.toString()) }

    private fun shortThirdRow(
        characters: String,
        leftCommand: KeyboardCommand,
        leftLabel: String,
    ): KeyboardRow = KeyboardRow(
        listOf(command(leftCommand, leftLabel), KeyboardKey.Gap(39f)) +
            textKeys(characters) +
            listOf(KeyboardKey.Gap(31f), command(KeyboardCommand.Backspace)),
    )

    private fun standardBottomRow(
        leftCommand: KeyboardCommand,
        leftLabel: String? = null,
    ): KeyboardRow = KeyboardRow(
        listOf(
            command(leftCommand, leftLabel),
            command(
                KeyboardCommand.Emoji,
                widthDp = 47f,
                alignment = KeyContentAlignment.Start,
            ),
            command(KeyboardCommand.Space, widthDp = 160f),
            KeyboardKey.Text(
                ".",
                widthDp = 47f,
                alignment = KeyContentAlignment.End,
            ),
            command(KeyboardCommand.Submit, widthDp = 47f),
        ),
        heightDp = 28f,
    )

    private fun command(
        command: KeyboardCommand,
        label: String? = null,
        widthDp: Float = 49f,
        alignment: KeyContentAlignment = KeyContentAlignment.Centre,
    ): KeyboardKey.Command = KeyboardKey.Command(command, label, widthDp, alignment)
}
