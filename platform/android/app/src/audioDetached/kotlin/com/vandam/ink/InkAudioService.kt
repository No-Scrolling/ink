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
    private lateinit var player: ExoPlayer
    private lateinit var session: MediaSession
    private val handler = Handler(Looper.getMainLooper())
    private val controllers = mutableSetOf<MediaSession.ControllerInfo>()
    private val idleStop = Runnable { stopSelf() }

    override fun onCreate() {
        super.onCreate()
        player = ExoPlayer.Builder(this)
            .setHandleAudioBecomingNoisy(true)
            .build()
            .apply {
                setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA)
                        .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
                        .build(),
                    true,
                )
                addListener(object : Player.Listener {
                    override fun onIsPlayingChanged(isPlaying: Boolean) = refreshIdleStop()
                })
            }
        session = MediaSession.Builder(this, player)
            .setId(packageName)
            .setCallback(object : MediaSession.Callback {
                override fun onConnect(
                    session: MediaSession,
                    controller: MediaSession.ControllerInfo,
                ): MediaSession.ConnectionResult {
                    if (controller.connectionHints.getBoolean(CONTROLLER_HINT)) {
                        controllers += controller
                        if (player.mediaItemCount == 0) {
                            val contentType = controller.connectionHints.getInt(
                                CONTENT_TYPE_HINT,
                                C.AUDIO_CONTENT_TYPE_MUSIC,
                            )
                            player.setAudioAttributes(
                                AudioAttributes.Builder()
                                    .setUsage(C.USAGE_MEDIA)
                                    .setContentType(contentType)
                                    .build(),
                                true,
                            )
                        }
                    }
                    refreshIdleStop()
                    return super.onConnect(session, controller)
                }

                override fun onDisconnected(
                    session: MediaSession,
                    controller: MediaSession.ControllerInfo,
                ) {
                    controllers -= controller
                    refreshIdleStop()
                }

                override fun onAddMediaItems(
                    mediaSession: MediaSession,
                    controller: MediaSession.ControllerInfo,
                    mediaItems: List<MediaItem>,
                ): ListenableFuture<List<MediaItem>> = Futures.immediateFuture(
                    mediaItems.map { item ->
                        val uri = item.requestMetadata.mediaUri
                        if (item.localConfiguration == null && uri != null) {
                            item.buildUpon().setUri(uri).build()
                        } else {
                            item
                        }
                    },
                )
            })
            .build()
        refreshIdleStop()
    }

    override fun onGetSession(
        controllerInfo: MediaSession.ControllerInfo,
    ): MediaSession = session

    override fun onDestroy() {
        handler.removeCallbacks(idleStop)
        session.release()
        player.release()
        super.onDestroy()
    }

    private fun refreshIdleStop() {
        handler.removeCallbacks(idleStop)
        if (!player.isPlaying && controllers.isEmpty()) {
            handler.postDelayed(idleStop, IDLE_STOP_MS)
        }
    }

    private companion object {
        private const val CONTROLLER_HINT = "ink.controller"
        private const val CONTENT_TYPE_HINT = "ink.contentType"
        private const val IDLE_STOP_MS = 15 * 60_000L
    }
}
