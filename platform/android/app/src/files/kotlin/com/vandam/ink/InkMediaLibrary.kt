package com.vandam.ink

import android.Manifest
import android.app.Activity
import android.content.ContentResolver
import android.content.ContentUris
import android.content.pm.PackageManager
import android.database.ContentObserver
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.graphics.Point
import android.media.ExifInterface
import android.net.Uri
import android.os.Bundle
import android.os.CancellationSignal
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingDeque
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

internal class InkMediaLibrary(private val activity: Activity, private val emit: (String) -> Unit) {
    private val executor = Executors.newSingleThreadExecutor()
    // Thumbnails requested last are nearest the reader, so they are fetched first.
    private val thumbnails = ThreadPoolExecutor(4, 4, 30, TimeUnit.SECONDS, NewestFirst())
    private val previews = Executors.newFixedThreadPool(4) { task -> Thread(task, "ink-media-previews") }
    private val pending = ConcurrentHashMap<Long, CancellationSignal>()
    private val consents = ConcurrentHashMap<Int, Consent>()
    private var observer: ContentObserver? = null
    private var observedVersion: String? = null
    @Volatile private var stopped = false

    private class Consent(val run: () -> String, val complete: NativeResultHandler)
    private class NewestFirst : LinkedBlockingDeque<Runnable>() {
        override fun offer(element: Runnable) = offerFirst(element)
    }

