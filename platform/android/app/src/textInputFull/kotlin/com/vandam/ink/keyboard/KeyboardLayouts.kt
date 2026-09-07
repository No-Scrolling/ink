package com.vandam.ink.keyboard

import android.icu.text.BreakIterator
import java.util.Locale
import com.vandam.ink.R

internal object KeyboardLayouts {
    fun actionIcon(action: KeyboardAction): KeyboardIcon = when (action) {
        KeyboardAction.Search -> KeyboardIcon.Search
        KeyboardAction.Done -> KeyboardIcon.Done
        KeyboardAction.Return -> KeyboardIcon(R.drawable.ink_keyboard_return, 30f)
    }

    fun commandIcon(command: KeyboardCommand, shifted: Boolean): KeyboardIcon? = when (command) {
        KeyboardCommand.Shift -> KeyboardIcon(if (shifted) R.drawable.ink_keyboard_arrow_down else R.drawable.ink_keyboard_arrow_up, 30f)
        KeyboardCommand.Emoji -> KeyboardIcon(R.drawable.ink_keyboard_mood, 24f)
        KeyboardCommand.Letters -> KeyboardIcon(R.drawable.ink_keyboard_match_case, 30f)
        else -> null
    }

    fun forMode(mode: KeyboardMode, emojis: List<KeyboardKey.Emoji>): KeyboardLayout = when (mode) {
        KeyboardMode.Numeric -> numericLayout
        KeyboardMode.Letters -> letters
        KeyboardMode.Numbers -> numbers
        KeyboardMode.Symbols -> symbols
        KeyboardMode.Emoji -> emoji(emojis)
    }

    private val emojiValues = listOf(
        "😅", "☺️", "🙃", "😍", "😜", "😂", "😭", "😎",
        "🙌", "👍", "👎", "🤞", "✌️", "👌", "👋", "🙏",
        "✨", "🔥", "❤️", "💔", "🏆", "🎯", "👑", "👀",
    )

    val defaultEmojis: List<KeyboardKey.Emoji> = emojiValues.map { KeyboardKey.Emoji(it) }

    fun parseEmojis(value: String?): List<KeyboardKey.Emoji> {
        if (value.isNullOrEmpty()) return defaultEmojis
        val iterator = BreakIterator.getCharacterInstance(Locale.ROOT).apply { setText(value) }
        val emojis = mutableListOf<KeyboardKey.Emoji>()
        var start = iterator.first()
        var end = iterator.next()
        while (end != BreakIterator.DONE && emojis.size < defaultEmojis.size) {
            value.substring(start, end)
                .takeUnless(String::isBlank)
                ?.let { emojis += KeyboardKey.Emoji(it) }
            start = end
            end = iterator.next()
        }
        return emojis.ifEmpty { defaultEmojis }
    }

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

    private fun emoji(emojis: List<KeyboardKey.Emoji>) = KeyboardLayout(
        emojis.chunked(8).map { KeyboardRow(it) } + KeyboardRow(
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
