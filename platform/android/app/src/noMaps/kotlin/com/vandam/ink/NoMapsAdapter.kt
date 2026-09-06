package com.vandam.ink

import android.widget.FrameLayout

internal fun createMapsAdapter(
    _activity: MainActivity,
    _root: FrameLayout,
    _updateController: (Long, String) -> Unit,
): MapsAdapter = object : MapsAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) = complete(unavailable())
    override fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler) = complete(unavailable())
    override fun cancel(requestId: Long) = Unit
    override fun syncPortal(portal: MapPortal?) = Unit
    override fun pause() = Unit
    override fun resume() = Unit
    override fun stop() = Unit
    private fun unavailable() = NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Maps are not enabled", false)
}
