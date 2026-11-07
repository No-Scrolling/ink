package com.vandam.ink

internal fun createAppNativeAdapters(activity: MainActivity): Map<String, NativeAdapter> =
    mapOf("packaging" to object : NativeAdapter {
        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
            complete(NativeResult.Success(RustPackagingProbe.value().toString()))
        }
        override fun cancel(requestId: Long) {}
    })

internal object RustPackagingProbe {
    init { System.loadLibrary("ink_android") }
    @JvmStatic external fun value(): Int
}
