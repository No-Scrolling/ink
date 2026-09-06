package com.vandam.ink

import android.Manifest
import android.app.Activity
import android.content.ContentResolver
import android.content.ContentUris
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.net.Uri
import android.os.Bundle
import android.os.CancellationSignal
import android.provider.MediaStore
import android.provider.Settings
import android.util.Size
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

internal class InkMediaLibrary(private val activity: Activity) {
    private val executor = Executors.newFixedThreadPool(2)
    private val pending = ConcurrentHashMap<Long, CancellationSignal>()
    private val preferences = activity.getSharedPreferences("ink-media-permissions", 0)
    @Volatile private var stopped = false
    private var permissionRequest: PermissionRequest? = null
    private data class PermissionRequest(val id: Long, val kind: String, val complete: NativeResultHandler)

    fun execute(id: Long, operation: String, data: JSONObject, complete: NativeResultHandler) {
        if (stopped) return
        if (!BuildConfig.INK_MEDIA_LIBRARY_ENABLED) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Media library is not included in this app", false))
            return
        }
        val signal = CancellationSignal()
        pending[id] = signal
        if (operation == "media-permission" || operation == "media-settings") {
            activity.runOnUiThread {
                if (stopped || signal.isCanceled) { pending.remove(id); return@runOnUiThread }
                try {
                    if (operation == "media-settings") {
                        activity.startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${activity.packageName}")))
                        deliver(id, NativeResult.Success("null"), complete)
                    } else {
                        val kind = kind(data)
                        val status = permission(kind)
                        if (!data.optBoolean("request") || status == "granted") deliver(id, NativeResult.Success(status), complete)
                        else {
                            check(permissionRequest == null) { "Media access is already being requested" }
                            permissionRequest = PermissionRequest(id, kind, complete)
                            val permissions = permissions(kind)
                            preferences.edit().apply { permissions.forEach { putBoolean(it, true) } }.apply()
                            try { activity.requestPermissions(permissions, PERMISSION_REQUEST) }
                            catch (error: Exception) { permissionRequest = null; throw error }
                        }
                    }
                } catch (error: Exception) { deliver(id, failure(error), complete) }
            }
            return
        }
        executor.execute {
            var temporary: File? = null
            var imported: JSONObject? = null
            var delivered = false
            try {
                signal.throwIfCanceled()
                val result = when (operation) {
                    "media-list" -> {
                        val kind = kind(data)
                        requireAccess(kind)
                        NativeResult.Success(list(kind, data, signal).toString())
                    }
                    "media-import" -> {
                        val media = media(data.getString("id"))
                        requireAccess(media.first)
                        imported = InkManagedFiles(activity).importUri(media.second) { stopped || signal.isCanceled }
                        NativeResult.Success(requireNotNull(imported).toString())
                    }
                    "image" -> {
                        val source = Uri.parse(data.getString("source"))
                        require(source.scheme == "ink-media" && source.host in setOf("image", "video")) { "Invalid media thumbnail source" }
                        val media = media("${source.host}:${source.path?.removePrefix("/")}")
                        requireAccess(media.first)
                        val bitmap = activity.contentResolver.loadThumbnail(media.second, Size(256, 256), signal)
                        var scaled: Bitmap? = null
                        try {
                            val ratio = minOf(1.0, 256.0 / bitmap.width, 256.0 / bitmap.height)
                            val thumbnail = if (ratio < 1) Bitmap.createScaledBitmap(bitmap, maxOf(1, (bitmap.width * ratio).toInt()), maxOf(1, (bitmap.height * ratio).toInt()), true).also { scaled = it } else bitmap
                            val output = File.createTempFile("ink-media-thumb-", ".jpg", activity.cacheDir).also { temporary = it }
                            output.outputStream().use { check(thumbnail.compress(Bitmap.CompressFormat.JPEG, 85, it)) { "Thumbnail could not be encoded" } }
                            NativeResult.File(output.path, deleteAfterRead = true)
                        } finally { scaled?.takeIf { it !== bitmap }?.recycle(); bitmap.recycle() }
                    }
                    else -> error("Unknown media library operation")
                }
                if (stopped || signal.isCanceled) {
                    temporary?.delete()
                    imported?.let { InkManagedFiles(activity).remove(it.getString("id")) }
                    pending.remove(id)
                } else {
                    delivered = deliver(id, result, complete)
                    if (!delivered) {
                        imported?.let { InkManagedFiles(activity).remove(it.getString("id")) }
                    }
                }
            } catch (error: Exception) {
                temporary?.delete()
                if (!delivered) imported?.let { runCatching { InkManagedFiles(activity).remove(it.getString("id")) } }
                deliver(id, failure(error), complete)
            }
        }
    }

    private fun list(kind: String, data: JSONObject, signal: CancellationSignal): JSONObject {
        val limit = data.optInt("limit", 50)
        require(limit in 1..60) { "Media page size must be between 1 and 60" }
        val selection = mutableListOf("${MediaStore.Files.FileColumns.MEDIA_TYPE} IN (${if (kind == "image") 1 else if (kind == "video") 3 else "1,3"})", "is_pending = 0", "is_trashed = 0")
        val arguments = mutableListOf<String>()
        val cursor = data.optString("cursor").takeIf { it.isNotEmpty() && it != "null" }
        if (cursor != null) {
            val match = Regex("([0-9]+):([0-9]+)").matchEntire(cursor) ?: error("Invalid media cursor")
            require(match.groupValues[1].toLongOrNull() != null && match.groupValues[2].toLongOrNull() != null) { "Invalid media cursor" }
            selection.add("(${MediaStore.Files.FileColumns.DATE_MODIFIED} < ? OR (${MediaStore.Files.FileColumns.DATE_MODIFIED} = ? AND ${MediaStore.Files.FileColumns._ID} < ?))")
            arguments.addAll(listOf(match.groupValues[1], match.groupValues[1], match.groupValues[2]))
        }
        val query = Bundle().apply {
            putString(ContentResolver.QUERY_ARG_SQL_SELECTION, selection.joinToString(" AND "))
            putStringArray(ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS, arguments.toTypedArray())
            putString(ContentResolver.QUERY_ARG_SQL_SORT_ORDER, "${MediaStore.Files.FileColumns.DATE_MODIFIED} DESC, ${MediaStore.Files.FileColumns._ID} DESC")
            putInt(ContentResolver.QUERY_ARG_LIMIT, limit + 1)
        }
        val projection = arrayOf(MediaStore.Files.FileColumns._ID, MediaStore.Files.FileColumns.MEDIA_TYPE, MediaStore.Files.FileColumns.DATE_MODIFIED, MediaStore.Files.FileColumns.DISPLAY_NAME, MediaStore.Files.FileColumns.MIME_TYPE, MediaStore.Files.FileColumns.WIDTH, MediaStore.Files.FileColumns.HEIGHT, MediaStore.Files.FileColumns.DURATION)
        val items = JSONArray()
        var lastCursor: String? = null
        var more = false
        requireNotNull(activity.contentResolver.query(MediaStore.Files.getContentUri("external"), projection, query, signal)) { "Media library could not be read" }.use { rows ->
            while (rows.moveToNext()) {
                signal.throwIfCanceled()
                if (items.length() == limit) { more = true; break }
                val id = rows.getLong(0)
                val type = if (rows.getInt(1) == 1) "image" else "video"
                val modified = rows.getLong(2)
                val item = JSONObject().put("id", "$type:$id").put("src", "ink-media://$type/$id?modified=$modified")
                    .put("name", rows.getString(3) ?: "Attachment")
                    .put("mimeType", rows.getString(4) ?: if (type == "image") "image/jpeg" else "video/mp4")
                    .put("width", maxOf(0, rows.getInt(5))).put("height", maxOf(0, rows.getInt(6)))
                if (type == "video" && !rows.isNull(7)) item.put("duration", maxOf(0L, rows.getLong(7)))
                items.put(item)
                lastCursor = "$modified:$id"
            }
        }
        return JSONObject().put("items", items).put("nextCursor", if (more) lastCursor else JSONObject.NULL)
    }

    private fun media(id: String): Pair<String, Uri> {
        val match = Regex("(image|video):([0-9]+)").matchEntire(id) ?: error("Invalid media identity")
        val number = requireNotNull(match.groupValues[2].toLongOrNull()) { "Invalid media identity" }
        require(number > 0) { "Invalid media identity" }
        val kind = match.groupValues[1]
        return kind to ContentUris.withAppendedId(if (kind == "image") MediaStore.Images.Media.EXTERNAL_CONTENT_URI else MediaStore.Video.Media.EXTERNAL_CONTENT_URI, number)
    }
    private fun kind(data: JSONObject): String = data.optString("kind", "all").also { require(it in setOf("image", "video", "all")) { "Invalid media kind" } }
    private fun permissions(kind: String): Array<String> = when (kind) {
        "image" -> arrayOf(Manifest.permission.READ_MEDIA_IMAGES)
        "video" -> arrayOf(Manifest.permission.READ_MEDIA_VIDEO)
        else -> arrayOf(Manifest.permission.READ_MEDIA_IMAGES, Manifest.permission.READ_MEDIA_VIDEO)
    }
    private fun permission(kind: String): String {
        val denied = permissions(kind).filter { activity.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
        return if (denied.isEmpty()) "granted" else if (denied.any { preferences.getBoolean(it, false) && !activity.shouldShowRequestPermissionRationale(it) }) "blocked" else "denied"
    }
    private fun requireAccess(kind: String) { if (permission(kind) != "granted") throw SecurityException("Allow full photo and video access to browse the media library") }
    fun onRequestPermissionsResult(requestCode: Int): Boolean {
        if (requestCode != PERMISSION_REQUEST) return false
        val request = permissionRequest ?: return true
        permissionRequest = null
        deliver(request.id, NativeResult.Success(permission(request.kind)), request.complete)
        return true
    }
    private fun deliver(id: Long, result: NativeResult, complete: NativeResultHandler): Boolean {
        val signal = pending.remove(id)
        if (!stopped && signal != null && !signal.isCanceled) { complete(result); return true }
        if (result is NativeResult.File && result.deleteAfterRead) File(result.path).delete()
        return false
    }
    fun cancel(id: Long) {
        pending[id]?.cancel()
    }
    fun stop() { stopped = true; permissionRequest = null; pending.values.forEach { it.cancel() }; executor.shutdownNow() }
    private fun failure(error: Throwable) = NativeResult.Failure(if (error is SecurityException) NativeErrorKind.PERMISSION_DENIED else NativeErrorKind.UNAVAILABLE, error.message ?: "Media library operation failed", true)
    private companion object { const val PERMISSION_REQUEST = 7305 }
}
