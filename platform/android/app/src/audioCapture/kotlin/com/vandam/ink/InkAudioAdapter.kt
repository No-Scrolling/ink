package com.vandam.ink

import android.Manifest
import android.content.pm.PackageManager
import org.json.JSONObject

internal fun createAudioAdapter(
    activity: MainActivity,
    processSamples: (ShortArray, Int) -> Unit,
    updateController: (Long, String) -> Unit,
    activateProcessor: (Long, String, String) -> Boolean,
    deactivateProcessor: (Long) -> Unit,
    setProcessorEnabled: (Long, Boolean) -> Boolean,
): AudioAdapter = InkAudioAdapter(
    activity,
    processSamples,
    updateController,
    activateProcessor,
    deactivateProcessor,
    setProcessorEnabled,
)

private class InkAudioAdapter(
    private val activity: MainActivity,
    processSamples: (ShortArray, Int) -> Unit,
    private val updateController: (Long, String) -> Unit,
    activateProcessor: (Long, String, String) -> Boolean,
    deactivateProcessor: (Long) -> Unit,
    setProcessorEnabled: (Long, Boolean) -> Boolean,
) : AudioAdapter {
    private val controllers = mutableMapOf<Long, String>()
    private val statusOnly = mutableSetOf<Long>()
    private val lastStatus = mutableMapOf<Long, String>()
    private val playback = createAudioPlayback(activity, updateController)
    private val microphone = createAudioMicrophone(activity, processSamples, ::updateCapture,
        activateProcessor, deactivateProcessor, setProcessorEnabled) { recording.active }
    private val recording: AudioRecording = createAudioRecording(activity, ::updateCapture) { microphone.active }

    private fun updateCapture(controller: Long, value: String) {
        activity.updateCaptureState(controller, value)
        val status = JSONObject(value).optString("status")
        val previous = lastStatus.put(controller, status)
        if (controller !in statusOnly || status != previous || status != "recording") updateController(controller, value)
    }

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (operation) {
            PERMISSION_STATUS -> complete(NativeResult.Success(permissionStatus()))
            REQUEST_PERMISSION -> {
                activity.getSharedPreferences(PREFERENCES, 0)
                    .edit()
                    .putBoolean(REQUESTED, true)
                    .apply()
                activity.requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO), 0)
                complete(NativeResult.Success(String()))
            }
            else -> complete(protocol("Unknown audio operation: $operation"))
        }
    }

    override fun executeController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (operation) {
            ACTIVATE -> activate(controller, payload, complete)
            DEACTIVATE -> deactivate(controller, complete)
            else -> executeActiveController(controller, operation, payload, complete)
        }
    }

    override fun cancel(requestId: Long) = Unit

    override fun pause() {
        playback.pause()
        recording.pause()
        microphone.pause()
    }

    override fun resume() = microphone.resume()

    override fun stop() {
        recording.stop()
        playback.stop()
        microphone.stop()
        controllers.keys.forEach(activity::removeCaptureState)
        controllers.clear()
        statusOnly.clear()
        lastStatus.clear()
    }

    private fun activate(
        controller: Long,
        payload: String,
        complete: NativeResultHandler,
    ) {
        val recipe = runCatching { JSONObject(payload) }.getOrElse {
            complete(protocol("Ink produced invalid audio controller configuration"))
            return
        }
        val kind = recipe.optString(KIND)
        val config = recipe.optJSONObject(CONFIG)?.toString() ?: "{}"
        if (recipe.optJSONObject(CONFIG)?.optString("updates") == "status") statusOnly.add(controller)
        val result = when (kind) {
            in SUPPORTED_PROCESSORS -> microphone.activate(controller, kind, config)
            PLAYER -> playback.activate(controller, config)
            RECORDER -> recording.activate(controller)
            else -> protocol("Unknown audio controller: $kind")
        }
        if (result is NativeResult.Failure) {
            statusOnly.remove(controller)
            lastStatus.remove(controller)
            activity.removeCaptureState(controller)
            complete(result)
            return
        }
        controllers[controller] = kind
        complete(result)
    }

    private fun deactivate(controller: Long, complete: NativeResultHandler) {
        when (controllers.remove(controller)) {
            in SUPPORTED_PROCESSORS -> microphone.deactivate(controller)
            PLAYER -> playback.deactivate(controller)
            RECORDER -> recording.deactivate()
            null -> Unit
        }
        statusOnly.remove(controller)
        lastStatus.remove(controller)
        activity.removeCaptureState(controller)
        complete(NativeResult.Success(""))
    }

    private fun executeActiveController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (controllers[controller]) {
            in SUPPORTED_PROCESSORS -> microphone.execute(controller, operation, complete)
            PLAYER -> {
                val result = if (operation == PLAY_RECORDING) {
                    val source = recording.source
                    if (source == null) {
                        playback.reportFailure(
                            controller,
                            "source",
                            "No saved recording",
                            true,
                        )
                        NativeResult.Failure(
                            NativeErrorKind.UNAVAILABLE,
                            "No saved recording",
                            true,
                        )
                    } else {
                        val item = JSONObject()
                            .put("src", source)
                            .put("title", "Recording")
                        playback.execute(
                            controller,
                            "play",
                            JSONObject().put("item", item).toString(),
                        )
                    }
                } else {
                    playback.execute(controller, operation, payload)
                }
                complete(result)
            }
            RECORDER -> complete(recording.execute(operation))
            null -> complete(protocol("Audio controller is not active"))
            else -> complete(protocol("Unknown audio controller"))
        }
    }

    private fun permissionStatus(): String {
        if (
            activity.checkSelfPermission(Manifest.permission.RECORD_AUDIO) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return "granted"
        }
        val requested = activity.getSharedPreferences(PREFERENCES, 0)
            .getBoolean(REQUESTED, false)
        return when {
            !requested -> "unknown"
            activity.shouldShowRequestPermissionRationale(Manifest.permission.RECORD_AUDIO) ->
                "denied"
            else -> "blocked"
        }
    }

    private fun protocol(message: String) = NativeResult.Failure(
        NativeErrorKind.PROTOCOL,
        message,
        false,
    )

    private companion object {
        private const val PERMISSION_STATUS = "permission-status"
        private const val REQUEST_PERMISSION = "request-permission"
        private const val ACTIVATE = "activate"
        private const val DEACTIVATE = "deactivate"
        private const val PLAY_RECORDING = "playRecording"
        private const val PLAYER = "player"
        private const val RECORDER = "recorder"
        private const val KIND = "kind"
        private const val CONFIG = "config"
        private const val PREFERENCES = "ink-audio"
        private const val REQUESTED = "microphone-permission-requested"
        private val SUPPORTED_PROCESSORS = setOf("level", "pitch")
    }
}
