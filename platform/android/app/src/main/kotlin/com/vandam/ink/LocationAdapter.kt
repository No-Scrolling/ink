package com.vandam.ink

internal interface LocationAdapter : NativeAdapter {
    fun requiredPermission(payload: String): String?
    fun stop()
}
