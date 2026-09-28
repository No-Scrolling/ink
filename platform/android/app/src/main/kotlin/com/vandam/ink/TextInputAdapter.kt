package com.vandam.ink

internal sealed interface TextEdit {
    data class Update(val payload: String) : TextEdit
    data object Submit : TextEdit
    data object Dismiss : TextEdit
}

internal interface TextInputAdapter {
    val hasEditor: Boolean get() = false
    fun sync(active: Boolean, action: Int, numeric: Boolean)
    fun setLightAppearance(light: Boolean)
    fun dismiss(): Boolean
    fun syncContext(context: String) {}
    fun framePresented() {}
    fun pause() { close() }
    fun close() {}
}

internal typealias TextEditHandler = (TextEdit) -> Unit
