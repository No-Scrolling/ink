package com.vandam.ink

import android.content.ComponentName
import android.content.Context
import android.nfc.cardemulation.CardEmulation
import android.nfc.cardemulation.HostApduService
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import org.json.JSONObject

internal class InkHostApduService : HostApduService() {
    private var savedConfig: String? = null
    private var responseConfig: JSONObject? = null

    override fun processCommandApdu(commandApdu: ByteArray, extras: Bundle?): ByteArray? {
        if (commandApdu.size !in 1..4096) return byteArrayOf(0x67, 0x00)
        val saved = getSharedPreferences("ink-nfc-hce", Context.MODE_PRIVATE).getString("config", null)
        if (saved != savedConfig) {
            savedConfig = saved
            responseConfig = saved?.let { runCatching { JSONObject(it) }.getOrNull() }
        }
        val config = responseConfig ?: return byteArrayOf(0x6D, 0x00)
        val command = commandApdu.joinToString("") { "%02X".format(it) }
        val fallback = InkApduBridge.bytes(config.getJSONObject("responses").optString(command, config.optString("fallback", "6D00")))
        return if (InkApduBridge.dispatch(command, config.optInt("deadline", 500), fallback, ::sendResponseApdu)) null else fallback
    }

    override fun onDeactivated(reason: Int) { InkApduBridge.deactivated() }
}

internal object InkApduBridge {
    private val handler = Handler(Looper.getMainLooper())
    private var listener: Pair<Long, NativeResultHandler>? = null
    private var reply: ((ByteArray) -> Unit)? = null
    private var sequence = 0L
    private var deadline: Runnable? = null
    private var fallback = byteArrayOf(0x6D, 0x00)

    fun execute(context: Context, requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        runCatching {
            when (operation) {
                "emulate" -> {
                    val input = JSONObject(payload)
                    val aids = input.getJSONArray("aids").let { array -> List(array.length()) { array.getString(it).uppercase() } }
                    require(aids.size in 1..32 && aids.all { Regex("[0-9A-F]{10,32}").matches(it) && it.length % 2 == 0 }) { "Use 1–32 hexadecimal AIDs of 5–16 bytes" }
                    val responses = JSONObject()
                    val rules = input.getJSONArray("responses")
                    require(rules.length() <= 256) { "At most 256 APDU responses are supported" }
                    repeat(rules.length()) { index ->
                        val rule = rules.getJSONObject(index)
                        val command = rule.getString("command").uppercase()
                        val response = rule.getString("response").uppercase()
                        require(bytes(command).size in 1..4096 && bytes(response).size in 2..4096) { "Invalid APDU response rule" }
                        responses.put(command, response)
                    }
                    val fallback = input.optString("fallback", "6D00").uppercase()
                    require(bytes(fallback).size in 2..4096) { "Invalid APDU fallback" }
                    val deadline = input.optInt("deadline", 500)
                    require(deadline in 50..2000) { "APDU deadline must be 50–2000 milliseconds" }
                    val adapter = android.nfc.NfcAdapter.getDefaultAdapter(context)
                        ?: error("NFC hardware is unavailable")
                    check(context.packageManager.hasSystemFeature("android.hardware.nfc.hce")) { "Host card emulation is unavailable" }
                    val service = ComponentName(context, InkHostApduService::class.java)
                    check(CardEmulation.getInstance(adapter).registerAidsForService(service, CardEmulation.CATEGORY_OTHER, aids)) { "AIDs could not be registered" }
                    val config = JSONObject().put("responses", responses).put("fallback", fallback).put("deadline", deadline)
                    context.getSharedPreferences("ink-nfc-hce", Context.MODE_PRIVATE).edit().putString("config", config.toString()).apply()
                    complete(NativeResult.Success(""))
                }
                "stop-emulation" -> {
                    android.nfc.NfcAdapter.getDefaultAdapter(context)?.let { adapter ->
                        CardEmulation.getInstance(adapter).removeAidsForService(ComponentName(context, InkHostApduService::class.java), CardEmulation.CATEGORY_OTHER)
                    }
                    context.getSharedPreferences("ink-nfc-hce", Context.MODE_PRIVATE).edit().remove("config").apply()
                    stopHandler()
                    complete(NativeResult.Success(""))
                }
                "apdu-next" -> {
                    check(listener == null) { "An APDU handler is already waiting" }
                    listener = requestId to complete
                }
                "apdu-response" -> {
                    val input = JSONObject(payload)
                    check(input.getLong("id") == sequence && reply != null) { "APDU request is no longer active" }
                    val response = bytes(input.getString("response"))
                    require(response.size in 2..4096) { "APDU response must contain 2–4096 bytes" }
                    finish(response)
                    complete(NativeResult.Success(""))
                }
                "apdu-stop" -> { stopHandler(); complete(NativeResult.Success("")) }
                else -> error("Unknown card emulation operation")
            }
        }.onFailure { complete(NativeResult.Failure(if (it is IllegalArgumentException) NativeErrorKind.PROTOCOL else NativeErrorKind.UNAVAILABLE, it.message ?: "Card emulation failed", false)) }
    }

    fun dispatch(command: String, delay: Int, fallback: ByteArray, send: (ByteArray) -> Unit): Boolean {
        val waiting = listener ?: return false
        listener = null
        deactivated()
        this.fallback = fallback
        reply = send
        val id = ++sequence
        deadline = Runnable { if (id == sequence) finish(fallback) }.also { handler.postDelayed(it, delay.toLong()) }
        waiting.second(NativeResult.Success(JSONObject().put("id", id).put("command", command).toString()))
        return true
    }

    fun cancel(requestId: Long) { if (listener?.first == requestId) listener = null }

    fun stopHandler() {
        val waiting = listener
        listener = null
        waiting?.second?.invoke(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "APDU handler stopped", false))
        if (reply != null) finish(fallback)
    }

    fun deactivated() {
        deadline?.let(handler::removeCallbacks)
        deadline = null
        reply = null
    }

    private fun finish(response: ByteArray) {
        val send = reply
        deactivated()
        send?.invoke(response)
    }

    fun bytes(hex: String): ByteArray {
        require(hex.length % 2 == 0 && hex.all { it in '0'..'9' || it.uppercaseChar() in 'A'..'F' }) { "Expected hexadecimal APDU bytes" }
        return ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
    }
}
