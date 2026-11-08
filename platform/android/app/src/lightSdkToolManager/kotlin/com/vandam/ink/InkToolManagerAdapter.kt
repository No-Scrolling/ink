package com.vandam.ink

import android.content.Context
import android.content.SharedPreferences
import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.FutureTask

internal fun createToolManagerAdapter(context: Context): ToolManagerAdapter = InkToolManagerAdapter(context.applicationContext)

private class InkToolManagerAdapter(private val context: Context) : ToolManagerAdapter {
    private val files = InkSharedFiles(context)
    private val executor = Executors.newSingleThreadExecutor()
    private val requests = ConcurrentHashMap<Long, Future<*>>()
    private val handler = Handler(Looper.getMainLooper())
    private val observations = ConcurrentHashMap<Long, Pair<NativeResultHandler, Runnable>>()
    private val preferences = context.getSharedPreferences("ink-tool-manager", Context.MODE_PRIVATE)
    private val listener = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
        if (key == "revision") observations.keys.toList().forEach(::finishObservation)
    }

    init { preferences.registerOnSharedPreferenceChangeListener(listener) }

    private fun finishObservation(id: Long) {
        observations.remove(id)?.let { (complete, timeout) ->
            handler.removeCallbacks(timeout)
            requests.remove(id)
            complete(NativeResult.Success(preferences.getLong("revision", 0).toString()))
        }
    }

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) =
        submit(requestId, operation, payload, null, complete)

    override fun executeBytes(requestId: Long, operation: String, payload: String, bytes: ByteArray, complete: NativeResultHandler) =
        submit(requestId, operation, payload, bytes, complete)

    private fun submit(id: Long, operation: String, payload: String, bytes: ByteArray?, complete: NativeResultHandler) {
        val future = FutureTask<Unit> {
            try {
                val request = JSONObject(payload)
                if (operation == "observe") {
                    val revision = request.getLong("revision")
                    val timeout = Runnable { finishObservation(id) }
                    synchronized(observations) {
                        if (!requests.containsKey(id)) return@FutureTask
                        observations[id] = complete to timeout
                        handler.postDelayed(timeout, 25_000)
                    }
                    if (preferences.getLong("revision", 0) != revision) finishObservation(id)
                } else {
                    val path = request.getString("path")
                    val result = when (operation) {
                        "list" -> NativeResult.Success(files.list(path).toString())
                        "exists" -> NativeResult.Success(files.resolve(path, true).exists().toString())
                        "read" -> NativeResult.Binary("", files.read(path, request.optLong("offset", 0), request.optInt("length", InkSharedFiles.MAX_BYTES)))
                        "read-text" -> {
                            val file = files.resolve(path)
                            require(file.length() <= 1024 * 1024) { "Shared text exceeds 1 MiB; read bytes in ranges" }
                            NativeResult.Success(file.readText())
                        }
                        "write", "write-text" -> {
                            val data = if (operation == "write") requireNotNull(bytes) else request.getString("text").toByteArray(Charsets.UTF_8)
                            files.write(path, data)
                            NativeResult.Success("")
                        }
                        "copy-from" -> {
                            files.copyFrom(path, request.getString("source"))
                            NativeResult.Success("")
                        }
                        "remove" -> NativeResult.Success(files.resolve(path).delete().toString())
                        "mkdir" -> {
                            val directory = files.resolve(path)
                            check(directory.isDirectory || directory.mkdirs()) { "Could not create shared directory" }
                            NativeResult.Success("")
                        }
                        else -> throw IllegalArgumentException("Unknown Tool Manager operation: $operation")
                    }
                    if (operation in setOf("write", "write-text", "copy-from", "remove", "mkdir")) {
                        check(preferences.edit().putLong("revision", preferences.getLong("revision", 0) + 1).commit())
                    }
                    if (requests.remove(id) != null) complete(result)
                }
            } catch (error: Exception) {
                if (requests.remove(id) != null) complete(NativeResult.Failure(
                    if (error is IllegalArgumentException) NativeErrorKind.PROTOCOL else NativeErrorKind.UNEXPECTED,
                    error.message ?: "Shared file operation failed", false))
            }
        }
        requests[id] = future
        executor.execute(future)
    }

    override fun cancel(requestId: Long) {
        synchronized(observations) {
            requests.remove(requestId)?.cancel(true)
            observations.remove(requestId)?.let { handler.removeCallbacks(it.second) }
        }
    }

    override fun stop() {
        preferences.unregisterOnSharedPreferenceChangeListener(listener)
        requests.keys.toList().forEach(::cancel)
        executor.shutdownNow()
    }
}
