package com.vandam.ink

import org.json.JSONObject

internal typealias NativeResultHandler = (NativeResult) -> Unit

internal fun javascriptResult(id: Long, result: NativeResult): String {
    val response = JSONObject().put("type", "result").put("id", id)
    when (result) {
        is NativeResult.Binary -> response.put("value", result.value)
        is NativeResult.Success -> response.put("value", result.value)
        is NativeResult.Bytes -> response.put("value", result.value.toString(Charsets.UTF_8))
        is NativeResult.Failure -> response
            .put("kind", result.kind.wireName)
            .put("message", result.message)
            .put("retryable", result.retryable)
        is NativeResult.Pixels -> response.put("kind", "protocol").put("message", "Pixel results require an image request")
        is NativeResult.File -> {
            if (result.deleteAfterRead) java.io.File(result.path).delete()
            response.put("kind", "protocol").put("message", "File results require a managed-file operation")
        }
    }
    val message = response.toString()
    return if (message.toByteArray(Charsets.UTF_8).size <= 1024 * 1024) message else
        JSONObject().put("type", "result").put("id", id)
            .put("kind", "protocol").put("message", "Native result exceeds the message limit").toString()
}

internal interface NativeAdapter {
    fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    )
    fun executeBytes(requestId: Long, operation: String, payload: String, bytes: ByteArray, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "This operation does not accept binary data", false))
    }
    fun cancel(requestId: Long)
}

internal interface NativeControllerAdapter : NativeAdapter {
    fun executeController(controller: Long, operation: String, payload: String, complete: NativeResultHandler)
}

internal sealed interface NativeResult {
    data class Success(val value: String) : NativeResult

    data class Binary(val value: String, val bytes: ByteArray) : NativeResult

    data class Bytes(val value: ByteArray) : NativeResult

    data class Pixels(val width: Int, val height: Int, val rgba: ByteArray) : NativeResult

    data class File(val path: String, val deleteAfterRead: Boolean = true) : NativeResult

    data class Failure(
        val kind: NativeErrorKind,
        val message: String,
        val retryable: Boolean,
    ) : NativeResult
}

internal enum class NativeErrorKind(val code: Int, val wireName: String) {
    UNAVAILABLE(0, "unavailable"),
    PERMISSION_DENIED(1, "permission-denied"),
    TIMEOUT(2, "timeout"),
    PROTOCOL(3, "protocol"),
    UNEXPECTED(4, "unexpected"),
    PERMISSION_BLOCKED(5, "permission-blocked"),
    LOCATION_DISABLED(6, "location-disabled"),
    NFC_DISABLED(7, "nfc-disabled"),
    BUSY(8, "busy"),
}

internal fun inkError(
    kind: String = "unexpected",
    message: String = "",
    retryable: Boolean = false,
): JSONObject = JSONObject()
    .put("kind", kind)
    .put("message", message)
    .put("retryable", retryable)

internal interface NotificationsAdapter : NativeAdapter {
    fun start()
    fun stop()

    fun executeController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    )

    fun refreshEvents()

    fun handleIntent(intent: android.content.Intent)
}

internal const val EXTRA_NOTIFICATION_HREF = "com.vandam.ink.notification.HREF"
internal const val EXTRA_NOTIFICATION_PARAMS = "com.vandam.ink.notification.PARAMS"
