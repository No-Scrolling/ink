package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.AtomicFile
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.common.Timeline
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.common.util.UnstableApi
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.Executors

@UnstableApi
internal class AudioQueuePersistence(context: Context, private val name: String, private val player: ExoPlayer, private val effects: AudioEffects, private val deferred: DeferredAudioQueue? = null) : Player.Listener {
    private val preferences = context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE)
    private val file = audioQueueFile(context, name)
    private val positions = context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE)
    private val handler = Handler(Looper.getMainLooper())
    private var restoring = false
    private var queueChanged = false
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
        // Wait for queued writes so the latest queue is read.
        writes.submit {}.get()
        player.skipSilenceEnabled = preferences.getBoolean("skipSilence:$name", false)
        // Queues saved before the queue file existed are read once from preferences, then rewritten as a file.
        val legacy = preferences.getString(name, null)
        if (legacy != null) {
            file.startWrite().apply { write(legacy.toByteArray()); file.finishWrite(this) }
            preferences.edit().remove(name).apply()
        }
        val saved = runCatching { file.readFully().toString(Charsets.UTF_8) }.getOrNull() ?: return
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
                        .setExtras(Bundle().apply {
                            putString("ink.source", item.getString("source"))
                            if (item.has("usage")) putString(ITEM_USAGE_EXTRA, AudioUsage.from(item.getString("usage")).value)
                        })
                        .build())
                    .build()
                    .withAudioUsageTag()
            }
            if (queue.isNotEmpty()) {
                player.setMediaItems(queue, state.optInt("index", 0).coerceIn(queue.indices), state.optLong("position", 0L).coerceAtLeast(0))
                player.playbackParameters = PlaybackParameters(state.optDouble("speed", 1.0).toFloat().coerceIn(0.25f, 4f))
                player.shuffleModeEnabled = state.optBoolean("shuffle", false)
                player.repeatMode = state.optInt("repeat", Player.REPEAT_MODE_OFF).takeIf { it in 0..2 } ?: Player.REPEAT_MODE_OFF
                player.playWhenReady = false
            }
        }.onFailure { file.delete() }
        restoring = false
    }

    override fun onSkipSilenceEnabledChanged(skipSilenceEnabled: Boolean) {
        preferences.edit().putBoolean("skipSilence:$name", skipSilenceEnabled).apply()
    }

    // Source updates refine timeline windows without changing the saved items.
    override fun onTimelineChanged(timeline: Timeline, reason: Int) {
        if (!restoring && reason == Player.TIMELINE_CHANGE_REASON_PLAYLIST_CHANGED) queueChanged = true
    }

    override fun onEvents(player: Player, events: Player.Events) {
        if (restoring) return
        if (queueChanged) {
            queueChanged = false
            saveQueue()
        }
        if (events.containsAny(Player.EVENT_TIMELINE_CHANGED, Player.EVENT_MEDIA_ITEM_TRANSITION,
                Player.EVENT_POSITION_DISCONTINUITY, Player.EVENT_PLAYBACK_PARAMETERS_CHANGED,
                Player.EVENT_PLAY_WHEN_READY_CHANGED, Player.EVENT_PLAYBACK_STATE_CHANGED,
                Player.EVENT_SHUFFLE_MODE_ENABLED_CHANGED, Player.EVENT_REPEAT_MODE_CHANGED)) save()
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
            .put("index", player.currentMediaItemIndex.coerceAtLeast(0) + (deferred?.remaining?.start ?: 0))
            .put("position", player.currentPosition.takeIf { it != C.TIME_UNSET && it >= 0 } ?: 0)
            .put("speed", player.playbackParameters.speed)
            .put("shuffle", player.shuffleModeEnabled)
            .put("repeat", player.repeatMode)
        positions.edit().putString(name, state.toString()).apply()
    }
    // Player state is read on its looper; encoding and writing run in order on one background thread.
    // A deferred queue is saved whole, with the items that have not joined the player yet.
    private fun saveQueue() {
        val queue = List(player.mediaItemCount, player::getMediaItemAt)
        val remaining = deferred?.remaining
        writes.execute {
            if (queue.isEmpty()) {
                file.delete()
                return@execute
            }
            val playing = queue.map { item ->
                AudioItem.from(item) to (item.localConfiguration?.uri ?: item.requestMetadata.mediaUri ?: return@execute).toString()
            }
            val items = JSONArray()
            (remaining?.before().orEmpty() + playing + remaining?.after().orEmpty()).forEach { (item, uri) ->
                items.put(JSONObject()
                    .put("uri", uri)
                    .put("id", item.id)
                    .put("source", item.src)
                    .put("title", item.title)
                    .put("artist", item.artist)
                    .put("album", item.album)
                    .put("artwork", item.artwork)
                    .put("duration", item.duration)
                    .put("usage", item.usage?.value))
            }
            val stream = file.startWrite()
            runCatching { stream.write(items.toString().toByteArray()) }
                .onSuccess { file.finishWrite(stream) }
                .onFailure { file.failWrite(stream) }
        }
    }

    private companion object {
        private val writes = Executors.newSingleThreadExecutor()
    }
}

internal fun audioQueueFile(context: Context, name: String) =
    AtomicFile(File(File(context.filesDir, "ink-audio-queues").apply { mkdirs() }, "$name.json"))
