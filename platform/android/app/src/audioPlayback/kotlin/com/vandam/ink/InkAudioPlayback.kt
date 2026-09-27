package com.vandam.ink

import android.content.ComponentName
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaItem.RequestMetadata
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import com.google.common.util.concurrent.ListenableFuture
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

internal fun createAudioPlayback(
    activity: MainActivity,
    updateController: (Long, String) -> Unit,
): AudioPlayback = InkAudioPlayback(activity, updateController)

@UnstableApi
private class InkAudioPlayback(
    private val activity: MainActivity,
    private val updateController: (Long, String) -> Unit,
) : AudioPlayback {
    private data class Entry(val mode: String, val usage: String, val playback: AudioSessionPlayback)
    private val sessions = mutableMapOf<String, Entry>()
    private val controllers = mutableMapOf<Long, String>()

    override fun activate(controller: Long, config: String): NativeResult {
        val options = runCatching { JSONObject(config) }.getOrNull()
            ?: return NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid audio configuration", false)
        val name = options.optString("session", "main")
        val mode = options.optString("playback", "attached")
        val usage = options.optString("usage", "music")
        if (!Regex("[A-Za-z0-9._-]{1,64}").matches(name)) {
            return NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid audio session name", false)
        }
        val existing = sessions[name]
        if (existing != null && (existing.mode != mode || existing.usage != usage)) {
            return NativeResult.Failure(NativeErrorKind.PROTOCOL, "Audio session options must agree", false)
        }
        if (existing == null && sessions.size >= 8) {
            return NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Too many audio sessions", false)
        }
        val entry = existing ?: Entry(mode, usage, AudioSessionPlayback(activity, updateController, name) {
            sessions.filterKeys { it != name }.values.forEach { it.playback.pauseForFocus() }
        }).also { sessions[name] = it }
        controllers[controller] = name
        return entry.playback.activate(controller, config)
    }

    override fun execute(controller: Long, operation: String, payload: String): NativeResult =
        sessions[controllers[controller]]?.playback?.execute(controller, operation, payload)
            ?: NativeResult.Failure(NativeErrorKind.PROTOCOL, "Audio player is not active", false)

    override fun reportFailure(controller: Long, kind: String, message: String, retryable: Boolean) {
        sessions[controllers[controller]]?.playback?.reportFailure(controller, kind, message, retryable)
    }

    override fun deactivate(controller: Long) {
        val name = controllers.remove(controller) ?: return
        sessions[name]?.playback?.deactivate(controller)
        if (name !in controllers.values) sessions.remove(name)
    }

    override fun pause() = sessions.values.forEach { it.playback.pause() }

    override fun stop() {
        sessions.values.forEach { it.playback.stop() }
        sessions.clear()
        controllers.clear()
    }
}

