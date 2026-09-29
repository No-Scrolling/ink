package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.net.http.HttpEngine
import android.net.http.HttpException
import android.net.http.UploadDataProvider
import android.net.http.UploadDataSink
import android.net.http.UrlRequest
import android.net.http.UrlResponseInfo
import android.os.Handler
import android.os.SystemClock
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.io.InputStream
import java.io.RandomAccessFile
import java.nio.ByteBuffer
import java.util.UUID
import java.util.concurrent.Executor
import java.util.concurrent.atomic.AtomicBoolean

internal class InkFetchStreams(
    private val context: Context,
    private val engine: HttpEngine,
    private val executor: Executor,
    private val handler: Handler,
) {
    private val streams = mutableMapOf<String, Stream>()
    private data class Upload(val file: File, val managed: Boolean, val copying: AtomicBoolean = AtomicBoolean(false), val closed: AtomicBoolean = AtomicBoolean(false), var touched: Long = SystemClock.uptimeMillis())
    private val uploads = mutableMapOf<String, Upload>()
    private val pending = mutableMapOf<Long, Stream>()
    private val copies = mutableMapOf<Long, AtomicBoolean>()
    private val expiry = Runnable { expire() }
    private var expiryScheduled = false

    @Synchronized fun execute(id: Long, operation: String, payload: String, complete: NativeResultHandler, inputBytes: ByteArray? = null) {
        try {
            val data = JSONObject(payload)
            when (operation) {
                "stream-file-read", "stream-file-read-bytes" -> {
                    val source = InkManagedFiles(context).resolve(data.getString("src"))
                    val offset = data.getLong("offset")
                    val size = data.getInt("size")
                    require(offset >= 0 && size in 0..32768 && offset <= source.length() - size) { "Invalid attachment range" }
                    fileWork(id, null, complete) {
                        val bytes = ByteArray(size)
                        RandomAccessFile(source, "r").use { it.seek(offset); it.readFully(bytes) }
                        if (operation.endsWith("-bytes")) NativeResult.Binary("", bytes) else NativeResult.Success(Base64.encodeToString(bytes, Base64.NO_WRAP))
                    }
                }
                "stream-upload-open" -> {
                    require(uploads.size < 8) { "Too many pending uploads" }
                    val key = UUID.randomUUID().toString()
                    uploads[key] = Upload(File.createTempFile("ink-upload-", ".body", context.cacheDir), data.optBoolean("managed"))
                    complete(NativeResult.Success(key) {
                        synchronized(this@InkFetchStreams) {
                            uploads.remove(key)?.let { it.closed.set(true); it.file.delete() }
                        }
                    })
                    scheduleExpiry()
                }
                "stream-upload-write" -> {
                    val upload = requireNotNull(uploads[data.getString("upload")]) { "Upload is closed" }
                    require(!upload.copying.get()) { "Upload already has a pending copy" }
                    val file = upload.file
                    val bytes = inputBytes ?: Base64.decode(data.getString("bytes"), Base64.DEFAULT)
                    require(bytes.size <= 32768 && (upload.managed || file.length() + bytes.size <= 64L * 1024 * 1024)) { "Upload exceeds 64 MiB" }
                    require(upload.copying.compareAndSet(false, true)) { "Upload already has pending file work" }
                    fileWork(id, upload, complete) {
                        file.appendBytes(bytes)
                        NativeResult.Success("null")
                    }
                }
                "stream-upload-file" -> {
                    val upload = requireNotNull(uploads[data.getString("upload")]) { "Upload is closed" }
                    require(upload.managed) { "Upload does not accept managed files" }
                    val source = InkManagedFiles(context).resolve(data.getString("src"))
                    val offset = data.getLong("offset")
                    val size = data.getLong("size")
                    require(offset >= 0 && size >= 0 && offset <= source.length() - size) { "Invalid attachment range" }
                    require(upload.copying.compareAndSet(false, true)) { "Upload already has a pending copy" }
                    fileWork(id, upload, complete) { cancelled ->
                        RandomAccessFile(source, "r").use { input ->
                            input.seek(offset)
                            FileOutputStream(upload.file, true).use { output ->
                                val buffer = ByteArray(64 * 1024)
                                var remaining = size
                                while (remaining > 0) {
                                    check(!cancelled.get() && !upload.closed.get()) { "Attachment copy cancelled" }
                                    val count = input.read(buffer, 0, minOf(buffer.size.toLong(), remaining).toInt())
                                    check(count > 0) { "Attachment ended before its requested range" }
                                    output.write(buffer, 0, count)
                                    remaining -= count
                                }
                            }
                        }
                        NativeResult.Success("null")
                    }
                }
                "stream-upload-close" -> {
                    uploads.remove(data.getString("upload"))?.let { it.closed.set(true); it.file.delete() }
                    complete(NativeResult.Success("null"))
                }
                "stream-open" -> open(id, data, complete)
                "stream-read", "stream-read-bytes" -> requireNotNull(streams[data.getString("stream")]) { "Response stream is closed" }.read(id, operation.endsWith("-bytes"), complete)
                "stream-close" -> {
                    streams.remove(data.getString("stream"))?.close()
                    complete(NativeResult.Success("null"))
                }
                else -> throw IllegalArgumentException("Unknown streaming network operation")
            }
        } catch (error: Exception) { complete(failure(error.message ?: "Invalid streaming request")) }
    }

    private fun fileWork(id: Long, upload: Upload?, complete: NativeResultHandler, operation: (AtomicBoolean) -> NativeResult) {
        val cancelled = AtomicBoolean(false)
        copies[id] = cancelled
        try { executor.execute {
            val result = try {
                check(!cancelled.get() && upload?.closed?.get() != true) { "File operation cancelled" }
                operation(cancelled)
            } catch (error: Exception) { failure(error.message ?: "File operation failed") }
            handler.post {
                synchronized(this@InkFetchStreams) {
                    if (copies[id] === cancelled) copies.remove(id)
                    upload?.copying?.set(false)
                    upload?.touched = SystemClock.uptimeMillis()
                    scheduleExpiry()
                }
                if (!cancelled.get() && upload?.closed?.get() != true) complete(result)
                else disposeNativeResult(result)
            }
        } } catch (error: Exception) {
            copies.remove(id)
            upload?.copying?.set(false)
            throw error
        }
    }

    private fun open(id: Long, data: JSONObject, complete: NativeResultHandler) {
        require(streams.size < 16) { "Too many open response streams" }
        val url = data.getString("url")
        val upload = data.optString("upload").takeIf { it.isNotEmpty() }?.let {
            val upload = requireNotNull(uploads[it]) { "Upload is closed" }
            require(!upload.copying.get()) { "Upload already has a pending copy" }
            uploads.remove(it)
            upload.file
        }
        val stream = Stream(UUID.randomUUID().toString(), upload)
        streams[stream.key] = stream
        pending[id] = stream
        stream.readerId = id
        stream.reader = complete
        try {
            if (url.startsWith("file://")) {
                require(data.optString("method", "GET") == "GET" && upload == null) { "Local files are read-only" }
                val root = File(context.noBackupFilesDir, "ink-camera").canonicalFile
                val file = File(requireNotNull(Uri.parse(url).path)).canonicalFile
                require(file.path.startsWith(root.path + File.separator) && file.isFile) { "File is not an Ink camera photo" }
                stream.fileInput = file.inputStream()
                stream.deliver(NativeResult.Success(JSONObject().put("status", 200).put("statusText", "OK")
                    .put("url", url).put("headers", JSONArray().put(JSONArray().put("content-type").put("image/jpeg")))
                    .put("stream", stream.key).toString()))
            } else {
                require(url.startsWith("https://") || url.startsWith("http://") &&
                    (BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED || BuildConfig.DEBUG &&
                        Uri.parse(url).host in setOf("localhost", "127.0.0.1", "::1", "10.0.2.2"))) {
                    "HTTP requests require the network-cleartext capability"
                }
                val method = data.optString("method", "GET")
                require(Regex("[!#$%&'*+.^_`|~0-9A-Za-z-]+").matches(method) && method.uppercase() !in setOf("CONNECT", "TRACE", "TRACK"))
                val builder = engine.newUrlRequestBuilder(url, executor, stream).setHttpMethod(method)
                val headers = data.getJSONObject("headers")
                headers.keys().forEach { name ->
                    require(name.lowercase() !in setOf("host", "connection", "content-length", "transfer-encoding", "upgrade", "accept-encoding", "proxy-authorization", "te"))
                    builder.addHeader(name, headers.getString(name))
                }
                if (upload != null) {
                    require(method.uppercase() !in setOf("GET", "HEAD"))
                    if (!headers.has("content-type")) builder.addHeader("Content-Type", "application/octet-stream")
                    builder.setUploadDataProvider(FileUpload(upload), executor)
                }
                stream.request = builder.build()
                stream.request?.start()
            }
            scheduleExpiry()
        } catch (error: Exception) {
            streams.remove(stream.key)
            stream.close()
            throw error
        }
    }

    @Synchronized fun cancel(id: Long) {
        copies[id]?.set(true)
        pending.remove(id)?.let { streams.remove(it.key); it.close() }
    }
    @Synchronized fun stop() {
        streams.values.toList().forEach { it.close() }
        streams.clear()
        copies.values.forEach { it.set(true) }
        copies.clear()
        uploads.values.forEach { it.closed.set(true); it.file.delete() }
        uploads.clear()
        handler.removeCallbacks(expiry)
        expiryScheduled = false
    }
    private fun scheduleExpiry() {
        if (expiryScheduled) return
        val touched = (streams.values.map { it.touched } + uploads.values.filter { !it.copying.get() }.map { it.touched }).minOrNull() ?: return
        expiryScheduled = true
        handler.postDelayed(expiry, (touched + 60_000 - SystemClock.uptimeMillis()).coerceAtLeast(0))
    }
    @Synchronized private fun expire() {
        expiryScheduled = false
        val before = SystemClock.uptimeMillis() - 60_000
        streams.values.filter { it.touched <= before }.toList().forEach {
            streams.remove(it.key)
            it.deliver(failure("Response stream was idle for 60 seconds"))
            it.close()
        }
        uploads.entries.removeAll { (_, upload) -> if (!upload.copying.get() && upload.touched <= before) { upload.closed.set(true); upload.file.delete(); true } else false }
        if (streams.isNotEmpty() || uploads.isNotEmpty()) scheduleExpiry()
    }

    private inner class Stream(val key: String, private val upload: File?) : UrlRequest.Callback {
        var request: UrlRequest? = null
        var fileInput: InputStream? = null
        var readerId: Long? = null
        var reader: NativeResultHandler? = null
        var binaryReader = false
        var touched = SystemClock.uptimeMillis()
        private val buffer = ByteBuffer.allocateDirect(32768)
        private var terminal: NativeResult? = null
        private var closed = false

        fun read(id: Long, binary: Boolean, complete: NativeResultHandler) {
            require(reader == null) { "Response already has a pending read" }
            touched = SystemClock.uptimeMillis()
            readerId = id
            reader = complete
            binaryReader = binary
            pending[id] = this
            terminal?.let { deliver(it); return }
            val input = fileInput
            if (input != null) {
                executor.execute {
                    synchronized(this@InkFetchStreams) {
                        if (closed) return@execute
                        try {
                            val bytes = ByteArray(32768)
                            val count = input.read(bytes)
                            deliver(chunk(if (count < 0) byteArrayOf() else bytes.copyOf(count), count < 0))
                        } catch (error: Exception) { deliver(failure(error.message ?: "File read failed")); close() }
                    }
                }
            } else request?.read(buffer)
        }
        fun deliver(result: NativeResult) {
            val delivered = if (result is NativeResult.Binary && !binaryReader)
                NativeResult.Success(JSONObject(result.value).put("bytes", Base64.encodeToString(result.bytes, Base64.NO_WRAP)).toString()) else result
            val callback = reader
            val id = readerId
            readerId = null
            reader = null
            // The bridge can hold its own lock while calling into this adapter.
            // Deliver outside the stream lock to avoid reversing that lock order.
            if (callback != null && id != null) handler.post {
                synchronized(this@InkFetchStreams) { pending.remove(id, this@Stream) }
                callback(delivered)
            }
        }
        fun close() {
            closed = true
            request?.cancel()
            fileInput?.close()
            upload?.delete()
            pending.entries.removeAll { it.value === this }
            readerId = null
            reader = null
        }
        override fun onRedirectReceived(request: UrlRequest, info: UrlResponseInfo, newLocationUrl: String) {
            synchronized(this@InkFetchStreams) { terminal = chunk(byteArrayOf(), true); headers(info); request.cancel() }
        }
        override fun onResponseStarted(request: UrlRequest, info: UrlResponseInfo) {
            synchronized(this@InkFetchStreams) { if (!closed) headers(info) }
        }
        private fun headers(info: UrlResponseInfo) {
            val headers = JSONArray()
            info.headers.asList.forEach { headers.put(JSONArray().put(it.key).put(it.value)) }
            deliver(NativeResult.Success(JSONObject().put("status", info.httpStatusCode).put("statusText", info.httpStatusText)
                .put("url", info.url).put("headers", headers).put("stream", key).toString()))
        }
        override fun onReadCompleted(request: UrlRequest, info: UrlResponseInfo, byteBuffer: ByteBuffer) {
            synchronized(this@InkFetchStreams) {
                if (closed) return
                byteBuffer.flip()
                val bytes = ByteArray(byteBuffer.remaining())
                byteBuffer.get(bytes)
                byteBuffer.clear()
                deliver(chunk(bytes, false))
            }
        }
        override fun onSucceeded(request: UrlRequest, info: UrlResponseInfo) {
            synchronized(this@InkFetchStreams) { terminal = chunk(byteArrayOf(), true); deliver(requireNotNull(terminal)); upload?.delete() }
        }
        override fun onFailed(request: UrlRequest, info: UrlResponseInfo?, error: HttpException) {
            synchronized(this@InkFetchStreams) { terminal = failure(error.message ?: "Network request failed"); deliver(requireNotNull(terminal)); upload?.delete() }
        }
        override fun onCanceled(request: UrlRequest, info: UrlResponseInfo?) {}
    }
    private fun chunk(bytes: ByteArray, done: Boolean) = NativeResult.Binary(JSONObject().put("done", done).toString(), bytes)
    private fun failure(message: String) = NativeResult.Failure(NativeErrorKind.UNAVAILABLE, message, false)
}

private class FileUpload(file: File) : UploadDataProvider() {
    private val input = RandomAccessFile(file, "r")
    override fun getLength() = input.length()
    override fun read(sink: UploadDataSink, buffer: ByteBuffer) {
        val bytes = ByteArray(minOf(buffer.remaining(), 32768))
        val count = input.read(bytes)
        if (count > 0) buffer.put(bytes, 0, count)
        sink.onReadSucceeded(false)
    }
    override fun rewind(sink: UploadDataSink) { input.seek(0); sink.onRewindSucceeded() }
    override fun close() { input.close() }
}
