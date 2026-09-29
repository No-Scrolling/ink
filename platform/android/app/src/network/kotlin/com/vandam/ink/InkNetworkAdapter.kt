package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.net.http.HttpEngine
import android.net.http.HttpException
import android.net.http.UrlRequest
import android.net.http.UrlResponseInfo
import org.json.JSONObject
import java.io.File
import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

internal fun createNetworkAdapter(context: Context, cacheName: String? = "ink-http"): NetworkAdapter =
    InkNetworkAdapter(context.applicationContext, cacheName)

private class InkNetworkAdapter(private val context: Context, cacheName: String?) : NetworkAdapter {
    private val handler = Handler(Looper.getMainLooper())
    private val executor = Executors.newFixedThreadPool(2)
    private val requests = ConcurrentHashMap<Long, UrlRequest>()
    private val httpEngine = lazy {
        HttpEngine.Builder(context)
            .apply {
                if (cacheName != null) {
                    val storage = File(context.cacheDir, cacheName).apply { mkdirs() }
                    setStoragePath(storage.absolutePath)
                    setEnableHttpCache(HttpEngine.Builder.HTTP_CACHE_DISK, CACHE_BYTES)
                }
            }
            .setEnableBrotli(true)
            .setEnableHttp2(true)
            .setEnableQuic(true)
            .build()
    }
    private val engine by httpEngine

    private val streamTransport = lazy { InkFetchStreams(context, engine, executor, handler) }
    private val streams by streamTransport
    private val socketTransport = lazy { InkWebSockets(handler) }
    private val sockets by socketTransport

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation.startsWith("stream-")) {
            streams.execute(requestId, operation, payload, complete)
            return
        }
        if (operation.startsWith("socket-")) {
            sockets.execute(requestId, operation, payload, complete)
            return
        }
        if (operation != IMAGE_OPERATION) {
            complete(protocol("Unknown network operation: $operation"))
            return
        }
        val recipe = runCatching { JSONObject(payload) }.getOrElse {
            complete(protocol("Ink produced an invalid network request"))
            return
        }
        val url = recipe.optString(URL_KEY)
        if (!allowedImageUrl(url)) {
            complete(protocol("Network requests require HTTPS"))
            return
        }
        startRequest(requestId, url, complete)
    }

    private fun allowedImageUrl(url: String): Boolean =
        url.startsWith(HTTPS_PREFIX) || url.startsWith("http://") &&
            (BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED || BuildConfig.DEBUG &&
                Uri.parse(url).host in setOf("localhost", "127.0.0.1", "::1", "10.0.2.2"))

    override fun executeBytes(requestId: Long, operation: String, payload: String, bytes: ByteArray, complete: NativeResultHandler) {
        when (operation) {
            "stream-upload-write" -> streams.execute(requestId, operation, payload, complete, bytes)
            "socket-send" -> sockets.execute(requestId, operation, payload, complete, bytes)
            else -> complete(protocol("Unknown binary network operation: $operation"))
        }
    }

    override fun cancel(requestId: Long) {
        if (streamTransport.isInitialized()) streams.cancel(requestId)
        if (socketTransport.isInitialized()) sockets.cancel(requestId)
        requests.remove(requestId)?.cancel()
    }

    override fun reset() {
        if (streamTransport.isInitialized()) streams.stop()
        if (socketTransport.isInitialized()) sockets.reset()
        requests.values.forEach(UrlRequest::cancel)
        requests.clear()
    }

    override fun stop() {
        reset()
        if (socketTransport.isInitialized()) sockets.stop()
        if (httpEngine.isInitialized()) engine.shutdown()
        executor.shutdownNow()
    }

    private fun startRequest(requestId: Long, url: String, complete: NativeResultHandler) {
        val callback = RequestCallback(requestId, complete)
        val request = runCatching {
            engine.newUrlRequestBuilder(url, executor, callback)
                .addHeader("Accept", "image/*")
                .build()
        }.getOrElse {
            callback.discard()
            complete(protocol("Image request was invalid"))
            return
        }
        requests[requestId] = request
        callback.request = request
        request.start()
    }

    private inner class RequestCallback(
        private val requestId: Long,
        private val complete: NativeResultHandler,
    ) : UrlRequest.Callback {
        var request: UrlRequest? = null
        private val file = File.createTempFile("ink-image-", ".download", context.cacheDir)
        private val output = file.outputStream().buffered()
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
            } else if (allowedImageUrl(newLocationUrl)) {
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
            if (receivedBytes > IMAGE_LIMIT) {
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
                finish(NativeResult.File(file.absolutePath), keepFile = true)
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
            requests.remove(requestId, request)
            discard()
        }

        fun discard() {
            runCatching { output.close() }
            file.delete()
        }

        private fun finish(result: NativeResult, keepFile: Boolean = false) {
            if (requests.remove(requestId, request)) {
                runCatching { output.close() }
                if (!keepFile) {
                    file.delete()
                }
                complete(result)
            } else discard()
        }

        private fun finishFailure(failure: NativeResult.Failure) {
            finish(failure)
        }
    }

    private companion object {
        private const val IMAGE_OPERATION = "image"
        private const val URL_KEY = "url"
        private const val HTTPS_PREFIX = "https://"
        private const val BUFFER_BYTES = 32 * 1024
        private const val REDIRECT_LIMIT = 5
        private const val IMAGE_LIMIT = 16 * 1024 * 1024
        private const val CACHE_BYTES = 16L * 1024L * 1024L

        private fun protocol(message: String) = NativeResult.Failure(
            NativeErrorKind.PROTOCOL,
            message,
            false,
        )
    }
}
