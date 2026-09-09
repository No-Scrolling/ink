package com.vandam.ink

import com.vandam.ink.keyboard.InkKeyboardView

internal fun createSpelling(activity: MainActivity, onEdit: TextEditHandler, keyboard: InkKeyboardView): Spelling =
    object : Spelling {
        override fun insert(text: String) = onEdit(TextEdit.Insert(text))
        override fun backspace() = onEdit(TextEdit.Backspace)
    }
