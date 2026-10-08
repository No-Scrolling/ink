package com.vandam.ink

import org.json.JSONObject

internal fun createAudioAdapter(
    activity: MainActivity,
    _processSamples: (ShortArray, Int) -> Unit,
    updateController: (Long, String) -> Unit,
    _activateProcessor: (Long, String, String) -> Boolean,
    _deactivateProcessor: (Long) -> Unit,
    _setProcessorEnabled: (Long, Boolean) -> Boolean,
): AudioAdapter = object : AudioAdapter {
    private val playback = createAudioPlayback(activity, updateController)
    private val controllers = mutableSetOf<Long>()

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Audio capture is not enabled", false))
    }

    override fun executeController(controller: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val result = try {
            when (operation) {
                "activate" -> {
                    val recipe = JSONObject(payload)
                    require(recipe.getString("kind") == "player") { "Audio capture is not enabled" }
                    playback.activate(controller, recipe.optJSONObject("config")?.toString() ?: "{}").also {
                        if (it !is NativeResult.Failure) controllers.add(controller)
                    }
                }
                "deactivate" -> {
                    controllers.remove(controller)
                    playback.deactivate(controller)
                    NativeResult.Success("")
                }
                else -> {
                    require(controller in controllers) { "Audio controller is not active" }
                    playback.execute(controller, operation, payload)
                }
            }
        } catch (error: Exception) {
            NativeResult.Failure(NativeErrorKind.PROTOCOL, error.message ?: "Invalid audio request", false)
        }
        complete(result)
    }
    override fun cancel(requestId: Long) = Unit
    override fun pause() = playback.pause()
    override fun stop() {
        controllers.clear()
        playback.stop()
    }
}
