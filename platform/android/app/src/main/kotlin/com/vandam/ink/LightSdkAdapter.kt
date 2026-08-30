package com.vandam.ink

import android.view.View

internal interface LightSdkAdapter {
    fun start()
    fun refresh()
    fun stop()
    fun performHaptic(view: View)
    fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    )
    fun cancel(requestId: Long)
}

internal typealias HapticsChangedHandler = (Boolean) -> Unit
internal typealias NativeResultHandler = (NativeResult) -> Unit

internal sealed interface NativeResult {
    data class Success(val value: String) : NativeResult

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
}
