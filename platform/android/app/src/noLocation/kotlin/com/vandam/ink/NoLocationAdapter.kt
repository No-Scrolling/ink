package com.vandam.ink

internal fun createLocationAdapter(_activity: MainActivity): LocationAdapter =
    object : LocationAdapter {
        override fun execute(
            requestId: Long,
            operation: String,
            payload: String,
            complete: NativeResultHandler,
        ) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    "Location is not enabled",
                    false,
                ),
            )
        }

        override fun cancel(requestId: Long) = Unit

        override fun requiredPermission(payload: String): String? = null

        override fun stop() = Unit
    }
