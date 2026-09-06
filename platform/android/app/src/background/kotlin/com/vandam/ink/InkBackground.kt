package com.vandam.ink

import android.content.Context
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.FutureTask

internal fun createBackgroundAdapter(context: Context): BackgroundAdapter =
    InkBackgroundAdapter(context.applicationContext)

private class InkBackgroundAdapter(private val context: Context) : BackgroundAdapter {
    private val executor = Executors.newSingleThreadExecutor()
    private val requests = ConcurrentHashMap<Long, Future<*>>()
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())
    private data class Observation(val revision: Long, val complete: NativeResultHandler, val timeout: Runnable)
    private val observations = ConcurrentHashMap<Long, Observation>()
    private val changed: () -> Unit = { handler.post { publishObservations() }; Unit }

    init { InkWorkerState.subscribe(changed) }

    private fun publishObservations() {
        val state = InkWorkerState.snapshot(context)
        observations.entries.forEach { (id, observation) ->
            if (observation.revision != state.getLong("revision") && observations.remove(id, observation)) {
                handler.removeCallbacks(observation.timeout)
                observation.complete(NativeResult.Success(state.toString()))
            }
        }
    }

    override fun enqueuePush(input: JSONObject): Boolean {
        val task = context.getSharedPreferences("ink-worker-state", Context.MODE_PRIVATE).getString("push-task", null) ?: return false
        InkWorkerJobs.enqueue(context, JSONObject().put("task", task).put("key", "ink.push." + java.util.UUID.randomUUID())
            .put("input", input))
        return true
    }

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation in setOf("task-state", "observe-task-state", "set-push-task", "enqueue-task", "cancel-task")) {
            val task = FutureTask<Unit> {
                if (!Thread.currentThread().isInterrupted) executeWorkerOperation(requestId, operation, payload) { result ->
                    if (requests.remove(requestId) != null) complete(result)
                }
            }
            requests[requestId] = task
            executor.execute(task)
            return
        }
        complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown background operation: $operation", false))
    }

    private fun executeWorkerOperation(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        if (operation == "task-state" || operation == "observe-task-state" || operation == "set-push-task") {
            try {
                val request = JSONObject(payload)
                if (operation == "set-push-task") {
                    val task = if (request.isNull("task")) null else request.getString("task")
                    require(task == null || task.matches(Regex("[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}"))) { "Invalid push task ID" }
                    check(context.getSharedPreferences("ink-worker-state", Context.MODE_PRIVATE).edit().putString("push-task", task).commit())
                    complete(NativeResult.Success("null"))
                } else {
                    val state = InkWorkerState.snapshot(context)
                    val revision = request.optLong("revision", -1)
                    if (operation == "task-state" || state.getLong("revision") != revision) complete(NativeResult.Success(state.toString()))
                    else {
                        val timeout = Runnable {
                            observations.remove(requestId)?.complete?.invoke(NativeResult.Success(InkWorkerState.snapshot(context).toString()))
                        }
                        synchronized(observations) {
                            if (requests.containsKey(requestId)) {
                                observations[requestId] = Observation(revision, complete, timeout)
                                handler.postDelayed(timeout, 25_000)
                            }
                        }
                        publishObservations()
                    }
                }
            } catch (error: Exception) {
                complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Could not read background state", false))
            }
            return
        }
        if (operation == "enqueue-task" || operation == "cancel-task") {
            try {
                val request = JSONObject(payload)
                if (operation == "enqueue-task") InkWorkerJobs.enqueue(context, request)
                else InkWorkerJobs.cancel(context, request.getString("key"))
                complete(NativeResult.Success(""))
            } catch (error: Exception) {
                complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Background scheduling failed", false))
            }
            return
        }
    }

    override fun cancel(requestId: Long) {
        synchronized(observations) {
            requests.remove(requestId)?.cancel(true)
            observations.remove(requestId)?.let { handler.removeCallbacks(it.timeout) }
        }
    }

    override fun stop() {
        InkWorkerState.unsubscribe(changed)
        synchronized(observations) {
            requests.values.forEach { it.cancel(true) }
            requests.clear()
            observations.values.forEach { handler.removeCallbacks(it.timeout) }
            observations.clear()
        }
        executor.shutdownNow()
    }
}
