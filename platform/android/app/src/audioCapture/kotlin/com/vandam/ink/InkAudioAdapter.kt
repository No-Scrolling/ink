package com.vandam.ink

import android.app.Activity
import android.Manifest
import android.content.pm.PackageManager
import org.json.JSONObject

internal fun createAudioAdapter(
    activity: Activity,
    bundle: InkBundle,
    clock: PlaybackClock,
    processing: AudioProcessing,
    updateController: (Long, String) -> Unit,
): AudioAdapter = InkAudioAdapter(activity, bundle, clock, processing, updateController)

private class InkAudioAdapter(
    private val activity: Activity,
    bundle: InkBundle,
    clock: PlaybackClock,
    private val processing: AudioProcessing,
    private val updateController: (Long, String) -> Unit,
) : AudioAdapter {
    private val controllers = mutableMapOf<Long, String>()
    private val observed = mutableSetOf<Long>()
    private val lastStatus = mutableMapOf<Long, String>()
    private val playback = createAudioPlayback(activity, updateController, bundle, clock)
    private val microphone = createAudioMicrophone(activity, processing::processAudioSamples, ::updateCapture,
        processing::activateAudioProcessor, processing::deactivateAudioProcessor, processing::setAudioProcessorEnabled) { recording.active }
    private val recording: AudioRecording = createAudioRecording(activity, ::updateCapture) { microphone.active }

    override fun warm() = playback.warm()

    private fun updateCapture(controller: Long, value: String) {
        processing.updateCaptureState(controller, value)
        val status = JSONObject(value).optString("status")
        val previous = lastStatus.put(controller, status)
        if (controller in observed || status != previous || status != "recording") updateController(controller, value)
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
        controllers.keys.forEach(processing::removeCaptureState)
        controllers.clear()
        observed.clear()
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
        val result = when (kind) {
            in SUPPORTED_PROCESSORS -> microphone.activate(controller, kind, config)
            PLAYER -> playback.activate(controller, config)
            RECORDER -> recording.activate(controller)
            else -> protocol("Unknown audio controller: $kind")
        }
        if (result is NativeResult.Failure) {
            observed.remove(controller)
            lastStatus.remove(controller)
            processing.removeCaptureState(controller)
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
        observed.remove(controller)
        lastStatus.remove(controller)
        processing.removeCaptureState(controller)
        complete(NativeResult.Success(""))
    }

    private fun executeActiveController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation == "observeMeasurements") {
            val kind = controllers[controller]
            if (kind != RECORDER && kind !in SUPPORTED_PROCESSORS) {
                complete(protocol("Audio capture is not active"))
                return
            }
            val enabled = runCatching { JSONObject(payload).getBoolean("enabled") }.getOrElse {
                complete(protocol("Invalid measurement subscription"))
                return
            }
            if (kind in SUPPORTED_PROCESSORS && !processing.observeAudioMeasurements(controller, enabled)) {
                complete(protocol("Audio processor is not active"))
                return
            }
            if (enabled) observed.add(controller) else observed.remove(controller)
            complete(NativeResult.Success(""))
            return
        }
        when (controllers[controller]) {
            in SUPPORTED_PROCESSORS -> microphone.execute(controller, operation, complete)
            PLAYER -> playback.execute(controller, operation, payload, complete)
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
        private const val PLAYER = "player"
        private const val RECORDER = "recorder"
        private const val KIND = "kind"
        private const val CONFIG = "config"
        private const val PREFERENCES = "ink-audio"
        private const val REQUESTED = "microphone-permission-requested"
        private val SUPPORTED_PROCESSORS = setOf("level", "pitch")
    }
}
