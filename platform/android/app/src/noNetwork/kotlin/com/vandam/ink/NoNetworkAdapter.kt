package com.vandam.ink

internal fun createNetworkAdapter(_activity: MainActivity): NetworkAdapter =
    object : NetworkAdapter {
        override fun execute(
            requestId: Long,
            operation: String,
            payload: String,
            complete: NativeResultHandler,
        ) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    "Networking is not enabled",
                    true,
                ),
            )
        }

        override fun cancel(requestId: Long) = Unit

        override fun stop() = Unit
    }
