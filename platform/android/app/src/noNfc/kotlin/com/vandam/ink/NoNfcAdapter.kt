package com.vandam.ink

import android.app.Activity

internal fun createNfcAdapter(_activity: Activity): NfcAdapter =
    object : NfcAdapter {
        override fun execute(
            requestId: Long,
            operation: String,
            payload: String,
            complete: NativeResultHandler,
        ) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    "NFC is not enabled for this app",
                    false,
                ),
            )
        }

        override fun cancel(requestId: Long) = Unit

        override fun resume() = Unit

        override fun pause() = Unit

        override fun stop() = Unit
    }
