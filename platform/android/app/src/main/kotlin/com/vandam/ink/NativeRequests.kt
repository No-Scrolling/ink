package com.vandam.ink

import android.os.Handler

internal class NativeRequests(
    private val handler: Handler,
    private val policy: Policy = Policy.JAVASCRIPT,
    private val reply: ((Long, NativeResult) -> Unit)? = null,
) : AutoCloseable {
    enum class Policy { JAVASCRIPT, SCENE }

    private class Request(val cancel: () -> Unit, val cancelled: () -> Unit, val timeout: Runnable)
    private val pending = mutableMapOf<Long, Request>()
    private var closed = false

    fun execute(
        id: Long,
        timeoutMs: Long,
        cancel: () -> Unit,
        reply: NativeResultHandler = { checkNotNull(this.reply)(id, it) },
        cancelled: () -> Unit = {},
        start: (NativeResultHandler) -> Unit,
    ) {
        lateinit var request: Request
        val timeout = Runnable {
            if (remove(id, request)) {
                cancel()
                reply(NativeResult.Failure(NativeErrorKind.TIMEOUT, "Native request timed out", true))
            }
        }
        request = Request(cancel, cancelled, timeout)
        val failure = synchronized(this) {
            if (closed) return
            when {
                (policy == Policy.JAVASCRIPT && id <= 0) || pending.containsKey(id) ->
                    NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid native request ID", false)
                policy == Policy.JAVASCRIPT && pending.size >= 256 ->
                    NativeResult.Failure(NativeErrorKind.BUSY, "Too many pending native requests", true)
                else -> {
                    pending[id] = request
                    // Scene deadlines come from the native engine, without JavaScript admission limits.
                    handler.postDelayed(timeout, if (policy == Policy.JAVASCRIPT) timeoutMs.coerceIn(1, 2_147_483_647) else timeoutMs)
                    null
                }
            }
        }
        if (failure != null) { reply(failure); return }
        val complete: NativeResultHandler = { result ->
            if (remove(id, request)) reply(result) else disposeNativeResult(result)
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

    fun cancel(id: Long): Boolean {
        val request = synchronized(this) { pending.remove(id) } ?: return false
        handler.removeCallbacks(request.timeout)
        request.cancelled()
        request.cancel()
        return true
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
