package com.vandam.ink

import android.app.Activity
import android.content.Intent

internal fun createLightPushAdapter(
    _activity: Activity,
    _haptic: () -> Unit,
    _update: (Long, String) -> Unit,
): LightPushAdapter = NoLightPushAdapter

private object NoLightPushAdapter : LightPushAdapter {
    override fun start() = Unit
    override fun stop() = Unit
    override fun refresh() = Unit
    override fun handleIntent(intent: Intent) = Unit
    override fun executeController(controller: Long, operation: String, payload: String) = false
}
