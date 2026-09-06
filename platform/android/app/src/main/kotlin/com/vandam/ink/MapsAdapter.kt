package com.vandam.ink

internal data class MapPortal(
    val controller: Long,
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int,
)

internal interface MapsAdapter : NativeAdapter {
    fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler)
    fun syncPortal(portal: MapPortal?)
    fun pause()
    fun resume()
    fun stop()
}
