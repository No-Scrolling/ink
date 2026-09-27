package com.vandam.ink

import android.os.Bundle
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
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
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
        val effects = AudioEffects.get(this, name)
        val player = ExoPlayer.Builder(this, InkAudioRenderersFactory(this, effects))
            .setMediaSourceFactory(audioMediaSourceFactory(this))
            .setHandleAudioBecomingNoisy(true)
            .build().apply {
                setAudioAttributes(AudioAttributes.Builder()
                    .setUsage(C.USAGE_MEDIA)
                    .setContentType(controllerInfo.connectionHints.getInt(CONTENT_TYPE_HINT, C.AUDIO_CONTENT_TYPE_MUSIC))
                    .build(), true)
                addListener(object : Player.Listener {
                    override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
                        effects.countSavings = playWhenReady
                        if (playWhenReady) sessions.filterKeys { it != name }.values.forEach { it.player.pause() }
                        refreshIdleStop()
                    }
                    override fun onIsPlayingChanged(isPlaying: Boolean) = refreshIdleStop()
                })
            }
        val persistence = AudioQueuePersistence(this, name, player, effects).apply { restore() }
        lateinit var readiness: AudioReadiness
        fun publishEffects(session: MediaSession) {
            session.setSessionExtras(Bundle().apply {
                putBoolean(AUDIO_PREPARED_EXTRA, readiness.prepared)
                putBoolean(SKIP_SILENCE_EXTRA, player.skipSilenceEnabled)
                putBoolean(VOICE_BOOST_EXTRA, effects.voiceBoost)
            })
        }
        val session = MediaSession.Builder(this, player)
            .setId("$packageName:$name")
            .setCallback(object : MediaSession.Callback {
                override fun onConnect(session: MediaSession, controller: MediaSession.ControllerInfo): MediaSession.ConnectionResult {
                    if (controller.connectionHints.getBoolean(CONTROLLER_HINT)) controllers += controller
                    refreshIdleStop()
                    val result = super.onConnect(session, controller)
                    if (controller.uid != android.os.Process.myUid()) return result
                    return MediaSession.ConnectionResult.AcceptedResultBuilder(session)
                        .setAvailablePlayerCommands(result.availablePlayerCommands)
                        .setAvailableSessionCommands(result.availableSessionCommands.buildUpon()
                            .add(SessionCommand(SKIP_SILENCE_COMMAND, Bundle.EMPTY))
                            .add(SessionCommand(VOICE_BOOST_COMMAND, Bundle.EMPTY)).build())
                        .build()
                }
                override fun onCustomCommand(session: MediaSession, controller: MediaSession.ControllerInfo,
                    customCommand: SessionCommand, args: Bundle): ListenableFuture<SessionResult> {
                    if (controller.uid == android.os.Process.myUid() && customCommand.customAction == SKIP_SILENCE_COMMAND) {
                        player.skipSilenceEnabled = args.getBoolean(SKIP_SILENCE_EXTRA)
                        return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
                    }
                    if (controller.uid == android.os.Process.myUid() && customCommand.customAction == VOICE_BOOST_COMMAND) {
                        effects.setVoiceBoost(args.getBoolean(VOICE_BOOST_EXTRA))
                        publishEffects(session)
                        return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
                    }
                    return super.onCustomCommand(session, controller, customCommand, args)
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
        readiness = AudioReadiness { publishEffects(session) }.also { player.addListener(it) }
        player.addListener(object : Player.Listener {
            override fun onSkipSilenceEnabledChanged(skipSilenceEnabled: Boolean) = publishEffects(session)
        })
        publishEffects(session)
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
