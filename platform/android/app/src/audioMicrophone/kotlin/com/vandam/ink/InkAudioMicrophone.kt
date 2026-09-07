package com.vandam.ink

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import org.json.JSONObject

internal fun createAudioMicrophone(
    activity: MainActivity,
    processSamples: (ShortArray, Int) -> Unit,
    updateController: (Long, String) -> Unit,
    activateProcessor: (Long, String, String) -> Boolean,
    deactivateProcessor: (Long) -> Unit,
    setProcessorEnabled: (Long, Boolean) -> Boolean,
    recordingActive: () -> Boolean,
): AudioMicrophone = InkAudioMicrophone(activity, processSamples, updateController,
    activateProcessor, deactivateProcessor, setProcessorEnabled, recordingActive)

private class InkAudioMicrophone(
    private val activity: MainActivity,
    private val processSamples: (ShortArray, Int) -> Unit,
    private val updateController: (Long, String) -> Unit,
    private val activateProcessor: (Long, String, String) -> Boolean,
    private val deactivateProcessor: (Long) -> Unit,
    private val setProcessorEnabled: (Long, Boolean) -> Boolean,
    private val recordingActive: () -> Boolean,
) : AudioMicrophone {
    private val controllers = mutableMapOf<Long, String>()
    private val enabled = mutableSetOf<Long>()
    private val suspended = mutableSetOf<Long>()
    private var foreground = true
    private val captureExecutor = Executors.newSingleThreadExecutor()
    private val capturing = AtomicBoolean(false)
    private var audioRecord: AudioRecord? = null
    override val active get() = enabled.isNotEmpty()

    override fun activate(controller: Long, kind: String, config: String): NativeResult {
        if (!activateProcessor(controller, kind, config)) return protocol("Unknown audio processor: $kind")
        controllers[controller] = kind
        updateController(controller, idle(kind))
        return NativeResult.Success("")
    }

    override fun deactivate(controller: Long) {
        controllers.remove(controller)
        disable(controller)
        deactivateProcessor(controller)
    }

    override fun execute(controller: Long, operation: String, complete: NativeResultHandler) {
        when (operation) {
            "start" -> start(controller, complete)
            "stop" -> {
                disable(controller)
                controllers[controller]?.let { updateController(controller, idle(it)) }
                complete(NativeResult.Success(""))
            }
            else -> complete(protocol("Unknown audio analyser operation: $operation"))
        }
    }

    override fun pause() {
        foreground = false
        enabled.toList().forEach { controller ->
            disable(controller)
            suspended += controller
            controllers[controller]?.let { updateController(controller, idle(it)) }
        }
    }

    override fun resume() {
        foreground = true
        val pending = suspended.toList()
        suspended.clear()
        pending.forEach { controller -> start(controller) {} }
    }

    override fun stop() {
        suspended.clear()
        enabled.toList().forEach(::disable)
        controllers.keys.forEach(deactivateProcessor)
        controllers.clear()
        stopCapture()
        captureExecutor.shutdownNow()
    }

    private fun start(controller: Long, complete: NativeResultHandler) {
        val kind = controllers[controller]
        if (kind == null) {
            complete(protocol("Audio controller is not active"))
            return
        }
        if (recordingActive()) {
            updateController(
                controller,
                error(kind, "unavailable", "Microphone is being used by the recorder"),
            )
            complete(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    "Microphone is being used by the recorder",
                    true,
                ),
            )
            return
        }
        if (
            activity.checkSelfPermission(Manifest.permission.RECORD_AUDIO) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            updateController(
                controller,
                error(kind, "permission-denied", "Microphone permission is not granted"),
            )
            complete(
                NativeResult.Failure(
                    NativeErrorKind.PERMISSION_DENIED,
                    "Microphone permission is not granted",
                    true,
                ),
            )
            return
        }
        if (!foreground) {
            suspended += controller
            complete(NativeResult.Success(""))
            return
        }
        if (!setProcessorEnabled(controller, true)) {
            complete(protocol("Audio processor could not be enabled"))
            return
        }
        enabled += controller
        updateController(controller, listening(kind))
        if (!startCapture()) {
            enabled -= controller
            setProcessorEnabled(controller, false)
            updateController(controller, error(kind, "unavailable", "Microphone is unavailable"))
            complete(
                NativeResult.Failure(
                    NativeErrorKind.UNAVAILABLE,
                    "Microphone is unavailable",
                    true,
                ),
            )
            return
        }
        complete(NativeResult.Success(String()))
    }

    private fun disable(controller: Long) {
        suspended -= controller
        enabled -= controller
        setProcessorEnabled(controller, false)
        if (enabled.isEmpty()) {
            stopCapture()
        }
    }

    private fun startCapture(): Boolean {
        if (capturing.get()) {
            return true
        }
        val minimum = AudioRecord.getMinBufferSize(
            SAMPLE_RATE,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT,
        )
        if (minimum <= 0) {
            return false
        }
        val recorder = runCatching {
            AudioRecord(
                MediaRecorder.AudioSource.UNPROCESSED,
                SAMPLE_RATE,
                AudioFormat.CHANNEL_IN_MONO,
                AudioFormat.ENCODING_PCM_16BIT,
                maxOf(minimum, FRAME_SAMPLES * Short.SIZE_BYTES * 2),
            )
        }.getOrNull() ?: return false
        if (recorder.state != AudioRecord.STATE_INITIALIZED) {
            recorder.release()
            return false
        }
        audioRecord = recorder
        capturing.set(true)
        recorder.startRecording()
        captureExecutor.execute {
            val frame = ShortArray(FRAME_SAMPLES)
            while (capturing.get()) {
                val read = recorder.read(frame, 0, frame.size, AudioRecord.READ_BLOCKING)
                if (read > 0) {
                    processSamples(frame.copyOf(read), SAMPLE_RATE)
                } else if (read < 0) {
                    activity.runOnUiThread { captureFailed() }
                    break
                }
            }
        }
        return true
    }

    private fun stopCapture() {
        if (!capturing.getAndSet(false)) {
            return
        }
        audioRecord?.runCatching { stop() }
        audioRecord?.release()
        audioRecord = null
    }

    private fun captureFailed() {
        val active = enabled.toList()
        enabled.clear()
        active.forEach { controller ->
            setProcessorEnabled(controller, false)
            controllers[controller]?.let {
                updateController(controller, error(it, "input", "Microphone capture failed"))
            }
        }
        stopCapture()
    }

    private fun protocol(message: String) = NativeResult.Failure(NativeErrorKind.PROTOCOL, message, false)

    private companion object {
        private const val SAMPLE_RATE = 48_000
        private const val FRAME_SAMPLES = 4_096

        private fun idle(kind: String) = state(kind, "idle")

        private fun listening(kind: String) = state(kind, "listening")

        private fun error(kind: String, errorKind: String, message: String) =
            state(kind, "error", inkError(errorKind, message, true))

        private fun state(kind: String, status: String, error: JSONObject = inkError()): String =
            if (kind == "level") {
                JSONObject()
                    .put("status", status)
                    .put("rms", 0.0)
                    .put("peak", 0.0)
                    .put("error", error)
                    .toString()
            } else {
                JSONObject()
                    .put("status", status)
                    .put("frequencyHz", 0.0)
                    .put("note", "")
                    .put("octave", 0)
                    .put("cents", 0.0)
                    .put("confidence", 0.0)
                    .put("error", error)
                    .toString()
            }
    }
}
