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

/** Extends Media3's legacy mobile play/pause gesture; other keys keep Media3's handling. */
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
            // Queue this key behind the resolved gesture, then let Media3 map it normally.
            if (hadPending) {
                MediaController(context, session.platformToken).dispatchMediaButtonEvent(event)
                return true
            }
            return false
        }
        val scheduler = handler ?: Handler(session.player.applicationLooper).also { handler = it }
        pending?.let(scheduler::removeCallbacks)
        presses++
        pending = Runnable {
            val count = presses
            presses = 0
            pending = null
            val transport = MediaController(context, session.platformToken).transportControls
            // Use the platform controller so Media3 still enforces session commands and its
            // prepare/restart behaviour, including seekToPrevious's default 3-second threshold.
            when (count) {
                1 -> if (Util.shouldShowPlayButton(session.player, session.showPlayButtonIfPlaybackIsSuppressed)) transport.play() else transport.pause()
                2 -> transport.skipToNext()
                else -> transport.skipToPrevious()
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
