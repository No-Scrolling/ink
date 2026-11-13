package com.vandam.ink

import android.content.ComponentName
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
import androidx.media3.session.SessionToken
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import java.util.concurrent.FutureTask
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.net.ServerSocket
import java.net.InetAddress
import java.util.concurrent.atomic.AtomicInteger

@UnstableApi
internal class AudioContracts(private val context: Context) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("audio external commands follow music and speech controls", ::externalCommands)
        test("audio attributes follow transitions, replacements and session fallback", ::itemUsage)
        test("audio item usage survives the media bridge and queue persistence", ::persistence)
        test("audio queue persistence saves playlist changes and skips source updates", ::queueWrites)
        test("audio offload excludes effects, speed, pitch and unsuitable routes", ::offloadPreferences)
        test("audio offload remembers PCM fallback for each track and route", ::offloadFallback)
        test("audio headset single, double and triple presses use Media3 commands", ::mediaButtons)
        test("audio skips from the app, lock screen and headset start playback", ::skipsStartPlayback)
        test("audio content sources play, persist and report access failures", ::contentSources)
        test("audio cache follows item identity across URL changes", ::cachedIdentity)
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
        val service = Intent(context, InkAudioService::class.java)
        val connection = onMain {
            androidx.media3.session.MediaController.Builder(context, SessionToken(context, ComponentName(context, InkAudioService::class.java)))
                .setConnectionHints(Bundle().apply {
                    putBoolean("ink.controller", true)
                    putString("ink.session", "headset-contract")
                    putString("ink.usage", "speech")
                }).buildAsync()
        }
        val player = connection.get(10, TimeUnit.SECONDS)
        // Media3 exposes the platform token on MediaSession, but not on a connected controller.
        val token = SessionToken::class.java.getDeclaredMethod("getPlatformToken").apply { isAccessible = true }
            .invoke(onMain { player.connectedToken }) as android.media.session.MediaSession.Token
        val controller = MediaController(context, token)
        val plainConnection = onMain {
            androidx.media3.session.MediaController.Builder(context, checkNotNull(player.connectedToken)).buildAsync()
        }
        val plain = plainConnection.get(10, TimeUnit.SECONDS)
        onMain {
            player.setMediaItems((0..2).map { index -> MediaItem.Builder().setMediaId("track-$index")
                .setUri(file.toURI().toString()).setMediaMetadata(MediaMetadata.Builder().setExtras(Bundle().apply {
                    putString(ITEM_USAGE_EXTRA, "speech")
                }).build()).build() }, 1, 0)
        }
        fun key(code: Int) {
            controller.dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_DOWN, code))
            controller.dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_UP, code))
        }
        fun presses(count: Int) {
            repeat(count) { key(KeyEvent.KEYCODE_HEADSETHOOK); if (it + 1 < count) SystemClock.sleep(40) }
        }
        try {
            eventually("speech session ready") { player.sessionExtras.getString(SESSION_USAGE_EXTRA) == "speech" }
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
            eventually("triple press restarts after three seconds") { player.currentMediaItemIndex == 1 && player.currentPosition < 3000 }
            onMain { player.replaceMediaItem(1, player.getMediaItemAt(1).buildUpon()
                .setMediaMetadata(MediaMetadata.Builder().setExtras(Bundle().apply { putString(ITEM_USAGE_EXTRA, "music") }).build()).build())
                player.seekTo(1, 0) }
            eventually("music usage applied") { player.sessionExtras.getString(SESSION_USAGE_EXTRA) == "music" }
            presses(2)
            eventually("double press transitions music to speech") { player.currentMediaItemIndex == 2 && player.sessionExtras.getString(SESSION_USAGE_EXTRA) == "speech" }
            onMain {
                check(plain.availableCommands.contains(Player.COMMAND_SEEK_TO_PREVIOUS)) { "Same-app controller lost previous after usage refresh" }
                check(plain.availableCommands.contains(Player.COMMAND_SEEK_FORWARD)) { "Same-app controller lost seek after usage refresh" }
            }
            presses(3)
            eventually("triple press works after music to speech refresh") { player.currentMediaItemIndex == 1 }
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
        } finally {
            onMain { plain.release(); player.release() }
            context.stopService(service)
            file.delete()
            audioQueueFile(context, "headset-contract").delete()
            context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE).edit().remove("headset-contract").apply()
        }
        val buttons = AudioMediaButtons(context)
        val receivedPresses = LinkedBlockingQueue<Unit>()
        val pendingPlayer = onMain { ExoPlayer.Builder(context).build() }
        val pendingSession = onMain { MediaSession.Builder(context, pendingPlayer).setId("pending-gesture-contract")
            .setCallback(object : MediaSession.Callback {
                override fun onMediaButtonEvent(session: MediaSession, controllerInfo: MediaSession.ControllerInfo, intent: Intent): Boolean {
                    val handled = buttons.onMediaButtonEvent(session, controllerInfo, intent)
                    receivedPresses.add(Unit)
                    return handled
                }
            }).build() }
        try {
            MediaController(context, pendingSession.platformToken)
                .dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_HEADSETHOOK))
            check(receivedPresses.poll(5, TimeUnit.SECONDS) != null) { "Pending press did not reach the session" }
            onMain { buttons.close() }
            SystemClock.sleep(ViewConfiguration.getDoubleTapTimeout().toLong() + 50)
            onMain { equal(false, pendingPlayer.playWhenReady) }
        } finally { onMain { buttons.close(); pendingSession.release(); pendingPlayer.release() } }
    }

    private fun skipsStartPlayback() {
        val name = "skip-contract"
        val file = File(context.cacheDir, "skip-contract.wav")
        val pcmBytes = 30 * 8000 * 2
        val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + pcmBytes).put("WAVEfmt ".toByteArray())
            .putInt(16).putShort(1).putShort(1).putInt(8000).putInt(16000).putShort(2).putShort(16)
            .put("data".toByteArray()).putInt(pcmBytes).array()
        file.outputStream().use { it.write(header); it.write(ByteArray(pcmBytes)) }
        val service = Intent(context, InkAudioService::class.java)
        // The Ink controller carries the app's own next and previous operations to detached playback.
        val ink = onMain {
            androidx.media3.session.MediaController.Builder(context, SessionToken(context, ComponentName(context, InkAudioService::class.java)))
                .setConnectionHints(Bundle().apply {
                    putBoolean("ink.controller", true)
                    putString("ink.session", name)
                })
                .buildAsync()
        }.get(10, TimeUnit.SECONDS)
        // Lock screens and notifications connect without Ink's hints.
        val lockScreen = onMain { androidx.media3.session.MediaController.Builder(context, checkNotNull(ink.connectedToken)).buildAsync() }
            .get(10, TimeUnit.SECONDS)
        val token = SessionToken::class.java.getDeclaredMethod("getPlatformToken").apply { isAccessible = true }
            .invoke(onMain { ink.connectedToken }) as android.media.session.MediaSession.Token
        val platform = MediaController(context, token)
        val attached = onMain { ExoPlayer.Builder(context).build() }
        fun pausedAt(index: Int, position: Long = 0) {
            onMain { ink.pause(); ink.seekTo(index, position) }
            eventually("paused at $index") { !ink.playWhenReady && ink.currentMediaItemIndex == index && ink.playbackState == Player.STATE_READY }
        }
        fun playsAt(description: String, index: Int, restarted: Boolean = false) =
            eventually(description) { ink.currentMediaItemIndex == index && ink.isPlaying && (!restarted || ink.currentPosition < 3000) }
        fun headset(presses: Int) = repeat(presses) {
            platform.dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_HEADSETHOOK))
            platform.dispatchMediaButtonEvent(KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_HEADSETHOOK))
            SystemClock.sleep(40)
        }
        try {
            onMain {
                ink.setMediaItems((0..2).map { MediaItem.Builder().setMediaId("track-$it").setUri(file.toURI().toString()).build() }, 1, 0)
                ink.prepare()
            }
            pausedAt(1)
            onMain { ink.seekToNextMediaItem() }
            playsAt("app next plays", 2)
            pausedAt(1)
            onMain { ink.seekToPreviousMediaItem() }
            playsAt("app previous plays", 0)
            pausedAt(1)
            onMain { lockScreen.seekToNext() }
            playsAt("lock screen next plays", 2)
            pausedAt(1)
            onMain { lockScreen.seekToPrevious() }
            playsAt("lock screen previous plays", 0)
            pausedAt(1)
            platform.transportControls.skipToNext()
            playsAt("platform next plays", 2)
            pausedAt(1)
            headset(2)
            playsAt("headset double press plays", 2)
            pausedAt(1)
            headset(3)
            playsAt("headset triple press plays", 0)
            pausedAt(1, 4000)
            onMain { lockScreen.seekToPrevious() }
            playsAt("previous after three seconds restarts and plays", 1, restarted = true)
            pausedAt(1, 4000)
            headset(3)
            playsAt("headset restart plays", 1, restarted = true)
            pausedAt(2, 1000)
            onMain { ink.seekToNextMediaItem(); lockScreen.seekToNext() }
            platform.transportControls.skipToNext()
            headset(2)
            SystemClock.sleep(ViewConfiguration.getDoubleTapTimeout() + 500L)
            onMain {
                equal(2, ink.currentMediaItemIndex)
                equal(false, ink.playWhenReady)
                check(ink.currentPosition in 1000L..1100L) { "Skipping past the last item moved to ${ink.currentPosition}" }
            }
            // Attached playback sends the app's skips straight to the session player, without Media3's command checks.
            onMain {
                attached.setMediaItems((0..1).map { MediaItem.Builder().setMediaId("attached-$it").setUri(file.toURI().toString()).build() }, 1, 1000)
                attached.prepare()
                AudioSkipPlayer(attached).apply { seekToNextMediaItem(); seekToNext() }
                equal(false, attached.playWhenReady)
                AudioSkipPlayer(attached).seekToPreviousMediaItem()
                equal(0, attached.currentMediaItemIndex)
                equal(true, attached.playWhenReady)
            }
        } finally {
            onMain { lockScreen.release(); ink.release(); attached.release() }
            context.stopService(service)
            file.delete()
            audioQueueFile(context, name).delete()
            context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE).edit().remove(name).apply()
        }
    }

    private fun contentSources() {
        val name = "audio-content-contract"
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
            audioQueueFile(context, name).delete()
            positions.edit().remove(name).commit()
            file.delete()
        }
    }

    private fun cachedIdentity() {
        val pcmBytes = 8000 * 2
        val audio = ByteBuffer.allocate(44 + pcmBytes).order(ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + pcmBytes).put("WAVEfmt ".toByteArray())
            .putInt(16).putShort(1).putShort(1).putInt(8000).putInt(16000).putShort(2).putShort(16)
            .put("data".toByteArray()).putInt(pcmBytes).put(ByteArray(pcmBytes)).array()
        val requests = AtomicInteger()
        val server = ServerSocket(0, 8, InetAddress.getByName("127.0.0.1"))
        val serving = Thread {
            while (!server.isClosed) {
                try {
                    server.accept().use { socket ->
                        val reader = socket.getInputStream().bufferedReader()
                        while (!reader.readLine().isNullOrEmpty()) Unit
                        requests.incrementAndGet()
                        socket.getOutputStream().apply {
                            write("HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: ${audio.size}\r\nConnection: close\r\n\r\n".toByteArray())
                            write(audio)
                            flush()
                        }
                    }
                } catch (error: java.io.IOException) {
                    if (!server.isClosed) throw error
                }
            }
        }.apply { start() }
        val id = "cache-contract-${System.nanoTime()}"
        fun play(id: String, path: String, session: String = "main", cache: Boolean = true) {
            val errors = LinkedBlockingQueue<PlaybackException>()
            val player = onMain {
                ExoPlayer.Builder(context).setMediaSourceFactory(audioMediaSourceFactory(context, cache, session)).build().apply {
                    addListener(object : Player.Listener {
                        override fun onPlayerError(error: PlaybackException) { errors.add(error) }
                    })
                    setMediaItem(MediaItem.Builder().setMediaId(id).setUri("http://127.0.0.1:${server.localPort}/$path.wav").build())
                    prepare()
                    play()
                }
            }
            try {
                eventually("cached audio finishes buffering") { player.playbackState == Player.STATE_READY && player.bufferedPosition == 1000L }
                equal(null, errors.poll())
            } finally { onMain { player.release() } }
        }
        try {
            play(id, "original")
            equal(1, requests.get())
            play(id, "renewed")
            equal(1, requests.get())
            play("$id-new", "renewed")
            equal(2, requests.get())
            play(id, "renewed", session = "other")
            equal(3, requests.get())
            play(id, "renewed", cache = false)
            equal(4, requests.get())
        } finally { server.close(); serving.join(5000); check(!serving.isAlive) }
    }

    private fun queueWrites() {
        val name = "queue-write-contract"
        val positions = context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE)
        audioQueueFile(context, name).delete()
        positions.edit().remove(name).commit()
        val file = File(context.cacheDir, "audio-content-contract.wav")
        val pcmBytes = 30 * 8000 * 2
        val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + pcmBytes).put("WAVEfmt ".toByteArray())
            .putInt(16).putShort(1).putShort(1).putInt(8000).putInt(16000).putShort(2).putShort(16)
            .put("data".toByteArray()).putInt(pcmBytes).array()
        file.outputStream().use { it.write(header); it.write(ByteArray(pcmBytes)) }
        fun content(id: String): MediaItem {
            val uri = android.net.Uri.parse("content://com.vandam.ink.contracttests.audio/$id")
            return item(id).buildUpon().setUri(uri).setRequestMetadata(MediaItem.RequestMetadata.Builder().setMediaUri(uri).build()).build()
        }
        // Restoring waits for queued writes, then reads what was saved.
        fun saved(): List<String> = onMain {
            val restored = ExoPlayer.Builder(context).build()
            val persistence = AudioQueuePersistence(context, name, restored, AudioEffects.get(context, name))
            try {
                persistence.restore()
                List(restored.mediaItemCount) { restored.getMediaItemAt(it).mediaId }
            } finally { persistence.close(); restored.release() }
        }
        val reasons = LinkedBlockingQueue<Int>()
        val player = onMain { ExoPlayer.Builder(context).build().apply {
            addListener(object : Player.Listener {
                override fun onTimelineChanged(timeline: androidx.media3.common.Timeline, reason: Int) { reasons.add(reason) }
            })
        } }
        val persistence = onMain { AudioQueuePersistence(context, name, player, AudioEffects.get(context, name)) }
        try {
            onMain { player.setMediaItems(listOf(content("first"), content("second"))) }
            equal(listOf("first", "second"), saved())
            audioQueueFile(context, name).delete()
            reasons.clear()
            onMain { player.prepare() }
            eventually("source prepared") { player.playbackState == Player.STATE_READY }
            check(Player.TIMELINE_CHANGE_REASON_SOURCE_UPDATE in reasons) { "Preparing did not update the source timeline" }
            equal(emptyList<String>(), saved())
            onMain { player.addMediaItem(content("third")) }
            equal(listOf("first", "second", "third"), saved())
        } finally {
            onMain { persistence.close(); player.release() }
            audioQueueFile(context, name).delete()
            positions.edit().remove(name).commit()
            file.delete()
        }
    }

    private fun persistence() {
        val name = "audio-usage-contract"
        val positions = context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE)
        audioQueueFile(context, name).delete()
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
            audioQueueFile(context, name).delete()
            positions.edit().remove(name).commit()
        }
    }
}
