package com.vandam.ink

internal fun createDownloadsAdapter(context: android.content.Context, publish: (Long, String) -> Unit): DownloadsAdapter =
    object : DownloadsAdapter {
        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) =
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Downloads are not enabled", false))
        override fun executeController(controller: Long, operation: String, payload: String, complete: NativeResultHandler) =
            execute(controller, operation, payload, complete)
        override fun cancel(requestId: Long) = Unit
        override fun stop() = Unit
    }
