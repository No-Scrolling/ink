package com.vandam.ink

internal fun createBarcodeAdapter(activity: MainActivity): BarcodeAdapter = object : BarcodeAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Barcode generation is not enabled", false))
    }
    override fun cancel(requestId: Long) = Unit
    override fun stop() = Unit
}
