package com.vandam.ink

import android.content.Context
import android.os.SystemClock
import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

internal class InkWorker(private val context: Context) : AutoCloseable {
    private val handle = AtomicLong()
    private val requests = ConcurrentHashMap<Long, NativeAdapter>()
    private val timeouts = ConcurrentHashMap<Long, Runnable>()
    private val stopAfterCancel = Runnable { close() }

    fun run(task: String, input: Any?): JSONObject {
        val source = context.assets.open("worker.js").bufferedReader().use { it.readText() }
        val runtime = nativeStart(source)
        check(runtime != 0L) { "Could not start background JavaScript" }
        check(handle.compareAndSet(0, runtime)) {
            nativeStop(runtime)
            "Worker is already running or closed"
        }
        var store: StoreAdapter? = null
        var network: NetworkAdapter? = null
        var background: BackgroundAdapter? = null
        var notifications: NativeAdapter? = null
        var location: LocationAdapter? = null
        val deadline = SystemClock.elapsedRealtime() + 120_000
        try {
            while (handle.get() == runtime && !Thread.currentThread().isInterrupted) {
                check(SystemClock.elapsedRealtime() < deadline) { "Background worker timed out" }
                val message = nativeNext(runtime)
                if (message.isEmpty()) continue
                val event = JSONObject(message)
                when (event.getString("type")) {
                    "worker-ready" -> check(nativeReceive(runtime, JSONObject()
                        .put("type", "run-task").put("task", task).put("input", input ?: JSONObject.NULL).toString()))
                    "task-result" -> return event.getJSONObject("result")
                    "worker-error" -> error(event.getString("message"))
                    "worker-stopped" -> error("Background JavaScript stopped without a result")
                    "log" -> Log.println(if (event.optString("level") == "error") Log.ERROR else Log.INFO,
                        "InkWorker", event.optString("message"))
                    "cancel" -> {
                        val id = event.getLong("id")
                        timeouts.remove(id)?.let(handler::removeCallbacks)
                        requests.remove(id)?.cancel(id)
                    }
                    "call" -> {
                        val id = event.getLong("id")
                        require(id > 0 && !requests.containsKey(id) && requests.size < 256) { "Invalid worker request ID" }
                        val operation = event.getString("operation")
                        val module = event.getString("module")
                        val adapter = when (module) {
                            "permissions" -> if (operation == "status" && event.optJSONObject("payload")?.optString("permission") == "notifications") {
                                notifications ?: createWorkerNotificationsAdapter(context).also { notifications = it }
                            } else null
                            "background" -> background ?: createBackgroundAdapter(context).also { background = it }
                            "notifications" -> notifications ?: createWorkerNotificationsAdapter(context).also { notifications = it }
                            "location" -> if (operation == "current") location ?: createWorkerLocationAdapter(context).also { location = it } else null
                            "store" -> store ?: StoreAdapter(context).also { store = it }
                            "network" -> network ?: createNetworkAdapter(context, null).also { network = it }
                            else -> null
                        }
                        if (adapter == null || event.has("controller")) {
                            reply(runtime, id, NativeResult.Failure(NativeErrorKind.UNAVAILABLE,
                                "This native operation is not available in a background worker", false))
                            continue
                        }
                        requests[id] = adapter
                        val timeout = Runnable {
                            timeouts.remove(id)
                            if (requests.remove(id, adapter)) {
                                adapter.cancel(id)
                                reply(runtime, id, NativeResult.Failure(NativeErrorKind.TIMEOUT, "Worker native request timed out", true))
                            }
                        }
                        timeouts[id] = timeout
                        handler.postDelayed(timeout, event.optLong("timeoutMs", 30_000).coerceIn(1, 120_000))
                        try {
                            adapter.execute(id, if (module == "permissions") "permission-status" else operation, event.opt("payload").toString()) { result ->
                                if (requests.remove(id, adapter)) {
                                    timeouts.remove(id)?.let(handler::removeCallbacks)
                                    reply(runtime, id, result)
                                }
                            }
                        } catch (error: Exception) {
                            requests.remove(id)
                            timeouts.remove(id)?.let(handler::removeCallbacks)
                            reply(runtime, id, NativeResult.Failure(NativeErrorKind.UNEXPECTED,
                                error.message ?: "Worker native request failed", false))
                        }
                    }
                    else -> error("Unknown background JavaScript message")
                }
            }
            throw InterruptedException("Background worker was cancelled")
        } finally {
            close()
            background?.stop()
            location?.stop()
            network?.stop()
            store?.stop()
        }
    }

    private fun reply(runtime: Long, id: Long, result: NativeResult) {
        val message = javascriptResult(id, result)
        if (handle.get() == runtime && !nativeReceive(runtime, message)) {
            Log.e("InkWorker", "Could not deliver native result to worker")
            close()
        }
    }

    override fun close() {
        handler.removeCallbacks(stopAfterCancel)
        val runtime = handle.getAndSet(-1)
        if (runtime > 0) nativeStop(runtime)
        timeouts.values.forEach(handler::removeCallbacks)
        timeouts.clear()
        requests.entries.forEach { (id, adapter) -> if (requests.remove(id, adapter)) adapter.cancel(id) }
    }

    fun cancel() {
        val runtime = handle.get()
        if (runtime <= 0 || !nativeReceive(runtime, "{\"type\":\"cancel-task\"}")) close()
        else handler.postDelayed(stopAfterCancel, 250)
    }

    companion object {
        private val handler = Handler(Looper.getMainLooper())
        init { System.loadLibrary("ink_android") }
        @JvmStatic private external fun nativeStart(source: String): Long
        @JvmStatic private external fun nativeNext(handle: Long): String
        @JvmStatic private external fun nativeReceive(handle: Long, message: String): Boolean
        @JvmStatic private external fun nativeStop(handle: Long)
    }
}
