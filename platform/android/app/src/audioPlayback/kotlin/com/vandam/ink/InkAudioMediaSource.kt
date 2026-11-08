package com.vandam.ink

import android.content.Context
import android.net.Uri
import android.util.Log
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.util.UnstableApi
import androidx.media3.common.util.Util
import androidx.media3.database.StandaloneDatabaseProvider
import androidx.media3.datasource.DataSource
import androidx.media3.datasource.DataSourceException
import androidx.media3.datasource.DataSpec
import androidx.media3.datasource.DefaultDataSource
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.datasource.TransferListener
import androidx.media3.datasource.cache.CacheDataSource
import androidx.media3.datasource.cache.LeastRecentlyUsedCacheEvictor
import androidx.media3.datasource.cache.SimpleCache
import androidx.media3.exoplayer.DefaultLoadControl
import androidx.media3.exoplayer.drm.DrmSessionManagerProvider
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.exoplayer.source.MediaPeriod
import androidx.media3.exoplayer.source.MediaSource
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import androidx.media3.exoplayer.source.WrappingMediaSource
import androidx.media3.exoplayer.upstream.Allocator
import androidx.media3.exoplayer.upstream.DefaultLoadErrorHandlingPolicy
import androidx.media3.exoplayer.upstream.LoadErrorHandlingPolicy
import java.io.File
import java.io.IOException
import java.util.IdentityHashMap
import java.util.function.BooleanSupplier

@UnstableApi
internal fun audioLoadControl(startupBuffer: Int): DefaultLoadControl = DefaultLoadControl.Builder()
    .setBufferDurationsMsForStreaming(
        DefaultLoadControl.DEFAULT_MIN_BUFFER_MS,
        DefaultLoadControl.DEFAULT_MAX_BUFFER_MS,
        startupBuffer,
        DefaultLoadControl.DEFAULT_BUFFER_FOR_PLAYBACK_AFTER_REBUFFER_MS,
    ).build()

@UnstableApi
private class ProviderAudioErrorPolicy : DefaultLoadErrorHandlingPolicy() {
    // Delay selection owns the bounded attempt count, including provider-specific delays.
    override fun getMinimumLoadableRetryCount(dataType: Int): Int = Int.MAX_VALUE
    override fun getRetryDelayMsFor(info: LoadErrorHandlingPolicy.LoadErrorInfo): Long {
        val retry = info.exception as? InkMediaRetryException
        if (retry != null) return if (info.errorCount <= retry.maxRetries) retry.retryAfterMillis else C.TIME_UNSET
        if (info.errorCount > super.getMinimumLoadableRetryCount(info.mediaLoadData.dataType)) return C.TIME_UNSET
        return super.getRetryDelayMsFor(info)
    }
}

@UnstableApi
internal fun audioMediaSourceFactory(context: Context, cache: Boolean = false, session: String = "main"): MediaSource.Factory {
    val http = DefaultHttpDataSource.Factory()
        .setAllowCrossProtocolRedirects(BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED)
    val upstream = if (cache) {
        DataSource.Factory { CachedHttpDataSource(context.applicationContext, http) }
    } else http
    val defaults = DataSource.Factory {
        val source = DefaultDataSource(context, upstream.createDataSource())
        object : DataSource by source {
            override fun open(dataSpec: DataSpec): Long = try {
                source.open(dataSpec)
            } catch (error: SecurityException) {
                throw DataSourceException(error, PlaybackException.ERROR_CODE_IO_NO_PERMISSION)
            }
        }
    }
    val fallback = DefaultMediaSourceFactory(defaults)
    return object : MediaSource.Factory {
        private var drm: DrmSessionManagerProvider? = null
        private var errors: LoadErrorHandlingPolicy? = ProviderAudioErrorPolicy()
        override fun setDrmSessionManagerProvider(provider: DrmSessionManagerProvider) = apply {
            drm = provider
            fallback.setDrmSessionManagerProvider(provider)
        }
        override fun setLoadErrorHandlingPolicy(policy: LoadErrorHandlingPolicy) = apply {
            errors = policy
            fallback.setLoadErrorHandlingPolicy(policy)
        }
        override fun getSupportedTypes(): IntArray = fallback.supportedTypes
        override fun createMediaSource(mediaItem: MediaItem): MediaSource {
            val configuration = mediaItem.localConfiguration
            val item = if (cache && configuration != null &&
                Util.inferContentTypeForUriAndMimeType(configuration.uri, configuration.mimeType) == C.CONTENT_TYPE_OTHER) {
                mediaItem.buildUpon().setCustomCacheKey("$session:${mediaItem.mediaId}").build()
            } else mediaItem
            if (item.localConfiguration?.uri?.scheme != "ink-source") return fallback.createMediaSource(item)
            val pending = mutableListOf<ProviderAudioDataSource>()
            val readers = IdentityHashMap<MediaPeriod, List<ProviderAudioDataSource>>()
            val factory = ProgressiveMediaSource.Factory(DataSource.Factory {
                ProviderAudioDataSource(context.applicationContext, defaults).also(pending::add)
            })
            drm?.let(factory::setDrmSessionManagerProvider)
            errors?.let(factory::setLoadErrorHandlingPolicy)
            return object : WrappingMediaSource(factory.createMediaSource(item)) {
                override fun createPeriod(id: MediaSource.MediaPeriodId, allocator: Allocator, startPositionUs: Long): MediaPeriod {
                    // Progressive sources create their data source synchronously with each period.
                    val period = super.createPeriod(id, allocator, startPositionUs)
                    readers[period] = pending.toList()
                    pending.clear()
                    return period
                }
                override fun releasePeriod(period: MediaPeriod) {
                    readers.remove(period)?.forEach { it.release() }
                    super.releasePeriod(period)
                }
                override fun releaseSourceInternal() {
                    pending.forEach { it.release() }
                    pending.clear()
                    readers.values.forEach { group -> group.forEach { it.release() } }
                    readers.clear()
                    super.releaseSourceInternal()
                }
            }
        }
    }
}

