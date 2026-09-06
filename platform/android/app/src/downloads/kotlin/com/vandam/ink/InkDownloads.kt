package com.vandam.ink

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import android.os.PersistableBundle
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArraySet
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

internal fun createDownloadsAdapter(context: Context, publish: (Long, String) -> Unit): DownloadsAdapter =
    InkDownloadsAdapter(context, publish)

private class InkDownloadsAdapter(context: Context, private val publish: (Long, String) -> Unit) : DownloadsAdapter {
    private val store = DownloadStore(context.applicationContext)
    private val observers = ConcurrentHashMap<Long, String>()
    private val stopped = AtomicBoolean(false)
    private val pending = ConcurrentHashMap.newKeySet<Long>()
    private val changed: (String) -> Unit = { id ->
        synchronized(DownloadStore.lock) {
            val snapshot = store.read(id)
            if (!stopped.get() && snapshot != null) observers.forEach { (controller, observed) ->
                if (observed == id) publish(controller, store.snapshot(snapshot).toString())
            }
        }
    }
    private val executor = Executors.newSingleThreadExecutor()
    init { DownloadStore.listeners.add(changed) }
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        if (stopped.get()) { complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Download adapter is stopped", false)); return }
        pending.add(requestId)
        try { executor.execute task@{
            if (!pending.contains(requestId) || stopped.get()) { pending.remove(requestId); return@task }
            val result = try {
                val input = JSONObject(payload)
                val output = when (operation) {
                    "enqueue" -> store.enqueue(input)
                    "get" -> store.snapshot(store.read(input.getString("id")) ?: error("Download not found"))
                    "pause", "resume", "cancel", "remove" -> {
                        store.control(input.getString("id"), operation)
                        null
                    }
                    else -> error("Unknown download operation")
                }
                NativeResult.Success(output?.toString().orEmpty())
            } catch (error: Exception) {
                NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Download operation failed", false)
            }
            if (pending.remove(requestId) && !stopped.get()) complete(result)
        } } catch (error: java.util.concurrent.RejectedExecutionException) {
            pending.remove(requestId)
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Download adapter is stopped", false))
        }
    }
    override fun executeController(controller: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            check(!stopped.get()) { "Download adapter is stopped" }
            synchronized(DownloadStore.lock) { when (operation) {
                "activate" -> {
                    val id = JSONObject(payload).getString("id")
                    val value = store.read(id) ?: error("Download not found")
                    observers[controller] = id
                    publish(controller, store.snapshot(value).toString())
                }
                "deactivate" -> observers.remove(controller)
                else -> error("Unknown download observation operation")
            } }
            complete(NativeResult.Success(""))
        } catch (error: Exception) {
            complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Download observation failed", false))
        }
    }
    override fun cancel(requestId: Long) { pending.remove(requestId) }
    override fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        observers.clear()
        pending.clear()
        DownloadStore.listeners.remove(changed)
        executor.shutdown()
    }
}

private class DownloadStore(private val context: Context) {
    private val prefs = context.getSharedPreferences("ink-downloads", Context.MODE_PRIVATE)
    private val scheduler = context.getSystemService(JobScheduler::class.java).forNamespace("ink.downloads")
    fun read(id: String): JSONObject? = synchronized(lock) {
        prefs.getString("download.$id", null)?.let(::JSONObject)
    }
    fun write(value: JSONObject) {
        val id = value.getString("id")
        synchronized(lock) { check(prefs.edit().putString("download.$id", value.toString()).commit()) { "Could not save download state" } }
        listeners.forEach { listener -> runCatching { listener(id) } }
    }
    fun snapshot(value: JSONObject): JSONObject = JSONObject()
        .put("id", value.getString("id")).put("state", value.getString("state"))
        .put("received", value.optLong("received")).put("total", value.opt("total") ?: JSONObject.NULL)
        .put("error", value.opt("error") ?: JSONObject.NULL)
        .put("file", value.opt("file") ?: JSONObject.NULL)

