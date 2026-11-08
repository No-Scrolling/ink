package com.vandam.ink

import android.Manifest
import android.app.Activity
import android.app.Instrumentation
import android.app.Notification
import android.app.NotificationManager
import android.content.Intent
import org.json.JSONObject
import java.io.File
import java.net.InetAddress
import java.net.ServerSocket
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

class MainActivity : Activity()

internal class PushContracts(private val instrumentation: Instrumentation) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("null Light push fetches a plaintext inbox without opting in") { receive("null") }
        test("empty Light push fetches a plaintext inbox without opting in") { receive("") }
        test("unparseable Light push fetches a plaintext inbox") { receive("not an envelope") }
        test("persisted legacy wake flags are ignored") { receive("{}", legacy = false); receive("{}", legacy = true) }
        test("encrypted inbox and direct delivery acknowledge the ciphertext hash") { receive("null", encrypted = true); receive("", encrypted = true, direct = true) }
        test("wake without an inbox is dropped without crashing", ::noInbox)
    }

    private val context get() = instrumentation.targetContext
    private val stateFile get() = File(context.noBackupFilesDir, "ink-light-push-v1.json")
    private val notifications get() = context.getSystemService(NotificationManager::class.java)

    private fun withAdapter(run: (LightPushAdapter) -> Unit) {
        stateFile.delete()
        SecureValues(context).remove("ink.push.key")
        instrumentation.uiAutomation.grantRuntimePermission(context.packageName, Manifest.permission.POST_NOTIFICATIONS)
        val activity = instrumentation.startActivitySync(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        val adapter = createLightPushAdapter(activity, {}, { _, _ -> })
        try {
            instrumentation.runOnMainSync { adapter.executeController(1, "activate", """{"kind":"light-push"}""") }
            run(adapter)
        } finally {
            instrumentation.runOnMainSync { adapter.stop(); activity.finish() }
            notifications.cancelAll()
            SecureValues(context).remove("ink.push.key")
            stateFile.delete()
        }
    }

    private fun receive(body: String, encrypted: Boolean = false, direct: Boolean = false, legacy: Boolean? = null) = withAdapter { adapter ->
        val key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
        val envelope = """{"version":1,"events":[{"operation":"show","id":"event-42","groupKey":"thread-7","title":"Inbox contract","body":"Fetched from the subscription server","route":"/inbox"}]}"""
        // Fixed AES-GCM fixture generated independently with Node crypto.
        val ciphertext = "ink1.AQEBAQEBAQEBAQEBDmK1Dwj6EZIMlN63lw0ZH9kLmCgt3G5NLdph1Y2O04XvRoVn2AiEjD+wwFz1DOJCwDGWW9zKULtXBlRfKanVe7hggMADuPaIHDTfQL5lwEI0TRYlMkaJrZ9/C8cK6h2Cg6cY32sj6XsCx0p1jv65FP1iTt6DlQf9EDVIYExOvpnveMkCAUAviEZiVbxQZ9rbpOafiU2b0a0VSlCqy23vKlWYedS9Tow4HIHRih/US2pqlG+GiJEEMmtLvQ=="
        val receipt = if (encrypted) "ec530a3cc6ce72be5cd00b793e3a203ad4c4557d3cb6c6dfe94f5c3d80c0d8ef" else "receipt-42"
        val entry = if (encrypted) JSONObject().put("encrypted", ciphertext) else JSONObject(envelope).put("id", receipt)
        val requests = LinkedBlockingQueue<Triple<String, Map<String, String>, String>>()
        val errors = LinkedBlockingQueue<Throwable>()
        val server = ServerSocket(0, 2, InetAddress.getByName("127.0.0.1")).apply { soTimeout = 10_000 }
        val thread = Thread {
            try {
                repeat(if (direct) 1 else 2) { index ->
                    server.accept().use { socket ->
                        socket.soTimeout = 10_000
                        val reader = socket.getInputStream().bufferedReader()
                        val line = reader.readLine()
                        val headers = mutableMapOf<String, String>()
                        while (true) {
                            val header = reader.readLine(); if (header.isEmpty()) break
                            headers[header.substringBefore(':').lowercase()] = header.substringAfter(':').trim()
                        }
                        val bytes = CharArray(headers["content-length"]?.toInt() ?: 0)
                        var offset = 0
                        while (offset < bytes.size) { val count = reader.read(bytes, offset, bytes.size - offset); check(count > 0); offset += count }
                        requests.add(Triple(line, headers, String(bytes)))
                        val response = if (!direct && index == 0) """{"messages":[$entry]}""" else """{"success":true}"""
                        socket.getOutputStream().write(("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ${response.toByteArray().size}\r\nConnection: close\r\n\r\n$response").toByteArray())
                    }
                }
            } catch (error: Throwable) { errors.add(error) }
        }.apply { start() }
        try {
            val registration = JSONObject().put("url", "http://127.0.0.1:${server.localPort}/v1/push/subscriptions")
                .put("bearerToken", "contract-token")
            if (encrypted) registration.put("encryptionKey", key)
            instrumentation.runOnMainSync { adapter.executeController(1, "register", registration.toString()) }
            val state = JSONObject(stateFile.readText())
            check(state.getBoolean("desired")) { "Registration rejected: $state" }
            check(!state.has("fetchOnWake"))
            if (legacy != null) stateFile.writeText(state.put("fetchOnWake", legacy).toString())
            check((SecureValues(context).get("ink.push.key") != null) == encrypted)
            receiveLightPush(context, (if (direct) entry.toString() else body).toByteArray())
            run {
                val path = "/v1/push/subscriptions/${state.getString("installationId")}"
                if (!direct) {
                    val get = checkNotNull(requests.poll(10, TimeUnit.SECONDS)) { "Inbox GET did not arrive: ${errors.peek()}" }
                    check(get.first == "GET $path HTTP/1.1") { get.first }
                    check(get.second["authorization"] == "Bearer contract-token")
                }
                val post = checkNotNull(requests.poll(10, TimeUnit.SECONDS)) { "Acknowledgement did not arrive: ${errors.peek()}" }
                check(post.first == "POST $path HTTP/1.1") { post.first }
                check(post.second["authorization"] == "Bearer contract-token")
                check(JSONObject(post.third).getJSONArray("ids").toString() == """["$receipt"]""") { post.third }
                val notification = notifications.activeNotifications.single { it.tag == "ink-push:thread-7" }.notification
                check(notification.extras.getString(Notification.EXTRA_TITLE) == "Inbox contract")
                check(notification.extras.getString(Notification.EXTRA_TEXT) == "Fetched from the subscription server")
                val saved = JSONObject(stateFile.readText()).getJSONArray("messages").getJSONObject(0)
                check(saved.getString("id") == "event-42")
                check(saved.getString("route") == "/inbox")
            }
            thread.join(5_000)
            check(!thread.isAlive)
            errors.poll()?.let { throw it }
        } finally { server.close(); thread.join(5_000) }
    }

    private fun noInbox() = withAdapter {
        receiveLightPush(context, "null".toByteArray())
        run {
            val state = JSONObject(stateFile.readText())
            check(state.getString("status") == "error")
            check(state.getJSONObject("error").getString("message").contains("no inbox is configured"))
            check(notifications.activeNotifications.isEmpty())
        }
    }
}
