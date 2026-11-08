package com.vandam.ink

import android.content.Context
import android.net.Uri
import java.io.File

internal fun audioSourceUri(context: Context, src: String, bundledAudio: (String) -> Uri): Uri {
    return when {
        src.startsWith("ink-source://") -> Uri.parse(src).also { InkMediaSources.source(context, it) }
        src.startsWith("https://") || src.startsWith("content://") -> Uri.parse(src)
        src.startsWith("http://") -> {
            require(BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED) {
                "HTTP audio requires the network-cleartext capability"
            }
            Uri.parse(src)
        }
        src.startsWith("asset:///") -> bundledAudio(src)
        src.startsWith("ink-file://") -> Uri.fromFile(InkManagedFiles(context).resolve(src))
        src.startsWith(RECORDING_PREFIX) -> {
            val recordingId = src.removePrefix(RECORDING_PREFIX)
            require(RECORDING_ID.matches(recordingId)) { "Invalid Ink recording source" }
            Uri.fromFile(File(context.filesDir, "recordings/$recordingId.m4a"))
        }
        else -> error(
            "Audio sources must be bundled assets, HTTPS URLs, content URIs, Ink files, Ink recordings or registered media sources",
        )
    }
}

private const val RECORDING_PREFIX = "ink://audio/recordings/"
private val RECORDING_ID = Regex("[0-9a-f-]{36}")
