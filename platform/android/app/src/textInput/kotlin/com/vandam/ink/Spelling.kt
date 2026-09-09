package com.vandam.ink

internal interface Spelling {
    fun sync(context: String) {}
    fun insert(text: String)
    fun backspace()
    fun dismiss(): Boolean = false
    fun close() {}
}
