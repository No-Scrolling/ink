package com.vandam.ink

internal interface DownloadsAdapter : NativeAdapter {
    fun executeController(controller: Long, operation: String, payload: String, complete: NativeResultHandler)
    fun stop()
}
