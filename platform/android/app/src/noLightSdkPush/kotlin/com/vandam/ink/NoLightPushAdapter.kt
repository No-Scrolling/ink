package com.vandam.ink

import android.content.Intent

internal fun createLightPushAdapter(
    _activity: MainActivity,
    _update: (Long, String) -> Unit,
): LightPushAdapter = NoLightPushAdapter

private object NoLightPushAdapter : LightPushAdapter {
    override fun start() = Unit
    override fun stop() = Unit
    override fun refresh() = Unit
    override fun handleIntent(intent: Intent) = Unit
    override fun executeController(controller: Long, operation: String, payload: String) = false
}
