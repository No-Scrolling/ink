package com.vandam.ink

internal interface BackgroundAdapter : NativeAdapter {
    fun enqueuePush(input: org.json.JSONObject): Boolean = false
    fun stop()
}
