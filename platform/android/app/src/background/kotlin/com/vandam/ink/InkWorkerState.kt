package com.vandam.ink

import android.app.job.JobScheduler
import android.content.Context
import android.os.PersistableBundle
import org.json.JSONArray
import org.json.JSONObject

internal object InkWorkerState {
    private val listeners = mutableSetOf<() -> Unit>()

    @Synchronized fun subscribe(listener: () -> Unit) { listeners.add(listener) }
    @Synchronized fun unsubscribe(listener: () -> Unit) { listeners.remove(listener) }

    @Synchronized fun record(context: Context, extras: PersistableBundle, status: String, reason: String? = null, onlyGeneration: String? = null) {
        val key = extras.getString("key") ?: return
        val preferences = context.getSharedPreferences("ink-worker-state", Context.MODE_PRIVATE)
        val previous = JSONArray(preferences.getString("jobs", "[]"))
        val existing = (0 until previous.length()).map { previous.getJSONObject(it) }.firstOrNull { it.getString("key") == key }
        if (onlyGeneration != null && existing?.optString("generation") != onlyGeneration) return
        if (status == "running" && existing != null && existing.optString("generation").isNotEmpty() && existing.optString("generation") != extras.getString("generation")
            && existing.getString("status") in setOf("queued", "running", "retrying")) return
        val next = JSONArray()
        next.put(JSONObject().put("key", key).put("task", extras.getString("task"))
            .put("generation", extras.getString("generation")).put("status", status).put("updatedAt", System.currentTimeMillis())
            .put("reason", reason ?: JSONObject.NULL))
        for (index in 0 until previous.length()) {
            val entry = previous.getJSONObject(index)
            if (entry.getString("key") != key && next.length() < 256) next.put(entry)
        }
        check(preferences.edit().putString("jobs", next.toString()).putLong("revision", preferences.getLong("revision", 0) + 1).commit()) {
            "Could not save background task state"
        }
        listeners.toList().forEach { it() }
    }

    @Synchronized fun snapshot(context: Context): JSONObject {
        val preferences = context.getSharedPreferences("ink-worker-state", Context.MODE_PRIVATE)
        val history = JSONArray(preferences.getString("jobs", "[]"))
        val saved = (0 until history.length()).map { history.getJSONObject(it) }.associateBy { it.getString("key") }.toMutableMap()
        val pending = context.getSystemService(JobScheduler::class.java).forNamespace("ink.javascript").allPendingJobs.filterNot { job ->
            val recorded = saved[job.extras.getString("key")]
            !job.isPeriodic && recorded?.optString("generation") == job.extras.getString("generation")
                && recorded?.optString("status") in setOf("succeeded", "failed", "cancelled")
        }
        val jobs = JSONArray()
        pending.groupBy { it.extras.getString("key") }.forEach { (key, scheduled) ->
            if (key == null || jobs.length() == 256) return@forEach
            val entry = saved.remove(key) ?: JSONObject().put("key", key).put("task", scheduled.first().extras.getString("task"))
                .put("status", "queued").put("updatedAt", 0).put("reason", JSONObject.NULL)
            jobs.put(entry.put("scheduled", true).put("periodic", scheduled.any { it.isPeriodic }))
        }
        saved.values.take(256 - jobs.length()).forEach { jobs.put(it.put("scheduled", false).put("periodic", false)) }
        return JSONObject().put("revision", preferences.getLong("revision", 0)).put("jobs", jobs)
    }
}
