package com.vandam.ink

import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture

@UnstableApi
internal class InkAudioService : MediaSessionService() {
    private data class Session(val player: ExoPlayer, val session: MediaSession, val persistence: AudioQueuePersistence)
    private val sessions = mutableMapOf<String, Session>()
    private val handler = Handler(Looper.getMainLooper())
    private val controllers = mutableSetOf<MediaSession.ControllerInfo>()
    private val idleStop = Runnable { stopSelf() }

    override fun onCreate() {
        super.onCreate()
        refreshIdleStop()
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? {
        if (!controllerInfo.connectionHints.getBoolean(CONTROLLER_HINT)) {
            return sessions.values.firstOrNull { it.player.isPlaying }?.session
                ?: sessions.values.firstOrNull()?.session
        }
        if (controllerInfo.uid != android.os.Process.myUid()) return null
        val name = controllerInfo.connectionHints.getString("ink.session", "main")
        if (!Regex("[A-Za-z0-9._-]{1,64}").matches(name)) return null
        sessions[name]?.let { return it.session }
        if (sessions.size >= 8) return null
        val player = ExoPlayer.Builder(this).setHandleAudioBecomingNoisy(true).build().apply {
            setAudioAttributes(AudioAttributes.Builder()
                .setUsage(C.USAGE_MEDIA)
                .setContentType(controllerInfo.connectionHints.getInt(CONTENT_TYPE_HINT, C.AUDIO_CONTENT_TYPE_MUSIC))
                .build(), true)
            addListener(object : Player.Listener {
                override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
                    if (playWhenReady) sessions.filterKeys { it != name }.values.forEach { it.player.pause() }
                    refreshIdleStop()
                }
                override fun onIsPlayingChanged(isPlaying: Boolean) = refreshIdleStop()
            })
        }
        val persistence = AudioQueuePersistence(this, name, player).apply { restore() }
        val session = MediaSession.Builder(this, player)
            .setId("$packageName:$name")
            .setCallback(object : MediaSession.Callback {
                override fun onConnect(session: MediaSession, controller: MediaSession.ControllerInfo): MediaSession.ConnectionResult {
                    if (controller.connectionHints.getBoolean(CONTROLLER_HINT)) controllers += controller
                    refreshIdleStop()
                    return super.onConnect(session, controller)
                }
                override fun onDisconnected(session: MediaSession, controller: MediaSession.ControllerInfo) {
                    controllers -= controller
                    refreshIdleStop()
                }
                override fun onAddMediaItems(mediaSession: MediaSession, controller: MediaSession.ControllerInfo,
                    mediaItems: List<MediaItem>): ListenableFuture<List<MediaItem>> = Futures.immediateFuture(
                    mediaItems.map { item ->
                        val uri = item.requestMetadata.mediaUri
                        if (item.localConfiguration == null && uri != null) item.buildUpon().setUri(uri).build() else item
                    })
            }).build()
        sessions[name] = Session(player, session, persistence)
        addSession(session)
        return session
    }

    override fun onDestroy() {
        handler.removeCallbacks(idleStop)
        sessions.values.forEach {
            it.persistence.close()
            it.session.release()
            it.player.release()
        }
        sessions.clear()
        super.onDestroy()
    }

    private fun refreshIdleStop() {
        handler.removeCallbacks(idleStop)
        if (sessions.values.none { it.player.playWhenReady } && controllers.isEmpty()) {
            handler.postDelayed(idleStop, IDLE_STOP_MS)
        }
    }

    private companion object {
        private const val CONTROLLER_HINT = "ink.controller"
        private const val CONTENT_TYPE_HINT = "ink.contentType"
        private const val IDLE_STOP_MS = 15 * 60_000L
    }
}
