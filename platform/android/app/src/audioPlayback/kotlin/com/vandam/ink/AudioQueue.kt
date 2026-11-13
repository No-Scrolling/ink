package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.ForwardingPlayer
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaLibraryInfo
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.Timeline
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import org.json.JSONArray
import org.json.JSONObject

internal const val AUDIO_QUEUE_COMMAND = "ink.audio.queue"
internal val AUDIO_QUEUE_OPERATIONS = setOf("add", "remove", "move", "replace", "playItem", "next", "previous")

@UnstableApi
internal fun executeAudioQueue(player: ExoPlayer, args: Bundle, takeFocus: () -> Unit = {}, deferred: DeferredAudioQueue? = null) {
    val body = JSONObject(requireNotNull(args.getString("payload")))
    val item = args.getBundle("item")?.let { MediaItem.fromBundle(it, MediaLibraryInfo.INTERFACE_VERSION).withAudioUsageTag() }
    fun indexOf(id: String): Int = (0 until player.mediaItemCount)
        .firstOrNull { player.getMediaItemAt(it).mediaId == id }
        ?: error("Audio item is not in the queue: $id")
    fun replacement(except: Int = -1): MediaItem {
        val value = requireNotNull(item) { "Audio queue edit requires an item" }
        require(value.mediaId.isNotBlank()) { "Audio item IDs must not be blank" }
        require((0 until player.mediaItemCount).none { it != except && player.getMediaItemAt(it).mediaId == value.mediaId }) {
            "Audio queue item IDs must be unique"
        }
        return value
    }
    fun position(field: String): Int {
        val value = body.getDouble(field)
        require(value.isFinite() && value >= 0 && value <= Int.MAX_VALUE && value % 1 == 0.0) { "Audio queue index must be a non-negative integer" }
        return value.toInt()
    }
    if (args.getString("operation") == "discard") {
        deferred?.discard()
        return
    }
    if (args.getString("operation") == "start") {
        val text = requireNotNull(args.getString("queue"))
        requireNotNull(deferred).start(requireNotNull(item) { "Audio queue start requires an item" },
            lazy { AudioItem.list(JSONObject(text).getJSONArray("items")) }, requireNotNull(args.getStringArrayList("uris")),
            body.getInt("startIndex"), body.getLong("position"))
        return
    }
    deferred?.fill()
    when (args.getString("operation")) {
        "next" -> AudioSkipPlayer(player, takeFocus, deferred).seekToNextMediaItem()
        "previous" -> AudioSkipPlayer(player, takeFocus, deferred).apply {
            if (body.optBoolean("restart")) seekToPrevious() else seekToPreviousMediaItem()
        }
        "playItem" -> {
            val index = indexOf(body.getString("id"))
            if (index != player.currentMediaItemIndex) player.seekTo(index, 0)
            startAudioPlayback(player, takeFocus)
        }
        "add" -> {
            val value = replacement()
            val index = when {
                !body.has("at") -> player.mediaItemCount
                body.opt("at") == "next" -> if (player.mediaItemCount == 0) 0 else player.currentMediaItemIndex + 1
                else -> position("at")
            }
            require(index in 0..player.mediaItemCount) { "Audio insertion index must be within the queue" }
            player.addMediaItem(index, value)
        }
        "remove" -> {
            val index = indexOf(body.getString("id"))
            val removingLastCurrent = index == player.currentMediaItemIndex && index == player.mediaItemCount - 1
            val retryNext = index == player.currentMediaItemIndex && player.playerError != null
            player.removeMediaItem(index)
            if (player.mediaItemCount == 0 || removingLastCurrent) {
                player.pause()
                player.stop()
            } else if (retryNext) player.prepare()
        }
        "move" -> {
            val index = indexOf(body.getString("id"))
            val target = position("index")
            require(target in 0 until player.mediaItemCount) { "Audio destination index must be within the queue" }
            player.moveMediaItem(index, target)
        }
        "replace" -> {
            val index = indexOf(body.getString("id"))
            val value = replacement(index)
            val changingCurrentSource = index == player.currentMediaItemIndex &&
                player.getMediaItemAt(index).localConfiguration?.uri != value.localConfiguration?.uri
            val retryReplacement = changingCurrentSource && player.playerError != null
            val playWhenReady = player.playWhenReady && player.playbackState != Player.STATE_ENDED
            player.replaceMediaItem(index, value)
            if (changingCurrentSource) {
                player.seekTo(index, 0)
                if (retryReplacement) player.prepare()
                player.playWhenReady = playWhenReady
            }
        }
        else -> error("Unknown audio queue edit")
    }
}

