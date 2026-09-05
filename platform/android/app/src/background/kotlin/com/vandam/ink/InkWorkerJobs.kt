package com.vandam.ink

import android.app.job.JobInfo
import android.app.job.JobParameters
import android.app.job.JobScheduler
import android.app.job.JobService
import android.content.ComponentName
import android.content.Context
import android.os.PersistableBundle
import android.util.Log
import org.json.JSONObject
import org.json.JSONTokener
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.UUID

internal object InkWorkerJobs {
    private const val NAMESPACE = "ink.javascript"
    private val active = mutableSetOf<Int>()

    @Synchronized
    fun started(id: Int) { active.add(id) }

    @Synchronized
    fun finished(context: Context, params: JobParameters, result: JSONObject, complete: (Boolean) -> Unit) {
        active.remove(params.jobId)
        val scheduler = context.getSystemService(JobScheduler::class.java).forNamespace(NAMESPACE)
        val source = scheduler.getPendingJob(params.jobId)
        val current = source != null && source.extras.getString("generation") == params.extras.getString("generation")
        if (current) InkWorkerState.record(context, params.extras,
            when (result.getString("status")) { "success" -> "succeeded"; "retry" -> "retrying"; else -> "failed" },
            result.optString("reason").takeIf { it.isNotEmpty() }, params.extras.getString("generation"))
        if (!current || result.getString("status") != "retry") {
            complete(false)
            return
        }
        val delay = if (result.has("delayMs")) result.getLong("delayMs") else 30_000L
        require(delay in 0..31_536_000_000L) { "Invalid background retry delay" }
        val jobs = scheduler.allPendingJobs
        val key = params.extras.getString("key")
        if (jobs.any { it.id != params.jobId && !it.isPeriodic && it.extras.getString("key") == key }) {
            complete(false)
            return
        }
        val original = requireNotNull(source)
        val id = (1..256).firstOrNull { candidate -> candidate !in active && jobs.none { it.id == candidate } }
        if (id == null) {
            complete(true)
            return
        }
        val retry = JobInfo.Builder(id, original.service)
            .setExtras(PersistableBundle(original.extras).apply { putString("generation", UUID.randomUUID().toString()) })
            .setPersisted(true)
            .setRequiresCharging(original.isRequireCharging)
            .apply { original.requiredNetwork?.let { setRequiredNetwork(it) } }
            .setMinimumLatency(delay)
            .build()
        val scheduled = scheduler.schedule(retry) == JobScheduler.RESULT_SUCCESS
        if (scheduled) InkWorkerState.record(context, retry.extras, "retrying", onlyGeneration = params.extras.getString("generation"))
        complete(!scheduled)
    }

    @Synchronized
    fun stopped(id: Int) { active.remove(id) }

