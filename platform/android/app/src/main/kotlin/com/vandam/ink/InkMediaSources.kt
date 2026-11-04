package com.vandam.ink

import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import java.io.Closeable
import java.io.File
import java.io.InputStream
import java.io.IOException
import java.util.function.BooleanSupplier

enum class InkMediaPurpose { PLAYBACK, DOWNLOAD }

/** A temporary source failure with a bounded retry policy for playback or background downloads. */
class InkMediaRetryException(message: String, val retryAfterMillis: Long, val maxRetries: Int = 3) : IOException(message) {
    init {
        require(retryAfterMillis > 0) { "Retry delay must be positive" }
        require(maxRetries >= 0) { "Retry limit must be non-negative" }
    }
}

/** A byte range of a stable resource. Closing must interrupt pending reads. */
class InkMediaStream(
    val input: InputStream,
    val position: Long,
    val total: Long?,
    val identity: String,
    val mimeType: String = "application/octet-stream",
    /** Optional absolute byte seek, relative to this resource's decoded start. */
    val seek: ((Long) -> Unit)? = null,
) : Closeable {
    init {
        require(position >= 0 && (total == null || total >= position))
        require(identity.isNotBlank())
    }
    override fun close() = input.close()
}

interface InkMediaSource {
    /** Called on a worker thread, including after a process restart. Never retain an Activity. */
    fun open(uri: Uri, position: Long, identity: String?, purpose: InkMediaPurpose): InkMediaStream

    /** Observe cancellation while opening a source, before a stream exists to close. */
    fun open(uri: Uri, position: Long, identity: String?, purpose: InkMediaPurpose, cancelled: BooleanSupplier): InkMediaStream =
        open(uri, position, identity, purpose)

    /** Download bytes may remain encrypted; record the private file for subsequent playback. */
    fun downloaded(uri: Uri, id: String, file: File) = Unit
    fun removed(uri: Uri, id: String) = Unit
}

interface InkMediaSourceFactory {
    fun create(context: Context): Map<String, InkMediaSource>
}

internal object InkMediaSources {
    private var sources: Map<String, InkMediaSource>? = null

    @Synchronized
    private fun sources(context: Context): Map<String, InkMediaSource> {
        sources?.let { return it }
        val app = context.applicationContext
        val info = app.packageManager.getApplicationInfo(app.packageName, PackageManager.GET_META_DATA)
        val name = info.metaData?.getString("ink.mediaSourceFactory")
        val registered = if (name == null) emptyMap() else {
            val factory = Class.forName(name, true, app.classLoader).getDeclaredConstructor().newInstance() as InkMediaSourceFactory
            factory.create(app).toMap()
        }
        require(registered.keys.all { Regex("[a-z][a-z0-9-]{0,63}").matches(it) }) { "Invalid media source name" }
        sources = registered
        return registered
    }

    fun source(context: Context, uri: Uri): InkMediaSource {
        require(uri.scheme == "ink-source" && uri.userInfo == null && uri.port == -1 && uri.fragment == null) { "Invalid media source URI" }
        return requireNotNull(sources(context)[uri.host]) { "Media source is not registered: ${uri.host}" }
    }
}