@UnstableApi
private class ProviderAudioDataSource(private val context: Context, private val defaults: DataSource.Factory) : DataSource {
    private var upstream: DataSource? = null
    @Volatile private var stream: InkMediaStream? = null
    private var uri: Uri? = null
    private var remaining = C.LENGTH_UNSET.toLong()
    private var spec: DataSpec? = null
    private var opened = false
    private var retainedUri: Uri? = null
    @Volatile private var released = false
    private val listeners = mutableListOf<TransferListener>()

    override fun addTransferListener(listener: TransferListener) {
        if (listener !in listeners) { listeners.add(listener); upstream?.addTransferListener(listener) }
    }

    override fun open(dataSpec: DataSpec): Long {
        uri = dataSpec.uri
        if (dataSpec.uri.scheme != "ink-source") {
            return defaults.createDataSource().also { source -> upstream = source; listeners.forEach(source::addTransferListener) }.open(dataSpec)
        }
        spec = dataSpec
        listeners.forEach { it.onTransferInitializing(this, dataSpec, true) }
        if (released) throw IOException("Media source is released")
        val retained = stream
        val active = if (retainedUri == dataSpec.uri && retained?.seek != null) {
            retained.seek.invoke(dataSpec.position)
            retained
        } else {
            discardStream()
            val created = InkMediaSources.source(context, dataSpec.uri).open(dataSpec.uri, dataSpec.position, null, InkMediaPurpose.PLAYBACK, BooleanSupplier { released })
            synchronized(this) {
                if (released) { created.close(); throw IOException("Media source is released") }
                stream = created
                retainedUri = dataSpec.uri
            }
            require(created.position == dataSpec.position) { "Media source did not honour the requested position" }
            created
        }
        remaining = if (dataSpec.length != C.LENGTH_UNSET.toLong()) dataSpec.length else active.total?.minus(dataSpec.position) ?: C.LENGTH_UNSET.toLong()
        opened = true
        listeners.forEach { it.onTransferStart(this, dataSpec, true) }
        return remaining
    }

    override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
        upstream?.let { return it.read(buffer, offset, length) }
        if (length == 0) return 0
        if (remaining == 0L) return C.RESULT_END_OF_INPUT
        val count = try {
            requireNotNull(stream).input.read(buffer, offset, if (remaining < 0) length else minOf(length.toLong(), remaining).toInt())
        } catch (error: IOException) { discardStream(); throw error }
        if (count > 0) { if (remaining >= 0) remaining -= count; listeners.forEach { it.onBytesTransferred(this, requireNotNull(spec), true, count) } }
        return count
    }

    override fun getUri(): Uri? = upstream?.uri ?: uri
    override fun getResponseHeaders(): Map<String, List<String>> = upstream?.responseHeaders ?: emptyMap()
    override fun close() {
        try { upstream?.close(); if (stream?.seek == null) discardStream() }
        finally { upstream = null; uri = null; if (opened) { opened = false; listeners.forEach { it.onTransferEnd(this, requireNotNull(spec), true) }; spec = null } }
    }
    private fun discardStream() {
        val active = synchronized(this) { stream.also { stream = null; retainedUri = null } }
        active?.close()
    }
    fun release() {
        synchronized(this) { released = true }
        discardStream()
    }
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
