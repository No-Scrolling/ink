package com.vandam.ink

import android.content.Context
import android.net.Uri
import java.io.File

internal fun createRingtoneFiles(context: Context): RingtoneFiles = InkRingtoneFiles(context)

private class InkRingtoneFiles(private val context: Context) : RingtoneFiles {
    private val root = File(context.filesDir, "shared/ringtones").canonicalFile
    private val preferences = context.getSharedPreferences("ink-ringtones", Context.MODE_PRIVATE)

    override fun stage(source: String): StagedRingtone {
        require(source.startsWith(ASSET_PREFIX)) { "Ringtone source is not a bundled asset" }
        val assetPath = source.removePrefix("asset:///")
        require(!assetPath.contains("..")) {
            "Ringtone asset path is invalid"
        }
        val name = File(assetPath).name
        require(name.isNotEmpty()) { "Ringtone asset name is invalid" }
        root.mkdirs()
        val target = resolve(name)
        val temporary = resolve(".$name.tmp")
        context.assets.open(assetPath).use { input ->
            temporary.outputStream().use { output ->
                input.copyTo(output)
                output.fd.sync()
            }
        }
        if (!temporary.renameTo(target)) {
            temporary.delete()
            error("Could not stage ringtone")
        }
        return StagedRingtone(
            relativePath = "ringtones/$name",
            uri = Uri.Builder()
                .scheme("content")
                .authority("${context.packageName}.lightfiles")
                .appendPath("ringtones")
                .appendPath(name)
                .build(),
        )
    }

    override fun commit(kind: String, staged: StagedRingtone) {
        val key = "current-$kind"
        val previous = preferences.getString(key, null)
        preferences.edit().putString(key, staged.relativePath).apply()
        if (previous != null && previous != staged.relativePath) {
            resolve(previous.removePrefix("ringtones/")).delete()
        }
    }

    override fun discard(staged: StagedRingtone) {
        val retained = listOf("ringtone", "notification", "alarm")
            .mapNotNull { preferences.getString("current-$it", null) }
            .toSet()
        if (staged.relativePath !in retained) {
            resolve(staged.relativePath.removePrefix("ringtones/")).delete()
        }
    }

    private fun resolve(name: String): File {
        val file = File(root, name).canonicalFile
        require(file.toPath().startsWith(root.toPath()) && file != root) {
            "Ringtone path escapes the shared directory"
        }
        return file
    }

    private companion object {
        const val ASSET_PREFIX = "asset:///ink/"
    }
}
