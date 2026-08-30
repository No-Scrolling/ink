package com.vandam.ink

internal sealed interface TextEdit {
    data class Insert(val text: String) : TextEdit
    data object Backspace : TextEdit
    data object Submit : TextEdit
    data object Dismiss : TextEdit
}

internal interface TextInputAdapter {
    fun sync(active: Boolean, action: Int)
    fun dismiss(): Boolean
}

internal typealias TextEditHandler = (TextEdit) -> Unit
