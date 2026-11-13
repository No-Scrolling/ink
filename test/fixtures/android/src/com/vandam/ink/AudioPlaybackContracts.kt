package com.vandam.ink

import android.app.Instrumentation
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import androidx.media3.common.util.UnstableApi
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import org.json.JSONArray
import org.json.JSONObject

@UnstableApi
internal class AudioPlaybackContracts(private val instrumentation: Instrumentation) {
    private val context = instrumentation.targetContext

    fun run(test: (String, () -> Unit) -> Unit) {
        test("audio state coalesces player events and sends the queue only when it changes", ::queueUpdates)
        test("detached audio sends the restored queue after connecting", ::detachedQueue)
    }

    private fun equal(expected: Any?, actual: Any?) {
        check(expected == actual) { "Expected <$expected>, got <$actual>" }
    }

    private fun <T> onMain(action: () -> T): T {
        var result: Result<T>? = null
        instrumentation.runOnMainSync { result = runCatching(action) }
        return checkNotNull(result).getOrThrow()
    }

    private fun item(id: String, title: String = id) =
        JSONObject().put("id", id).put("src", "content://com.vandam.ink.contracttests.audio/$id").put("title", title)

    private fun ids(state: JSONObject): List<String> {
        val queue = state.getJSONArray("queue")
        return List(queue.length()) { queue.getJSONObject(it).getString("id") }
    }

