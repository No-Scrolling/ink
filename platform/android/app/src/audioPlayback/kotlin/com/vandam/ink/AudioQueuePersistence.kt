package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.common.util.UnstableApi
import org.json.JSONArray
import org.json.JSONObject

@UnstableApi
internal class AudioQueuePersistence(context: Context, private val name: String, private val player: ExoPlayer, private val effects: AudioEffects) : Player.Listener {
    private val preferences = context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE)
    private val positions = context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE)
    private val handler = Handler(Looper.getMainLooper())
    private var restoring = false
    private var checkpointScheduled = false
    private val checkpoint = object : Runnable {
        override fun run() {
            checkpointScheduled = false
            save()
            scheduleCheckpoint()
        }
    }

    init { player.addListener(this) }

    fun restore() {
        player.skipSilenceEnabled = preferences.getBoolean("skipSilence:$name", false)
        val saved = preferences.getString(name, null) ?: return
        restoring = true
        runCatching {
            val state = JSONObject(positions.getString(name, "{}") ?: "{}")
            val items = JSONArray(saved)
            val queue = List(items.length()) { index ->
                val item = items.getJSONObject(index)
                val uri = Uri.parse(item.getString("uri"))
                MediaItem.Builder()
                    .setMediaId(item.getString("id"))
                    .setUri(uri)
                    .setRequestMetadata(MediaItem.RequestMetadata.Builder().setMediaUri(uri).build())
                    .setMediaMetadata(MediaMetadata.Builder()
                        .setTitle(item.optString("title"))
                        .setArtist(item.optString("artist"))
                        .setDurationMs(item.optLong("duration", 0).takeIf { it > 0 })
                        .setAlbumTitle(item.optString("album"))
                        .apply { item.optString("artwork").takeIf(String::isNotEmpty)?.let { setArtworkUri(Uri.parse(it)) } }
                        .setExtras(Bundle().apply { putString("ink.source", item.getString("source")) })
                        .build())
                    .build()
            }
            if (queue.isNotEmpty()) {
                player.setMediaItems(queue, state.optInt("index", 0).coerceIn(queue.indices), state.optLong("position", 0L).coerceAtLeast(0))
                player.playbackParameters = PlaybackParameters(state.optDouble("speed", 1.0).toFloat().coerceIn(0.25f, 4f))
                player.playWhenReady = false
                player.prepare()
            }
        }.onFailure { preferences.edit().remove(name).apply() }
        restoring = false
    }

    override fun onSkipSilenceEnabledChanged(skipSilenceEnabled: Boolean) {
        preferences.edit().putBoolean("skipSilence:$name", skipSilenceEnabled).apply()
    }

    override fun onEvents(player: Player, events: Player.Events) {
        if (restoring) return
        if (events.contains(Player.EVENT_TIMELINE_CHANGED)) saveQueue()
        if (events.containsAny(Player.EVENT_TIMELINE_CHANGED, Player.EVENT_MEDIA_ITEM_TRANSITION,
                Player.EVENT_POSITION_DISCONTINUITY, Player.EVENT_PLAYBACK_PARAMETERS_CHANGED,
                Player.EVENT_PLAY_WHEN_READY_CHANGED, Player.EVENT_PLAYBACK_STATE_CHANGED)) save()
        scheduleCheckpoint()
    }

    private fun scheduleCheckpoint() {
        if (!player.isPlaying) {
            handler.removeCallbacks(checkpoint)
            checkpointScheduled = false
        } else if (!checkpointScheduled) {
            checkpointScheduled = true
            handler.postDelayed(checkpoint, 30_000L)
        }
    }

    fun close() {
        save()
        handler.removeCallbacks(checkpoint)
        player.removeListener(this)
    }

    private fun save() {
        if (restoring) return
        effects.save()
        if (player.mediaItemCount == 0) {
            positions.edit().remove(name).apply()
            return
        }
        val state = JSONObject()
            .put("index", player.currentMediaItemIndex.coerceAtLeast(0))
            .put("position", player.currentPosition.takeIf { it != C.TIME_UNSET && it >= 0 } ?: 0)
            .put("speed", player.playbackParameters.speed)
        positions.edit().putString(name, state.toString()).apply()
    }
    private fun saveQueue() {
        if (player.mediaItemCount == 0) {
            preferences.edit().remove(name).apply()
            return
        }
        val items = JSONArray()
        repeat(player.mediaItemCount) { index ->
            val item = player.getMediaItemAt(index)
            val metadata = item.mediaMetadata
            val uri = item.localConfiguration?.uri ?: item.requestMetadata.mediaUri ?: return
            items.put(JSONObject()
                .put("uri", uri.toString())
                .put("id", item.mediaId)
                .put("source", metadata.extras?.getString("ink.source").orEmpty())
                .put("title", metadata.title?.toString().orEmpty())
                .put("artist", metadata.artist?.toString().orEmpty())
                .put("album", metadata.albumTitle?.toString().orEmpty())
                .put("artwork", metadata.artworkUri?.toString().orEmpty())
                .put("duration", metadata.durationMs))
        }
        preferences.edit().putString(name, items.toString()).apply()
    }

}
