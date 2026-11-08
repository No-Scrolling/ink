package com.vandam.ink

import android.content.Context

internal fun createToolManagerAdapter(context: Context): ToolManagerAdapter = object : ToolManagerAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Tool Manager is not enabled", false))
    }
    override fun cancel(requestId: Long) = Unit
    override fun stop() = Unit
}
