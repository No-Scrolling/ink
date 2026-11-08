package com.vandam.ink

import android.bluetooth.BluetoothA2dp
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Handler
import android.util.Log
import androidx.media3.common.AudioAttributes
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackParameters
import androidx.media3.common.Player
import androidx.media3.common.TrackSelectionParameters.AudioOffloadPreferences
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.analytics.AnalyticsListener
import androidx.media3.exoplayer.audio.AudioSink

internal data class AudioOffloadRoute(val key: String, val avoidsOffload: Boolean)

internal class AudioOffloadPolicy(private val requested: Boolean) {
    private val pcmFallbackRouteKeys = mutableSetOf<String>()
    private val pcmFallbackTrackIds = mutableSetOf<String>()

    fun allows(mediaId: String?, route: AudioOffloadRoute, processing: Boolean): Boolean =
        requested && mediaId != null && !processing && !route.avoidsOffload &&
            mediaId !in pcmFallbackTrackIds && route.key !in pcmFallbackRouteKeys

    fun rememberFallback(mediaId: String, route: AudioOffloadRoute) {
        pcmFallbackTrackIds += mediaId
        pcmFallbackRouteKeys += route.key
    }
}

/** Owns offload eligibility and recovery for one player, in either playback mode. */
@UnstableApi
internal class AudioOffload(
    context: Context,
    private val player: ExoPlayer,
    private val effects: AudioEffects,
    requested: Boolean,
    private val outputRoute: () -> AudioOffloadRoute = {
        val manager = context.getSystemService(AudioManager::class.java)
        val devices = manager.getAudioDevicesForAttributes(player.audioAttributes.platformAudioAttributes)
        AudioOffloadRoute(devices.map { it.type }.distinct().sorted().joinToString(",").ifEmpty { "unknown" }, devices.any {
            it.type in setOf(AudioDeviceInfo.TYPE_BUILTIN_SPEAKER, AudioDeviceInfo.TYPE_USB_ACCESSORY,
                AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_HEADSET,
                AudioDeviceInfo.TYPE_WIRED_HEADPHONES, AudioDeviceInfo.TYPE_WIRED_HEADSET)
        })
    },
) : Player.Listener, AnalyticsListener {
    private val applicationContext = context.applicationContext
    private val manager = context.getSystemService(AudioManager::class.java)
    private val handler = Handler(player.applicationLooper)
    private val policy = AudioOffloadPolicy(requested)
    private var route = outputRoute()
    private var offloaded = false
    private var positionAdvanced = false
    private var watchdog: Runnable? = null
    private val effectsChanged: () -> Unit = { refresh() }
    private val devicesChanged = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(addedDevices: Array<AudioDeviceInfo>) = refresh()
        override fun onAudioDevicesRemoved(removedDevices: Array<AudioDeviceInfo>) = refresh()
    }
    private val routeChanged = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) = refresh()
    }

    init {
        player.addListener(this)
        player.addAnalyticsListener(this)
        effects.addListener(effectsChanged)
        manager.registerAudioDeviceCallback(devicesChanged, handler)
        applicationContext.registerReceiver(routeChanged, IntentFilter().apply {
            addAction(AudioManager.ACTION_AUDIO_BECOMING_NOISY)
            addAction(BluetoothA2dp.ACTION_CONNECTION_STATE_CHANGED)
            addAction("android.bluetooth.a2dp.profile.action.ACTIVE_DEVICE_CHANGED")
        }, null, handler, Context.RECEIVER_EXPORTED)
        refresh()
    }

    private fun refresh() {
        val nextRoute = outputRoute()
        if (route != nextRoute) {
            cancelWatchdog()
            route = nextRoute
        }
        val parameters = player.playbackParameters
        val processing = effects.voiceBoost || player.skipSilenceEnabled || parameters.speed != 1f || parameters.pitch != 1f
        val enabled = policy.allows(player.currentMediaItem?.mediaId, route, processing)
        val mode = if (enabled) AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_ENABLED else AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_DISABLED
        val preferences = AudioOffloadPreferences.Builder()
            .setAudioOffloadMode(mode)
            // Preserve encoder delay/padding. Speed changes use the software chain instead.
            .setIsGaplessSupportRequired(enabled)
            .setIsSpeedChangeSupportRequired(false)
            .build()
        if (player.trackSelectionParameters.audioOffloadPreferences != preferences) {
            player.trackSelectionParameters = player.trackSelectionParameters.buildUpon()
                .setAudioOffloadPreferences(preferences).build()
        }
        if (!enabled) cancelWatchdog()
    }

    override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
        cancelWatchdog()
        refresh()
    }

    override fun onAudioAttributesChanged(audioAttributes: AudioAttributes) = refresh()
    override fun onPlaybackParametersChanged(playbackParameters: PlaybackParameters) = refresh()
    override fun onSkipSilenceEnabledChanged(skipSilenceEnabled: Boolean) = refresh()

    override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
        if (playWhenReady) {
            refresh()
            if (offloaded && !positionAdvanced) scheduleWatchdog()
        } else cancelWatchdog()
    }

    override fun onPlaybackStateChanged(playbackState: Int) {
        if (playbackState == Player.STATE_IDLE || playbackState == Player.STATE_ENDED) cancelWatchdog()
    }

    override fun onAudioTrackInitialized(eventTime: AnalyticsListener.EventTime, audioTrackConfig: AudioSink.AudioTrackConfig) {
        refresh()
        Log.d("InkAudio", "output mediaId=${player.currentMediaItem?.mediaId} offload=${audioTrackConfig.offload} route=${route.key}")
        offloaded = audioTrackConfig.offload
        positionAdvanced = false
        cancelWatchdog()
        if (offloaded) scheduleWatchdog()
    }

    override fun onAudioTrackReleased(eventTime: AnalyticsListener.EventTime, audioTrackConfig: AudioSink.AudioTrackConfig) {
        offloaded = false
        cancelWatchdog()
    }

    override fun onAudioPositionAdvancing(eventTime: AnalyticsListener.EventTime, playoutStartSystemTimeMs: Long) {
        positionAdvanced = true
        cancelWatchdog()
    }

    private fun scheduleWatchdog() {
        cancelWatchdog()
        val mediaId = player.currentMediaItem?.mediaId ?: return
        val initialRoute = route
        val startPosition = player.currentPosition.coerceAtLeast(0)
        watchdog = Runnable {
            watchdog = null
            refresh()
            if (player.currentMediaItem?.mediaId == mediaId && route == initialRoute &&
                player.playWhenReady && player.playbackSuppressionReason == Player.PLAYBACK_SUPPRESSION_REASON_NONE &&
                player.playerError == null && offloaded &&
                player.trackSelectionParameters.audioOffloadPreferences.audioOffloadMode == AudioOffloadPreferences.AUDIO_OFFLOAD_MODE_ENABLED &&
                player.totalBufferedDuration >= 1000 && player.currentPosition <= startPosition + 250) {
                retryWithoutOffload(mediaId)
            }
        }.also { handler.postDelayed(it, 1500) }
    }

    private fun retryWithoutOffload(mediaId: String) {
        val index = player.currentMediaItemIndex
        if (index !in 0 until player.mediaItemCount) return
        val position = player.currentPosition.coerceAtLeast(0)
        val playWhenReady = player.playWhenReady
        policy.rememberFallback(mediaId, route)
        Log.w("InkAudio", "fallback mediaId=$mediaId route=${route.key} reason=offload-audio-track-did-not-advance")
        player.stop()
        refresh()
        player.seekTo(index, position)
        player.prepare()
        player.playWhenReady = playWhenReady
    }

    private fun cancelWatchdog() {
        watchdog?.let(handler::removeCallbacks)
        watchdog = null
    }

    fun close() {
        cancelWatchdog()
        applicationContext.unregisterReceiver(routeChanged)
        manager.unregisterAudioDeviceCallback(devicesChanged)
        effects.removeListener(effectsChanged)
        player.removeAnalyticsListener(this)
        player.removeListener(this)
    }
}
