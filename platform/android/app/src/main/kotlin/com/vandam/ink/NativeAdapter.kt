package com.vandam.ink

internal typealias NativeResultHandler = (NativeResult) -> Unit

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
