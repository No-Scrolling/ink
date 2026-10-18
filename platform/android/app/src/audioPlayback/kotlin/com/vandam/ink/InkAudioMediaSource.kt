package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.util.Log
import androidx.media3.common.util.UnstableApi
import androidx.media3.database.StandaloneDatabaseProvider
import androidx.media3.datasource.DataSource
import androidx.media3.datasource.DataSpec
import androidx.media3.datasource.DefaultDataSource
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.datasource.TransferListener
import androidx.media3.datasource.cache.CacheDataSource
import androidx.media3.datasource.cache.LeastRecentlyUsedCacheEvictor
import androidx.media3.datasource.cache.SimpleCache
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import java.io.File

@UnstableApi
internal fun audioMediaSourceFactory(context: Context, cache: Boolean = false): DefaultMediaSourceFactory {
    val http = DefaultHttpDataSource.Factory()
        .setAllowCrossProtocolRedirects(BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED)
    val upstream = if (cache) {
        DataSource.Factory { CachedHttpDataSource(context.applicationContext, http) }
    } else http
    return DefaultMediaSourceFactory(DefaultDataSource.Factory(context, upstream))
}

@UnstableApi
private object AudioStreamCache {
    private var initialised = false
    private var cache: SimpleCache? = null

    @Synchronized
    fun get(context: Context): SimpleCache? {
        if (!initialised) {
            initialised = true
            cache = runCatching {
                SimpleCache(
                    File(context.cacheDir, "ink-audio"),
                    LeastRecentlyUsedCacheEvictor(256L * 1024 * 1024),
                    StandaloneDatabaseProvider(context),
                ).also {
                    try {
                        it.checkInitialization()
                    } catch (error: Exception) {
                        it.release()
                        throw error
                    }
                }
            }.onFailure { Log.w("InkAudio", "Stream cache unavailable; using network playback", it) }.getOrNull()
        }
        return cache
    }
}

@UnstableApi
private class CachedHttpDataSource(
    private val context: Context,
    private val upstream: DataSource.Factory,
) : DataSource {
    private val listeners = mutableListOf<TransferListener>()
    private var source: DataSource? = null

    override fun addTransferListener(listener: TransferListener) {
        if (listener !in listeners) {
            listeners += listener
            source?.addTransferListener(listener)
        }
    }

    override fun open(dataSpec: DataSpec): Long {
        val active = source ?: run {
            val created = AudioStreamCache.get(context)?.let { cache ->
                CacheDataSource.Factory()
                    .setCache(cache)
                    .setUpstreamDataSourceFactory(upstream)
                    .setFlags(CacheDataSource.FLAG_IGNORE_CACHE_ON_ERROR)
                    .createDataSource()
            } ?: upstream.createDataSource()
            listeners.forEach(created::addTransferListener)
            source = created
            created
        }
        return active.open(dataSpec)
    }

    override fun read(buffer: ByteArray, offset: Int, length: Int): Int = checkNotNull(source).read(buffer, offset, length)
    override fun getUri(): Uri? = source?.uri
    override fun getResponseHeaders(): Map<String, List<String>> = source?.responseHeaders ?: emptyMap()
    override fun close() { source?.close() }
}
