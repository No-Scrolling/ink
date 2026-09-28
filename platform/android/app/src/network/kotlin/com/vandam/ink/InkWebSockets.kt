package com.vandam.ink

import android.os.Handler
import android.net.Uri
import android.util.Base64
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString
import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID
import java.util.concurrent.TimeUnit

internal class InkWebSockets(private val handler: Handler) {
    private val client = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .pingInterval(20, TimeUnit.SECONDS)
        .build()
    private val sockets = mutableMapOf<String, Socket>()
    private val pending = mutableMapOf<Long, Socket>()

    @Synchronized fun execute(id: Long, operation: String, payload: String, complete: NativeResultHandler, inputBytes: ByteArray? = null) {
        try {
            val data = JSONObject(payload)
            if (operation == "socket-open") {
                require(sockets.size < 8) { "Too many open WebSockets" }
                val url = data.getString("url")
                require(url.startsWith("wss://") || (BuildConfig.DEBUG || BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED) && url.startsWith("ws://") && Uri.parse(url).host in setOf("localhost", "127.0.0.1", "::1")) { "WebSockets require WSS" }
                val request = Request.Builder().url(url)
                val protocols = data.getJSONArray("protocols")
                if (protocols.length() > 0) request.header("Sec-WebSocket-Protocol", (0 until protocols.length()).joinToString(", ") { protocols.getString(it) })
                val key = UUID.randomUUID().toString()
                val socket = Socket(key)
                sockets[key] = socket
                socket.connection = client.newWebSocket(request.build(), socket)
                complete(NativeResult.Success(key))
                return
            }
            val key = data.getString("socket")
            if (operation == "socket-dispose") {
                sockets.remove(key)?.dispose()
                complete(NativeResult.Success("null"))
                return
            }
            val socket = requireNotNull(sockets[key]) { "WebSocket is closed" }
            when (operation) {
                "socket-read", "socket-read-bytes" -> {
                    socket.binaryReader = operation.endsWith("-bytes")
                    require(socket.reader == null) { "WebSocket already has a pending read" }
                    socket.reader = complete
                    socket.readerId = id
                    pending[id] = socket
                    if (socket.events.isNotEmpty()) socket.deliver()
                    else handler.postDelayed(socket.timeout, 25_000)
                }
                "socket-send" -> {
                    val bytes = inputBytes ?: Base64.decode(data.getString("bytes"), Base64.DEFAULT)
                    require(bytes.size <= 256 * 1024) { "WebSocket message exceeds 256 KiB" }
                    val connection = requireNotNull(socket.connection)
                    require(connection.queueSize() + bytes.size <= 512 * 1024) { "WebSocket send queue is full" }
                    val accepted = if (data.getBoolean("text")) connection.send(bytes.toString(Charsets.UTF_8)) else connection.send(bytes.toByteString())
                    require(accepted) { "WebSocket is closing" }
                    complete(NativeResult.Success(connection.queueSize().toString()))
                }
                "socket-close" -> {
                    socket.connection?.close(data.optInt("code", 1000), data.optString("reason"))
                    handler.postDelayed(socket.closeTimeout, 30_000)
                    complete(NativeResult.Success("null"))
                }
                else -> throw IllegalArgumentException("Unknown WebSocket operation")
            }
        } catch (error: Exception) { complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, error.message ?: "Invalid WebSocket request", false)) }
    }
    @Synchronized fun cancel(id: Long) { pending.remove(id)?.let { sockets.remove(it.key); it.dispose() } }
    @Synchronized fun stop() {
        sockets.values.toList().forEach(Socket::dispose)
        sockets.clear()
        client.dispatcher.cancelAll()
        client.dispatcher.executorService.shutdown()
        client.connectionPool.evictAll()
    }
    private data class SocketEvent(val value: JSONObject, val bytes: ByteArray?)
    private inner class Socket(val key: String) : WebSocketListener() {
        var connection: WebSocket? = null
        val events = ArrayDeque<SocketEvent>()
        var binaryReader = false
        var reader: NativeResultHandler? = null
        var readerId: Long? = null
        private var queuedBytes = 0
        private var terminal = false
        val timeout = Runnable { synchronized(this@InkWebSockets) { deliver() } }
        val closeTimeout = Runnable { synchronized(this@InkWebSockets) { fail("WebSocket close handshake timed out") } }
        fun dispose() {
            terminal = true
            connection?.cancel()
            handler.removeCallbacks(timeout)
            handler.removeCallbacks(closeTimeout)
            readerId?.let(pending::remove)
            reader = null
            readerId = null
        }
        fun deliver() {
            val callback = reader ?: return
            handler.removeCallbacks(timeout)
            readerId?.let(pending::remove)
            readerId = null
            reader = null
            val batch = JSONArray()
            val bytes = java.io.ByteArrayOutputStream()
            while (events.isNotEmpty()) {
                val event = events.removeFirst()
                if (event.bytes != null) {
                    if (binaryReader) {
                        event.value.put("offset", bytes.size()).put("length", event.bytes.size)
                        bytes.write(event.bytes)
                    } else event.value.put("bytes", Base64.encodeToString(event.bytes, Base64.NO_WRAP))
                }
                batch.put(event.value)
            }
            queuedBytes = 0
            val value = JSONObject().put("events", batch).put("bufferedAmount", connection?.queueSize() ?: 0).toString()
            callback(if (binaryReader) NativeResult.Binary(value, bytes.toByteArray()) else NativeResult.Success(value))
        }
        private fun emit(event: JSONObject, bytes: ByteArray? = null) {
            if (terminal) return
            val size = event.toString().toByteArray().size + (bytes?.size ?: 0)
            if (queuedBytes + size > 512 * 1024 || events.size >= 64) { fail("WebSocket receive queue is full"); return }
            events.add(SocketEvent(event, bytes))
            queuedBytes += size
            deliver()
        }
        private fun fail(message: String) {
            if (terminal) return
            events.clear()
            events.add(SocketEvent(JSONObject().put("type", "error").put("message", message), null))
            events.add(SocketEvent(JSONObject().put("type", "close").put("code", 1006).put("reason", "").put("wasClean", false), null))
            terminal = true
            connection?.cancel()
            handler.removeCallbacks(closeTimeout)
            deliver()
        }
        override fun onOpen(webSocket: WebSocket, response: Response) {
            synchronized(this@InkWebSockets) { emit(JSONObject().put("type", "open").put("protocol", response.header("Sec-WebSocket-Protocol") ?: "")) }
        }
        override fun onMessage(webSocket: WebSocket, text: String) {
            synchronized(this@InkWebSockets) {
                if (text.toByteArray().size > 256 * 1024) fail("WebSocket message exceeds 256 KiB")
                else emit(JSONObject().put("type", "message").put("text", text))
            }
        }
        override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
            synchronized(this@InkWebSockets) {
                if (bytes.size > 256 * 1024) fail("WebSocket message exceeds 256 KiB")
                else emit(JSONObject().put("type", "message"), bytes.toByteArray())
            }
        }
        override fun onClosing(webSocket: WebSocket, code: Int, reason: String) { webSocket.close(code, reason) }
        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            synchronized(this@InkWebSockets) {
                emit(JSONObject().put("type", "close").put("code", code).put("reason", reason).put("wasClean", true))
                terminal = true
                handler.removeCallbacks(closeTimeout)
            }
        }
        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            synchronized(this@InkWebSockets) { fail(t.message ?: "WebSocket connection failed") }
        }
    }
}
