package com.vandam.ink

import org.json.JSONObject

internal typealias NativeResultHandler = (NativeResult) -> Unit

internal fun javascriptResult(id: Long, result: NativeResult): String {
    val response = JSONObject().put("type", "result").put("id", id)
    when (result) {
        is NativeResult.Success -> response.put("value", result.value)
        is NativeResult.Bytes -> response.put("value", result.value.toString(Charsets.UTF_8))
        is NativeResult.Failure -> response
            .put("kind", result.kind.name.lowercase())
            .put("message", result.message)
            .put("retryable", result.retryable)
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
    fun cancel(requestId: Long)
}

internal sealed interface NativeResult {
    data class Success(val value: String) : NativeResult

    data class Bytes(val value: ByteArray) : NativeResult

    data class File(val path: String, val deleteAfterRead: Boolean = true) : NativeResult

    data class Failure(
        val kind: NativeErrorKind,
        val message: String,
        val retryable: Boolean,
    ) : NativeResult
}

internal enum class NativeErrorKind(val code: Int) {
    UNAVAILABLE(0),
    PERMISSION_DENIED(1),
    TIMEOUT(2),
    PROTOCOL(3),
    UNEXPECTED(4),
    PERMISSION_BLOCKED(5),
    LOCATION_DISABLED(6),
    NFC_DISABLED(7),
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