@UnstableApi
internal fun startAudioPlayback(player: Player, takeFocus: () -> Unit) {
    if (player.mediaItemCount == 0) return
    val duration = player.duration.takeIf { it != C.TIME_UNSET && it > 0 } ?: player.mediaMetadata.durationMs ?: 0
    if (player.playbackState == Player.STATE_ENDED || (duration > 0 && player.currentPosition >= duration)) {
        player.seekToDefaultPosition()
    }
    if (player.playbackState == Player.STATE_IDLE) player.prepare()
    takeFocus()
    player.play()
}

/** Starts playback after a skip from the app, lock screen, notification or headset. */
@UnstableApi
internal class AudioSkipPlayer(player: Player, private val takeFocus: () -> Unit = {}, private val deferred: DeferredAudioQueue? = null) : ForwardingPlayer(player) {
    // Deferred items can be skipped to before they join the player.
    override fun getAvailableCommands(): Player.Commands {
        val commands = super.getAvailableCommands()
        val remaining = deferred?.remaining ?: return commands
        return commands.buildUpon().apply {
            if (remaining.start > 0) addAll(Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM)
            if (remaining.start < remaining.size - 1) addAll(Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM)
        }.build()
    }

    override fun isCommandAvailable(command: Int) = availableCommands.contains(command)

    // Media3 sessions read available commands from ExoPlayer's callback, so the deferred skips are reported after it.
    private val listeners = mutableListOf<Player.Listener>()
    private val commands = object : Player.Listener {
        override fun onAvailableCommandsChanged(availableCommands: Player.Commands) {
            if (deferred?.remaining != null) report()
        }

        // A new deferred queue can change its skips without changing ExoPlayer's commands.
        override fun onTimelineChanged(timeline: Timeline, reason: Int) {
            if (reason == Player.TIMELINE_CHANGE_REASON_PLAYLIST_CHANGED) report()
        }

        private fun report() = listeners.toList().forEach { it.onAvailableCommandsChanged(availableCommands) }
    }

    override fun addListener(listener: Player.Listener) {
        super.addListener(listener)
        if (deferred == null) return
        listeners += listener
        wrappedPlayer.removeListener(commands)
        wrappedPlayer.addListener(commands)
    }

    override fun removeListener(listener: Player.Listener) {
        super.removeListener(listener)
        listeners -= listener
    }

    override fun seekToNext() = skip(Player.COMMAND_SEEK_TO_NEXT) { super.seekToNext() }
    override fun seekToNextMediaItem() = skip(Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM) { super.seekToNextMediaItem() }
    override fun seekToPrevious() = skip(Player.COMMAND_SEEK_TO_PREVIOUS) { super.seekToPrevious() }
    override fun seekToPreviousMediaItem() = skip(Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM) { super.seekToPreviousMediaItem() }

    // Media3 only offers a skip command when the skip would seek, so skipping past the last item stays paused.
    private fun skip(command: Int, seek: () -> Unit) {
        deferred?.fill()
        val seeks = wrappedPlayer.isCommandAvailable(command)
        seek()
        if (seeks) startAudioPlayback(wrappedPlayer, takeFocus)
    }
}

/**
 * Media3's per-item work grows with the queue, so a long queue starts with only its selected item.
 * The rest joins once that item is ready, before a skip or queue edit, or when playback fails or ends.
 * Until then its items can stay as the JSON text that carried them.
 */
@UnstableApi
internal class DeferredAudioQueue(private val player: ExoPlayer, private val filled: () -> Unit = {}) : Player.Listener {
    class Remaining(private val items: Lazy<List<AudioItem>>, private val uris: List<String>, val start: Int) {
        val size get() = uris.size
        fun before() = (0 until start).map { items.value[it] to uris[it] }
        fun after() = (start + 1 until size).map { items.value[it] to uris[it] }
    }

    private val handler = Handler(Looper.getMainLooper())
    private val fillLater = Runnable { fill() }
    private var filling = false
    var remaining: Remaining? = null
        private set
    val pending get() = remaining != null || filling

