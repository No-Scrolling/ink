package com.vandam.ink

internal fun createCryptoAdapter(): NativeAdapter = object : NativeAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) =
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Crypto is not enabled", false))
    override fun cancel(requestId: Long) = Unit
}
