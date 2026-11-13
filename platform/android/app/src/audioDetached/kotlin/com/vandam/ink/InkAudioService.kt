package com.vandam.ink

import android.app.PendingIntent
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionError
import androidx.media3.session.SessionResult
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture

@UnstableApi
internal class InkAudioService : MediaSessionService() {
    private data class Session(val player: ExoPlayer, val session: MediaSession, val deferred: DeferredAudioQueue, val persistence: AudioQueuePersistence, val usage: AudioUsage, val cache: Boolean, val offload: Boolean, val preloadNext: Long, val startupBuffer: Int, val audioOffload: AudioOffload?, val mediaButtons: AudioMediaButtons)
    private val sessions = mutableMapOf<String, Session>()
    private val handler = Handler(Looper.getMainLooper())
    private val controllers = mutableSetOf<MediaSession.ControllerInfo>()
    private val idleStop = Runnable { stopSelf() }

    override fun onCreate() {
        super.onCreate()
        refreshIdleStop()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)
        return START_NOT_STICKY
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? {
        if (!controllerInfo.connectionHints.getBoolean(CONTROLLER_HINT)) {
            return sessions.values.firstOrNull { it.player.isPlaying }?.session
                ?: sessions.values.firstOrNull()?.session
        }
        if (controllerInfo.uid != android.os.Process.myUid()) return null
        val name = controllerInfo.connectionHints.getString("ink.session", "main")
        if (!Regex("[A-Za-z0-9._-]{1,64}").matches(name)) return null
        val usage = runCatching { AudioUsage.from(controllerInfo.connectionHints.getString(USAGE_HINT, "music")) }.getOrNull() ?: return null
        val cache = controllerInfo.connectionHints.getBoolean(CACHE_HINT, false)
        val offload = controllerInfo.connectionHints.getBoolean(OFFLOAD_HINT, false)
        val preloadNext = controllerInfo.connectionHints.getLong(PRELOAD_NEXT_HINT, 0)
        val startupBuffer = controllerInfo.connectionHints.getInt(STARTUP_BUFFER_HINT, 1000)
        if (startupBuffer !in 0..30_000) return null
        if (preloadNext !in 0..30_000) return null
        sessions[name]?.let {
            return if (it.cache == cache && it.offload == offload && it.preloadNext == preloadNext && it.startupBuffer == startupBuffer && it.usage == usage) it.session else null
        }
        if (sessions.size >= 8) return null
        val effects = AudioEffects.get(this, name)
        val player = ExoPlayer.Builder(this, InkAudioRenderersFactory(this, effects))
            .setLoadControl(audioLoadControl(startupBuffer))
            .setMediaSourceFactory(audioMediaSourceFactory(this, cache, name))
            .setHandleAudioBecomingNoisy(true)
            .setSeekBackIncrementMs(AUDIO_SEEK_INCREMENT_MS)
            .setSeekForwardIncrementMs(AUDIO_SEEK_INCREMENT_MS)
            .build().apply {
                preloadConfiguration = ExoPlayer.PreloadConfiguration(preloadNext * 1000)
                addListener(object : Player.Listener {
                    override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
                        effects.countSavings = playWhenReady
                        if (playWhenReady) sessions.filterKeys { it != name }.values.forEach { it.player.pause() }
                        refreshIdleStop()
                    }
                    override fun onIsPlayingChanged(isPlaying: Boolean) = refreshIdleStop()
                })
            }
        val audioOffload = if (offload) AudioOffload(this, player, effects, true) else null
        val mediaButtons = AudioMediaButtons(this)
        val deferred = DeferredAudioQueue(player)
        val persistence = AudioQueuePersistence(this, name, player, effects, deferred).apply { restore() }
        lateinit var readiness: AudioReadiness
        var effectiveUsage = player.currentMediaItem?.itemUsage() ?: usage
        fun publishEffects(session: MediaSession) {
            session.setSessionExtras(Bundle().apply {
                putString(SESSION_USAGE_EXTRA, effectiveUsage.value)
                putBoolean(AUDIO_PREPARED_EXTRA, readiness.prepared)
                putBoolean(SKIP_SILENCE_EXTRA, player.skipSilenceEnabled)
                putBoolean(VOICE_BOOST_EXTRA, effects.voiceBoost)
            })
        }
        val sessionBuilder = MediaSession.Builder(this, AudioSkipPlayer(player, deferred = deferred))
            .setId("$packageName:$name")
            .setCallback(object : MediaSession.Callback {
                override fun onMediaButtonEvent(session: MediaSession, controllerInfo: MediaSession.ControllerInfo, intent: Intent): Boolean =
                    mediaButtons.onMediaButtonEvent(session, controllerInfo, intent)

                override fun onConnect(session: MediaSession, controller: MediaSession.ControllerInfo): MediaSession.ConnectionResult {
                    val ownController = isInkController(controller)
                    // A new Ink controller reads its queue from the playlist.
                    if (ownController) {
                        controllers += controller
                        deferred.fill()
                    }
                    refreshIdleStop()
                    val result = super.onConnect(session, controller)
                    if (!ownController) return MediaSession.ConnectionResult.AcceptedResultBuilder(session)
                        .setAvailableSessionCommands(result.availableSessionCommands)
                        .setAvailablePlayerCommands(externalAudioCommands(effectiveUsage))
                        .build()
                    return MediaSession.ConnectionResult.AcceptedResultBuilder(session)
                        .setAvailablePlayerCommands(result.availablePlayerCommands)
                        .setAvailableSessionCommands(result.availableSessionCommands.buildUpon()
                            .add(SessionCommand(SKIP_SILENCE_COMMAND, Bundle.EMPTY))
                            .add(SessionCommand(AUDIO_QUEUE_COMMAND, Bundle.EMPTY))
                            .add(SessionCommand(VOICE_BOOST_COMMAND, Bundle.EMPTY)).build())
                        .build()
                }
                override fun onCustomCommand(session: MediaSession, controller: MediaSession.ControllerInfo,
                    customCommand: SessionCommand, args: Bundle): ListenableFuture<SessionResult> {
                    if (controller.uid == android.os.Process.myUid() && customCommand.customAction == AUDIO_QUEUE_COMMAND) {
                        val result = runCatching {
                            executeAudioQueue(player, args, deferred = deferred)
                            SessionResult(SessionResult.RESULT_SUCCESS)
                        }.getOrElse { SessionResult(SessionError(SessionError.ERROR_BAD_VALUE, it.message ?: "Invalid audio queue command")) }
                        return Futures.immediateFuture(result)
                    }
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
                        (if (item.localConfiguration == null && uri != null) item.buildUpon().setUri(uri).build() else item).withAudioUsageTag()
                    })
            })
        packageManager.getLaunchIntentForPackage(packageName)?.let { intent ->
            sessionBuilder.setSessionActivity(PendingIntent.getActivity(this, 0, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE))
        }
        val session = sessionBuilder.build()
        readiness = AudioReadiness { publishEffects(session) }.also { player.addListener(it) }
        player.addListener(object : Player.Listener {
            override fun onSkipSilenceEnabledChanged(skipSilenceEnabled: Boolean) = publishEffects(session)
        })
        followAudioUsage(player, usage) { next ->
            effectiveUsage = next
            publishEffects(session)
            session.connectedControllers.filterNot(::isInkController).forEach { controller ->
                session.setAvailableCommands(controller, MediaSession.ConnectionResult.DEFAULT_SESSION_COMMANDS, externalAudioCommands(next))
            }
        }
        publishEffects(session)
        sessions[name] = Session(player, session, deferred, persistence, usage, cache, offload, preloadNext, startupBuffer, audioOffload, mediaButtons)
        addSession(session)
        return session
    }

    override fun onDestroy() {
        handler.removeCallbacks(idleStop)
        sessions.values.forEach {
            it.persistence.close()
            it.deferred.close()
            it.mediaButtons.close()
            it.session.release()
            it.audioOffload?.close()
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

    private fun isInkController(controller: MediaSession.ControllerInfo): Boolean =
        controller.uid == android.os.Process.myUid() && controller.connectionHints.getBoolean(CONTROLLER_HINT)

    private companion object {
        private const val CONTROLLER_HINT = "ink.controller"
        private const val USAGE_HINT = "ink.usage"
        private const val CACHE_HINT = "ink.cache"
        private const val PRELOAD_NEXT_HINT = "ink.preloadNext"
        private const val STARTUP_BUFFER_HINT = "ink.startupBuffer"
        private const val IDLE_STOP_MS = 15 * 60_000L
    }
}
