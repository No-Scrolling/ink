package com.vandam.ink

import android.os.Handler

internal class NativeRequests(
    private val handler: Handler,
    private val reply: (Long, NativeResult) -> Unit,
) : AutoCloseable {
    private class Request(val cancel: () -> Unit, val timeout: Runnable)
    private val pending = mutableMapOf<Long, Request>()
    private var closed = false

    fun execute(id: Long, timeoutMs: Long, cancel: () -> Unit, start: (NativeResultHandler) -> Unit) {
        lateinit var request: Request
        val timeout = Runnable {
            if (remove(id, request)) {
                cancel()
                reply(id, NativeResult.Failure(NativeErrorKind.TIMEOUT, "Native request timed out", true))
            }
        }
        request = Request(cancel, timeout)
        val failure = synchronized(this) {
            if (closed) return
            when {
                id <= 0 || pending.containsKey(id) ->
                    NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid native request ID", false)
                pending.size >= 256 ->
                    NativeResult.Failure(NativeErrorKind.BUSY, "Too many pending native requests", true)
                else -> {
                    pending[id] = request
                    handler.postDelayed(timeout, timeoutMs.coerceIn(1, 2_147_483_647))
                    null
                }
            }
        }
        if (failure != null) { reply(id, failure); return }
        val complete: NativeResultHandler = { result ->
            if (remove(id, request)) reply(id, result) else disposeNativeResult(result)
        }
        // Adapters can complete on a worker while holding their own lock.
        try { start(complete) } catch (error: Exception) {
            complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Native request failed", false))
        }
    }

    @Synchronized
    private fun remove(id: Long, request: Request): Boolean {
        if (pending[id] !== request) return false
        pending.remove(id)
        handler.removeCallbacks(request.timeout)
        return true
    }

    fun cancel(id: Long) {
        val request = synchronized(this) { pending.remove(id) } ?: return
        handler.removeCallbacks(request.timeout)
        request.cancel()
    }

    override fun close() {
        val requests = synchronized(this) {
            if (closed) return
            closed = true
            pending.values.toList().also { pending.clear() }
        }
        requests.forEach { handler.removeCallbacks(it.timeout); it.cancel() }
    }
}

internal fun disposeNativeResult(result: NativeResult) {
    if (result is NativeResult.File && result.deleteAfterRead) java.io.File(result.path).delete()
    if (result is NativeResult.Success) result.dispose?.invoke()
}
