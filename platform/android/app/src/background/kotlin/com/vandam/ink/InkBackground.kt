package com.vandam.ink

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import android.net.Uri
import android.os.PersistableBundle
import android.util.AtomicFile
import android.util.Log
import org.json.JSONObject
import org.json.JSONTokener
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.SocketTimeoutException
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.FutureTask

internal fun createBackgroundAdapter(activity: MainActivity): BackgroundAdapter =
    InkBackgroundAdapter(activity.applicationContext)

private class InkBackgroundAdapter(private val context: Context) : BackgroundAdapter {
    private val executor = Executors.newSingleThreadExecutor()
    private val requests = ConcurrentHashMap<Long, Future<*>>()

    override fun reconcile() {
        runCatching { BackgroundRegistry(context).reconcile() }
            .onFailure { Log.e(LOG_TAG, "Could not reconcile background jobs", it) }
    }

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation != PERIODIC_JSON_OPERATION) {
            complete(failure("unexpected", "Unknown background operation: $operation", false))
            return
        }
        val key = runCatching { JSONObject(payload).getString("key") }.getOrElse {
            complete(failure("unexpected", "Ink produced an invalid background request", false))
            return
        }
        val task = FutureTask {
            val result = runCatching {
                val job = BackgroundRegistry(context).jobs.singleOrNull { it.key == key }
                    ?: return@runCatching failure(
                        "unavailable",
                        "Background job $key is not installed",
                        false,
                    )
                NativeResult.Bytes(BackgroundStore(context).readState(job).toString().toByteArray())
            }.getOrElse {
                failure("storage", it.message ?: "Could not read background state", true)
            }
            requests.remove(requestId)
            complete(result)
        }
        requests[requestId] = task
        executor.execute(task)
    }

    override fun cancel(requestId: Long) {
        requests.remove(requestId)?.cancel(true)
    }

    override fun stop() {
        requests.values.forEach { it.cancel(true) }
        requests.clear()
        executor.shutdownNow()
    }
}

class InkBackgroundJobService : JobService() {
    private val executor: ExecutorService = Executors.newSingleThreadExecutor()
    private val running = ConcurrentHashMap<Int, Future<*>>()

    override fun onStartJob(params: JobParameters): Boolean {
        val task = FutureTask {
            val key = params.extras.getString(EXTRA_KEY, "")
            val job = runCatching {
                BackgroundRegistry(applicationContext).jobs.singleOrNull { it.key == key }
            }.getOrNull()
            if (job == null) {
                Log.e(LOG_TAG, "Scheduled background job $key is not in the installed registry")
            } else {
                BackgroundRunner(applicationContext).run(job)
            }
            running.remove(params.jobId)
            jobFinished(params, false)
        }
        running[params.jobId] = task
        executor.execute(task)
        return true
    }

    override fun onStopJob(params: JobParameters): Boolean {
        running.remove(params.jobId)?.cancel(true)
        return false
    }

    override fun onDestroy() {
        running.values.forEach { it.cancel(true) }
        running.clear()
        executor.shutdownNow()
        super.onDestroy()
    }
}

private class BackgroundRegistry(private val context: Context) {
    val jobs: List<BackgroundJob> by lazy {
        context.assets.open(REGISTRY_ASSET).bufferedReader().use { reader ->
            val registry = JSONObject(reader.readText())
            val values = registry.getJSONArray("jobs")
            List(values.length()) { index -> BackgroundJob(values.getJSONObject(index)) }
        }
    }

    fun reconcile() {
        val scheduler = context.getSystemService(JobScheduler::class.java)
            .forNamespace(SCHEDULER_NAMESPACE)
        val installedIds = jobs.flatMap { listOf(it.bootstrapJobId, it.periodicJobId) }.toSet()
        scheduler.allPendingJobs
            .filter { it.id !in installedIds }
            .forEach { scheduler.cancel(it.id) }
        val store = BackgroundStore(context)
        jobs.forEach { job ->
            val state = runCatching { store.readRecord(job.key) }.getOrNull()
            val needsBootstrap = state == null ||
                state.optString("requestFingerprint") != job.requestFingerprint
            if (state != null && needsBootstrap) {
                runCatching { store.writeRecord(job.key, job.emptyRecord()) }
                    .onFailure { return@forEach }
            }
            schedule(scheduler, job, periodic = true, store)
            if (needsBootstrap) {
                schedule(scheduler, job, periodic = false, store)
            }
        }
    }