    init { player.addListener(this) }

    fun start(item: MediaItem, items: Lazy<List<AudioItem>>, uris: List<String>, start: Int, position: Long) {
        handler.removeCallbacks(fillLater)
        remaining = Remaining(items, uris, start)
        player.setMediaItems(listOf(item), 0, position)
        player.prepare()
    }

    fun fill() {
        handler.removeCallbacks(fillLater)
        val remaining = remaining ?: return
        this.remaining = null
        filling = true
        try {
            player.addMediaItems(remaining.after().map { (item, uri) -> item.mediaItem(Uri.parse(uri)) })
            player.addMediaItems(0, remaining.before().map { (item, uri) -> item.mediaItem(Uri.parse(uri)) })
        } finally { filling = false }
        filled()
    }

    fun discard() {
        handler.removeCallbacks(fillLater)
        remaining = null
    }

    fun close() {
        handler.removeCallbacks(fillLater)
        player.removeListener(this)
    }

    override fun onPlaybackStateChanged(playbackState: Int) {
        if (remaining == null) return
        if (playbackState == Player.STATE_ENDED) fill()
        else if (playbackState == Player.STATE_READY) {
            handler.removeCallbacks(fillLater)
            handler.postDelayed(fillLater, FILL_DELAY_MS)
        }
    }

    override fun onPlayerError(error: PlaybackException) = fill()

    private companion object {
        private const val FILL_DELAY_MS = 1000L
    }
}

@UnstableApi
internal fun MediaItem.audioItemJson(): JSONObject = AudioItem.from(this).json()

@UnstableApi
internal data class AudioItem(
    val id: String,
    val src: String,
    val title: String,
    val artist: String,
    val album: String,
    val artwork: String,
    val duration: Long?,
    val usage: AudioUsage?,
) {
    fun mediaItem(context: Context, bundle: InkBundle) = mediaItem(audioSourceUri(context, src, bundle::audioUri))

    fun mediaItem(uri: Uri): MediaItem {
        val metadata = MediaMetadata.Builder()
            .setTitle(title)
            .setDurationMs(duration)
            .setArtist(artist.ifEmpty { null })
            .setAlbumTitle(album.ifEmpty { null })
            .apply { if (artwork.isNotEmpty()) setArtworkUri(Uri.parse(artwork)) }
            .setExtras(Bundle().apply {
                putString("ink.source", src)
                usage?.let { putString(ITEM_USAGE_EXTRA, it.value) }
            })
            .build()
        return MediaItem.Builder()
            .setMediaId(id)
            .setUri(uri)
            .setRequestMetadata(MediaItem.RequestMetadata.Builder().setMediaUri(uri).build())
            .setMediaMetadata(metadata)
            .build()
            .withAudioUsageTag()
    }

    fun json(): JSONObject = JSONObject()
        .put("id", id).put("src", src).put("title", title).put("artist", artist).put("album", album)
        .put("artwork", artwork).put("duration", duration).put("usage", usage?.value)

    companion object {
        fun from(item: MediaItem): AudioItem {
            val metadata = item.mediaMetadata
            return AudioItem(item.mediaId,
                metadata.extras?.getString("ink.source") ?: item.requestMetadata.mediaUri?.toString() ?: item.localConfiguration?.uri?.toString().orEmpty(),
                metadata.title?.toString().orEmpty(), metadata.artist?.toString().orEmpty(), metadata.albumTitle?.toString().orEmpty(),
                metadata.artworkUri?.toString().orEmpty(), metadata.durationMs, item.itemUsage())
        }

        fun from(value: JSONObject): AudioItem {
            val src = value.getString("src")
            return AudioItem(
                id = value.getString("id").also { require(it.isNotBlank()) { "Audio item IDs must not be blank" } },
                src = src,
                title = value.getString("title"),
                artist = value.optString("artist"),
                album = value.optString("album"),
                artwork = value.optString("artwork"),
                duration = if (value.has("duration")) value.getDouble("duration").also {
                    require(it.isFinite() && it >= 0) { "Audio duration must be non-negative milliseconds" }
                }.toLong() else null,
                usage = if (value.has("usage")) AudioUsage.from(value.getString("usage")) else null,
            )
        }

        fun list(values: JSONArray): List<AudioItem> =
            List(values.length()) { from(values.getJSONObject(it)) }
    }
}