    @Synchronized
    fun enqueue(context: Context, request: JSONObject) {
        val key = request.getString("key")
        require(key.isNotEmpty() && key.length <= 256) { "Invalid task key" }
        val task = request.getString("task")
        require(task.matches(Regex("[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}"))) { "Invalid task ID" }
        val input = request.get("input").toString()
        val encodedInput = if (request.get("input") is String) JSONObject.quote(input) else input
        require(encodedInput.toByteArray().size <= 8192) { "Background input exceeds 8 KiB" }
        val interval = if (request.has("intervalMinutes")) request.getLong("intervalMinutes") else null
        require(interval == null || interval in 15..525600) { "Invalid background interval" }
        val constraints = request.optJSONObject("constraints") ?: JSONObject()
        require(!constraints.has("network") || constraints.getString("network") == "connected") { "Invalid network constraint" }
        val scheduler = context.getSystemService(JobScheduler::class.java).forNamespace(NAMESPACE)
        val existing = scheduler.allPendingJobs
        val matching = existing.filter { it.extras.getString("key") == key }
        val extras = PersistableBundle().apply {
            putString("key", key)
            putString("task", task)
            putString("input", encodedInput)
            putString("generation", UUID.randomUUID().toString())
        }
        fun build(id: Int, periodic: Boolean) = JobInfo.Builder(id, ComponentName(context, InkWorkerJobService::class.java))
            .setExtras(PersistableBundle(extras).apply {
                if (periodic) matching.firstOrNull { it.id == id }?.extras?.getString("generation")?.let { putString("generation", it) }
            })
            .setPersisted(true)
            .setRequiredNetworkType(if (constraints.has("network")) JobInfo.NETWORK_TYPE_ANY else JobInfo.NETWORK_TYPE_NONE)
            .setRequiresCharging(constraints.optBoolean("charging", false))
            .apply { if (periodic) setPeriodic(requireNotNull(interval) * 60_000, 5 * 60_000) else setMinimumLatency(0) }
            .build()
        val used = (existing.map { it.id } + active).toMutableSet()
        fun id(periodic: Boolean): Int = matching.firstOrNull { it.isPeriodic == periodic && (periodic || it.id !in active) }?.id
            ?: (1..256).firstOrNull { it !in used }?.also { used.add(it) }
            ?: error("Too many background jobs")
        if (interval != null) {
            val periodic = build(id(true), true)
            if (matching.none { it == periodic }) {
                check(scheduler.schedule(periodic) == JobScheduler.RESULT_SUCCESS) { "Android rejected the periodic task" }
            }
        } else matching.filter { it.isPeriodic }.forEach { scheduler.cancel(it.id) }
        check(scheduler.schedule(build(id(false), false)) == JobScheduler.RESULT_SUCCESS) { "Android rejected the task" }
        InkWorkerState.record(context, extras, "queued")
    }

    @Synchronized
    fun cancel(context: Context, key: String) {
        require(key.isNotEmpty() && key.length <= 256) { "Invalid task key" }
        val scheduler = context.getSystemService(JobScheduler::class.java).forNamespace(NAMESPACE)
        val jobs = scheduler.allPendingJobs.filter { it.extras.getString("key") == key }
        jobs.forEach { scheduler.cancel(it.id) }
        jobs.firstOrNull()?.let { InkWorkerState.record(context, it.extras, "cancelled") }
    }
}

class InkWorkerJobService : JobService() {
    private val executor = Executors.newSingleThreadExecutor()
    private val running = ConcurrentHashMap<Int, InkWorker>()

    override fun onStartJob(params: JobParameters): Boolean {
        val worker = InkWorker(applicationContext)
        running[params.jobId] = worker
        InkWorkerJobs.started(params.jobId)
        executor.execute {
            if (running[params.jobId] !== worker) return@execute
            var result = JSONObject().put("status", "failed").put("reason", "worker-error")
            try {
                InkWorkerState.record(applicationContext, params.extras, "running")
                val input = JSONTokener(params.extras.getString("input", "null")).nextValue()
                result = worker.run(params.extras.getString("task", ""), input)
                Log.i("InkWorker", "Task ${params.extras.getString("key")} returned $result")
            } catch (_: InterruptedException) {
                Log.i("InkWorker", "Task ${params.extras.getString("key")} cancelled")
            } catch (error: Exception) {
                Log.e("InkWorker", "Background task failed", error)
            } finally {
                worker.close()
                if (running.remove(params.jobId, worker)) {
                    try {
                        InkWorkerJobs.finished(applicationContext, params, result) { retry -> jobFinished(params, retry) }
                    } catch (error: Exception) {
                        Log.e("InkWorker", "Could not complete background task", error)
                        jobFinished(params, false)
                    }
                }
            }
        }
        return true
    }

    override fun onStopJob(params: JobParameters): Boolean {
        running.remove(params.jobId)?.cancel()
        InkWorkerJobs.stopped(params.jobId)
        val scheduled = applicationContext.getSystemService(JobScheduler::class.java).forNamespace("ink.javascript").getPendingJob(params.jobId)
        if (scheduled?.extras?.getString("generation") == params.extras.getString("generation")) {
            InkWorkerState.record(applicationContext, params.extras, "queued", "interrupted", params.extras.getString("generation"))
        }
        return true
    }

    override fun onDestroy() {
        running.forEach { (id, worker) ->
            worker.close()
            InkWorkerJobs.stopped(id)
        }
        running.clear()
        executor.shutdownNow()
        super.onDestroy()
    }
}