    private fun clearQueue(name: String) {
        context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE).edit().remove(name).commit()
        context.getSharedPreferences("ink-audio-progress", Context.MODE_PRIVATE).edit().remove(name).commit()
    }

    private fun writeAudio(): File {
        val file = File(context.cacheDir, "audio-content-contract.wav")
        val pcmBytes = 30 * 8000 * 2
        val header = ByteBuffer.allocate(44).order(ByteOrder.LITTLE_ENDIAN)
            .put("RIFF".toByteArray()).putInt(36 + pcmBytes).put("WAVEfmt ".toByteArray())
            .putInt(16).putShort(1).putShort(1).putInt(8000).putInt(16000).putShort(2).putShort(16)
            .put("data".toByteArray()).putInt(pcmBytes).array()
        file.outputStream().use { it.write(header); it.write(ByteArray(pcmBytes)) }
        return file
    }

    private val clock = object : PlaybackClock {
        override fun updatePlaybackClock(controller: Long, position: Long, duration: Long, playing: Boolean, speed: Float) = Unit
        override fun removePlaybackClock(controller: Long) = Unit
    }

    private fun queueUpdates() {
        val name = "publish-contract"
        clearQueue(name)
        val file = writeAudio()
        val activity = instrumentation.startActivitySync(Intent(context, PermissionContractActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        // Only the main thread publishes and reads.
        val sent = mutableListOf<Pair<Long, JSONObject>>()
        val playback = onMain { createAudioPlayback(activity, { controller, value -> sent += controller to JSONObject(value) }, InkBundle(activity, "contract"), clock) }
        val config = JSONObject().put("session", name).toString()
        fun since(start: Int) = onMain { sent.drop(start) }
        // Runs a command and returns the states published before it completed and after the main looper drained.
        fun command(operation: String, payload: JSONObject = JSONObject()): Pair<List<Pair<Long, JSONObject>>, List<Pair<Long, JSONObject>>> {
            val start = onMain { sent.size }
            val results = LinkedBlockingQueue<Pair<NativeResult, Int>>()
            onMain { playback.execute(1, operation, payload.toString()) { results.add(it to sent.size) } }
            val (result, completed) = checkNotNull(results.poll(10, TimeUnit.SECONDS)) { "$operation did not complete" }
            check(result is NativeResult.Success) { "$operation failed: $result" }
            return since(start).take(completed - start) to since(start)
        }
        // Edits publish one state that carries the edited queue and a new revision.
        fun edit(operation: String, payload: JSONObject, expected: List<String>, revision: Int): JSONObject {
            val (beforeCompletion, drained) = command(operation, payload)
            equal(1, beforeCompletion.size)
            check(drained.drop(1).none { it.second.has("queue") }) { "$operation sent its queue twice" }
            val state = beforeCompletion.single().second
            equal(expected, ids(state))
            check(state.getInt("queueRevision") > revision) { "$operation kept queue revision $revision" }
            return state
        }
        try {
            onMain { playback.activate(1, config) }
            val activated = since(0)
            equal(1, activated.size)
            equal(emptyList<String>(), ids(activated.single().second))
            var revision = activated.single().second.getInt("queueRevision")

            revision = edit("setQueue", JSONObject().put("items", JSONArray(listOf(item("one"), item("two"), item("three")))).put("prepare", false),
                listOf("one", "two", "three"), revision).getInt("queueRevision")
            revision = edit("add", JSONObject().put("item", item("four")), listOf("one", "two", "three", "four"), revision).getInt("queueRevision")
            revision = edit("move", JSONObject().put("id", "four").put("index", 0), listOf("four", "one", "two", "three"), revision).getInt("queueRevision")
            val replaced = edit("replace", JSONObject().put("id", "four").put("item", item("four", "Renamed")), listOf("four", "one", "two", "three"), revision)
            equal("Renamed", replaced.getJSONArray("queue").getJSONObject(0).getString("title"))
            revision = edit("remove", JSONObject().put("id", "four"), listOf("one", "two", "three"), replaced.getInt("queueRevision")).getInt("queueRevision")

            val (shuffled, _) = command("setShuffle", JSONObject().put("value", true))
            equal(1, shuffled.size)
            equal(true, shuffled.single().second.getBoolean("shuffle"))
            equal(false, shuffled.single().second.has("queue"))
            equal(revision, shuffled.single().second.getInt("queueRevision"))

            var start = onMain { sent.size }
            onMain { playback.activate(2, config) }
            val attached = since(start).toMap()
            equal(setOf(1L, 2L), attached.keys)
            equal(false, attached.getValue(1).has("queue"))
            equal(listOf("one", "two", "three"), ids(attached.getValue(2)))
            equal(revision, attached.getValue(2).getInt("queueRevision"))

            val state = LinkedBlockingQueue<NativeResult>()
            onMain { playback.execute(2, "getState", "") { state.add(it) } }
            val current = state.poll(10, TimeUnit.SECONDS)
            check(current is NativeResult.Success) { "getState failed: $current" }
            equal(listOf("one", "two", "three"), ids(JSONObject(current.value)))

            // Preparing the source updates the timeline without changing the queue.
            start = onMain { sent.size }
            command("play")
            val deadline = SystemClock.elapsedRealtime() + 10_000
            while (since(start).none { it.second.getString("status") == "playing" }) {
                check(SystemClock.elapsedRealtime() < deadline) { "Playback did not start" }
                SystemClock.sleep(20)
            }
            val playing = since(start).map { it.second }
            check(playing.any { it.getLong("durationMs") == 30_000L }) { "Playback states never reported the prepared source duration" }
            check(playing.none { it.has("queue") || it.getInt("queueRevision") != revision }) { "A source update resent the queue" }

            // Pausing changes play intent, playing state and playback state together.
            val (before, after) = onMain {
                val before = sent.size
                playback.pause()
                before to sent.size
            }
            equal(before, after)
            val drained = since(before)
            equal(listOf(1L, 2L), drained.map { it.first }.sorted())
            drained.forEach { (_, value) ->
                equal(false, value.getBoolean("playWhenReady"))
                equal(false, value.has("queue"))
            }

            onMain { playback.deactivate(2) }
            start = onMain { sent.size }
            onMain { playback.activate(2, config) }
            equal(listOf("one", "two", "three"), ids(since(start).toMap().getValue(2)))
        } finally {
            onMain { playback.stop() }
            instrumentation.runOnMainSync { activity.finish() }
            clearQueue(name)
            file.delete()
        }
    }

    private fun detachedQueue() {
        val name = "publish-detached-contract"
        clearQueue(name)
        context.getSharedPreferences("ink-audio-queues", Context.MODE_PRIVATE).edit().putString(name, JSONArray(listOf("first", "second").map { id ->
            JSONObject().put("uri", "content://com.vandam.ink.contracttests.audio/$id").put("id", id).put("source", "content://com.vandam.ink.contracttests.audio/$id")
        }).toString()).commit()
        val activity = instrumentation.startActivitySync(Intent(context, PermissionContractActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        val sent = mutableListOf<JSONObject>()
        val playback = onMain { createAudioPlayback(activity, { _, value -> sent += JSONObject(value) }, InkBundle(activity, "contract"), clock) }
        try {
            onMain { playback.activate(1, JSONObject().put("session", name).put("playback", "detached").toString()) }
            val deadline = SystemClock.elapsedRealtime() + 10_000
            while (onMain { sent.none { it.getBoolean("ready") } }) {
                check(SystemClock.elapsedRealtime() < deadline) { "Detached playback did not connect" }
                SystemClock.sleep(20)
            }
            val states = onMain { sent.toList() }
            equal(emptyList<String>(), ids(states.first()))
            val connected = states.first { it.getBoolean("ready") }
            check(connected.getInt("queueRevision") != states.first().getInt("queueRevision")) { "Connecting kept the queue revision" }
            equal(listOf("first", "second"), ids(connected))
        } finally {
            onMain { playback.stop() }
            instrumentation.runOnMainSync { activity.finish() }
            context.stopService(Intent(context, InkAudioService::class.java))
            clearQueue(name)
        }
    }
}
