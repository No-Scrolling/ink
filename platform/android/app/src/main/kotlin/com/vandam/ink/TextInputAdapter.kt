package com.vandam.ink

internal sealed interface TextEdit {
    data class Insert(val text: String) : TextEdit
    data class Assistance(val payload: String) : TextEdit
    data object Backspace : TextEdit
    data object Submit : TextEdit
    data object Dismiss : TextEdit
}

internal interface TextInputAdapter {
    fun sync(active: Boolean, action: Int, numeric: Boolean)
    fun setLightAppearance(light: Boolean)
    fun dismiss(): Boolean
    fun syncContext(context: String) {}
    fun pause() { close() }
    fun close() {}
}

internal typealias TextEditHandler = (TextEdit) -> Unit
