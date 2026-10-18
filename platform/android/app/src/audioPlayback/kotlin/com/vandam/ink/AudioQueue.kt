package com.vandam.ink

import android.os.Bundle
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaLibraryInfo
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import org.json.JSONObject

internal const val AUDIO_QUEUE_COMMAND = "ink.audio.queue"
internal val AUDIO_QUEUE_OPERATIONS = setOf("add", "remove", "move", "replace", "playItem")

@UnstableApi
internal fun executeAudioQueue(player: ExoPlayer, args: Bundle, takeFocus: () -> Unit = {}) {
    val body = JSONObject(requireNotNull(args.getString("payload")))
    val item = args.getBundle("item")?.let { MediaItem.fromBundle(it, MediaLibraryInfo.INTERFACE_VERSION) }
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
    when (args.getString("operation")) {
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

@UnstableApi
internal fun MediaItem.audioItemJson(): JSONObject {
    val metadata = mediaMetadata
    return JSONObject()
        .put("id", mediaId)
        .put("src", metadata.extras?.getString("ink.source") ?: requestMetadata.mediaUri?.toString() ?: localConfiguration?.uri?.toString().orEmpty())
        .put("title", metadata.title?.toString().orEmpty())
        .put("artist", metadata.artist?.toString().orEmpty())
        .put("album", metadata.albumTitle?.toString().orEmpty())
        .put("artwork", metadata.artworkUri?.toString().orEmpty())
        .put("duration", metadata.durationMs)
}
