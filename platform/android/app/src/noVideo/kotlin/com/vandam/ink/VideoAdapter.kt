package com.vandam.ink

import android.app.Activity
import android.widget.FrameLayout

internal fun createVideoAdapter(
    _activity: Activity,
    _root: FrameLayout,
    _updateController: (Long, String) -> Unit,
    _bundle: InkBundle,
    _clock: PlaybackClock,
): VideoAdapter = object : VideoAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) = complete(unavailable())
    override fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler) = complete(unavailable())
    override fun cancel(requestId: Long) = Unit
    override fun syncPortal(portal: VideoPortal?) = Unit
    override fun pause() = Unit
    override fun resume() = Unit
    override fun stop() = Unit
    private fun unavailable() = NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Video playback is not enabled", false)
}
