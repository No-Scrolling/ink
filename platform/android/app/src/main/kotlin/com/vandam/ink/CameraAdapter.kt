package com.vandam.ink

internal data class CameraPortal(
    val controller: Long,
    val kind: String,
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int,
)

internal interface CameraAdapter : NativeAdapter {
    fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    )

    fun permissionDenied(
        requestId: Long,
        controller: Long,
        blocked: Boolean,
        complete: NativeResultHandler,
    )

    fun syncPortal(portal: CameraPortal?)

    fun pause()

    fun resume()

    fun stop()
}
