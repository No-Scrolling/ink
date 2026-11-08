package com.vandam.ink

import android.content.Context
import android.content.Intent
import android.media.AudioDeviceInfo
import android.media.session.MediaController
import android.os.SystemClock
import android.view.KeyEvent
import android.view.ViewConfiguration
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaLibraryInfo
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.PlaybackException
import androidx.media3.common.TrackSelectionParameters.AudioOffloadPreferences
import androidx.media3.session.MediaSession
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
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
        test("audio offload excludes effects, speed, pitch and unsuitable routes", ::offloadPreferences)
        test("audio offload remembers PCM fallback for each track and route", ::offloadFallback)
        test("audio headset single, double and triple presses use Media3 commands", ::mediaButtons)
        test("audio content sources play, persist and report access failures", ::contentSources)
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

    private fun offloadFallback() {
        val policy = AudioOffloadPolicy(true)
        val bluetooth = AudioOffloadRoute("8", false)
        val other = AudioOffloadRoute("26", false)
        check(policy.allows("first", bluetooth, false))
        policy.rememberFallback("first", bluetooth)
        check(!policy.allows("first", other, false))
        check(!policy.allows("second", bluetooth, false))
        check(policy.allows("second", other, false))
        check(!policy.allows(null, other, false))
        check(!AudioOffloadPolicy(false).allows("second", other, false))
    }

    private fun offloadPreferences() = onMain {
        val effects = AudioEffects.get(context, "offload-contract")
        effects.setVoiceBoost(false)
        val player = ExoPlayer.Builder(context, InkAudioRenderersFactory(context, effects)).build()
        var route = AudioOffloadRoute(AudioDeviceInfo.TYPE_BLUETOOTH_A2DP.toString(), false)
        val offload = AudioOffload(context, player, effects, true) { route }
        fun expect(enabled: Boolean) {
            val preferences = player.trackSelectionParameters.audioOffloadPreferences
            equal(if (enabled) AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_ENABLED else AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_DISABLED,
                preferences.audioOffloadMode)
            equal(enabled, preferences.isGaplessSupportRequired)
            equal(false, preferences.isSpeedChangeSupportRequired)
        }
        try {
            expect(false)
            player.setMediaItems(listOf(item("first"), item("second")))
            expect(true)
            player.skipSilenceEnabled = true
            expect(false)
            player.skipSilenceEnabled = false
            expect(true)
            effects.setVoiceBoost(true)
            expect(false)
            effects.setVoiceBoost(false)
            expect(true)
            player.playbackParameters = PlaybackParameters(1.5f)
            expect(false)
            player.playbackParameters = PlaybackParameters(1f, 1.2f)
            expect(false)
            player.playbackParameters = PlaybackParameters.DEFAULT
            expect(true)
            route = AudioOffloadRoute("2", true)
            player.seekTo(1, 0)
            expect(false)
            route = AudioOffloadRoute("8", false)
            player.seekTo(0, 0)
            expect(true)
        } finally {
            offload.close()
            player.release()
            effects.setVoiceBoost(false)
        }
        val speaker = ExoPlayer.Builder(context).build()
        val disabled = AudioOffload(context, speaker, effects, false)
        try {
            speaker.setMediaItem(item("default-disabled"))
            equal(AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_DISABLED, speaker.trackSelectionParameters.audioOffloadPreferences.audioOffloadMode)
        } finally { disabled.close(); speaker.release() }
        val routed = ExoPlayer.Builder(context).build()
        val actualRoute = AudioOffload(context, routed, effects, true)
        try {
            routed.setMediaItem(item("speaker"))
            val devices = context.getSystemService(android.media.AudioManager::class.java)
                .getAudioDevicesForAttributes(routed.audioAttributes.platformAudioAttributes)
            if (devices.any { it.type == AudioDeviceInfo.TYPE_BUILTIN_SPEAKER }) {
                equal(AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_DISABLED, routed.trackSelectionParameters.audioOffloadPreferences.audioOffloadMode)
            }
        } finally { actualRoute.close(); routed.release() }
    }

    private fun eventually(description: String, predicate: () -> Boolean) {
        val deadline = SystemClock.elapsedRealtime() + 5000
        while (!onMain(predicate)) {
            check(SystemClock.elapsedRealtime() < deadline) { "Timed out: $description" }
            SystemClock.sleep(10)
        }
    }

    private fun mediaButtons() {
        val file = File(context.cacheDir, "headset-contract.wav")
        val pcmBytes = 30 * 8000 * 2
        val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + pcmBytes).put("WAVEfmt ".toByteArray())
            .putInt(16).putShort(1).putShort(1).putInt(8000).putInt(16000).putShort(2).putShort(16)
            .put("data".toByteArray()).putInt(pcmBytes).array()
        file.outputStream().use { it.write(header); it.write(ByteArray(pcmBytes)) }
        val buttons = AudioMediaButtons(context)
        val receivedPresses = LinkedBlockingQueue<Unit>()
        val player = onMain { ExoPlayer.Builder(context).build().apply {
            setMediaItems((0..2).map { MediaItem.Builder().setMediaId("track-$it").setUri(file.toURI().toString()).build() }, 1, 0)
        } }
        val session = onMain { MediaSession.Builder(context, player).setId("headset-contract")
            .setCallback(object : MediaSession.Callback {
                override fun onMediaButtonEvent(session: MediaSession, controllerInfo: MediaSession.ControllerInfo, intent: Intent): Boolean {
                    val handled = buttons.onMediaButtonEvent(session, controllerInfo, intent)
                    val event = intent.getParcelableExtra(Intent.EXTRA_KEY_EVENT, KeyEvent::class.java)
                    if (event?.keyCode == KeyEvent.KEYCODE_HEADSETHOOK && event.action == KeyEvent.ACTION_DOWN) receivedPresses.add(Unit)
                    return handled
                }
            }).build() }
        val controller = MediaController(context, session.platformToken)
        fun key(code: Int) {
            controller.dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_DOWN, code))
            controller.dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_UP, code))
        }
        fun presses(count: Int) {
            repeat(count) { key(KeyEvent.KEYCODE_HEADSETHOOK); if (it + 1 < count) SystemClock.sleep(40) }
        }
        try {
            presses(1)
            SystemClock.sleep(100)
            onMain { equal(false, player.playWhenReady) }
            eventually("single press plays") { player.isPlaying }
            presses(1)
            eventually("single press pauses") { !player.playWhenReady }
            onMain { player.seekTo(1, 0) }
            presses(2)
            eventually("double press skips next") { player.currentMediaItemIndex == 2 }
            onMain { player.seekTo(1, 0) }
            presses(3)
            eventually("triple press skips previous without first skipping next") { player.currentMediaItemIndex == 0 }
            onMain { player.seekTo(1, 4000) }
            presses(3)
            eventually("triple press restarts after three seconds") { player.currentMediaItemIndex == 1 && player.currentPosition == 0L }
            key(KeyEvent.KEYCODE_MEDIA_NEXT)
            eventually("dedicated next") { player.currentMediaItemIndex == 2 }
            key(KeyEvent.KEYCODE_MEDIA_PREVIOUS)
            eventually("dedicated previous") { player.currentMediaItemIndex == 1 }
            key(KeyEvent.KEYCODE_MEDIA_PLAY)
            eventually("dedicated play") { player.isPlaying }
            key(KeyEvent.KEYCODE_MEDIA_PAUSE)
            eventually("dedicated pause") { !player.playWhenReady }
            onMain { player.seekTo(2, 30_000); player.play() }
            eventually("end of queue") { player.playbackState == Player.STATE_ENDED }
            presses(1)
            eventually("single press restarts an ended queue") { player.isPlaying && player.currentPosition < 5000 }
            key(KeyEvent.KEYCODE_MEDIA_PAUSE)
            eventually("pause restarted queue") { !player.playWhenReady }
            key(KeyEvent.KEYCODE_MEDIA_STOP)
            eventually("dedicated stop keeps the queue") { player.playbackState == Player.STATE_IDLE && player.mediaItemCount == 3 }
            // A release must discard a pending gesture rather than act on a released player.
            receivedPresses.clear()
            presses(1)
            check(receivedPresses.poll(5, TimeUnit.SECONDS) != null) { "Pending press did not reach the session" }
            onMain { buttons.close() }
            SystemClock.sleep(ViewConfiguration.getDoubleTapTimeout().toLong() + 50)
            onMain { equal(false, player.playWhenReady) }
        } finally {
            onMain { buttons.close(); session.release(); player.release() }
            file.delete()
        }
    }

    private fun contentSources() {
        val name = "audio-content-contract"
        val preferences = context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE)
        val positions = context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE)
        val file = File(context.cacheDir, "audio-content-contract.wav")
        val pcmBytes = 30 * 8000 * 2
        val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + pcmBytes).put("WAVEfmt ".toByteArray())
            .putInt(16).putShort(1).putShort(1).putInt(8000).putInt(16000).putShort(2).putShort(16)
            .put("data".toByteArray()).putInt(pcmBytes).array()
        file.outputStream().use { it.write(header); it.write(ByteArray(pcmBytes)) }
        val src = "content://com.vandam.ink.contracttests.audio/track"
        val uri = audioSourceUri(context, src) { error("Content must not resolve a bundled asset") }
        equal(src, uri.toString())
        for (unsupported in listOf("ftp://example.com/track.mp3", "file:///sdcard/track.mp3", "data:audio/wav;base64,AAAA")) {
            check(runCatching { audioSourceUri(context, unsupported) { error("Unexpected asset") } }.isFailure) {
                "Unsupported audio source accepted: $unsupported"
            }
        }
        val errors = LinkedBlockingQueue<PlaybackException>()
        fun player() = ExoPlayer.Builder(context).setMediaSourceFactory(audioMediaSourceFactory(context, true)).build()
        val player = onMain { player().apply {
            addListener(object : Player.Listener {
                override fun onPlayerError(error: PlaybackException) { errors.add(error) }
            })
        } }
        val saved = onMain { AudioQueuePersistence(context, name, player, AudioEffects.get(context, name)) }
        val cache = File(context.cacheDir, "ink-audio")
        val cacheExisted = cache.exists()
        try {
            onMain {
                player.setMediaItem(item("content-track").buildUpon().setUri(uri)
                    .setRequestMetadata(MediaItem.RequestMetadata.Builder().setMediaUri(uri).build())
                    .setMediaMetadata(MediaMetadata.Builder().setTitle("Local track").setExtras(Bundle().apply {
                        putString("ink.source", src)
                    }).build()).build())
                player.prepare()
                player.play()
            }
            eventually("content playback advances") { player.isPlaying && player.currentPosition > 100 }
            equal(null, errors.poll())
            equal(cacheExisted, cache.exists())
            onMain {
                player.pause()
                saved.close()
                val restored = player()
                val persistence = AudioQueuePersistence(context, name, restored, AudioEffects.get(context, name))
                try {
                    persistence.restore()
                    equal(1, restored.mediaItemCount)
                    equal(src, restored.currentMediaItem?.localConfiguration?.uri.toString())
                    equal(src, restored.currentMediaItem?.requestMetadata?.mediaUri.toString())
                    equal(src, restored.currentMediaItem?.audioItemJson()?.getString("src"))
                    equal("content-track", restored.currentMediaItem?.mediaId)
                    equal(false, restored.playWhenReady)
                } finally { persistence.close(); restored.release() }
            }
            for ((path, code) in listOf("denied" to PlaybackException.ERROR_CODE_IO_NO_PERMISSION,
                    "missing" to PlaybackException.ERROR_CODE_IO_FILE_NOT_FOUND)) {
                onMain {
                    player.setMediaItem(MediaItem.fromUri(audioSourceUri(context, "content://com.vandam.ink.contracttests.audio/$path") { error("Unexpected asset") }))
                    player.prepare()
                }
                equal(code, checkNotNull(errors.poll(10, TimeUnit.SECONDS)) { "No playback error for $path" }.errorCode)
            }
        } finally {
            onMain { saved.close(); player.release() }
            preferences.edit().remove(name).commit()
            positions.edit().remove(name).commit()
            file.delete()
        }
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
