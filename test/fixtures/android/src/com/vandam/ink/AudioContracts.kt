package com.vandam.ink

import android.content.Context
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaLibraryInfo
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import java.util.concurrent.FutureTask
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

@UnstableApi
internal class AudioContracts(private val context: Context) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("audio external commands follow music and speech controls", ::externalCommands)
        test("audio attributes follow transitions, replacements and session fallback", ::itemUsage)
        test("audio item usage survives the media bridge and queue persistence", ::persistence)
    }

    private fun equal(expected: Any?, actual: Any?) {
        check(expected == actual) { "Expected <$expected>, got <$actual>" }
    }

    private fun <T> onMain(action: () -> T): T {
        val task = FutureTask(action)
        check(Handler(Looper.getMainLooper()).post(task))
        return task.get(10, TimeUnit.SECONDS)
    }

    private fun item(id: String, usage: String? = null): MediaItem = MediaItem.Builder()
        .setMediaId(id)
        .setUri("https://example.com/$id.mp3")
        .setMediaMetadata(MediaMetadata.Builder().setTitle(id).setExtras(Bundle().apply {
            putString("ink.source", "https://example.com/$id.mp3")
            usage?.let { putString("ink.usage", it) }
        }).build())
        .build()
        .withAudioUsageTag()

    private fun externalCommands() {
        val music = externalAudioCommands(AudioUsage.MUSIC)
        val speech = externalAudioCommands(AudioUsage.SPEECH)
        for (command in listOf(Player.COMMAND_PLAY_PAUSE, Player.COMMAND_PREPARE, Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM)) {
            check(music.contains(command)); check(speech.contains(command))
        }
        for (command in listOf(Player.COMMAND_SEEK_BACK, Player.COMMAND_SEEK_FORWARD)) {
            check(!music.contains(command)); check(speech.contains(command))
        }
        for (command in listOf(Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM)) {
            check(music.contains(command)); check(!speech.contains(command))
        }
    }

    private fun itemUsage() {
        val player = onMain { ExoPlayer.Builder(context).build() }
        val changes = LinkedBlockingQueue<String>()
        fun expectUsage(usage: String, contentType: Int) {
            equal(usage, changes.poll(10, TimeUnit.SECONDS))
            onMain {
                equal(contentType, player.audioAttributes.contentType)
                equal(C.USAGE_MEDIA, player.audioAttributes.usage)
            }
        }
        try {
            onMain { followAudioUsage(player, AudioUsage.MUSIC) { changes.add(it.value) } }
            expectUsage("music", C.AUDIO_CONTENT_TYPE_MUSIC)
            onMain { player.setMediaItems(listOf(item("episode", "speech"), item("fallback"))) }
            expectUsage("speech", C.AUDIO_CONTENT_TYPE_SPEECH)
            onMain { player.replaceMediaItem(0, item("episode", "music")) }
            expectUsage("music", C.AUDIO_CONTENT_TYPE_MUSIC)
            onMain { player.replaceMediaItem(0, item("episode", "speech")) }
            expectUsage("speech", C.AUDIO_CONTENT_TYPE_SPEECH)
            onMain { player.seekTo(1, 0) }
            expectUsage("music", C.AUDIO_CONTENT_TYPE_MUSIC)
            onMain { player.seekTo(0, 0) }
            expectUsage("speech", C.AUDIO_CONTENT_TYPE_SPEECH)
            onMain { player.clearMediaItems() }
            expectUsage("music", C.AUDIO_CONTENT_TYPE_MUSIC)
        } finally { onMain { player.release() } }
    }

    private fun persistence() {
        val name = "audio-usage-contract"
        val preferences = context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE)
        val positions = context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE)
        preferences.edit().remove(name).commit()
        positions.edit().remove(name).commit()
        val player = onMain { ExoPlayer.Builder(context).build() }
        val persistence = onMain {
            AudioQueuePersistence(context, name, player, AudioEffects.get(context, name)).also {
                val queue = listOf(item("episode", "speech"), item("track", "music"), item("fallback"))
                player.setMediaItems(queue.map { mediaItem ->
                    MediaItem.fromBundle(mediaItem.toBundleIncludeLocalConfiguration(MediaLibraryInfo.INTERFACE_VERSION), MediaLibraryInfo.INTERFACE_VERSION)
                })
            }
        }
        onMain { persistence.close(); player.release() }
        try {
            onMain {
                val restored = ExoPlayer.Builder(context).build()
                val saved = AudioQueuePersistence(context, name, restored, AudioEffects.get(context, name))
                try {
                    saved.restore()
                    equal(3, restored.mediaItemCount)
                    equal("speech", restored.getMediaItemAt(0).audioItemJson().getString("usage"))
                    equal("music", restored.getMediaItemAt(1).audioItemJson().getString("usage"))
                    equal(false, restored.getMediaItemAt(2).audioItemJson().has("usage"))
                    followAudioUsage(restored, AudioUsage.MUSIC)
                    equal(C.AUDIO_CONTENT_TYPE_SPEECH, restored.audioAttributes.contentType)
                } finally { saved.close(); restored.release() }
            }
        } finally {
            preferences.edit().remove(name).commit()
            positions.edit().remove(name).commit()
        }
    }
}
