package com.vandam.ink

import android.content.Context
import android.net.http.HttpEngine
import android.os.Handler
import android.os.HandlerThread
import org.json.JSONObject
import java.io.File
import java.net.ServerSocket
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executor
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

internal class ContractTests(private val context: Context) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("store persists opaque values and rejects stale writes", ::storePersistence)
        test("store concurrent CAS commits exactly one writer", ::storeConcurrency)
        test("store notifies other live adapters only after committed writes", ::storeNotifications)
        test("store rejects downgraded versions and oversized values without mutation", ::storeValidation)
        test("native requests cancel and dispose late owned resources", ::requestCancellation)
        test("native requests timeout, close and suppress duplicate completion", ::requestLifetime)
        test("native requests enforce capacity and request identity", ::requestIdentity)
        test("native result JSON escaping and UTF-8 message limits", ::resultEncoding)
        test("secure store uses Android Keystore and survives reopening", ::securePersistence)
        test("secure store authenticates ciphertext and credential names", ::secureAuthentication)
        test("managed files hide paths and reject traversal and outside storage", ::managedFiles)
        test("streams read managed ranges and local camera bytes", ::fileStreams)
        test("streams cancel queued file work and close upload ownership", ::streamCancellation)
        test("streams send a real HTTP upload and receive response bytes", ::httpStreams)
        test("idle streams and uploads expire after 60 seconds", ::streamExpiry)
    }

    private fun equal(expected: Any?, actual: Any?) {
        check(expected == actual) { "Expected <$expected>, got <$actual>" }
    }
    private fun take(queue: LinkedBlockingQueue<NativeResult>): NativeResult =
        checkNotNull(queue.poll(10, TimeUnit.SECONDS)) { "Native completion did not arrive within 10 seconds" }
    private fun value(result: NativeResult): String = (result as? NativeResult.Success)?.value
        ?: error("Expected success, got $result")
    private fun failure(result: NativeResult, kind: NativeErrorKind? = null): NativeResult.Failure {
        check(result is NativeResult.Failure) { "Expected failure, got $result" }
        if (kind != null) equal(kind, result.kind)
        return result
    }
    private fun call(adapter: NativeAdapter, operation: String, input: JSONObject): NativeResult {
        val results = LinkedBlockingQueue<NativeResult>()
        adapter.execute(1, operation, input.toString()) { results.add(it) }
        return take(results)
    }
    private fun input(key: String, revision: Long = 0, version: Long = 1, text: String = "initial") =
        JSONObject().put("key", key).put("revision", revision).put("version", version).put("value", text)
    private fun entry(adapter: StoreAdapter, key: String) = JSONObject(value(call(adapter, "get", input(key))))
    private fun committed(result: NativeResult) = JSONObject(value(result)).getBoolean("committed")

    private fun storePersistence() {
        val key = "persistence"
        val stored = "{\"text\":\"hello 😀\\n\",\"count\":9}"
        val first = StoreAdapter(context)
        try {
            equal("null", value(call(first, "get", input(key))))
            equal(true, committed(call(first, "write", input(key, text = stored))))
            equal(false, committed(call(first, "write", input(key, text = "lost"))))
        } finally { first.stop() }
        val reopened = StoreAdapter(context)
        try {
            val saved = entry(reopened, key)
            equal(1L, saved.getLong("revision")); equal(1L, saved.getLong("version")); equal(stored, saved.getString("value"))
            equal(true, committed(call(reopened, "replace", input(key, revision = 999, text = "replacement"))))
            equal(2L, entry(reopened, key).getLong("revision"))
            equal("replacement", entry(reopened, key).getString("value"))
        } finally { reopened.stop() }
    }

    private fun storeConcurrency() {
        val stores = List(12) { StoreAdapter(context) }
        val results = LinkedBlockingQueue<NativeResult>()
        try {
            stores.forEachIndexed { index, store ->
                store.execute(index + 1L, "write", input("contested", text = "writer-$index").toString()) { results.add(it) }
            }
            equal(1, List(stores.size) { committed(take(results)) }.count { it })
            val saved = entry(stores.first(), "contested")
            equal(1L, saved.getLong("revision"))
            check(saved.getString("value") in (0..11).map { "writer-$it" })
        } finally { stores.forEach { it.stop() } }
    }

    private fun storeNotifications() {
        val writerEvents = LinkedBlockingQueue<String>()
        val siblingEvents = LinkedBlockingQueue<String>()
        val stoppedEvents = LinkedBlockingQueue<String>()
        val writer = StoreAdapter(context) { writerEvents.add(it) }
        val sibling = StoreAdapter(context) { siblingEvents.add(it) }
        val stopped = StoreAdapter(context) { stoppedEvents.add(it) }
        stopped.stop()
        try {
            equal(true, committed(call(writer, "write", input("notification"))))
            equal("notification", siblingEvents.poll(5, TimeUnit.SECONDS))
            equal("initial", entry(sibling, "notification").getString("value"))
            equal(false, committed(call(writer, "write", input("notification"))))
            equal(null, siblingEvents.poll(100, TimeUnit.MILLISECONDS))
            equal(null, writerEvents.poll()); equal(null, stoppedEvents.poll())
        } finally { writer.stop(); sibling.stop() }
    }

    private fun storeValidation() {
        val store = StoreAdapter(context)
        try {
            equal(true, committed(call(store, "write", input("validated", version = 2, text = "stable"))))
            failure(call(store, "replace", input("validated", version = 1, text = "downgrade")))
            failure(call(store, "replace", input("validated", version = 2, text = "😀".repeat(300_000))))
            val saved = entry(store, "validated")
            equal(1L, saved.getLong("revision")); equal(2L, saved.getLong("version")); equal("stable", saved.getString("value"))
            for (key in listOf("", "x".repeat(257))) failure(call(store, "write", input(key)))
            for (version in listOf(0L, 9_007_199_254_740_992L)) failure(call(store, "write", input("version-$version", version = version)))
            failure(call(store, "unknown", input("unknown")), NativeErrorKind.PROTOCOL)
        } finally { store.stop() }
    }

    private fun withHandler(operation: (Handler) -> Unit) {
        val thread = HandlerThread("ink-contract-handler").apply { start() }
        try { operation(Handler(thread.looper)) }
        finally { thread.quitSafely(); thread.join(5000); check(!thread.isAlive) }
    }
    private fun drain(handler: Handler) {
        val done = CountDownLatch(1)
        check(handler.post { done.countDown() })
        check(done.await(5, TimeUnit.SECONDS)) { "Handler did not drain" }
    }

    private fun requestCancellation() = withHandler { handler ->
        val replies = LinkedBlockingQueue<NativeResult>()
        val cancels = AtomicInteger()
        val disposed = AtomicInteger()
        val requests = NativeRequests(handler) { _, result -> replies.add(result) }
        lateinit var complete: NativeResultHandler
        try {
            requests.execute(1, 1000, { cancels.incrementAndGet() }) { complete = it }
            requests.cancel(1); requests.cancel(1)
            complete(NativeResult.Success("owned", { disposed.incrementAndGet() }))
            equal(1, cancels.get()); equal(1, disposed.get()); equal(null, replies.poll())
            val temporary = File.createTempFile("late-result", ".body", context.cacheDir)
            val durable = File.createTempFile("retained-result", ".body", context.cacheDir)
            try {
                complete(NativeResult.File(temporary.path)); complete(NativeResult.File(durable.path, false))
                check(!temporary.exists()); check(durable.exists())
            } finally { temporary.delete(); durable.delete() }
        } finally { requests.close() }
    }

    private fun requestLifetime() = withHandler { handler ->
        val replies = LinkedBlockingQueue<NativeResult>()
        val cancels = AtomicInteger()
        val disposed = AtomicInteger()
        val requests = NativeRequests(handler) { _, result -> replies.add(result) }
        lateinit var timedOut: NativeResultHandler
        lateinit var closed: NativeResultHandler
        requests.execute(1, 25, { cancels.incrementAndGet() }) { timedOut = it }
        equal(true, failure(take(replies), NativeErrorKind.TIMEOUT).retryable)
        timedOut(NativeResult.Success("late", { disposed.incrementAndGet() }))
        requests.execute(2, 1000, { cancels.incrementAndGet() }) { complete ->
            complete(NativeResult.Success("first"))
            complete(NativeResult.Success("duplicate", { disposed.incrementAndGet() }))
        }
        equal("first", value(take(replies)))
        requests.execute(3, 1000, { cancels.incrementAndGet() }) { closed = it }
        requests.close(); requests.close()
        closed(NativeResult.Success("after close", { disposed.incrementAndGet() }))
        var startedAfterClose = false
        requests.execute(4, 1, {}) { startedAfterClose = true }
        drain(handler)
        equal(2, cancels.get()); equal(3, disposed.get()); equal(false, startedAfterClose); equal(null, replies.poll())
    }

    private fun requestIdentity() = withHandler { handler ->
        val replies = LinkedBlockingQueue<NativeResult>()
        val cancels = AtomicInteger()
        val requests = NativeRequests(handler) { _, result -> replies.add(result) }
        val callbacks = mutableMapOf<Long, NativeResultHandler>()
        try {
            requests.execute(0, 1000, {}) { error("Invalid ID reached adapter") }
            failure(take(replies), NativeErrorKind.PROTOCOL)
            for (id in 1L..256L) requests.execute(id, 60_000, { cancels.incrementAndGet() }) { callbacks[id] = it }
            requests.execute(1, 1000, {}) { error("Duplicate reached adapter") }
            failure(take(replies), NativeErrorKind.PROTOCOL)
            requests.execute(257, 1000, {}) { error("Overflow reached adapter") }
            equal(true, failure(take(replies), NativeErrorKind.BUSY).retryable)
            requests.cancel(1)
            requests.execute(1, 60_000, { cancels.incrementAndGet() }) { current ->
                callbacks.getValue(1)(NativeResult.Success("old"))
                current(NativeResult.Success("new"))
            }
            equal("new", value(take(replies))); equal(null, replies.poll())
            requests.cancel(2)
            requests.execute(258, 1000, {}) { throw IllegalStateException("adapter exploded") }
            equal("adapter exploded", failure(take(replies), NativeErrorKind.UNEXPECTED).message)
        } finally { requests.close() }
        equal(256, cancels.get())
    }

    private fun resultEncoding() {
        val text = "quote\" slash\\ newline\n emoji😀"
        val encoded = JSONObject(javascriptResult(42, NativeResult.Success(text)))
        equal("result", encoded.getString("type")); equal(42L, encoded.getLong("id")); equal(text, encoded.getString("value"))
        val byteEncoded = JSONObject(javascriptResult(43, NativeResult.Bytes("café 😀".toByteArray())))
        equal("café 😀", byteEncoded.getString("value"))
        val overhead = "{\"type\":\"result\",\"id\":9007199254740991,\"value\":\"\"}".toByteArray().size
        equal(true, javascriptResultFits("a".repeat(1024 * 1024 - overhead)))
        equal(false, javascriptResultFits("a".repeat(1024 * 1024 - overhead + 1)))
        equal(false, javascriptResultFits("😀".repeat(300_000)))
        val disposed = AtomicInteger()
        val oversized = JSONObject(javascriptResult(7, NativeResult.Success("😀".repeat(300_000), { disposed.incrementAndGet() })))
        equal("protocol", oversized.getString("kind")); equal(7L, oversized.getLong("id")); equal(1, disposed.get())
        val temporary = File.createTempFile("invalid-file-result", ".body", context.cacheDir)
        equal("protocol", JSONObject(javascriptResult(8, NativeResult.File(temporary.path))).getString("kind")); check(!temporary.exists())
    }

    private fun securePersistence() {
        val adapter = SecureStoreAdapter(context)
        val key = "account.refresh-token"
        val secret = "test credential café 😀 with \"quotes\""
        equal("null", value(call(adapter, "get", input(key))))
        equal("null", value(call(adapter, "set", input(key, text = secret))))
        val reopened = SecureStoreAdapter(context)
        equal(JSONObject.quote(secret), value(call(reopened, "get", input(key))))
        failure(call(reopened, "set", input(key, text = "😀".repeat(70_000))))
        equal(JSONObject.quote(secret), value(call(reopened, "get", input(key))))
        equal("null", value(call(reopened, "set", input(key, text = "replacement"))))
        equal("\"replacement\"", value(call(adapter, "get", input(key))))
        equal("null", value(call(adapter, "remove", input(key))))
        equal("null", value(call(reopened, "get", input(key))))
        equal("null", value(call(reopened, "remove", input(key))))
        for (invalid in listOf("", "   ", "x".repeat(257))) failure(call(adapter, "set", input(invalid)))
    }

    private fun secureAuthentication() {
        val adapter = SecureStoreAdapter(context)
        val directory = File(context.noBackupFilesDir, "credentials")
        val before = directory.listFiles()!!.toSet()
        equal("null", value(call(adapter, "set", input("credential-one", text = "first test secret"))))
        val first = (directory.listFiles()!!.toSet() - before).single()
        val firstCiphertext = first.readBytes()
        check(!first.readText().contains("first test secret"))
        equal("null", value(call(adapter, "set", input("credential-one", text = "first test secret"))))
        check(!firstCiphertext.contentEquals(first.readBytes())) { "Repeated encryption reused its nonce" }
        val afterFirst = directory.listFiles()!!.toSet()
        equal("null", value(call(adapter, "set", input("credential-two", text = "second test secret"))))
        val second = (directory.listFiles()!!.toSet() - afterFirst).single()
        second.writeBytes(first.readBytes())
        failure(call(adapter, "get", input("credential-two")))
        val encrypted = JSONObject(first.readText())
        val ciphertext = android.util.Base64.decode(encrypted.getString("value"), android.util.Base64.NO_WRAP)
        ciphertext[ciphertext.lastIndex] = (ciphertext.last().toInt() xor 1).toByte()
        first.writeText(encrypted.put("value", android.util.Base64.encodeToString(ciphertext, android.util.Base64.NO_WRAP)).toString())
        failure(call(adapter, "get", input("credential-one")))
        call(adapter, "remove", input("credential-one")); call(adapter, "remove", input("credential-two"))
    }

    private fun attachment(name: String, contents: ByteArray): File = File(context.filesDir, "ink-files/$name").apply {
        parentFile!!.mkdirs(); writeBytes(contents)
    }
    private fun rejected(operation: () -> Unit) {
        var rejected = false
        try { operation() } catch (_: IllegalArgumentException) { rejected = true }
        check(rejected) { "Unsafe managed path was accepted" }
    }
    private fun managedFiles() {
        val files = InkManagedFiles(context)
        val original = attachment("original.data", byteArrayOf(3, 7, 11))
        val metadata = files.adopt(original, "application/octet-stream", "report.bin", "contract-attachment")
        equal(false, metadata.has("path")); equal(3L, metadata.getLong("size"))
        equal(original.canonicalFile, files.resolve(metadata.getString("src")))
        original.appendBytes(byteArrayOf(13))
        equal(4L, files.open("contract-attachment")!!.getLong("size"))
        rejected { files.resolve("ink-file://../outside") }
        rejected { files.resolve("file://${original.path}") }
        val outside = File(context.cacheDir, "outside.data").apply { writeText("secret") }
        try { rejected { files.adopt(outside, "text/plain", id = "outside") } } finally { outside.delete() }
        files.remove("contract-attachment")
        check(!original.exists()); equal(null, files.open("contract-attachment"))
        files.remove("contract-attachment")
    }

    private class QueuedExecutor : Executor {
        private val work = LinkedBlockingQueue<Runnable>()
        override fun execute(command: Runnable) { work.add(command) }
        fun runNext() { checkNotNull(work.poll(5, TimeUnit.SECONDS)).run() }
    }
    private fun withStreams(executor: Executor = Executors.newCachedThreadPool(), operation: (InkFetchStreams, Handler) -> Unit) = withHandler { handler ->
        val engine = HttpEngine.Builder(context).build()
        val streams = InkFetchStreams(context, engine, executor, handler)
        try { operation(streams, handler) }
        finally { streams.stop(); engine.shutdown(); (executor as? java.util.concurrent.ExecutorService)?.shutdownNow() }
    }
    private fun streamCall(streams: InkFetchStreams, operation: String, input: JSONObject = JSONObject(), bytes: ByteArray? = null): NativeResult {
        val results = LinkedBlockingQueue<NativeResult>()
        streams.execute(1, operation, input.toString(), { results.add(it) }, bytes)
        return take(results)
    }
    private fun cameraFile(name: String, bytes: ByteArray) = File(context.noBackupFilesDir, "ink-camera/$name").apply {
        parentFile!!.mkdirs(); writeBytes(bytes)
    }
    private fun openFile(streams: InkFetchStreams, file: File): String {
        val response = JSONObject(value(streamCall(streams, "stream-open", JSONObject().put("url", file.toURI().toString().replace("file:/", "file:///")))))
        equal(200, response.getInt("status")); equal("OK", response.getString("statusText"))
        return response.getString("stream")
    }
    private fun readAll(streams: InkFetchStreams, key: String): ByteArray {
        val output = java.io.ByteArrayOutputStream()
        repeat(100) {
            val chunk = streamCall(streams, "stream-read-bytes", JSONObject().put("stream", key)) as? NativeResult.Binary
                ?: error("Expected binary response chunk")
            output.write(chunk.bytes)
            if (JSONObject(chunk.value).getBoolean("done")) return output.toByteArray()
        }
        error("Response did not terminate within 100 reads")
    }

    private fun fileStreams() = withStreams { streams, _ ->
        val original = attachment("range.data", byteArrayOf(2, 3, 5, 7, 11, 13))
        val src = InkManagedFiles(context).adopt(original, "application/octet-stream", id = "range").getString("src")
        val range = JSONObject().put("src", src).put("offset", 2).put("size", 3)
        val bytes = streamCall(streams, "stream-file-read-bytes", range) as NativeResult.Binary
        check(bytes.bytes.contentEquals(byteArrayOf(5, 7, 11)))
        equal("BQcL", value(streamCall(streams, "stream-file-read", range)))
        failure(streamCall(streams, "stream-file-read-bytes", JSONObject(range.toString()).put("size", 5)))
        failure(streamCall(streams, "stream-file-read-bytes", JSONObject(range.toString()).put("offset", -1)))
        val photoBytes = ByteArray(70_001) { (it % 251).toByte() }
        val photo = cameraFile("chunks.jpg", photoBytes)
        val key = openFile(streams, photo)
        check(photoBytes.contentEquals(readAll(streams, key)))
        streamCall(streams, "stream-close", JSONObject().put("stream", key))
        failure(streamCall(streams, "stream-read-bytes", JSONObject().put("stream", key)))
        failure(streamCall(streams, "stream-open", JSONObject().put("url", original.toURI().toString().replace("file:/", "file:///"))))
    }

    private fun streamCancellation() {
        val executor = QueuedExecutor()
        withStreams(executor) { streams, handler ->
            val upload = value(streamCall(streams, "stream-upload-open"))
            val files = context.cacheDir.listFiles()!!.filter { it.name.startsWith("ink-upload-") }
            equal(1, files.size)
            val completions = LinkedBlockingQueue<NativeResult>()
            streams.execute(20, "stream-upload-write", JSONObject().put("upload", upload).toString(), { completions.add(it) }, byteArrayOf(1, 2, 3))
            failure(streamCall(streams, "stream-upload-write", JSONObject().put("upload", upload), byteArrayOf(4)))
            streams.cancel(20)
            executor.runNext(); drain(handler)
            equal(null, completions.poll()); equal(0L, files.single().length())
            streams.execute(21, "stream-upload-write", JSONObject().put("upload", upload).toString(), { completions.add(it) }, byteArrayOf(5))
            streamCall(streams, "stream-upload-close", JSONObject().put("upload", upload))
            executor.runNext(); drain(handler)
            equal(null, completions.poll()); check(!files.single().exists())
            val handles = List(8) { streamCall(streams, "stream-upload-open") as NativeResult.Success }
            failure(streamCall(streams, "stream-upload-open"))
            disposeNativeResult(handles.first())
            val replacement = streamCall(streams, "stream-upload-open") as NativeResult.Success
            disposeNativeResult(replacement); handles.drop(1).forEach(::disposeNativeResult)
            equal(0, context.cacheDir.listFiles()!!.count { it.name.startsWith("ink-upload-") })
            val next = value(streamCall(streams, "stream-upload-open"))
            failure(streamCall(streams, "stream-upload-write", JSONObject().put("upload", next), ByteArray(32769)))
            streamCall(streams, "stream-upload-close", JSONObject().put("upload", next))
        }
    }

    private fun httpStreams() {
        val server = ServerSocket(0, 1, java.net.InetAddress.getByName("127.0.0.1"))
        server.soTimeout = 10_000
        val received = LinkedBlockingQueue<Any>()
        val serverThread = Thread {
            try {
                server.accept().use { socket ->
                    socket.soTimeout = 10_000
                    val input = socket.getInputStream()
                    fun line(): String {
                        val out = java.io.ByteArrayOutputStream()
                        while (true) {
                            val byte = input.read(); check(byte >= 0)
                            if (byte == 10) return out.toString("US-ASCII").trimEnd('\r')
                            out.write(byte)
                        }
                    }
                    val requestLine = line()
                    val headers = mutableMapOf<String, String>()
                    while (true) {
                        val header = line(); if (header.isEmpty()) break
                        headers[header.substringBefore(':').lowercase()] = header.substringAfter(':').trim()
                    }
                    val body = ByteArray(headers.getValue("content-length").toInt())
                    var offset = 0
                    while (offset < body.size) { val count = input.read(body, offset, body.size - offset); check(count > 0); offset += count }
                    received.add(Triple(requestLine, headers, body))
                    socket.getOutputStream().write("HTTP/1.1 201 Created\r\nContent-Type: application/octet-stream\r\nContent-Length: 5\r\nConnection: close\r\n\r\n".toByteArray() + byteArrayOf(9, 8, 0, -1, 7))
                    socket.getOutputStream().flush()
                }
            } catch (error: Throwable) { received.add(error) }
        }.apply { start() }
        try {
            withStreams { streams, _ ->
                val upload = value(streamCall(streams, "stream-upload-open", JSONObject().put("managed", true)))
                val source = attachment("http-upload.data", byteArrayOf(0, 13, 17, 0))
                val src = InkManagedFiles(context).adopt(source, "application/octet-stream", id = "http-upload").getString("src")
                streamCall(streams, "stream-upload-write", JSONObject().put("upload", upload), byteArrayOf(2, 3))
                streamCall(streams, "stream-upload-file", JSONObject().put("upload", upload).put("src", src).put("offset", 1).put("size", 2))
                streamCall(streams, "stream-upload-write", JSONObject().put("upload", upload), byteArrayOf(5, 7, 11))
                val response = JSONObject(value(streamCall(streams, "stream-open", JSONObject()
                    .put("url", "http://127.0.0.1:${server.localPort}/upload")
                    .put("method", "POST").put("upload", upload)
                    .put("headers", JSONObject().put("x-contract", "yes")))))
                equal(201, response.getInt("status"))
                check(readAll(streams, response.getString("stream")).contentEquals(byteArrayOf(9, 8, 0, -1, 7)))
                val request = received.poll(10, TimeUnit.SECONDS)
                if (request is Throwable) throw request
                @Suppress("UNCHECKED_CAST")
                val receivedRequest = request as Triple<String, Map<String, String>, ByteArray>
                equal("POST /upload HTTP/1.1", receivedRequest.first)
                equal("yes", receivedRequest.second["x-contract"])
                equal("application/octet-stream", receivedRequest.second["content-type"])
                check(receivedRequest.third.contentEquals(byteArrayOf(2, 3, 13, 17, 5, 7, 11)))
                equal(0, context.cacheDir.listFiles()!!.count { it.name.startsWith("ink-upload-") })
            }
        } finally { server.close(); serverThread.join(12_000); check(!serverThread.isAlive) }
    }

    private fun streamExpiry() = withStreams { streams, handler ->
        val upload = value(streamCall(streams, "stream-upload-open"))
        val key = openFile(streams, cameraFile("expires.jpg", byteArrayOf(1)))
        Thread.sleep(61_000)
        drain(handler)
        equal(0, context.cacheDir.listFiles()!!.count { it.name.startsWith("ink-upload-") })
        failure(streamCall(streams, "stream-upload-write", JSONObject().put("upload", upload), byteArrayOf(1)))
        failure(streamCall(streams, "stream-read-bytes", JSONObject().put("stream", key)))
    }
}
