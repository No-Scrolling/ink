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
    private var controller: Long? = null
    private var contentType = C.AUDIO_CONTENT_TYPE_MUSIC
    private var detached = false
    private var player: Player? = null
    private var connection: ListenableFuture<MediaController>? = null
    private val pending = mutableListOf<(Player) -> Unit>()
    private var failure: Failure? = null

    override fun activate(controller: Long, config: String): NativeResult {
        if (this.controller != null && this.controller != controller) {
            return failure("Only one audio player can be active")
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
        this.controller = controller
        if (detached) {
            connectDetached()
        }
        publish()
        return NativeResult.Success("")
    }

    override fun execute(controller: Long, operation: String, payload: String): NativeResult {
        if (this.controller != controller) {
            return failure("Audio player is not active")
        }
        return runCatching {
            val body = JSONObject(payload.ifEmpty { "{}" })
            when (operation) {
                "play" -> {
                    body.optJSONObject("item")?.let { setQueue(listOf(Item.from(it)), 0) }
                    dispatch { if (it.mediaItemCount > 0) it.play() }
                }
                "setQueue" -> setQueue(
                    Item.list(body.getJSONArray("items")),
                    body.optInt("startIndex"),
                )
                "pause" -> dispatch(Player::pause)
                "toggle" -> dispatch { if (it.isPlaying) it.pause() else it.play() }
                "stop" -> clear()
                "seekTo" -> dispatch {
                    it.seekTo(body.getDouble("value").toLong().coerceAtLeast(0))
                }
                "skipBack" -> seekBy(-SKIP_INTERVAL_MS)
                "skipForward" -> seekBy(SKIP_INTERVAL_MS)
                "previous" -> dispatch(Player::seekToPreviousMediaItem)
                "next" -> dispatch(Player::seekToNextMediaItem)
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
        if (this.controller != controller) {
            return
        }
        failure = Failure(kind, message, retryable)
        publish()
    }

    override fun deactivate(controller: Long) {
        if (this.controller != controller) {
            return
        }
        release()
        this.controller = null
    }

    override fun pause() {
        if (!detached) {
            player?.pause()
            publish()
        }
    }

    override fun stop() {
        release()
        controller = null
    }

    override fun onPlaybackStateChanged(playbackState: Int) = publish()

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

    private fun attachedPlayer(): Player {
        player?.let { return it }
        val created = ExoPlayer.Builder(activity)
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
            .setConnectionHints(Bundle().apply {
                putBoolean(CONTROLLER_HINT, true)
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
        if (controller == null) {
            connected.release()
            return
        }
        player = connected
        connected.addListener(this)
        val actions = pending.toList()
        pending.clear()
        actions.forEach { it(connected) }
        publish()
    }

    private fun setQueue(items: List<Item>, startIndex: Int) {
        require(items.isNotEmpty()) { "Audio queues cannot be empty" }
        require(startIndex in items.indices) { "Start index must reference an audio item" }
        val mediaItems = items.map { item -> item.mediaItem(activity) }
        failure = null
        dispatch { player ->
            player.setMediaItems(mediaItems, startIndex, C.TIME_UNSET)
            player.prepare()
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
        val controller = controller ?: return
        val player = player
        val mediaItem = player?.currentMediaItem
        val metadata = mediaItem?.mediaMetadata
        val index = player
            ?.takeIf { it.mediaItemCount > 0 }
            ?.currentMediaItemIndex
            ?.takeIf { it >= 0 }
            ?: -1
        val state = JSONObject()
            .put("status", status(player))
            .put("id", mediaItem?.mediaId.orEmpty())
            .put("src", metadata?.extras?.getString(SOURCE_METADATA_KEY).orEmpty())
            .put("title", metadata?.title?.toString().orEmpty())
            .put("artist", metadata?.artist?.toString().orEmpty())
            .put("album", metadata?.albumTitle?.toString().orEmpty())
            .put("artwork", metadata?.artworkUri?.toString().orEmpty())
            .put("index", index)
            .put("positionMs", player?.currentPosition?.coerceAtLeast(0) ?: 0)
            .put("durationMs", player?.duration?.validTime() ?: 0)
            .put("bufferedMs", player?.bufferedPosition?.coerceAtLeast(0) ?: 0)
            .put("speed", player?.playbackParameters?.speed ?: 1f)
            .put(
                "error",
                inkError(
                    failure?.kind ?: "unexpected",
                    failure?.message.orEmpty(),
                    failure?.retryable ?: false,
                ),
            )
        updateController(controller, state.toString())
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
    ) {
        fun mediaItem(activity: MainActivity): MediaItem {
            val uri = when {
                src.startsWith("https://") -> Uri.parse(src)
                src.startsWith("asset:///") -> Uri.parse(src)
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
