package com.vandam.ink

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.nio.file.Files
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

internal class ToolManagerContracts(private val context: Context) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("shared files preserve binary bytes, ranges and modification times") {
            withAdapter { adapter ->
                val bytes = byteArrayOf(0, -1, -128, 13, 10, 65)
                call(adapter, "write", JSONObject().put("path", "contract/music.bin"), bytes)
                val file = File(context.filesDir, "shared/contract/music.bin")
                check(file.readBytes().contentEquals(bytes))
                check(file.setLastModified(1000))
                val before = System.currentTimeMillis()
                call(adapter, "write", JSONObject().put("path", "contract/music.bin"), byteArrayOf(42, 0, -1, 99))
                check(file.lastModified() >= before) { "Rewrites must advance an old modification time" }
                val read = call(adapter, "read", JSONObject().put("path", "contract/music.bin").put("offset", 1).put("length", 2))
                check(read is NativeResult.Binary && read.bytes.contentEquals(byteArrayOf(0, -1)))
                val large = File(context.filesDir, "ink-files/tool-manager-source.bin").apply {
                    parentFile!!.mkdirs()
                    outputStream().use { output -> repeat(262145) { output.write(byteArrayOf(0, -1, 42, -128)) } }
                }
                val managed = InkManagedFiles(context).adopt(large, "application/octet-stream")
                try {
                    call(adapter, "copy-from", JSONObject().put("path", "contract/copied.bin").put("source", managed.getString("src")))
                    check(File(context.filesDir, "shared/contract/copied.bin").readBytes().contentEquals(large.readBytes()))
                    check(call(adapter, "read", JSONObject().put("path", "contract/copied.bin").put("length", 524289)) is NativeResult.Failure)
                } finally { InkManagedFiles(context).remove(managed.getString("id")) }
                val list = call(adapter, "list", JSONObject().put("path", "contract"))
                check(list is NativeResult.Success)
                val entries = JSONArray(list.value)
                val entry = (0 until entries.length()).map { entries.getJSONObject(it) }.single { it.getString("name") == "music.bin" }
                check(entry.getString("path") == "contract/music.bin" && entry.getLong("size") == 4L)
                check(entry.getString("uri") == "content://com.vandam.ink.contracttests.lightfileprovider/contract/music.bin")
            }
        }
        test("shared paths reject sibling traversal, symlinks and root deletion") {
            withAdapter { adapter ->
                val sentinel = File(context.filesDir, "shared-sibling/sentinel.txt").apply { parentFile!!.mkdirs(); writeText("keep") }
                val link = File(context.filesDir, "shared/contract-link")
                Files.createSymbolicLink(link.toPath(), sentinel.parentFile!!.toPath())
                try {
                    for (path in listOf("../shared-sibling/sentinel.txt", "contract-link/sentinel.txt", "/tmp/file", "")) {
                        val result = call(adapter, "remove", JSONObject().put("path", path))
                        check(result is NativeResult.Failure) { "Accepted unsafe path: $path" }
                    }
                    check(sentinel.readText() == "keep")
                } finally { link.delete(); sentinel.parentFile!!.deleteRecursively() }
            }
        }
        test("shared file observation survives restarts, sees local writes and releases cancelled waits") {
            withAdapter { adapter ->
                val preferences = context.getSharedPreferences("ink-tool-manager", Context.MODE_PRIVATE)
                preferences.edit().putLong("revision", 7).commit()
                val first = call(adapter, "observe", JSONObject().put("revision", -1))
                check(first is NativeResult.Success && first.value == "7")
                val results = LinkedBlockingQueue<NativeResult>()
                adapter.execute(100, "observe", """{"revision":7}""") { results.add(it) }
                adapter.cancel(100)
                preferences.edit().putLong("revision", 8).commit()
                check(call(adapter, "observe", JSONObject().put("revision", 7)) == NativeResult.Success("8"))
                check(results.poll(100, TimeUnit.MILLISECONDS) == null) { "Cancelled observer completed" }
                adapter.execute(101, "observe", """{"revision":8}""") { results.add(it) }
                check(call(adapter, "write-text", JSONObject().put("path", "contract/local.txt").put("text", "Local change")) is NativeResult.Success)
                check(results.poll(5, TimeUnit.SECONDS) == NativeResult.Success("9")) { "Local write did not refresh observers" }
                check(File(context.filesDir, "shared/contract/local.txt").readText() == "Local change")
                val restarted = createToolManagerAdapter(context)
                try { check(call(restarted, "observe", JSONObject().put("revision", -1)) == NativeResult.Success("9")) }
                finally { restarted.stop() }
            }
        }
    }

    private fun withAdapter(run: (ToolManagerAdapter) -> Unit) {
        val adapter = createToolManagerAdapter(context)
        try { run(adapter) }
        finally { adapter.stop(); File(context.filesDir, "shared").deleteRecursively() }
    }

    private var next = 0L
    private fun call(adapter: ToolManagerAdapter, operation: String, payload: JSONObject, bytes: ByteArray? = null): NativeResult {
        val results = LinkedBlockingQueue<NativeResult>()
        val id = ++next
        if (bytes == null) adapter.execute(id, operation, payload.toString()) { results.add(it) }
        else adapter.executeBytes(id, operation, payload.toString(), bytes) { results.add(it) }
        return results.poll(5, TimeUnit.SECONDS) ?: error("Tool Manager operation timed out: $operation")
    }
}