    private fun schedule(
        scheduler: JobScheduler,
        job: BackgroundJob,
        periodic: Boolean,
        store: BackgroundStore,
    ) {
        val id = if (periodic) job.periodicJobId else job.bootstrapJobId
        val fingerprint = job.configFingerprint(periodic)
        val existing = scheduler.getPendingJob(id)
        if (existing?.extras?.getString(EXTRA_FINGERPRINT) == fingerprint) {
            return
        }
        if (existing != null) {
            scheduler.cancel(id)
        }
        val extras = PersistableBundle().apply {
            putString(EXTRA_KEY, job.key)
            putString(EXTRA_FINGERPRINT, fingerprint)
        }
        val builder = JobInfo.Builder(
            id,
            ComponentName(context, InkBackgroundJobService::class.java),
        )
            .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY)
            .setPersisted(true)
            .setExtras(extras)
        if (periodic) {
            builder.setPeriodic(job.everyMinutes * 60_000L)
        } else {
            builder.setMinimumLatency(0)
        }
        if (scheduler.schedule(builder.build()) != JobScheduler.RESULT_SUCCESS) {
            val error = BackgroundFailure(
                "scheduler",
                "Android rejected the background schedule for ${job.key}",
                true,
                System.currentTimeMillis(),
            )
            runCatching { store.writeFailure(job, error) }
        }
    }
}

private data class BackgroundJob(private val source: JSONObject) {
    val key: String = source.getString("key")
    val url: String = source.getString("url")
    val query: JSONObject = source.getJSONObject("query")
    val headers: JSONObject = source.getJSONObject("headers")
    val schema: Any = source.get("schema")
    val everyMinutes: Long = source.getLong("everyMinutes")
    val timeoutMs: Int = source.getInt("timeoutMs")
    val requestFingerprint: String = source.getString("requestFingerprint")
    val bootstrapJobId: Int = source.getInt("bootstrapJobId")
    val periodicJobId: Int = source.getInt("periodicJobId")

    fun emptyRecord(): JSONObject = JSONObject()
        .put("requestFingerprint", requestFingerprint)

    fun configFingerprint(periodic: Boolean): String = sha256(
        "$requestFingerprint:$timeoutMs:$everyMinutes:$periodic",
    )

    fun resolvedUrl(): String {
        val builder = Uri.parse(url).buildUpon()
        query.keys().forEach { name -> builder.appendQueryParameter(name, query.get(name).toString()) }
        return builder.build().toString()
    }
}

private class BackgroundStore(context: Context) {
    private val directory = File(context.noBackupFilesDir, STORE_DIRECTORY).apply { mkdirs() }

    fun readState(job: BackgroundJob): JSONObject {
        val record = readRecord(job.key)
        if (record == null || record.optString("requestFingerprint") != job.requestFingerprint) {
            return JSONObject().put("status", "waiting")
        }
        val success = record.optJSONObject("success")
        val error = record.optJSONObject("failure")
        return when {
            success != null && error != null -> JSONObject()
                .put("status", "stale")
                .put("value", success.get("value"))
                .put("updatedAtMs", success.getLong("updatedAtMs"))
                .put("error", error)
            success != null -> JSONObject()
                .put("status", "ready")
                .put("value", success.get("value"))
                .put("updatedAtMs", success.getLong("updatedAtMs"))
            error != null -> JSONObject().put("status", "error").put("error", error)
            else -> JSONObject().put("status", "waiting")
        }
    }

    fun readRecord(key: String): JSONObject? {
        val file = atomicFile(key)
        if (!file.baseFile.exists()) {
            return null
        }
        return file.openRead().bufferedReader().use { JSONObject(it.readText()) }
    }

