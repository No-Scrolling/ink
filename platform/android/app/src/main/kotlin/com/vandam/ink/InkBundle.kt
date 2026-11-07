package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.util.AtomicFile
import java.io.File
import org.json.JSONObject

internal class InkBundle(private val context: Context, val name: String, directory: File? = null) {
    var directory = directory
        private set

    fun open(path: String) = directory?.let { File(it, path).inputStream() } ?: context.assets.open(path)

    fun image(path: String): NativeResult =
        directory?.let { NativeResult.File(File(it, path).path, deleteAfterRead = false) } ?: NativeResult.Asset(path)

    fun source(directory: File?) = directory?.let { File(it, "$name.js").readText() }

    fun icons(directory: File?) = directory?.let { File(it, "ink-icons-v1.bin").readBytes() }

    fun compatibility(directory: File?): String {
        val manifest = directory?.let { File(it, "ink-bundle-v1.json").readText() }
            ?: context.assets.open("ink-bundle-v1.json").bufferedReader().use { it.readText() }
        return JSONObject(manifest).optString("refreshCompatibilityHash")
    }

    fun select(directory: File?) { this.directory = directory }

    fun development(generation: String): File {
        require(generation.matches(Regex("[0-9]+"))) { "Invalid bundle generation" }
        return File(context.filesDir, "ink-dev/$generation").also {
            check(File(it, "complete").isFile) { "Bundle transfer is incomplete: ${it.absolutePath}/complete" }
        }
    }

    fun markActive(generation: String) {
        File(context.filesDir, "ink-dev/active-worker").writeText(generation)
        File(context.filesDir, "ink-dev").listFiles().orEmpty()
            .filter { it.isDirectory && it.name.matches(Regex("[0-9]+")) }
            .sortedByDescending { it.name }.drop(3).filter { it != directory }
            .forEach { it.deleteRecursively() }
    }

    fun clearActive() { File(context.filesDir, "ink-dev/active-worker").delete() }

    fun audioUri(source: String): Uri =
        if (BuildConfig.DEBUG && directory != null) Uri.fromFile(mediaFile(source)) else Uri.parse(source)

    fun mediaFile(source: String): File {
        val path = source.removePrefix("asset:///")
        require(path.matches(Regex("ink-assets/[a-f0-9]{64}\\.(mp3|mp4|mov|webm)"))) { "Invalid bundled media source" }
        val media = File(context.filesDir, "ink-media").apply { mkdirs() }
        val pinned = File(media, path.substringAfterLast('/'))
        if (!pinned.isFile) {
            val atomic = AtomicFile(pinned)
            val stream = atomic.startWrite()
            try {
                open(path).use { it.copyTo(stream) }
                atomic.finishWrite(stream)
            } catch (error: Exception) {
                atomic.failWrite(stream)
                throw error
            }
        }
        return pinned
    }

    companion object {
        fun overlay(context: Context): InkBundle {
            val directory = if (BuildConfig.DEBUG) {
                File(context.filesDir, "ink-dev/active-worker").takeIf { it.isFile }?.readText()
                    ?.takeIf { it.matches(Regex("[0-9]+")) }
                    ?.let { File(context.filesDir, "ink-dev/$it") }
                    ?.takeIf { File(it, "complete").isFile && File(it, "overlay.js").isFile }
            } else null
            return InkBundle(context, "overlay", directory)
        }
    }
}
