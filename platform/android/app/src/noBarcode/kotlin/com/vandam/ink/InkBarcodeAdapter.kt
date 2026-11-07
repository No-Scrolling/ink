package com.vandam.ink

import android.app.Activity

internal fun createBarcodeAdapter(activity: Activity): BarcodeAdapter = object : BarcodeAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Barcode generation is not enabled", false))
    }
    override fun cancel(requestId: Long) = Unit
    override fun stop() = Unit
}
