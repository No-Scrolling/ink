package com.vandam.ink

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.media.ExifInterface
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.provider.OpenableColumns
import org.json.JSONObject
import java.io.File
import java.util.UUID

/** Durable attachments and their metadata; no caller-supplied filesystem paths. */
internal class InkManagedFiles(private val context: Context) {
    private val directory get() = File(context.filesDir, "ink-files").apply { check(isDirectory || mkdirs()) { "Attachment storage is unavailable" } }
    private fun metadata(id: String): File {
        require(Regex("[A-Za-z0-9-]{1,80}").matches(id)) { "Invalid attachment identity" }
        return File(directory, "$id.json")
    }
    fun open(id: String): JSONObject? {
        val metadata = metadata(id)
        if (!metadata.exists()) return null
        val stored = JSONObject(metadata.readText())
        val file = storedFile(stored)
        if (!file.isFile) return null
        return JSONObject(stored.toString()).apply { remove("path"); put("size", file.length()) }
    }
    private fun storedFile(data: JSONObject): File {
        val file = File(data.getString("path")).canonicalFile
        val roots = listOf(directory, File(context.filesDir, "recordings"), File(context.noBackupFilesDir, "ink-camera"))
        require(roots.any { file.path.startsWith(it.canonicalPath + File.separator) }) { "Attachment is outside managed storage" }
        return file
    }
    fun resolve(src: String): File {
        require(src.startsWith("ink-file://")) { "Invalid managed source" }
        val id = src.removePrefix("ink-file://")
        val data = JSONObject(metadata(id).readText())
        return storedFile(data).also { require(it.isFile) { "Attachment has been removed" } }
    }
    fun adopt(file: File, mimeType: String, name: String = file.name, id: String = UUID.randomUUID().toString(), width: Int? = null, height: Int? = null, duration: Long? = null): JSONObject {
        val data = JSONObject().put("id", id).put("src", "ink-file://$id").put("name", name)
            .put("mimeType", mimeType).put("size", file.length()).put("path", file.canonicalPath)
        storedFile(data)
        if (width != null) data.put("width", width)
        if (height != null) data.put("height", height)
        if (duration != null) data.put("duration", duration)
        val temporary = File(directory, "$id.json.partial")
        try {
            temporary.writeText(data.toString())
            check(temporary.renameTo(metadata(id))) { "Attachment metadata could not be saved" }
        } finally { temporary.delete() }
        return JSONObject(data.toString()).apply { remove("path") }
    }
    fun importUri(uri: Uri, cancelled: () -> Boolean = { false }): JSONObject {
        val resolver = context.contentResolver
        var name = "Attachment"
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst()) name = cursor.getString(0) ?: name
        }
        val mime = resolver.getType(uri) ?: "application/octet-stream"
        val id = UUID.randomUUID().toString()
        val partial = File(directory, "$id.partial")
        val output = File(directory, "$id.data")
        try {
            requireNotNull(resolver.openInputStream(uri)) { "Attachment cannot be opened" }.use { input ->
                partial.outputStream().buffered().use { sink ->
                    val bytes = ByteArray(32768)
                    while (true) {
                        check(!cancelled()) { "Selection was cancelled" }
                        val count = input.read(bytes)
                        if (count < 0) break
                        sink.write(bytes, 0, count)
                    }
                }
            }
            check(!cancelled()) { "Selection was cancelled" }
            check(partial.renameTo(output)) { "Attachment could not be committed" }
            val dimensions = if (mime.startsWith("image/")) BitmapFactory.Options().apply { inJustDecodeBounds = true; BitmapFactory.decodeFile(output.path, this) } else null
            var duration: Long? = null
            var width = dimensions?.outWidth?.takeIf { it > 0 }
            var height = dimensions?.outHeight?.takeIf { it > 0 }
            if (mime.startsWith("audio/") || mime.startsWith("video/")) runCatching {
                val retriever = MediaMetadataRetriever()
                try {
                    retriever.setDataSource(output.path)
                    duration = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull()
                    width = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH)?.toIntOrNull()
                    height = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT)?.toIntOrNull()
                } finally { retriever.release() }
            }
            return adopt(output, mime, name, id, width, height, duration)
        } catch (error: Exception) { output.delete(); throw error }
        finally { partial.delete() }
    }
    fun remove(id: String) {
        val metadata = metadata(id)
        if (!metadata.exists()) return
        val file = storedFile(JSONObject(metadata.readText()))
        check(!file.exists() || file.delete()) { "Attachment could not be removed" }
        check(metadata.delete()) { "Attachment metadata could not be removed" }
        File(context.cacheDir, "ink-shares/$id").deleteRecursively()
    }
    fun prepareImage(id: String, maxWidth: Int, maxHeight: Int): JSONObject {
        require(maxWidth > 0 && maxHeight > 0) { "Image bounds must be positive" }
        val original = requireNotNull(open(id)) { "Attachment has been removed" }
        require(original.getString("mimeType").startsWith("image/")) { "Attachment is not an image" }
        val file = resolve(original.getString("src"))
        val orientation = runCatching { ExifInterface(file.path).getAttributeInt(ExifInterface.TAG_ORIENTATION, 1) }.getOrDefault(1)
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true; BitmapFactory.decodeFile(file.path, this) }
        require(bounds.outWidth > 0 && bounds.outHeight > 0) { "Image cannot be decoded" }
        val rotated = orientation in 5..8
        val targetWidth = if (rotated) maxHeight else maxWidth
        val targetHeight = if (rotated) maxWidth else maxHeight
        var sample = 1
        while (bounds.outWidth / (sample * 2) >= targetWidth && bounds.outHeight / (sample * 2) >= targetHeight) sample *= 2
        val decoded = requireNotNull(BitmapFactory.decodeFile(file.path, BitmapFactory.Options().apply { inSampleSize = sample })) { "Image cannot be decoded" }
        var oriented: Bitmap? = null
        var scaled: Bitmap? = null
        val output = File(directory, "${UUID.randomUUID()}.jpg")
        try {
            val matrix = Matrix().apply {
                when (orientation) {
                    2 -> setScale(-1f, 1f)
                    3 -> setRotate(180f)
                    4 -> setScale(1f, -1f)
                    5 -> { setRotate(90f); postScale(-1f, 1f) }
                    6 -> setRotate(90f)
                    7 -> { setRotate(270f); postScale(-1f, 1f) }
                    8 -> setRotate(270f)
                }
            }
            val image = Bitmap.createBitmap(decoded, 0, 0, decoded.width, decoded.height, matrix, true).also { oriented = it }
            val ratio = minOf(1.0, maxWidth.toDouble() / image.width, maxHeight.toDouble() / image.height)
            val result = Bitmap.createScaledBitmap(image, maxOf(1, (image.width * ratio).toInt()), maxOf(1, (image.height * ratio).toInt()), true).also { scaled = it }
            output.outputStream().use { check(result.compress(Bitmap.CompressFormat.JPEG, 90, it)) { "Image could not be encoded" } }
            return adopt(output, "image/jpeg", original.getString("name").substringBeforeLast('.') + ".jpg", width = result.width, height = result.height)
        } catch (error: Exception) { output.delete(); throw error }
        finally { scaled?.takeIf { it !== oriented && it !== decoded }?.recycle(); oriented?.takeIf { it !== decoded }?.recycle(); decoded.recycle() }
    }
}
