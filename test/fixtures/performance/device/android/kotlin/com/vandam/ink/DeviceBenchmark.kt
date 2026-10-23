package com.vandam.ink

import android.graphics.BitmapFactory
import android.os.SystemClock
import android.util.Log
import java.io.File
import java.security.MessageDigest
import org.json.JSONArray
import org.json.JSONObject

internal fun createAppNativeAdapters(activity: MainActivity): Map<String, NativeAdapter> =
    mapOf("device-benchmark" to DeviceBenchmark(activity))

// Fixture preparation only. Image requests still pass through the production assets/files/network adapters.
private class DeviceBenchmark(private val activity: MainActivity) : NativeAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            val input = JSONObject(payload)
            val result = when (operation) {
                "prepare" -> {
                    val sources = input.getJSONArray("sources")
                    val observations = JSONArray()
                    val directory = File(activity.filesDir, "ink-files").apply { check(isDirectory || mkdirs()) }
                    for (index in 0 until sources.length()) {
                        val source = sources.getString(index)
                        require(source.startsWith("asset://"))
                        val bytes = activity.openBundleAsset(source.removePrefix("asset://")).use { it.readBytes() }
                        val digest = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
                        val dimensions = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, dimensions)
                        check(dimensions.outWidth == 512 && dimensions.outHeight == 512)
                        // Untimed independent encoded-file validation; this bitmap never enters Ink's image completion path.
                        val decoded = checkNotNull(BitmapFactory.decodeByteArray(bytes, 0, bytes.size))
                        val decodedWidth = decoded.width
                        val decodedHeight = decoded.height
                        decoded.recycle()
                        check(decodedWidth == 512 && decodedHeight == 512)
                        val id = "device-benchmark-$index"
                        val file = File(directory, "$id.data")
                        if (!file.isFile || !file.readBytes().contentEquals(bytes)) file.writeBytes(bytes)
                        val reference = InkManagedFiles(activity).adopt(file, "image/png", "$index.png", id, 512, 512)
                        val observation = JSONObject().put("index", index).put("asset", source)
                            .put("file", reference).put("path", file.canonicalPath).put("sha256", digest)
                            .put("bytes", bytes.size).put("width", dimensions.outWidth).put("height", dimensions.outHeight)
                            .put("validationDecodedWidth", decodedWidth).put("validationDecodedHeight", decodedHeight)
                        observations.put(observation)
                        Log.i("Ink", "InkDeviceFile $observation")
                    }
                    JSONObject().put("version", 1).put("images", observations)
                }
                "mark" -> input.put("bootBeforeNs", SystemClock.elapsedRealtimeNanos())
                    .put("ns", System.nanoTime()).put("bootAfterNs", SystemClock.elapsedRealtimeNanos())
                    .also { Log.i("Ink", "InkDeviceMark $it") }
                else -> error("Unknown device benchmark operation")
            }
            complete(NativeResult.Success(result.toString()))
        } catch (error: Exception) {
            complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, error.message ?: "Device fixture failed", false))
        }
    }

    override fun cancel(requestId: Long) = Unit
}
