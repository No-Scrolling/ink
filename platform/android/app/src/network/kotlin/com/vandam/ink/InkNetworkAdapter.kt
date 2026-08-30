package com.vandam.ink

import android.net.Uri
import android.net.http.HttpEngine
import android.net.http.HttpException
import android.net.http.UrlRequest
import android.net.http.UrlResponseInfo
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.OutputStream
import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

internal fun createNetworkAdapter(activity: MainActivity): NetworkAdapter =
    InkNetworkAdapter(activity)

private class InkNetworkAdapter(private val activity: MainActivity) : NetworkAdapter {
    private val executor = Executors.newFixedThreadPool(2)
    private val requests = ConcurrentHashMap<Long, UrlRequest>()
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
        if (operation != JSON_OPERATION && operation != IMAGE_OPERATION) {
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
        val callback = RequestCallback(
            requestId,
            if (operation == JSON_OPERATION) JSON_LIMIT else IMAGE_LIMIT,
            operation == IMAGE_OPERATION,
            complete,
        )
        val request = runCatching {
            engine.newUrlRequestBuilder(resolvedUrl, executor, callback)
                .addHeader(
                    "Accept",
                    if (operation == JSON_OPERATION) "application/json" else "image/*",
                )
                .apply {
                    val headers = recipe.optJSONObject(HEADERS_KEY)
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

    override fun cancel(requestId: Long) {
        requests.remove(requestId)?.cancel()
    }

    override fun stop() {
        requests.values.forEach(UrlRequest::cancel)
        requests.clear()
        engine.shutdown()
        executor.shutdownNow()
    }

    private inner class RequestCallback(
        private val requestId: Long,
        private val limit: Int,
        image: Boolean,
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
                finish(protocol("Network request followed too many redirects"))
                request.cancel()
            } else if (newLocationUrl.startsWith(HTTPS_PREFIX)) {
                request.followRedirect()
            } else {
                finish(protocol("Network redirect did not use HTTPS"))
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
                finish(protocol("Network response exceeded Ink's size limit"))
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
                    ?: NativeResult.Bytes(requireNotNull(body).toByteArray())
                finish(result, file != null)
            } else {
                finish(
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
            finish(
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
    }

    private companion object {
        private const val JSON_OPERATION = "json"
        private const val IMAGE_OPERATION = "image"
        private const val URL_KEY = "url"
        private const val QUERY_KEY = "query"
        private const val HEADERS_KEY = "headers"
        private const val HTTPS_PREFIX = "https://"
        private const val BUFFER_BYTES = 32 * 1024
        private const val REDIRECT_LIMIT = 5
        private const val JSON_LIMIT = 1024 * 1024
        private const val IMAGE_LIMIT = 16 * 1024 * 1024
        private const val CACHE_BYTES = 16L * 1024L * 1024L

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
    }
}