    fun enqueue(input: JSONObject): JSONObject = synchronized(lock) {
        val key = input.getString("key")
        val url = input.getString("url")
        val name = input.getString("name")
        val network = input.optString("network", "connected")
        require(key.length in 1..256 && name.length in 1..256) { "Invalid download key or name" }
        validateUrl(url)
        require(network == "connected" || network == "unmetered") { "Invalid network constraint" }
        val existing = prefs.all.entries.firstOrNull {
            it.key.startsWith("download.") && JSONObject(it.value as String).optString("key") == key
        }?.value?.let { JSONObject(it as String) }
        if (existing != null) {
            require(existing.getString("url") == url && existing.getString("network") == network && existing.getString("name") == name) {
                "Download key already identifies different content"
            }
            return@synchronized snapshot(existing)
        }
        require(prefs.all.entries.count { it.key.startsWith("download.") && JSONObject(it.value as String).has("key") } < 1024) { "Too many downloads; remove old entries" }
        val jobId = prefs.getInt("nextId", 1)
        check(jobId < Int.MAX_VALUE) { "Download identifiers exhausted" }
        check(prefs.edit().putInt("nextId", jobId + 1).commit())
        val value = JSONObject().put("id", UUID.randomUUID().toString()).put("key", key)
            .put("jobId", jobId).put("url", url).put("name", name).put("network", network)
            .put("state", "queued").put("received", 0)
        write(value)
        schedule(value)
        snapshot(value)
    }
    private fun schedule(value: JSONObject) {
        val job = JobInfo.Builder(value.getInt("jobId"), ComponentName(context, InkDownloadJobService::class.java))
            .setRequiredNetworkType(if (value.getString("network") == "unmetered") JobInfo.NETWORK_TYPE_UNMETERED else JobInfo.NETWORK_TYPE_ANY)
            .setPersisted(true)
            .setExtras(PersistableBundle().apply { putString("id", value.getString("id")) })
            .build()
        if (scheduler.schedule(job) != JobScheduler.RESULT_SUCCESS) {
            write(value.put("state", "failed").put("error", "Android could not schedule the download"))
            error("Android could not schedule the download")
        }
    }
    fun control(id: String, operation: String) = synchronized(lock) {
        val value = read(id) ?: error("Download not found")
        if (operation == "resume") {
            require(value.getString("state") in listOf("paused", "failed", "queued")) { "Download cannot be resumed" }
            write(value.put("state", "queued").put("error", JSONObject.NULL))
            schedule(value)
        } else {
            if (operation == "pause") require(value.getString("state") in listOf("queued", "running", "paused")) { "Download cannot be paused" }
            if (operation == "cancel") require(value.getString("state") != "completed") { "Use remove for a completed download" }
            runs[id]?.stop()
            scheduler.cancel(value.getInt("jobId"))
            write(value.put("state", if (operation == "pause") "paused" else "cancelled"))
            if (operation != "pause") {
                partial(id).delete()
                if (operation == "remove") {
                    value.optJSONObject("file")?.let { InkManagedFiles(context).remove(it.getString("id")) }
                    write(value.put("file", JSONObject.NULL).put("received", 0))
                    check(prefs.edit().remove("download.$id").commit()) { "Could not remove download" }
                }
            }
        }
    }
    fun partial(id: String): File = File(context.filesDir, "ink-downloads/$id.part").also { it.parentFile!!.mkdirs() }
    fun validateUrl(url: String) {
        val parsed = URI(url)
        require(parsed.userInfo == null && parsed.host != null && (parsed.scheme == "https" ||
            (BuildConfig.DEBUG && parsed.scheme == "http" && parsed.host in listOf("localhost", "127.0.0.1", "10.0.2.2")))) {
            "Downloads require HTTPS"
        }
    }
    companion object {
        val lock = Any()
        val listeners = CopyOnWriteArraySet<(String) -> Unit>()
        val runs = ConcurrentHashMap<String, DownloadRun>()
    }
}

private class DownloadRun {
    val stopped = AtomicBoolean(false)
    @Volatile var connection: HttpURLConnection? = null
    fun stop() { stopped.set(true); connection?.disconnect() }
}

