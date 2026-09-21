package com.vandam.ink

import androidx.media3.common.Player

internal const val AUDIO_PREPARED_EXTRA = "ink.audioPrepared"

internal class AudioReadiness(private val changed: () -> Unit) : Player.Listener {
    var prepared = false
        private set

    override fun onEvents(player: Player, events: Player.Events) {
        var next = prepared
        if (events.contains(Player.EVENT_MEDIA_ITEM_TRANSITION) || player.playbackState == Player.STATE_IDLE) next = false
        if (player.playbackState == Player.STATE_READY || player.playbackState == Player.STATE_ENDED) next = true
        if (next != prepared) {
            prepared = next
            changed()
        }
    }
}