    fun execute(id: Long, operation: String, data: JSONObject, complete: NativeResultHandler) {
        if (stopped) return
        if (!BuildConfig.INK_MEDIA_LIBRARY_ENABLED) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Media library is not included in this app", false))
            return
        }
        when (operation) {
            "media-watch", "media-unwatch" -> {
                activity.runOnUiThread {
                    try { watch(operation == "media-watch"); complete(NativeResult.Success(version())) }
                    catch (error: Exception) { complete(failure(error)) }
                }
                return
            }
        }
        val signal = CancellationSignal()
        pending[id] = signal
        val queue = if (operation == "image") thumbnails else executor
        queue.execute {
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
                    "media-counts" -> {
                        requireAccess("all")
                        NativeResult.Success(counts(data, signal).toString())
                    }
                    "media-import" -> {
                        val media = media(data.getString("id"))
                        requireAccess(media.first)
                        val file = InkManagedFiles(activity).importUri(media.second) { stopped || signal.isCanceled }
                        imported = file
                        NativeResult.Success(file.toString()) { InkManagedFiles(activity).remove(file.getString("id")) }
                    }
                    "media-previews" -> {
                        requireAccess("all")
                        NativeResult.Success(warmPreviews(data.getJSONArray("ids"), signal).toString())
                    }
                    "media-delete" -> {
                        val ids = data.getJSONArray("ids")
                        val uris = (0 until ids.length()).map { media(ids.getString(it)).second }
                        requireAccess("all")
                        if (pending.remove(id) != null) withConsent(id, MediaStore.createDeleteRequest(activity.contentResolver, uris), complete) { delete(uris).toString() }
                        return@execute
                    }
                    "media-rotate" -> {
                        val media = media(data.getString("id"))
                        require(media.first == "image") { "Only photos can be rotated" }
                        val degrees = data.getInt("degrees")
                        require(degrees % 90 == 0) { "Rotation must be a quarter turn" }
                        requireAccess("image")
                        if (pending.remove(id) != null) withConsent(id, MediaStore.createWriteRequest(activity.contentResolver, listOf(media.second)), complete) { rotate(media.second, degrees).toString() }
                        return@execute
                    }
                    "image" -> {
                        val source = Uri.parse(data.getString("source"))
                        require(source.scheme == "ink-media" && source.host in setOf("image", "video")) { "Invalid media thumbnail source" }
                        val media = media("${source.host}:${source.path?.removePrefix("/")}")
                        requireAccess(media.first)
                        val bytes = if (source.getQueryParameter("full") == "1" && media.first == "image") {
                            requireNotNull(activity.contentResolver.openInputStream(media.second)) { "Photo could not be opened" }.use { it.readBytes() }
                        } else thumbnail(media.second, signal)
                        NativeResult.EncodedImage(bytes, bytes.size)
                    }
                    else -> error("Unknown media library operation")
                }
                if (stopped || signal.isCanceled) {
                    imported?.let { InkManagedFiles(activity).remove(it.getString("id")) }
                    pending.remove(id)
                } else {
                    delivered = deliver(id, result, complete)
                    if (!delivered) {
                        imported?.let { InkManagedFiles(activity).remove(it.getString("id")) }
                    }
                }
            } catch (error: Exception) {
                if (!delivered) imported?.let { runCatching { InkManagedFiles(activity).remove(it.getString("id")) } }
                deliver(id, failure(error), complete)
            }
        }
    }

    // MediaProvider keeps generated thumbnails, so this returns its cached JPEG without re-encoding.
    private fun thumbnail(uri: Uri, signal: CancellationSignal?, size: Int = THUMBNAIL_SIZE): ByteArray {
        val options = Bundle().apply { putParcelable(ContentResolver.EXTRA_SIZE, Point(size, size)) }
        return requireNotNull(activity.contentResolver.openTypedAssetFile(uri, "image/*", options, signal)) { "Thumbnail unavailable" }
            .use { it.createInputStream().readBytes() }
    }

    private fun warmPreviews(ids: JSONArray, signal: CancellationSignal): Int {
        val query = Bundle().apply {
            putString(ContentResolver.QUERY_ARG_SQL_SELECTION, (selection("all") + identities(ids)).joinToString(" AND "))
            putString(ContentResolver.QUERY_ARG_SQL_SORT_ORDER, "${MediaStore.Files.FileColumns.DATE_ADDED} DESC, ${MediaStore.Files.FileColumns._ID} DESC")
        }
        val projection = arrayOf(MediaStore.Files.FileColumns._ID, MediaStore.Files.FileColumns.MEDIA_TYPE, MediaStore.Files.FileColumns.DATE_MODIFIED)
        val items = mutableListOf<Pair<String, Uri>>()
        activity.contentResolver.query(MediaStore.Files.getContentUri("external"), projection, query, signal)?.use { rows ->
            while (rows.moveToNext()) {
                val type = if (rows.getInt(1) == 1) "image" else "video"
                val url = "ink-media://$type/${rows.getLong(0)}?modified=${rows.getLong(2)}"
                items.add(url to media("$type:${rows.getLong(0)}").second)
            }
        }
        return storePreviews(items, signal)
    }

    private fun storePreviews(items: List<Pair<String, Uri>>, signal: CancellationSignal): Int {
        return previews.invokeAll(items.filter { (url, _) -> InkNative.nativeMediaPreviewNeeded(url) }.map { (url, uri) -> Callable {
            signal.throwIfCanceled()
            if (stopped) 0 else runCatching {
                if (InkNative.nativeMediaPreviewStore(url, thumbnail(uri, signal, 64))) 1 else 0
            }.getOrDefault(0)
        } }).sumOf { it.get() }
    }

    private fun withConsent(id: Long, request: android.app.PendingIntent, complete: NativeResultHandler, run: () -> String) {
        val finish: NativeResultHandler = { result -> if (!stopped) complete(result) else disposeNativeResult(result) }
        // Direct changes work with all-files access; otherwise Android asks once for these items.
        try { finish(NativeResult.Success(run())); return }
        catch (error: SecurityException) {}
        catch (error: Exception) { finish(failure(error)); return }
        val code = CONSENT_REQUEST + (id % 1000).toInt()
        consents[code] = Consent(run, finish)
        activity.runOnUiThread {
            try { activity.startIntentSenderForResult(request.intentSender, code, null, 0, 0, 0) }
            catch (error: Exception) { consents.remove(code)?.complete?.invoke(failure(error)) }
        }
    }

    fun onActivityResult(requestCode: Int, resultCode: Int): Boolean {
        val consent = consents.remove(requestCode) ?: return false
        if (resultCode != Activity.RESULT_OK) { consent.complete(NativeResult.Success("null")); return true }
        executor.execute {
            try { consent.complete(NativeResult.Success(consent.run())) }
            catch (error: Exception) { consent.complete(failure(error)) }
        }
        return true
    }

    private fun delete(uris: List<Uri>): JSONArray {
        val deleted = JSONArray()
        for (uri in uris) {
            // After consent the provider may already have removed the item.
            activity.contentResolver.delete(uri, null)
            deleted.put("${if (uri.toString().contains("/video/")) "video" else "image"}:${ContentUris.parseId(uri)}")
        }
        return deleted
    }

    private fun rotate(uri: Uri, degrees: Int): JSONObject {
        val turns = ((degrees / 90) % 4 + 4) % 4
        val mime = activity.contentResolver.getType(uri)
        if (mime == "image/jpeg") {
            // JPEG rotation changes only the orientation tag, keeping the original pixels and metadata.
            requireNotNull(activity.contentResolver.openFileDescriptor(uri, "rw")) { "Photo could not be opened" }.use { descriptor ->
                val exif = ExifInterface(descriptor.fileDescriptor)
                val current = exif.getAttributeInt(ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL)
                exif.setAttribute(ExifInterface.TAG_ORIENTATION, ORIENTATIONS[(ORIENTATIONS.indexOf(current).coerceAtLeast(0) + turns) % 4].toString())
                exif.saveAttributes()
            }
        } else {
            val bitmap = requireNotNull(activity.contentResolver.openInputStream(uri)?.use(BitmapFactory::decodeStream)) { "Photo could not be decoded" }
            val rotated = Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, Matrix().apply { postRotate(turns * 90f) }, true)
            try {
                requireNotNull(activity.contentResolver.openOutputStream(uri, "wt")) { "Photo could not be saved" }.use {
                    check(rotated.compress(if (mime == "image/png") Bitmap.CompressFormat.PNG else Bitmap.CompressFormat.WEBP_LOSSLESS, 100, it)) { "Photo could not be saved" }
                }
            } finally { rotated.recycle(); bitmap.recycle() }
        }
        val modified = activity.contentResolver.query(uri, arrayOf(MediaStore.MediaColumns.DATE_MODIFIED), null, null)?.use { if (it.moveToFirst()) it.getLong(0) else null }
        // Files changed through a descriptor are rescanned after closing, which can lag behind.
        val now = System.currentTimeMillis() / 1000
        val updated = if (modified == null || modified < now - 1) {
            activity.contentResolver.update(uri, android.content.ContentValues().apply { put(MediaStore.MediaColumns.DATE_MODIFIED, now) }, null)
            now
        } else modified
        val id = ContentUris.parseId(uri)
        // The grid's instant preview and thumbnail are ready before the app shows the new version.
        runCatching { thumbnail(uri, null) }.getOrNull()?.let { InkNative.nativeMediaPreviewStore("ink-media://image/$id?modified=$updated", it) }
        return item(uri, "image", id, updated)
    }

    private fun item(uri: Uri, type: String, id: Long, modified: Long): JSONObject {
        val projection = arrayOf(MediaStore.MediaColumns.DISPLAY_NAME, MediaStore.MediaColumns.MIME_TYPE, MediaStore.MediaColumns.WIDTH, MediaStore.MediaColumns.HEIGHT, MediaStore.MediaColumns.DATE_ADDED)
        return requireNotNull(activity.contentResolver.query(uri, projection, null, null)?.use { rows ->
            if (!rows.moveToFirst()) null
            else JSONObject().put("id", "$type:$id").put("src", "ink-media://$type/$id?modified=$modified")
                .put("name", rows.getString(0) ?: "Photo").put("mimeType", rows.getString(1) ?: "image/jpeg")
                .put("width", maxOf(0, rows.getInt(2))).put("height", maxOf(0, rows.getInt(3))).put("added", rows.getLong(4))
        }) { "Photo is no longer available" }
    }

    private fun selection(kind: String) = mutableListOf("${MediaStore.Files.FileColumns.MEDIA_TYPE} IN (${if (kind == "image") 1 else if (kind == "video") 3 else "1,3"})", "is_pending = 0", "is_trashed = 0", "${MediaStore.MediaColumns.SIZE} > 0")

    private fun identities(ids: JSONArray): String {
        val images = mutableListOf<Long>()
        val videos = mutableListOf<Long>()
        for (index in 0 until ids.length()) {
            val (kind, uri) = media(ids.getString(index))
            (if (kind == "image") images else videos).add(ContentUris.parseId(uri))
        }
        val terms = mutableListOf<String>()
        if (images.isNotEmpty()) terms.add("(media_type = 1 AND _id IN (${images.joinToString(",")}))")
        if (videos.isNotEmpty()) terms.add("(media_type = 3 AND _id IN (${videos.joinToString(",")}))")
        return if (terms.isEmpty()) "0" else "(${terms.joinToString(" OR ")})"
    }

    private fun counts(data: JSONObject, signal: CancellationSignal): JSONObject {
        fun count(kind: String, ids: JSONArray? = null): Long {
            val where = selection(kind)
            if (ids != null) where.add(identities(ids))
            val query = Bundle().apply { putString(ContentResolver.QUERY_ARG_SQL_SELECTION, where.joinToString(" AND ")) }
            return requireNotNull(activity.contentResolver.query(MediaStore.Files.getContentUri("external"), arrayOf(MediaStore.Files.FileColumns._ID), query, signal)) { "Media counts could not be read" }
                .use { rows -> rows.count.toLong() }
        }
        val images = count("image")
        val videos = count("video")
        val groups = data.getJSONObject("groups")
        val totals = JSONObject()
        for (key in groups.keys()) totals.put(key, count("all", groups.getJSONArray(key)))
        return JSONObject().put("all", images + videos).put("image", images).put("video", videos).put("groups", totals)
    }

    private val projection = arrayOf(MediaStore.Files.FileColumns._ID, MediaStore.Files.FileColumns.MEDIA_TYPE, MediaStore.Files.FileColumns.DATE_MODIFIED, MediaStore.Files.FileColumns.DISPLAY_NAME, MediaStore.Files.FileColumns.MIME_TYPE, MediaStore.Files.FileColumns.WIDTH, MediaStore.Files.FileColumns.HEIGHT, MediaStore.Files.FileColumns.DURATION, MediaStore.Files.FileColumns.DATE_ADDED)

    private fun row(rows: android.database.Cursor): JSONObject {
        val id = rows.getLong(0)
        val type = if (rows.getInt(1) == 1) "image" else "video"
        val modified = rows.getLong(2)
        val item = JSONObject().put("id", "$type:$id").put("src", "ink-media://$type/$id?modified=$modified")
            .put("name", rows.getString(3) ?: "Attachment")
            .put("mimeType", rows.getString(4) ?: if (type == "image") "image/jpeg" else "video/mp4")
            .put("width", maxOf(0, rows.getInt(5))).put("height", maxOf(0, rows.getInt(6))).put("added", rows.getLong(8))
        if (type == "video" && !rows.isNull(7)) item.put("duration", maxOf(0L, rows.getLong(7)))
        return item
    }

    private fun list(kind: String, data: JSONObject, signal: CancellationSignal): JSONObject {
        val limit = data.optInt("limit", 50)
        require(limit in 1..1000) { "Media page size must be between 1 and 1000" }
        // Sorting by date added keeps edited photos in place; the picker keeps its recent-change order.
        val column = if (data.optString("order") == "added") MediaStore.Files.FileColumns.DATE_ADDED else MediaStore.Files.FileColumns.DATE_MODIFIED
        val selection = selection(kind)
        if (data.has("ids") && !data.isNull("ids")) selection.add(identities(data.getJSONArray("ids")))
        val arguments = mutableListOf<String>()
        val cursor = data.optString("cursor").takeIf { it.isNotEmpty() && it != "null" }
        if (cursor != null) {
            val match = Regex("([0-9]+):([0-9]+)").matchEntire(cursor) ?: error("Invalid media cursor")
            require(match.groupValues[1].toLongOrNull() != null && match.groupValues[2].toLongOrNull() != null) { "Invalid media cursor" }
            selection.add("($column < ? OR ($column = ? AND ${MediaStore.Files.FileColumns._ID} < ?))")
            arguments.addAll(listOf(match.groupValues[1], match.groupValues[1], match.groupValues[2]))
        }
        val query = Bundle().apply {
            putString(ContentResolver.QUERY_ARG_SQL_SELECTION, selection.joinToString(" AND "))
            putStringArray(ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS, arguments.toTypedArray())
            putString(ContentResolver.QUERY_ARG_SQL_SORT_ORDER, "$column DESC, ${MediaStore.Files.FileColumns._ID} DESC")
            putInt(ContentResolver.QUERY_ARG_LIMIT, limit + 1)
        }
        val items = JSONArray()
        var lastCursor: String? = null
        var more = false
        requireNotNull(activity.contentResolver.query(MediaStore.Files.getContentUri("external"), projection, query, signal)) { "Media library could not be read" }.use { rows ->
            val sortIndex = if (column == MediaStore.Files.FileColumns.DATE_ADDED) 8 else 2
            while (rows.moveToNext()) {
                signal.throwIfCanceled()
                if (items.length() == limit) { more = true; break }
                items.put(row(rows))
                lastCursor = "${rows.getLong(sortIndex)}:${rows.getLong(0)}"
            }
        }
        // Publish a page only once its tiny previews can be drawn in the first frame.
        if (data.optBoolean("previews")) {
            storePreviews((0 until items.length()).map { index ->
                val item = items.getJSONObject(index)
                item.getString("src") to media(item.getString("id")).second
            }, signal)
        }
        return JSONObject().put("items", items).put("nextCursor", if (more) lastCursor else JSONObject.NULL).put("version", version())
    }

    private fun version() = "${MediaStore.getVersion(activity)}:${MediaStore.getGeneration(activity, MediaStore.VOLUME_EXTERNAL_PRIMARY)}"

    private fun watch(enabled: Boolean) {
        if (enabled && observer == null && !stopped) {
            observedVersion = version()
            val handler = Handler(Looper.getMainLooper())
            val notify = Runnable { changed() }
            observer = object : ContentObserver(handler) {
                // Camera saves arrive as several updates; one event follows the last of them.
                override fun onChange(selfChange: Boolean) { handler.removeCallbacks(notify); handler.postDelayed(notify, 150) }
            }.also { activity.contentResolver.registerContentObserver(MediaStore.Files.getContentUri("external"), true, it) }
        } else if (!enabled) {
            observer?.let(activity.contentResolver::unregisterContentObserver)
            observer = null
        }
    }

    /** Reports changes made while the app was in the background. */
    fun resume() { if (observer != null) activity.runOnUiThread { changed() } }

    private fun changed() {
        if (stopped || observer == null) return
        val current = version()
        if (current == observedVersion) return
        observedVersion = current
        emit(JSONObject().put("type", "media-changed").put("data", JSONObject().put("version", current)).toString())
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
    private fun requireAccess(kind: String) {
        if (Environment.isExternalStorageManager()) return
        if (permissions(kind).any { activity.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }) {
            throw SecurityException("Allow full photo and video access to browse the media library")
        }
    }
    private fun deliver(id: Long, result: NativeResult, complete: NativeResultHandler): Boolean {
        val signal = pending.remove(id)
        if (!stopped && signal != null && !signal.isCanceled) { complete(result); return true }
        disposeNativeResult(result)
        return false
    }
    fun cancel(id: Long) {
        pending[id]?.cancel()
    }
    fun stop() {
        stopped = true
        activity.runOnUiThread { watch(false) }
        pending.values.forEach { it.cancel() }
        consents.clear()
        executor.shutdownNow(); thumbnails.shutdownNow(); previews.shutdownNow()
    }
    private fun failure(error: Throwable) = NativeResult.Failure(if (error is SecurityException) NativeErrorKind.PERMISSION_DENIED else NativeErrorKind.UNAVAILABLE, error.message ?: "Media library operation failed", true)

    private companion object {
        const val THUMBNAIL_SIZE = 512
        const val CONSENT_REQUEST = 7400
        val ORIENTATIONS = listOf(ExifInterface.ORIENTATION_NORMAL, ExifInterface.ORIENTATION_ROTATE_90, ExifInterface.ORIENTATION_ROTATE_180, ExifInterface.ORIENTATION_ROTATE_270)
    }
}