    fun writeSuccess(job: BackgroundJob, value: Any) {
        val record = job.emptyRecord().put(
            "success",
            JSONObject().put("value", value).put("updatedAtMs", System.currentTimeMillis()),
        )
        writeRecord(job.key, record)
    }

    fun writeFailure(job: BackgroundJob, failure: BackgroundFailure) {
        val existing = readRecord(job.key)
        val record = if (existing?.optString("requestFingerprint") == job.requestFingerprint) {
            existing
        } else {
            job.emptyRecord()
        }
        record.put("failure", failure.json())
        writeRecord(job.key, record)
    }

    fun writeRecord(key: String, value: JSONObject) {
        val file = atomicFile(key)
        val stream = file.startWrite()
        try {
            stream.write(value.toString().toByteArray())
            file.finishWrite(stream)
        } catch (error: Throwable) {
            file.failWrite(stream)
            throw error
        }
    }

    private fun atomicFile(key: String): AtomicFile =
        AtomicFile(File(directory, "${sha256(key)}.json"))
}

private class BackgroundRunner(context: Context) {
    private val store = BackgroundStore(context)

    fun run(job: BackgroundJob) {
        var failure: BackgroundFailure? = null
        for (attempt in 0 until MAX_ATTEMPTS) {
            if (Thread.currentThread().isInterrupted) {
                return
            }
            when (val result = fetch(job)) {
                is FetchResult.Success -> {
                    runCatching { store.writeSuccess(job, result.value) }
                        .onFailure {
                            failure = BackgroundFailure(
                                "storage",
                                it.message ?: "Could not store the background response",
                                true,
                                System.currentTimeMillis(),
                            )
                        }
                    if (failure == null) {
                        return
                    }
                }
                is FetchResult.Failed -> failure = result.failure
            }
            if (!requireNotNull(failure).retryable || attempt == MAX_ATTEMPTS - 1) {
                break
            }
            try {
                Thread.sleep(RETRY_BASE_DELAY_MS shl attempt)
            } catch (_: InterruptedException) {
                Thread.currentThread().interrupt()
                return
            }
        }
        failure?.let { terminal ->
            runCatching { store.writeFailure(job, terminal) }
                .onFailure { Log.e(LOG_TAG, "Could not store background failure", it) }
        }
    }

