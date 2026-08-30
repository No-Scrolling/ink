package com.vandam.ink

import android.nfc.NdefRecord
import android.nfc.NfcAdapter.ReaderCallback
import android.nfc.Tag
import android.nfc.tech.Ndef
import android.os.Bundle
import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.nio.charset.Charset

internal fun createNfcAdapter(activity: MainActivity): NfcAdapter = InkNfcAdapter(activity)

private class InkNfcAdapter(private val activity: MainActivity) : NfcAdapter {
    private val platformAdapter = android.nfc.NfcAdapter.getDefaultAdapter(activity)
    private val requests = mutableMapOf<Long, NativeResultHandler>()
    private var resumed = false
    private var readerEnabled = false
    private val callback = ReaderCallback(::tagDiscovered)

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation != READ_OPERATION || payload.isNotEmpty()) {
            complete(protocol("Unknown NFC operation: $operation"))
            return
        }
        val adapter = platformAdapter
        if (adapter == null) {
            complete(unavailable("NFC hardware is not available"))
            return
        }
        if (!adapter.isEnabled) {
            complete(nfcDisabled())
            return
        }
        requests[requestId] = complete
        enableReader()
    }

    override fun cancel(requestId: Long) {
        requests.remove(requestId)
        if (requests.isEmpty()) {
            disableReader()
        }
    }

    override fun resume() {
        resumed = true
        if (platformAdapter?.isEnabled == false) {
            finishAll(nfcDisabled())
        } else {
            enableReader()
        }
    }

    override fun pause() {
        resumed = false
        disableReader()
    }

    override fun stop() {
        pause()
        requests.clear()
    }

    private fun enableReader() {
        val adapter = platformAdapter ?: return
        if (!resumed || readerEnabled || requests.isEmpty()) {
            return
        }
        if (!adapter.isEnabled) {
            finishAll(nfcDisabled())
            return
        }
        runCatching {
            adapter.enableReaderMode(
                activity,
                callback,
                READER_FLAGS,
                Bundle().apply {
                    putInt(android.nfc.NfcAdapter.EXTRA_READER_PRESENCE_CHECK_DELAY, 250)
                },
            )
        }.onSuccess {
            readerEnabled = true
        }.onFailure {
            finishAll(unexpected("Could not start NFC reading"))
        }
    }

    private fun disableReader() {
        val adapter = platformAdapter
        if (!readerEnabled || adapter == null) {
            return
        }
        readerEnabled = false
        runCatching { adapter.disableReaderMode(activity) }
    }

    private fun tagDiscovered(tag: Tag) {
        val result = runCatching { tagResult(tag) }.getOrElse {
            protocol("Could not read the NFC tag")
        }
        activity.runOnUiThread { finishAll(result) }
    }

    private fun finishAll(result: NativeResult) {
        if (requests.isEmpty()) {
            return
        }
        val completions = requests.values.toList()
        requests.clear()
        disableReader()
        completions.forEach { complete -> complete(result) }
    }

    private fun tagResult(tag: Tag): NativeResult {
        val message = Ndef.get(tag)?.cachedNdefMessage
        if (message != null && message.toByteArray().size > MAX_NDEF_BYTES) {
            return protocol("NDEF data exceeds 64 KiB")
        }
        val records = message?.records.orEmpty().map(::recordJson)
        val textRecord = records.firstOrNull { it.getString(KIND_KEY) == TEXT_KIND }
        val uriRecord = records.firstOrNull { it.getString(KIND_KEY) == URI_KIND }
        val text = textRecord?.getString(VALUE_KEY).orEmpty()
        val uri = uriRecord?.getString(VALUE_KEY).orEmpty()
        val jsonRecords = JSONArray().apply { records.forEach(::put) }
        val value = JSONObject()
            .put(SERIAL_NUMBER_KEY, tag.id.joinToString("") { byte -> "%02X".format(byte) })
            .put(HAS_TEXT_KEY, textRecord != null)
            .put(TEXT_KEY, text)
            .put(HAS_URI_KEY, uriRecord != null)
            .put(URI_KEY, uri)
            .put(RECORDS_KEY, jsonRecords)
            .toString()
            .toByteArray(Charsets.UTF_8)
        return NativeResult.Bytes(value)
    }

    private fun recordJson(record: NdefRecord): JSONObject {
        val text = record.textValue()
        if (text != null) {
            return record(TEXT_KIND, text.value, text.languageTag, "", "")
        }
        val uri = record.uriValue()
        if (uri != null) {
            return record(URI_KIND, uri, "", "", "")
        }
        val mimeType = if (record.tnf == NdefRecord.TNF_MIME_MEDIA) {
            record.type.toString(Charsets.US_ASCII)
        } else {
            ""
        }
        return record(
            BINARY_KIND,
            "",
            "",
            mimeType,
            Base64.encodeToString(record.payload, Base64.NO_WRAP),
        )
    }

    private fun NdefRecord.textValue(): TextValue? {
        if (tnf != NdefRecord.TNF_WELL_KNOWN || !type.contentEquals(NdefRecord.RTD_TEXT)) {
            return null
        }
        val status = payload.firstOrNull()?.toInt()?.and(0xff) ?: return null
        val languageLength = status and LANGUAGE_LENGTH_MASK
        if (payload.size < 1 + languageLength) {
            return null
        }
        val language = payload.copyOfRange(1, 1 + languageLength).toString(Charsets.US_ASCII)
        val charset = if (status and UTF_16_FLAG == 0) Charsets.UTF_8 else UTF_16
        val value = payload.copyOfRange(1 + languageLength, payload.size).toString(charset)
        return TextValue(value, language)
    }

    private fun NdefRecord.uriValue(): String? {
        if (tnf != NdefRecord.TNF_WELL_KNOWN || !type.contentEquals(NdefRecord.RTD_URI)) {
            return null
        }
        return runCatching { toUri()?.toString() }.getOrNull()
    }

    private data class TextValue(val value: String, val languageTag: String)

    private companion object {
        private const val READ_OPERATION = "read"
        private const val MAX_NDEF_BYTES = 65_536
        private const val LANGUAGE_LENGTH_MASK = 0x3f
        private const val UTF_16_FLAG = 0x80
        private val UTF_16: Charset = Charset.forName("UTF-16")
        private const val TEXT_KIND = "text"
        private const val URI_KIND = "uri"
        private const val BINARY_KIND = "binary"
        private const val SERIAL_NUMBER_KEY = "serialNumber"
        private const val HAS_TEXT_KEY = "hasText"
        private const val TEXT_KEY = "text"
        private const val HAS_URI_KEY = "hasUri"
        private const val URI_KEY = "uri"
        private const val RECORDS_KEY = "records"
        private const val KIND_KEY = "kind"
        private const val VALUE_KEY = "value"
        private const val LANGUAGE_TAG_KEY = "languageTag"
        private const val MIME_TYPE_KEY = "mimeType"
        private const val PAYLOAD_BASE64_KEY = "payloadBase64"
        private const val READER_FLAGS =
            android.nfc.NfcAdapter.FLAG_READER_NFC_A or
                android.nfc.NfcAdapter.FLAG_READER_NFC_B or
                android.nfc.NfcAdapter.FLAG_READER_NFC_F or
                android.nfc.NfcAdapter.FLAG_READER_NFC_V or
                android.nfc.NfcAdapter.FLAG_READER_NFC_BARCODE

        private fun record(
            kind: String,
            value: String,
            languageTag: String,
            mimeType: String,
            payloadBase64: String,
        ) = JSONObject()
            .put(KIND_KEY, kind)
            .put(VALUE_KEY, value)
            .put(LANGUAGE_TAG_KEY, languageTag)
            .put(MIME_TYPE_KEY, mimeType)
            .put(PAYLOAD_BASE64_KEY, payloadBase64)

        private fun protocol(message: String) = NativeResult.Failure(
            NativeErrorKind.PROTOCOL,
            message,
            false,
        )

        private fun unavailable(message: String) = NativeResult.Failure(
            NativeErrorKind.UNAVAILABLE,
            message,
            false,
        )

        private fun nfcDisabled() = NativeResult.Failure(
            NativeErrorKind.NFC_DISABLED,
            "NFC is switched off",
            true,
        )

        private fun unexpected(message: String) = NativeResult.Failure(
            NativeErrorKind.UNEXPECTED,
            message,
            true,
        )
    }
}
