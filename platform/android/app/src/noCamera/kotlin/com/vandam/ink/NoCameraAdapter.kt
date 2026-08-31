package com.vandam.ink

import android.widget.FrameLayout

internal fun createCameraAdapter(
    _activity: MainActivity,
    _root: FrameLayout,
    _updateController: (Long, String) -> Unit,
    _updateReview: (Long, String?) -> Unit,
    _requestOpen: (Long) -> Unit,
): CameraAdapter = object : CameraAdapter {
    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) = complete(unavailable())

    override fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) = complete(unavailable())

    override fun permissionDenied(
        requestId: Long,
        controller: Long,
        blocked: Boolean,
        complete: NativeResultHandler,
    ) = complete(unavailable())

    override fun cancel(requestId: Long) = Unit

    override fun syncPortal(portal: CameraPortal?) = Unit

    override fun pause() = Unit

    override fun resume() = Unit

    override fun stop() = Unit

    private fun unavailable() = NativeResult.Failure(
        NativeErrorKind.UNAVAILABLE,
        "Camera is not enabled",
        false,
    )
}