class InkDownloadJobService : JobService() {
    private val executor = Executors.newSingleThreadExecutor()
    private val active = ConcurrentHashMap<Int, Pair<String, DownloadRun>>()
    override fun onStartJob(params: JobParameters): Boolean {
        val id = params.extras.getString("id") ?: return false
        val run = DownloadRun()
        active[params.jobId] = id to run
        DownloadStore.runs.put(id, run)?.stop()
        executor.execute {
            try { transfer(id, run) }
            finally {
                DownloadStore.runs.remove(id, run)
                if (active.remove(params.jobId, id to run)) jobFinished(params, false)
            }
        }
        return true
    }
    override fun onStopJob(params: JobParameters): Boolean {
        val (id, run) = active.remove(params.jobId) ?: return false
        run.stop()
        val store = DownloadStore(this)
        synchronized(DownloadStore.lock) {
            store.read(id)?.takeIf { it.optString("state") == "running" }?.let { store.write(it.put("state", "queued")) }
        }
        return store.read(id)?.optString("state") in listOf("queued", "running")
    }
    override fun onDestroy() {
        active.values.forEach { it.second.stop() }
        executor.shutdown()
        super.onDestroy()
    }
    private fun transfer(id: String, run: DownloadRun) {
        val store = DownloadStore(this)
        val value = store.read(id) ?: return
        if (run.stopped.get() || value.getString("state") !in listOf("queued", "running")) return
        val partial = store.partial(id)
        var connection: HttpURLConnection? = null
        try {
            synchronized(DownloadStore.lock) {
                if (run.stopped.get()) return
                store.write(value.put("state", "running").put("error", JSONObject.NULL))
            }
            val validator = value.optString("validator")
            var offset = if (validator.isNotEmpty() && partial.exists()) partial.length() else 0L
            var url = value.getString("url")
            var responseTotal: Long? = null
            for (redirect in 0..5) {
                store.validateUrl(url)
                connection = URL(url).openConnection() as HttpURLConnection
                val current = connection!!
                run.connection = current
                current.connectTimeout = 15000
                current.readTimeout = 30000
                current.instanceFollowRedirects = false
                current.setRequestProperty("Accept-Encoding", "identity")
                if (offset > 0) {
                    current.setRequestProperty("Range", "bytes=$offset-")
                    current.setRequestProperty("If-Range", validator)
                }
                if (run.stopped.get()) return
                val status = current.responseCode
                if (status == 416 && offset > 0) {
                    // A process may stop after the final byte but before committing the file.
                    require(redirect < 5) { "Could not restart an expired range" }
                    offset = 0
                    current.disconnect()
                    continue
                }
                if (status in listOf(301, 302, 303, 307, 308)) {
                    require(redirect < 5) { "Too many redirects" }
                    url = URL(URL(url), current.getHeaderField("Location") ?: error("Missing redirect destination")).toString()
                    current.disconnect()
                    continue
                }
                require(status == 200 || status == 206) { "Download failed (HTTP $status)" }
                if (status == 206) {
                    val range = Regex("bytes ([0-9]+)-([0-9]+)/([0-9]+|\\*)").matchEntire(current.getHeaderField("Content-Range").orEmpty())
                    require(range != null && range.groupValues[1].toLongOrNull() == offset) { "Invalid partial response" }
                    val end = requireNotNull(range.groupValues[2].toLongOrNull()) { "Invalid partial response" }
                    val total = range.groupValues[3].toLongOrNull()
                    require(end >= offset && total != null && end == total - 1) { "Partial response does not contain the complete remaining file" }
                    require(current.contentLengthLong < 0 || current.contentLengthLong == end - offset + 1) { "Invalid partial response length" }
                    responseTotal = total
                } else offset = 0
                break
            }
            val current = connection ?: error("No download connection")
            val length = current.contentLengthLong
            val total = responseTotal ?: if (length >= 0) offset + length else null
            val etag = current.getHeaderField("ETag")?.takeUnless { it.startsWith("W/") }
            value.put("validator", etag ?: current.getHeaderField("Last-Modified").orEmpty())
                .put("total", total ?: JSONObject.NULL).put("received", offset)
            var received = offset
            var checkpoint = System.nanoTime()
            current.inputStream.use { input ->
                java.io.FileOutputStream(partial, offset > 0).use { output ->
                    val buffer = ByteArray(65536)
                    while (!run.stopped.get()) {
                        val size = input.read(buffer)
                        if (size < 0) break
                        output.write(buffer, 0, size)
                        received += size
                        if (System.nanoTime() - checkpoint > 1_000_000_000L) {
                            synchronized(DownloadStore.lock) {
                                if (!run.stopped.get()) store.write(value.put("received", received))
                            }
                            checkpoint = System.nanoTime()
                        }
                    }
                    output.fd.sync()
                }
            }
            if (run.stopped.get()) return
            require(total == null || received == total) { "Download ended before completion" }
            synchronized(DownloadStore.lock) {
                if (run.stopped.get()) return
                val file = File(filesDir, "ink-files/$id").also { it.parentFile!!.mkdirs() }
                check(partial.renameTo(file)) { "Could not save downloaded file" }
                try {
                    val metadata = InkManagedFiles(this).adopt(file, current.contentType?.substringBefore(';') ?: "application/octet-stream", value.getString("name"), id)
                    store.write(value.put("state", "completed").put("received", received).put("file", metadata))
                } catch (error: Exception) {
                    runCatching { InkManagedFiles(this).remove(id) }
                    file.delete()
                    value.remove("file")
                    throw error
                }
            }
        } catch (error: Exception) {
            synchronized(DownloadStore.lock) {
                if (!run.stopped.get()) store.write(value.put("state", "failed").put("error", error.message ?: "Download failed"))
            }
        } finally {
            connection?.disconnect()
            run.connection = null
            synchronized(DownloadStore.lock) {
                if (run.stopped.get() && DownloadStore.runs[id] === run && store.read(id)?.optString("state") in listOf(null, "cancelled")) partial.delete()
            }
        }
    }
}
