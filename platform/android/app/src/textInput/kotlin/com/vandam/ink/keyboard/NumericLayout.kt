package com.vandam.ink.keyboard

internal val numericLayout = KeyboardLayout(
    listOf("123", "456", "789").map { digits ->
        KeyboardRow(digits.map { KeyboardKey.Text(it.toString(), widthDp = 72f) }, heightDp = 40f)
    } + KeyboardRow(
        listOf(
            KeyboardKey.Command(KeyboardCommand.Backspace, widthDp = 72f),
            KeyboardKey.Text("0", widthDp = 72f),
            KeyboardKey.Command(KeyboardCommand.Submit, widthDp = 72f),
        ),
        heightDp = 40f,
    ),
)
