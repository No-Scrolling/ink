package com.vandam.ink

import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.session.MediaController
import android.os.Handler
import android.view.KeyEvent
import android.view.ViewConfiguration
import androidx.media3.common.util.UnstableApi
import androidx.media3.common.util.Util
import androidx.media3.session.MediaSession

/**
 * Extends Media3's legacy mobile play/pause gesture and sends skips straight to the player, since the
 * platform session withholds them from speech for LightOS's controls; other keys keep Media3's handling.
 */
@UnstableApi
internal class AudioMediaButtons(private val context: Context) {
    private var handler: Handler? = null
    private var presses = 0
    private var pending: Runnable? = null

    fun onMediaButtonEvent(session: MediaSession, controller: MediaSession.ControllerInfo, intent: Intent): Boolean {
        val event = intent.getParcelableExtra(Intent.EXTRA_KEY_EVENT, KeyEvent::class.java) ?: return false
        if (event.action != KeyEvent.ACTION_DOWN) return false
        val toggle = event.keyCode == KeyEvent.KEYCODE_HEADSETHOOK || event.keyCode == KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE
        if (!toggle || event.repeatCount != 0 ||
            controller.controllerVersion != MediaSession.ControllerInfo.LEGACY_CONTROLLER_VERSION ||
            context.packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)) {
            val hadPending = pending != null
            flush()
            when (event.keyCode) {
                KeyEvent.KEYCODE_MEDIA_NEXT -> session.player.seekToNext()
                KeyEvent.KEYCODE_MEDIA_PREVIOUS -> session.player.seekToPrevious()
                // Queue this key behind the resolved gesture, then let Media3 map it normally.
                else -> if (hadPending) MediaController(context, session.platformToken).dispatchMediaButtonEvent(event) else return false
            }
            return true
        }
        val scheduler = handler ?: Handler(session.player.applicationLooper).also { handler = it }
        pending?.let(scheduler::removeCallbacks)
        presses++
        pending = Runnable {
            val count = presses
            presses = 0
            pending = null
            when (count) {
                1 -> Util.handlePlayPauseButtonAction(session.player, session.showPlayButtonIfPlaybackIsSuppressed)
                2 -> session.player.seekToNext()
                else -> session.player.seekToPrevious()
            }
        }
        if (presses == 3) flush()
        else scheduler.postDelayed(pending!!, ViewConfiguration.getDoubleTapTimeout().toLong())
        return true
    }

    private fun flush() {
        val task = pending ?: return
        handler?.removeCallbacks(task)
        task.run()
    }

    fun close() {
        pending?.let { handler?.removeCallbacks(it) }
        pending = null
        presses = 0
    }
}
