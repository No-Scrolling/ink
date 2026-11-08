package com.vandam.ink

import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.Timeline
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.session.MediaSession

internal const val SESSION_USAGE_EXTRA = "com.thelightphone.audio.USAGE"
internal const val ITEM_USAGE_EXTRA = "ink.usage"
internal const val AUDIO_SEEK_INCREMENT_MS = 15_000L

internal enum class AudioUsage(val value: String, val contentType: Int) {
    MUSIC("music", C.AUDIO_CONTENT_TYPE_MUSIC),
    SPEECH("speech", C.AUDIO_CONTENT_TYPE_SPEECH);

    companion object {
        fun from(value: String): AudioUsage = requireNotNull(entries.firstOrNull { it.value == value }) {
            "Audio usage must be music or speech"
        }
    }
}

internal fun MediaItem.itemUsage(): AudioUsage? = mediaMetadata.extras?.getString(ITEM_USAGE_EXTRA)?.let(AudioUsage::from)

// Media3 ignores metadata extras in equality checks; the tag makes usage-only replacements observable.
@UnstableApi
internal fun MediaItem.withAudioUsageTag(): MediaItem = buildUpon().setTag(itemUsage()?.value).build()

@UnstableApi
internal fun followAudioUsage(player: ExoPlayer, usage: AudioUsage, onUsageChanged: (AudioUsage) -> Unit = {}) {
    player.addListener(object : Player.Listener {
        private var applied: AudioUsage? = null

        fun applyUsage() {
            val next = player.currentMediaItem?.itemUsage() ?: usage
            if (next == applied) return
            applied = next
            player.setAudioAttributes(AudioAttributes.Builder()
                .setUsage(C.USAGE_MEDIA)
                .setContentType(next.contentType)
                .build(), true)
            onUsageChanged(next)
        }

        override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) = applyUsage()
        override fun onTimelineChanged(timeline: Timeline, reason: Int) = applyUsage()
    }.apply { applyUsage() })
}

@UnstableApi
internal fun externalAudioCommands(usage: AudioUsage): Player.Commands =
    MediaSession.ConnectionResult.DEFAULT_PLAYER_COMMANDS.buildUpon().apply {
        when (usage) {
            AudioUsage.MUSIC -> removeAll(Player.COMMAND_SEEK_BACK, Player.COMMAND_SEEK_FORWARD)
            AudioUsage.SPEECH -> removeAll(Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM,
                Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM)
        }
    }.build()
