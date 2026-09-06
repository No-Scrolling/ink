package com.vandam.ink

import android.nfc.NfcAdapter
import android.nfc.Tag
import android.nfc.tech.IsoDep
import android.nfc.tech.NfcA
import android.nfc.tech.TagTechnology
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicReference
import org.json.JSONArray
import org.json.JSONObject

internal class InkNfcConnection(private val activity: MainActivity) {
    private val adapter = NfcAdapter.getDefaultAdapter(activity)
    private val executor = Executors.newFixedThreadPool(2)
    private val technology = AtomicReference<TagTechnology?>(null)
    private var pending: Pair<Long, NativeResultHandler>? = null
    @Volatile private var generation = 0L
    private var identifier: String? = null
    private var readerEnabled = false
    val active: Boolean get() = readerEnabled || identifier != null || pending != null

    fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val body = runCatching { JSONObject(payload) }.getOrElse {
            complete(error("Invalid NFC connection request")); return
        }
        when (operation) {
            "connect" -> {
                if (active) { complete(error("An NFC connection is already active")); return }
                if (adapter == null || !adapter.isEnabled) {
                    complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "NFC is unavailable or disabled", true)); return
                }
                val kind = body.optString("technology")
                if (kind != "iso-dep" && kind != "nfc-a") { complete(error("Unsupported NFC technology")); return }
                pending = requestId to complete
                val token = ++generation
                runCatching {
                    adapter.enableReaderMode(activity, { tag -> connect(tag, kind, token) },
                        NfcAdapter.FLAG_READER_NFC_A or NfcAdapter.FLAG_READER_NFC_B or NfcAdapter.FLAG_READER_SKIP_NDEF_CHECK, null)
                    readerEnabled = true
                }.onFailure { finish(error(it.message ?: "NFC reader could not start")); close() }
            }
            "transceive" -> {
                val selected = technology.get()
                if (selected == null || body.optString("connection") != identifier || pending != null) {
                    complete(error("NFC connection is unavailable or busy")); return
                }
                val bytes = runCatching {
                    val values = body.getJSONArray("bytes")
                    require(values.length() in 1..65_536) { "NFC commands must contain 1–65536 bytes" }
                    ByteArray(values.length()) { index ->
                        val byte = values.getInt(index)
                        require(byte in 0..255) { "Invalid NFC command byte" }
                        byte.toByte()
                    }
                }.getOrElse { complete(error(it.message ?: "Invalid NFC command")); return }
                val timeout = body.optInt("timeout", 5000).coerceIn(1, 10_000)
                pending = requestId to complete
                val token = generation
                executor.execute {
                    val result = runCatching {
                        val response = when (selected) {
                            is IsoDep -> { selected.timeout = timeout; selected.transceive(bytes) }
                            is NfcA -> { selected.timeout = timeout; selected.transceive(bytes) }
                            else -> kotlin.error("Unsupported NFC technology")
                        }
                        require(response.size <= 65_536) { "NFC response exceeds 64 KiB" }
                        NativeResult.Success(JSONArray(response.map { it.toInt() and 255 }).toString())
                    }.getOrElse { NativeResult.Failure(NativeErrorKind.UNAVAILABLE, it.message ?: "NFC exchange failed", true) }
                    activity.runOnUiThread {
                        if (token == generation) {
                            finish(result)
                            if (result is NativeResult.Failure) close()
                        }
                    }
                }
            }
            "close" -> {
                if (body.optString("connection") == identifier) close()
                complete(NativeResult.Success(""))
            }
            else -> complete(error("Unknown NFC connection operation"))
        }
    }

    fun cancel(requestId: Long) { if (pending?.first == requestId) close() }

    fun close() {
        generation++
        pending?.second?.invoke(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "NFC connection closed", true))
        pending = null
        identifier = null
        if (readerEnabled) runCatching { adapter?.disableReaderMode(activity) }
        readerEnabled = false
        technology.getAndSet(null)?.let { selected -> executor.execute { runCatching { selected.close() } } }
    }

    fun stop() { close(); executor.shutdown() }

    private fun connect(tag: Tag, kind: String, token: Long) {
        activity.runOnUiThread {
            if (token != generation || technology.get() != null) return@runOnUiThread
            val selected = if (kind == "iso-dep") IsoDep.get(tag) else NfcA.get(tag)
            if (selected == null) return@runOnUiThread
            technology.set(selected)
            executor.execute {
                val result = runCatching {
                    selected.connect()
                    if (token != generation) { selected.close(); kotlin.error("NFC connection was cancelled") }
                    val maximum = when (selected) {
                        is IsoDep -> selected.maxTransceiveLength
                        is NfcA -> selected.maxTransceiveLength
                        else -> 0
                    }
                    JSONObject().put("connection", token.toString()).put("technology", kind)
                        .put("serialNumber", tag.id.joinToString("") { "%02X".format(it) })
                        .put("maxTransceiveLength", maximum).toString()
                }
                activity.runOnUiThread {
                    if (token != generation) return@runOnUiThread
                    result.onSuccess {
                        identifier = token.toString()
                        finish(NativeResult.Success(it))
                    }.onFailure {
                        finish(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, it.message ?: "NFC connection failed", true))
                        close()
                    }
                }
            }
        }
    }

    private fun finish(result: NativeResult) {
        val completion = pending?.second
        pending = null
        completion?.invoke(result)
    }

    private fun error(message: String) = NativeResult.Failure(NativeErrorKind.PROTOCOL, message, false)
}
