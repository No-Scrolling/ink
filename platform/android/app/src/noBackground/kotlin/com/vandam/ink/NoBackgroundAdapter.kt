package com.vandam.ink

internal fun createBackgroundAdapter(_activity: MainActivity): BackgroundAdapter =
    object : BackgroundAdapter {
        override fun execute(
            requestId: Long,
            operation: String,
            payload: String,
            complete: NativeResultHandler,
        ) = complete(
            NativeResult.Failure(
                NativeErrorKind.UNAVAILABLE,
                "Background work is not enabled",
                false,
            ),
        )

        override fun cancel(requestId: Long) = Unit
        override fun reconcile() = Unit
        override fun stop() = Unit
    }
