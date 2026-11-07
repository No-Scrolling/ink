package com.vandam.ink

import android.os.Handler

internal class InkWebSockets(handler: Handler) {
    fun execute(id: Long, operation: String, payload: String, complete: NativeResultHandler, inputBytes: ByteArray? = null) {
        complete(NativeResult.Failure(NativeErrorKind.PROTOCOL,
            "WebSockets are not packaged. Add capabilities = [\"web-sockets\"] to ink.toml for dynamically resolved WebSocket code.", false))
    }
    fun cancel(id: Long) {}
    fun reset() {}
    fun stop() {}
}
