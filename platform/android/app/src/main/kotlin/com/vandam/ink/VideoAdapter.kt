package com.vandam.ink

internal data class VideoPortal(
    val controller: Long,
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int,
)

internal interface VideoAdapter : NativeAdapter {
    fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler)
    fun syncPortal(portal: VideoPortal?)
    fun pause()
    fun resume()
    fun stop()
}
