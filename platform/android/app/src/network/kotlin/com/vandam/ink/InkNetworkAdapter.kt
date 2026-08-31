package com.vandam.ink

import android.net.Uri
import android.net.http.HttpEngine
import android.net.http.HttpException
import android.net.http.UploadDataProvider
import android.net.http.UploadDataSink
import android.net.http.UrlRequest
import android.net.http.UrlResponseInfo
import android.util.AtomicFile
import org.json.JSONObject
import org.json.JSONTokener
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.OutputStream
import java.nio.ByteBuffer
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.FutureTask

internal fun createNetworkAdapter(activity: MainActivity): NetworkAdapter =
    InkNetworkAdapter(activity)

private class InkNetworkAdapter(private val activity: MainActivity) : NetworkAdapter {
    private val executor = Executors.newFixedThreadPool(2)
    private val requests = ConcurrentHashMap<Long, UrlRequest>()
    private val cacheReads = ConcurrentHashMap<Long, FutureTask<Unit>>()
    private val engine: HttpEngine

    init {
        val storage = File(activity.cacheDir, "ink-http").apply { mkdirs() }
        engine = HttpEngine.Builder(activity.applicationContext)
            .setStoragePath(storage.absolutePath)
            .setEnableHttpCache(HttpEngine.Builder.HTTP_CACHE_DISK, CACHE_BYTES)
            .setEnableBrotli(true)
            .setEnableHttp2(true)
            .setEnableQuic(true)
            .build()
    }

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (
            operation != JSON_OPERATION &&
            operation != CACHED_JSON_OPERATION &&
            operation != MUTATION_OPERATION &&
            operation != IMAGE_OPERATION
        ) {
            complete(protocol("Unknown network operation: $operation"))
            return
        }
        val recipe = runCatching { JSONObject(payload) }.getOrElse {
            complete(protocol("Ink produced an invalid network request"))
            return
        }
        val url = recipe.optString(URL_KEY)
        if (!url.startsWith(HTTPS_PREFIX)) {
            complete(protocol("Network requests require HTTPS"))
            return
        }
        val resolvedUrl = runCatching {
            val builder = Uri.parse(url).buildUpon()
            val query = recipe.optJSONObject(QUERY_KEY)
            query?.keys()?.forEach { name ->
                builder.appendQueryParameter(name, query.get(name).toString())
            }
            builder.build().toString()
        }.getOrElse {
            complete(protocol("Network request URL was invalid"))
            return
        }
        val cache = if (operation == CACHED_JSON_OPERATION) {
            cachedRequest(recipe, resolvedUrl).getOrElse {
                complete(protocol("Cached network options were invalid"))
                return
            }
        } else {
            null
        }
        if (cache != null) {
            val read = FutureTask {
                val fresh = cache.fresh()
                activity.runOnUiThread {
                    if (cacheReads.remove(requestId) != null) {
                        if (fresh == null) {
                            startRequest(requestId, operation, recipe, resolvedUrl, cache, complete)
                        } else {
                            complete(NativeResult.Bytes(fresh))
                        }
                    }
                }
            }
            cacheReads[requestId] = read
            executor.execute(read)
            return
        }
        startRequest(requestId, operation, recipe, resolvedUrl, null, complete)
    }

    override fun cancel(requestId: Long) {
        cacheReads.remove(requestId)?.cancel(true)
        requests.remove(requestId)?.cancel()
    }

    override fun stop() {
        cacheReads.values.forEach { it.cancel(true) }
        cacheReads.clear()
        requests.values.forEach(UrlRequest::cancel)
        requests.clear()
        engine.shutdown()
        executor.shutdownNow()
    }

    private fun startRequest(
        requestId: Long,
        operation: String,
        recipe: JSONObject,
        resolvedUrl: String,
        cache: CachedRequest?,
        complete: NativeResultHandler,
    ) {
        val callback = RequestCallback(
            requestId,
            if (operation == IMAGE_OPERATION) IMAGE_LIMIT else JSON_LIMIT,
            operation == IMAGE_OPERATION,
            cache,
            complete,
        )
        val request = runCatching {
            val headers = recipe.optJSONObject(HEADERS_KEY)
            engine.newUrlRequestBuilder(resolvedUrl, executor, callback)
                .addHeader(
                    "Accept",
                    if (operation == IMAGE_OPERATION) "image/*" else "application/json",
                )
                .apply {
                    if (operation == MUTATION_OPERATION) {
                        val method = recipe.getString(METHOD_KEY)
                        require(method in MUTATION_METHODS)
                        setHttpMethod(method)
                        if (recipe.has(BODY_KEY)) {
                            val body = recipe.getJSONObject(BODY_KEY).toString().toByteArray()
                            setUploadDataProvider(ByteArrayUpload(body), executor)
                            if (headers?.keys()?.asSequence()?.none {
                                    it.equals("content-type", ignoreCase = true)
                                } != false
                            ) {
                                addHeader("Content-Type", "application/json")
                            }
                        }
                    }
                    headers?.keys()?.forEach { name ->
                        val value = headers.getString(name)
                        require(validHeader(name, value))
                        addHeader(name, value)
                    }
                }
                .build()
        }.getOrElse {
            callback.discard()
            complete(protocol("Network request headers were invalid"))
            return
        }
        requests[requestId] = request
        request.start()
    }

    private inner class RequestCallback(
        private val requestId: Long,
        private val limit: Int,
        image: Boolean,
        private val cache: CachedRequest?,
        private val complete: NativeResultHandler,
    ) : UrlRequest.Callback {
        private val file = if (image) {
            File.createTempFile("ink-image-", ".download", activity.cacheDir)
        } else {
            null
        }
        private val body = if (image) null else ByteArrayOutputStream()
        private val output: OutputStream = file?.outputStream()?.buffered() ?: requireNotNull(body)
        private val buffer = ByteBuffer.allocateDirect(BUFFER_BYTES)
        private var redirects = 0
        private var receivedBytes = 0

        override fun onRedirectReceived(
            request: UrlRequest,
            info: UrlResponseInfo,
            newLocationUrl: String,
        ) {
            redirects += 1
            if (redirects > REDIRECT_LIMIT) {
                finishFailure(protocol("Network request followed too many redirects"))
                request.cancel()
            } else if (newLocationUrl.startsWith(HTTPS_PREFIX)) {
                request.followRedirect()
            } else {
                finishFailure(protocol("Network redirect did not use HTTPS"))
                request.cancel()
            }
        }

        override fun onResponseStarted(request: UrlRequest, info: UrlResponseInfo) {
            request.read(buffer)
        }

        override fun onReadCompleted(
            request: UrlRequest,
            info: UrlResponseInfo,
            byteBuffer: ByteBuffer,
        ) {
            byteBuffer.flip()
            val bytes = ByteArray(byteBuffer.remaining())
            byteBuffer.get(bytes)
            receivedBytes += bytes.size
            if (receivedBytes > limit) {
                finishFailure(protocol("Network response exceeded Ink's size limit"))
                request.cancel()
                return
            }
            output.write(bytes)
            byteBuffer.clear()
            request.read(byteBuffer)
        }

        override fun onSucceeded(request: UrlRequest, info: UrlResponseInfo) {
            val status = info.httpStatusCode
            if (status in 200..299) {
                val result = file?.let { NativeResult.File(it.absolutePath) }
                    ?: requireNotNull(body).toByteArray().let { response ->
                        cache?.store(response)
                            ?: NativeResult.Bytes(
                                if (response.isEmpty()) "null".toByteArray() else response,
                            )
                    }
                finish(result, file != null)
            } else {
                finishFailure(
                    NativeResult.Failure(
                        if (status >= 500) NativeErrorKind.UNAVAILABLE else NativeErrorKind.PROTOCOL,
                        "HTTP $status ${info.httpStatusText}".trim(),
                        status >= 500 || status == 408 || status == 429,
                    ),
                )
            }
        }

        override fun onFailed(
            request: UrlRequest,
            info: UrlResponseInfo?,
            error: HttpException,
        ) {
            finishFailure(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    error.message ?: "Network request failed",
                    true,
                ),
            )
        }

        override fun onCanceled(request: UrlRequest, info: UrlResponseInfo?) {
            if (requests.remove(requestId) != null) {
                discard()
            }
        }

        fun discard() {
            runCatching { output.close() }
            file?.delete()
        }

        private fun finish(result: NativeResult, keepFile: Boolean = false) {
            if (requests.remove(requestId) != null) {
                runCatching { output.close() }
                if (!keepFile) {
                    file?.delete()
                }
                complete(result)
            }
        }

        private fun finishFailure(failure: NativeResult.Failure) {
            finish(cache?.stale(failure) ?: failure)
        }
    }

    private fun cachedRequest(recipe: JSONObject, resolvedUrl: String): Result<CachedRequest> =
        runCatching {
            val maxAgeMs = recipe.getLong(MAX_AGE_MS_KEY)
            val staleIfErrorMs = recipe.getLong(STALE_IF_ERROR_MS_KEY)
            require(maxAgeMs in 0..MAX_CACHE_AGE_MS)
            require(staleIfErrorMs in 0..MAX_STALE_IF_ERROR_MS)
            val headers = recipe.optJSONObject(HEADERS_KEY)?.toString().orEmpty()
            val schema = recipe.get(SCHEMA_KEY)
            val digest = MessageDigest.getInstance("SHA-256")
                .digest("$resolvedUrl\n$headers\n$schema".toByteArray())
                .joinToString("") { "%02x".format(it) }
            val directory = File(activity.cacheDir, JSON_CACHE_DIRECTORY).apply { mkdirs() }
            CachedRequest(File(directory, "$digest.json"), schema, maxAgeMs, staleIfErrorMs)
        }

    private class CachedRequest(
        private val file: File,
        private val schema: Any,
        private val maxAgeMs: Long,
        private val staleIfErrorMs: Long,
    ) {
        private val cached: ByteArray? by lazy {
            if (!file.isFile || file.length() > JSON_CACHE_ENTRY_BYTES) {
                file.delete()
                null
            } else {
                runCatching { file.readBytes() }.getOrNull()
            }
        }

        fun fresh(): ByteArray? {
            val bytes = cached ?: return null
            val age = age(bytes) ?: return null
            return bytes.takeIf { age <= maxAgeMs }
        }

        fun store(value: ByteArray): NativeResult {
            val decoded = runCatching { JSONTokener(value.toString(Charsets.UTF_8)).nextValue() }
                .getOrElse { return NativeResult.Bytes(value) }
            if (!validInkJson(schema, decoded)) return NativeResult.Bytes(value)
            val updatedAtMs = System.currentTimeMillis()
            val envelope = (
                "{\"status\":\"ready\",\"value\":" +
                    value.toString(Charsets.UTF_8) +
                    ",\"updatedAtMs\":$updatedAtMs}"
            ).toByteArray()
            runCatching {
                val target = AtomicFile(file)
                val output = target.startWrite()
                try {
                    output.write(envelope)
                    target.finishWrite(output)
                } catch (error: Throwable) {
                    target.failWrite(output)
                    throw error
                }
                trim(file.parentFile)
            }
            return NativeResult.Bytes(envelope)
        }

        fun stale(failure: NativeResult.Failure): NativeResult? {
            val bytes = cached ?: return null
            val age = age(bytes) ?: return null
            if (age > maxAgeMs + staleIfErrorMs) return null
            return runCatching {
                val envelope = JSONObject(bytes.toString(Charsets.UTF_8))
                envelope.put("status", "stale")
                envelope.put(
                    "error",
                    JSONObject()
                        .put("kind", failure.kind.resourceName())
                        .put("message", failure.message)
                        .put("retryable", failure.retryable)
                        .put("attemptedAtMs", System.currentTimeMillis()),
                )
                NativeResult.Bytes(envelope.toString().toByteArray())
            }.getOrNull()
        }

        private fun age(bytes: ByteArray): Long? = runCatching {
            val updatedAtMs = JSONObject(bytes.toString(Charsets.UTF_8)).getLong("updatedAtMs")
            (System.currentTimeMillis() - updatedAtMs).coerceAtLeast(0)
        }.getOrNull()

        private fun trim(directory: File?) {
            val files = directory?.listFiles()?.sortedByDescending(File::lastModified) ?: return
            var bytes = 0L
            for (file in files) {
                bytes += file.length()
                if (bytes > JSON_CACHE_BYTES) file.delete()
            }
        }
    }

    private companion object {
        private const val JSON_OPERATION = "json"
        private const val CACHED_JSON_OPERATION = "cached-json"
        private const val MUTATION_OPERATION = "mutation"
        private const val IMAGE_OPERATION = "image"
        private const val URL_KEY = "url"
        private const val QUERY_KEY = "query"
        private const val HEADERS_KEY = "headers"
        private const val METHOD_KEY = "method"
        private const val BODY_KEY = "body"
        private const val SCHEMA_KEY = "schema"
        private const val MAX_AGE_MS_KEY = "maxAgeMs"
        private const val STALE_IF_ERROR_MS_KEY = "staleIfErrorMs"
        private const val JSON_CACHE_DIRECTORY = "ink-json"
        private const val HTTPS_PREFIX = "https://"
        private const val BUFFER_BYTES = 32 * 1024
        private const val REDIRECT_LIMIT = 5
        private const val JSON_LIMIT = 1024 * 1024
        private const val IMAGE_LIMIT = 16 * 1024 * 1024
        private const val CACHE_BYTES = 16L * 1024L * 1024L
        private const val JSON_CACHE_BYTES = 8L * 1024L * 1024L
        private const val JSON_CACHE_ENTRY_BYTES = JSON_LIMIT + 64L * 1024L
        private const val MAX_CACHE_AGE_MS = 604_800_000L
        private const val MAX_STALE_IF_ERROR_MS = 2_592_000_000L

        private fun protocol(message: String) = NativeResult.Failure(
            NativeErrorKind.PROTOCOL,
            message,
            false,
        )

        private fun validHeader(name: String, value: String): Boolean =
            name.isNotEmpty() &&
                name.lowercase() !in RESERVED_HEADERS &&
                name.all { it.isLetterOrDigit() || it in "!#$%&'*+-.^_`|~" } &&
                value.none { it == '\r' || it == '\n' || it.code < 0x20 && it != '\t' }

        private val RESERVED_HEADERS = setOf(
            "accept-encoding",
            "connection",
            "content-length",
            "host",
            "proxy-authorization",
            "te",
            "transfer-encoding",
            "upgrade",
        )
        private val MUTATION_METHODS = setOf("POST", "PUT", "PATCH", "DELETE")
    }
}

private class ByteArrayUpload(private val bytes: ByteArray) : UploadDataProvider() {
    private var offset = 0

    override fun getLength(): Long = bytes.size.toLong()

    override fun read(sink: UploadDataSink, buffer: ByteBuffer) {
        val length = minOf(buffer.remaining(), bytes.size - offset)
        buffer.put(bytes, offset, length)
        offset += length
        sink.onReadSucceeded(false)
    }

    override fun rewind(sink: UploadDataSink) {
        offset = 0
        sink.onRewindSucceeded()
    }
}

private fun NativeErrorKind.resourceName(): String = when (this) {
    NativeErrorKind.UNAVAILABLE -> "unavailable"
    NativeErrorKind.TIMEOUT -> "timeout"
    NativeErrorKind.PROTOCOL -> "protocol"
    else -> "unexpected"
}
