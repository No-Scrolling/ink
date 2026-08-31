package com.vandam.ink

import android.content.Intent

internal interface LightPushAdapter {
    fun start()
    fun stop()
    fun refresh()
    fun handleIntent(intent: Intent)
    fun executeController(controller: Long, operation: String, payload: String): Boolean
}