@UnstableApi
private class AudioSessionPlayback(
    private val activity: MainActivity,
    private val updateController: (Long, String) -> Unit,
    private val name: String,
    private val takeFocus: () -> Unit,
) : AudioPlayback, Player.Listener {
    private val handler = Handler(Looper.getMainLooper())
    private val progress = object : Runnable {
        override fun run() {
            publish()
            if (player?.isPlaying == true) {
                handler.postDelayed(this, PROGRESS_INTERVAL_MS)
            }
        }
    }
    private val controllers = mutableSetOf<Long>()
    private var persistence: AudioQueuePersistence? = null
    private var contentType = C.AUDIO_CONTENT_TYPE_MUSIC
    private var detached = false
    private var player: Player? = null
    private var connection: ListenableFuture<MediaController>? = null
    private val pending = mutableListOf<(Player) -> Unit>()
    private var failure: Failure? = null

    override fun activate(controller: Long, config: String): NativeResult {
        if (!controllers.add(controller)) return NativeResult.Success("")
        if (controllers.size > 1) {
            publish()
            return NativeResult.Success("")
        }
        val configuredUsage = runCatching { JSONObject(config).optString("usage", "music") }
            .getOrDefault("music")
        contentType = if (configuredUsage == "speech") {
            C.AUDIO_CONTENT_TYPE_SPEECH
        } else {
            C.AUDIO_CONTENT_TYPE_MUSIC
        }
        detached = runCatching {
            JSONObject(config).optString("playback", "attached") == "detached"
        }.getOrDefault(false)
        if (detached) {
            connectDetached()
        } else {
            attachedPlayer()
        }
        publish()
        return NativeResult.Success("")
    }

    override fun execute(controller: Long, operation: String, payload: String): NativeResult {
        if (controller !in controllers) {
            return failure("Audio player is not active")
        }
        return runCatching {
            val body = JSONObject(payload.ifEmpty { "{}" })
            when (operation) {
                "play" -> {
                    body.optJSONObject("item")?.let { setQueue(listOf(Item.from(it)), 0) }
                    dispatch(::startPlayback)
                }
                "setQueue" -> setQueue(
                    Item.list(body.getJSONArray("items")),
                    body.optInt("startIndex"),
                    body.optLong("startPosition", 0),
                    body.optBoolean("prepare", true),
                )
                "pause" -> dispatch(Player::pause)
                "toggle" -> dispatch { if (it.playWhenReady && it.playbackState != Player.STATE_ENDED) it.pause() else startPlayback(it) }
                "stop" -> clear()
                "seekTo" -> dispatch {
                    it.seekTo(body.getDouble("value").toLong().coerceAtLeast(0))
                    if (it.playbackState == Player.STATE_IDLE) it.prepare()
                }
                "skipBack" -> seekBy(-SKIP_INTERVAL_MS)
                "skipForward" -> seekBy(SKIP_INTERVAL_MS)
                "previous" -> dispatch(Player::seekToPreviousMediaItem)
                "next" -> dispatch(Player::seekToNextMediaItem)
                "setSkipSilence", "setVoiceBoost" -> {
                    val enabled = body.getBoolean("value")
                    val voice = operation == "setVoiceBoost"
                    dispatch { player ->
                        when (player) {
                            is ExoPlayer -> {
                                if (voice) effects.setVoiceBoost(enabled)
                                else player.skipSilenceEnabled = enabled
                            }
                            is MediaController -> {
                                val result = player.sendCustomCommand(SessionCommand(if (voice) VOICE_BOOST_COMMAND else SKIP_SILENCE_COMMAND, Bundle.EMPTY),
                                    Bundle().apply { putBoolean(if (voice) VOICE_BOOST_EXTRA else SKIP_SILENCE_EXTRA, enabled) })
                                result.addListener({
                                    runCatching { check(result.get().resultCode == SessionResult.RESULT_SUCCESS) { "Could not change audio effect" } }
                                        .onFailure { failure = Failure("unexpected", it.message ?: "Could not change audio effect", false) }
                                    publish()
                                }, activity.mainExecutor)
                            }
                        }
                    }
                }
                "setSpeed" -> {
                    val speed = body.getDouble("value").toFloat()
                    require(speed in MIN_SPEED..MAX_SPEED) {
                        "Playback speed must be between 0.25 and 4"
                    }
                    dispatch { it.playbackParameters = PlaybackParameters(speed) }
                }
                else -> return failure("Unknown audio player operation: $operation")
            }
            publish()
            NativeResult.Success("")
        }.getOrElse { error ->
            val message = error.message ?: "Audio playback failed"
            this.failure = Failure("unexpected", message, false)
            publish()
            NativeResult.Failure(NativeErrorKind.UNEXPECTED, message, false)
        }
    }

    override fun reportFailure(
        controller: Long,
        kind: String,
        message: String,
        retryable: Boolean,
    ) {
        if (controller !in controllers) {
            return
        }
        failure = Failure(kind, message, retryable)
        publish()
    }

    override fun deactivate(controller: Long) {
        if (controller !in controllers) {
            return
        }
        controllers.remove(controller)
        if (controllers.isEmpty()) release()
    }

    fun pauseForFocus() { player?.pause() }

    override fun pause() {
        if (!detached) {
            player?.pause()
            publish()
        }
    }

    override fun stop() {
        release()
        controllers.clear()
    }

    override fun onPlaybackStateChanged(playbackState: Int) = publish()

    override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
        if (!detached) effects.countSavings = playWhenReady
        publish()
    }

    override fun onSkipSilenceEnabledChanged(skipSilenceEnabled: Boolean) = publish()

    override fun onEvents(player: Player, events: Player.Events) {
        if (events.containsAny(Player.EVENT_TIMELINE_CHANGED, Player.EVENT_MEDIA_METADATA_CHANGED, Player.EVENT_PLAYBACK_PARAMETERS_CHANGED)) publish()
    }

    override fun onIsPlayingChanged(isPlaying: Boolean) {
        handler.removeCallbacks(progress)
        if (isPlaying) {
            handler.post(progress)
        } else {
            publish()
        }
    }

    override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) = publish()

    override fun onPlayerError(error: PlaybackException) {
        val code = error.errorCodeName
        val kind = when {
            "IO" in code -> "source"
            "DECOD" in code || "PARSING" in code -> "unsupported"
            "AUDIO_TRACK" in code -> "output"
            else -> "unexpected"
        }
        failure = Failure(kind, error.message ?: "Audio playback failed", kind == "source")
        publish()
    }

    private val effects = AudioEffects.get(activity, name)
    private var readiness: AudioReadiness? = null

    private fun attachedPlayer(): Player {
        player?.let { return it }
        val created = ExoPlayer.Builder(activity, InkAudioRenderersFactory(activity, effects))
            .setMediaSourceFactory(audioMediaSourceFactory(activity))
            .setHandleAudioBecomingNoisy(true)
            .build()
            .apply {
                setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(C.USAGE_MEDIA)
                        .setContentType(contentType)
                        .build(),
                    true,
                )
            }
        readiness = AudioReadiness { publish() }.also { created.addListener(it) }
        persistence = AudioQueuePersistence(activity, name, created, effects).apply { restore() }
        connect(created)
        return created
    }

    private fun dispatch(action: (Player) -> Unit) {
        player?.let {
            action(it)
            return
        }
        if (detached) {
            pending += action
            connectDetached()
        } else {
            action(attachedPlayer())
        }
    }

    private fun connectDetached() {
        if (player != null || connection != null) {
            return
        }
        val future = MediaController.Builder(
            activity,
            SessionToken(activity, ComponentName(activity, InkAudioService::class.java)),
        )
            .setListener(object : MediaController.Listener {
                override fun onExtrasChanged(controller: MediaController, extras: Bundle) = publish()
            })
            .setConnectionHints(Bundle().apply {
                putBoolean(CONTROLLER_HINT, true)
                putString("ink.session", name)
                putInt(CONTENT_TYPE_HINT, contentType)
            })
            .buildAsync()
        connection = future
        future.addListener(
            {
                if (connection !== future) {
                    MediaController.releaseFuture(future)
                    return@addListener
                }
                connection = null
                runCatching(future::get)
                    .onSuccess(::connect)
                    .onFailure { error ->
                        pending.clear()
                        failure = Failure(
                            "unexpected",
                            error.message ?: "Detached playback is unavailable",
                            true,
                        )
                        publish()
                    }
            },
            activity.mainExecutor,
        )
    }

    private fun connect(connected: Player) {
        if (controllers.isEmpty()) {
            connected.release()
            return
        }
        player = connected
        connected.addListener(this)
        val actions = pending.toList()
        pending.clear()
        actions.forEach { it(connected) }
        publish()
        handler.removeCallbacks(progress)
        if (connected.isPlaying) handler.postDelayed(progress, PROGRESS_INTERVAL_MS)
    }

    private fun startPlayback(player: Player) {
        if (player.mediaItemCount == 0) return
        val duration = player.duration.validTime()?.takeIf { it > 0 } ?: player.mediaMetadata.durationMs ?: 0
        if (player.playbackState == Player.STATE_ENDED || (duration > 0 && player.currentPosition >= duration)) {
            player.seekToDefaultPosition()
        }
        if (player.playbackState == Player.STATE_IDLE) player.prepare()
        takeFocus()
        player.play()
    }

    private fun setQueue(items: List<Item>, startIndex: Int, startPosition: Long = 0, prepare: Boolean = true) {
        require(items.isNotEmpty()) { "Audio queues cannot be empty" }
        require(startPosition >= 0) { "Start position must be non-negative" }
        require(startIndex in items.indices) { "Start index must reference an audio item" }
        val mediaItems = items.map { item -> item.mediaItem(activity) }
        failure = null
        dispatch { player ->
            if (!prepare) {
                player.pause()
                player.stop()
            }
            player.setMediaItems(mediaItems, startIndex, startPosition)
            if (prepare) player.prepare()
        }
    }

    private fun clear() {
        failure = null
        dispatch { player ->
            player.stop()
            player.clearMediaItems()
        }
        publish()
    }

    private fun seekBy(deltaMs: Long) {
        dispatch { player ->
            val duration = player.duration.validTime()
            val maximum = if (duration == 0L) Long.MAX_VALUE else duration
            player.seekTo((player.currentPosition + deltaMs).coerceIn(0, maximum))
        }
    }

    private fun publish() {
        if (controllers.isEmpty()) return
        val player = player
        val mediaItem = player?.currentMediaItem
        val metadata = mediaItem?.mediaMetadata
        val index = player
            ?.takeIf { it.mediaItemCount > 0 }
            ?.currentMediaItemIndex
            ?.takeIf { it >= 0 }
            ?: -1
        val state = JSONObject()
            .put("ready", !detached || player != null)
            .put("status", status(player))
            .put("loading", player?.playbackState == Player.STATE_BUFFERING && when (player) {
                is MediaController -> !player.sessionExtras.getBoolean(AUDIO_PREPARED_EXTRA)
                else -> readiness?.prepared != true
            })
            .put("playWhenReady", player?.playWhenReady == true && player.playbackState != Player.STATE_ENDED)
            .put("id", mediaItem?.mediaId.orEmpty())
            .put("src", metadata?.extras?.getString(SOURCE_METADATA_KEY).orEmpty())
            .put("title", metadata?.title?.toString().orEmpty())
            .put("artist", metadata?.artist?.toString().orEmpty())
            .put("album", metadata?.albumTitle?.toString().orEmpty())
            .put("artwork", metadata?.artworkUri?.toString().orEmpty())
            .put("index", index)
            .put("positionMs", player?.currentPosition?.coerceAtLeast(0) ?: 0)
            .put("durationMs", player?.duration?.validTime()?.takeIf { it > 0 } ?: metadata?.durationMs ?: 0)
            .put("bufferedMs", player?.bufferedPosition?.coerceAtLeast(0) ?: 0)
            .put("speed", player?.playbackParameters?.speed ?: 1f)
            .put("silenceSavedMs", effects.silenceSavedMs)
            .put("voiceBoost", if (player is MediaController) player.sessionExtras.getBoolean(VOICE_BOOST_EXTRA) else effects.voiceBoost)
            .put("skipSilence", when (player) {
                is ExoPlayer -> player.skipSilenceEnabled
                is MediaController -> player.sessionExtras.getBoolean(SKIP_SILENCE_EXTRA)
                else -> false
            })
            .put(
                "error",
                inkError(
                    failure?.kind ?: "unexpected",
                    failure?.message.orEmpty(),
                    failure?.retryable ?: false,
                ),
            )
        val encoded = state.toString()
        controllers.forEach { updateController(it, encoded) }
    }

    private fun status(player: Player?): String = when {
        failure != null -> "error"
        player == null && connection != null -> "loading"
        player == null || player.mediaItemCount == 0 -> "idle"
        player.playbackState == Player.STATE_BUFFERING -> "loading"
        player.playbackState == Player.STATE_ENDED -> "ended"
        player.isPlaying -> "playing"
        else -> "paused"
    }

    private fun release() {
        handler.removeCallbacks(progress)
        pending.clear()
        connection?.let(MediaController::releaseFuture)
        connection = null
        persistence?.close()
        persistence = null
        player?.removeListener(this)
        player?.release()
        player = null
        failure = null
    }

    private fun failure(message: String) = NativeResult.Failure(
        NativeErrorKind.PROTOCOL,
        message,
        false,
    )

    private data class Failure(val kind: String, val message: String, val retryable: Boolean)

    private data class Item(
        val id: String,
        val src: String,
        val title: String,
        val artist: String,
        val album: String,
        val artwork: String,
        val duration: Long?,
    ) {
        fun mediaItem(activity: MainActivity): MediaItem {
            val uri = when {
                src.startsWith("https://") -> Uri.parse(src)
                src.startsWith("http://") -> {
                    require(BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED) {
                        "HTTP audio requires the network-cleartext capability"
                    }
                    Uri.parse(src)
                }
                src.startsWith("asset:///") -> activity.bundledAudioUri(src)
                src.startsWith("ink-file://") -> Uri.fromFile(InkManagedFiles(activity).resolve(src))
                src.startsWith(RECORDING_PREFIX) -> {
                    val recordingId = src.removePrefix(RECORDING_PREFIX)
                    require(RECORDING_ID.matches(recordingId)) { "Invalid Ink recording source" }
                    Uri.fromFile(File(activity.filesDir, "recordings/$recordingId.m4a"))
                }
                else -> error(
                    "Audio sources must be bundled assets, HTTPS URLs or Ink recordings",
                )
            }
            val metadata = MediaMetadata.Builder()
                .setTitle(title)
                .setDurationMs(duration)
                .setArtist(artist.ifEmpty { null })
                .setAlbumTitle(album.ifEmpty { null })
                .apply { if (artwork.isNotEmpty()) setArtworkUri(Uri.parse(artwork)) }
                .setExtras(Bundle().apply { putString(SOURCE_METADATA_KEY, src) })
                .build()
            return MediaItem.Builder()
                .setMediaId(id)
                .setUri(uri)
                .setRequestMetadata(RequestMetadata.Builder().setMediaUri(uri).build())
                .setMediaMetadata(metadata)
                .build()
        }

        companion object {
            fun from(value: JSONObject): Item {
                val src = value.getString("src")
                return Item(
                    id = value.optString("id").ifEmpty { src },
                    src = src,
                    title = value.getString("title"),
                    artist = value.optString("artist"),
                    album = value.optString("album"),
                    artwork = value.optString("artwork"),
                    duration = if (value.has("duration")) value.getDouble("duration").also {
                        require(it.isFinite() && it >= 0) { "Audio duration must be non-negative milliseconds" }
                    }.toLong() else null,
                )
            }

            fun list(values: JSONArray): List<Item> =
                List(values.length()) { from(values.getJSONObject(it)) }
        }
    }

    private companion object {
        private const val PROGRESS_INTERVAL_MS = 250L
        private const val SKIP_INTERVAL_MS = 15_000L
        private const val MIN_SPEED = 0.25f
        private const val MAX_SPEED = 4f
        private const val RECORDING_PREFIX = "ink://audio/recordings/"
        private const val SOURCE_METADATA_KEY = "ink.source"
        private const val CONTROLLER_HINT = "ink.controller"
        private const val CONTENT_TYPE_HINT = "ink.contentType"
        private val RECORDING_ID = Regex("[0-9a-f-]{36}")

        private fun Long.validTime() = takeIf { it != C.TIME_UNSET && it >= 0 } ?: 0
    }
}
