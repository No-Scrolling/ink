package com.vandam.ink

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.provider.MediaStore
import androidx.core.content.FileProvider
import org.json.JSONObject
import java.util.concurrent.Executors
import java.util.concurrent.ConcurrentHashMap

internal fun createFilesAdapter(activity: Activity): FilesAdapter = InkFilesAdapter(activity)

private class InkFilesAdapter(private val activity: Activity) : FilesAdapter {
    private val files = InkManagedFiles(activity)
    private val library = lazy { InkMediaLibrary(activity) }
    private val executor = Executors.newSingleThreadExecutor()
    private val cancelled = ConcurrentHashMap.newKeySet<Long>()
    private val requests = ConcurrentHashMap.newKeySet<Long>()
    private var pending: Pending? = null
    @Volatile private var stopped = false
    private data class Pending(val id: Long, val operation: String, val fileId: String?, val complete: NativeResultHandler)

    override fun execute(requestId: Long, operation: String, payload: String, resultHandler: NativeResultHandler) {
        if (stopped) { resultHandler(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Files adapter is stopped", false)); return }
        val complete: NativeResultHandler = { result ->
            requests.remove(requestId)
            val wasCancelled = cancelled.remove(requestId)
            if (!stopped && !wasCancelled) resultHandler(result)
            else if (result is NativeResult.File && result.deleteAfterRead) java.io.File(result.path).delete()
        }
        val data = runCatching { JSONObject(payload) }.getOrElse { complete(failure(it)); return }
        if (operation.startsWith("media-") || operation == "image" && data.optString("source").startsWith("ink-media://")) {
            library.value.execute(requestId, operation, data, complete)
            return
        }
        requests.add(requestId)
        if (operation == "image") {
            executor.execute image@{
                if (stopped || cancelled.contains(requestId)) { requests.remove(requestId); cancelled.remove(requestId); return@image }
                try { complete(NativeResult.File(files.resolve(data.getString("source")).path, deleteAfterRead = false)) }
                catch (error: Exception) { complete(failure(error)) }
            }
            return
        }
        if (operation == "share") {
            executor.execute {
                try {
                        val file = requireNotNull(files.open(data.getString("id"))) { "Attachment has been removed" }
                        val source = files.resolve(file.getString("src"))
                        val name = java.io.File(file.getString("name")).name.takeUnless { it.isEmpty() || it == "." || it == ".." } ?: "Attachment"
                        val shared = java.io.File(activity.cacheDir, "ink-shares/${file.getString("id")}/$name")
                        shared.parentFile?.mkdirs()
                        try { source.copyTo(shared, overwrite = true) }
                        catch (error: Exception) { shared.delete(); throw error }
                        val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.ink.files", shared)
                        val intent = Intent(Intent.ACTION_SEND).setType(file.getString("mimeType"))
                            .putExtra(Intent.EXTRA_STREAM, uri).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                        intent.clipData = ClipData.newUri(activity.contentResolver, file.getString("name"), uri)
                        activity.runOnUiThread {
                            if (stopped || cancelled.remove(requestId)) { shared.delete(); return@runOnUiThread }
                            try {
                                activity.startActivity(Intent.createChooser(intent, "Share attachment"))
                                complete(NativeResult.Success("null"))
                            } catch (error: Exception) { shared.delete(); complete(failure(error)) }
                        }
                } catch (error: Exception) { complete(failure(error)) }
            }
            return
        }
        if (operation == "pick" || operation == "save") {
            activity.runOnUiThread {
                if (stopped || cancelled.remove(requestId)) return@runOnUiThread
                try {
                    check(pending == null) { "Another file picker is open" }
                    val intent = if (operation == "save") {
                        val file = requireNotNull(files.open(data.getString("id"))) { "Attachment has been removed" }
                        Intent(Intent.ACTION_CREATE_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)
                            .setType(file.getString("mimeType")).putExtra(Intent.EXTRA_TITLE, file.getString("name"))
                    } else {
                        val kind = data.optString("kind")
                        if (kind in listOf("image", "video", "all")) {
                            Intent(MediaStore.ACTION_PICK_IMAGES).apply {
                                if (kind != "all") type = "$kind/*"
                            }
                        } else {
                            val types = data.optJSONArray("types")?.let { array -> Array(array.length()) { array.getString(it) } }.orEmpty()
                            Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)
                                .setType(if (types.size == 1) types[0] else "*/*")
                                .apply { if (types.size > 1) putExtra(Intent.EXTRA_MIME_TYPES, types) }
                        }
                    }
                    pending = Pending(requestId, operation, data.optString("id").takeIf { it.isNotEmpty() }, complete)
                    try { activity.startActivityForResult(intent, REQUEST_CODE) }
                    catch (error: Exception) { pending = null; throw error }
                } catch (error: Exception) { complete(failure(error)) }
            }
            return
        }
        executor.execute {
            try {
                val result = when (operation) {
                    "open" -> files.open(data.getString("id"))?.toString() ?: "null"
                    "remove" -> { files.remove(data.getString("id")); "null" }
                    "prepare-image" -> {
                        check(BuildConfig.INK_FILE_IMAGES_ENABLED) { "Image preparation is not included in this app" }
                        files.prepareImage(data.getString("id"), data.getInt("maxWidth"), data.getInt("maxHeight")).toString()
                    }
                    else -> error("Unknown file operation")
                }
                if (stopped || cancelled.remove(requestId)) {
                    if (operation == "prepare-image") files.remove(JSONObject(result).getString("id"))
                } else complete(NativeResult.Success(result))
            } catch (error: Exception) { if (!stopped && !cancelled.remove(requestId)) complete(failure(error)) }
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, intent: Intent?): Boolean {
        if (requestCode != REQUEST_CODE) return false
        val request = pending ?: return true
        pending = null
        if (cancelled.remove(request.id) || stopped) return true
        val uri = intent?.data
        if (resultCode != Activity.RESULT_OK || uri == null) { request.complete(NativeResult.Success("null")); return true }
        executor.execute {
            var imported: JSONObject? = null
            try {
                if (request.operation == "pick") {
                    imported = files.importUri(uri) { stopped || cancelled.contains(request.id) }
                } else {
                    val file = requireNotNull(files.open(requireNotNull(request.fileId))) { "Attachment has been removed" }
                    files.resolve(file.getString("src")).inputStream().use { source ->
                        requireNotNull(activity.contentResolver.openOutputStream(uri, "wt")) { "Destination cannot be opened" }.use { source.copyTo(it) }
                    }
                }
                if (stopped || cancelled.remove(request.id)) imported?.let { files.remove(it.getString("id")) }
                else request.complete(NativeResult.Success(imported?.toString() ?: "null"))
            } catch (error: Exception) {
                if (request.operation == "save") runCatching { android.provider.DocumentsContract.deleteDocument(activity.contentResolver, uri) }
                if (!stopped && !cancelled.remove(request.id)) request.complete(failure(error))
            }
        }
        return true
    }
    override fun onRequestPermissionsResult(requestCode: Int): Boolean = library.isInitialized() && library.value.onRequestPermissionsResult(requestCode)
    override fun cancel(requestId: Long) { if (requests.remove(requestId)) cancelled.add(requestId); if (library.isInitialized()) library.value.cancel(requestId) }
    override fun stop() { stopped = true; pending = null; requests.clear(); cancelled.clear(); executor.shutdownNow(); if (library.isInitialized()) library.value.stop() }
    private fun failure(error: Throwable) = NativeResult.Failure(NativeErrorKind.UNAVAILABLE, error.message ?: "File operation failed", true)
    private companion object { const val REQUEST_CODE = 7304 }
}
