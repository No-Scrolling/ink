package com.vandam.ink

import android.content.Context
import java.util.concurrent.atomic.AtomicLong

internal const val SKIP_SILENCE_COMMAND = "ink.audio.skipSilence"
internal const val SKIP_SILENCE_EXTRA = "ink.skipSilence"
internal const val VOICE_BOOST_COMMAND = "ink.audio.voiceBoost"
internal const val VOICE_BOOST_EXTRA = "ink.voiceBoost"

internal class AudioEffects private constructor(context: Context, private val name: String) {
    private val preferences = context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE)
    @Volatile var voiceBoost = preferences.getBoolean("voiceBoost:$name", false)
        private set

    @Volatile var countSavings = false
    private val savedUs = AtomicLong(preferences.getLong("silenceSavedUs:$name", 0))
    private var persistedUs = savedUs.get()
    val silenceSavedMs: Long get() = savedUs.get() / 1000

    fun addSavedTime(microseconds: Long) {
        if (countSavings) savedUs.addAndGet(microseconds)
    }

    fun save() {
        val total = savedUs.get()
        if (total == persistedUs) return
        preferences.edit().putLong("silenceSavedUs:$name", total).apply()
        persistedUs = total
    }

    fun setVoiceBoost(enabled: Boolean) {
        voiceBoost = enabled
        preferences.edit().putBoolean("voiceBoost:$name", enabled).apply()
    }

    companion object {
        private val sessions = mutableMapOf<String, AudioEffects>()
        fun get(context: Context, name: String): AudioEffects =
            sessions.getOrPut(name) { AudioEffects(context.applicationContext, name) }
    }
}
