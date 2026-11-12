package com.vandam.ink

import android.content.Context

internal fun createContactsAdapter(_context: Context): ContactsAdapter =
    object : ContactsAdapter {
        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Contacts are not enabled for this app", false))
        }
        override fun cancel(requestId: Long) = Unit
        override fun stop() = Unit
    }