    private fun fetch(job: BackgroundJob): FetchResult {
        val attemptedAtMs = System.currentTimeMillis()
        return try {
            var url = job.resolvedUrl()
            repeat(MAX_REDIRECTS + 1) { redirect ->
                val connection = URL(url).openConnection() as HttpURLConnection
                try {
                    connection.instanceFollowRedirects = false
                    connection.requestMethod = "GET"
                    connection.connectTimeout = job.timeoutMs
                    connection.readTimeout = job.timeoutMs
                    connection.setRequestProperty("Accept", "application/json")
                    job.headers.keys().forEach { name ->
                        connection.setRequestProperty(name, job.headers.getString(name))
                    }
                    val status = connection.responseCode
                    if (status in 300..399) {
                        val location = connection.getHeaderField("Location")
                        if (redirect == MAX_REDIRECTS || location == null) {
                            return FetchResult.Failed(
                                BackgroundFailure(
                                    "http",
                                    "Background request followed too many redirects",
                                    false,
                                    attemptedAtMs,
                                ),
                            )
                        }
                        url = URL(URL(url), location).toString()
                        if (!url.startsWith(HTTPS_PREFIX)) {
                            return FetchResult.Failed(
                                BackgroundFailure(
                                    "http",
                                    "Background redirect did not use HTTPS",
                                    false,
                                    attemptedAtMs,
                                ),
                            )
                        }
                        return@repeat
                    }
                    if (status !in 200..299) {
                        return FetchResult.Failed(
                            BackgroundFailure(
                                "http",
                                "HTTP $status ${connection.responseMessage}".trim(),
                                status >= 500 || status == 408 || status == 429,
                                attemptedAtMs,
                            ),
                        )
                    }
                    val bytes = connection.inputStream.use(::readBounded)
                    val value = JSONTokener(bytes.toString(Charsets.UTF_8)).nextValue()
                    if (!validInkJson(job.schema, value)) {
                        return FetchResult.Failed(
                            BackgroundFailure(
                                "invalid-data",
                                "Background response did not match its declared type",
                                false,
                                attemptedAtMs,
                            ),
                        )
                    }
                    return FetchResult.Success(value)
                } finally {
                    connection.disconnect()
                }
            }
            FetchResult.Failed(
                BackgroundFailure("http", "Background redirect failed", false, attemptedAtMs),
            )
        } catch (_: SocketTimeoutException) {
            FetchResult.Failed(
                BackgroundFailure("timeout", "Background request timed out", true, attemptedAtMs),
            )
        } catch (error: ResponseTooLarge) {
            FetchResult.Failed(
                BackgroundFailure("invalid-data", error.message.orEmpty(), false, attemptedAtMs),
            )
        } catch (error: org.json.JSONException) {
            FetchResult.Failed(
                BackgroundFailure(
                    "invalid-data",
                    error.message ?: "Background response was not valid JSON",
                    false,
                    attemptedAtMs,
                ),
            )
        } catch (error: IOException) {
            FetchResult.Failed(
                BackgroundFailure(
                    "unavailable",
                    error.message ?: "Background network request failed",
                    true,
                    attemptedAtMs,
                ),
            )
        } catch (error: Throwable) {
            FetchResult.Failed(
                BackgroundFailure(
                    "unexpected",
                    error.message ?: "Background request failed unexpectedly",
                    false,
                    attemptedAtMs,
                ),
            )
        }
    }
}

private sealed interface FetchResult {
    data class Success(val value: Any) : FetchResult
    data class Failed(val failure: BackgroundFailure) : FetchResult
}

private data class BackgroundFailure(
    val kind: String,
    val message: String,
    val retryable: Boolean,
    val attemptedAtMs: Long,
) {
    fun json(): JSONObject = JSONObject()
        .put("kind", kind)
        .put("message", message)
        .put("retryable", retryable)
        .put("attemptedAtMs", attemptedAtMs)
}

private fun readBounded(stream: java.io.InputStream): ByteArray {
    val output = ByteArrayOutputStream()
    val buffer = ByteArray(8 * 1024)
    var total = 0
    while (true) {
        val count = stream.read(buffer)
        if (count < 0) {
            break
        }
        total += count
        if (total > MAX_RESPONSE_BYTES) {
            throw ResponseTooLarge("Background response exceeded Ink's 1 MiB limit")
        }
        output.write(buffer, 0, count)
    }
    return output.toByteArray()
}

private fun failure(kind: String, message: String, retryable: Boolean): NativeResult.Bytes =
    NativeResult.Bytes(
        JSONObject()
            .put("status", "error")
            .put(
                "error",
                BackgroundFailure(kind, message, retryable, System.currentTimeMillis()).json(),
            )
            .toString()
            .toByteArray(),
    )

private fun sha256(value: String): String = MessageDigest.getInstance("SHA-256")
    .digest(value.toByteArray())
    .joinToString("") { "%02x".format(it) }

private class ResponseTooLarge(message: String) : IOException(message)

private const val LOG_TAG = "InkBackground"
private const val REGISTRY_ASSET = "ink-background-v1.json"
private const val STORE_DIRECTORY = "ink-background-v1"
private const val SCHEDULER_NAMESPACE = "ink.background"
private const val PERIODIC_JSON_OPERATION = "periodic-json"
private const val EXTRA_KEY = "key"
private const val EXTRA_FINGERPRINT = "fingerprint"
private const val HTTPS_PREFIX = "https://"
private const val MAX_RESPONSE_BYTES = 1024 * 1024
private const val MAX_REDIRECTS = 5
private const val MAX_ATTEMPTS = 3
private const val RETRY_BASE_DELAY_MS = 1_000L
