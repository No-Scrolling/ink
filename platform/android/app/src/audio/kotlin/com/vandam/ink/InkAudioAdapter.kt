package com.vandam.ink

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaMetadataRetriever
import android.media.MediaRecorder
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import java.io.File
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
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
    private val processSamples: (ShortArray, Int) -> Unit,
    private val updateController: (Long, String) -> Unit,
    private val activateProcessor: (Long, String, String) -> Boolean,
    private val deactivateProcessor: (Long) -> Unit,
    private val setProcessorEnabled: (Long, Boolean) -> Boolean,
) : AudioAdapter {
    private val controllers = mutableMapOf<Long, String>()
    private val enabled = mutableSetOf<Long>()
    private val captureExecutor = Executors.newSingleThreadExecutor()
    private val capturing = AtomicBoolean(false)
    private val playback = createAudioPlayback(activity, updateController)
    private val handler = Handler(Looper.getMainLooper())
    private val recorderProgress = object : Runnable {
        override fun run() {
            publishRecorder("recording")
            if (mediaRecorder != null) {
                handler.postDelayed(this, PROGRESS_INTERVAL_MS)
            }
        }
    }
    private var audioRecord: AudioRecord? = null
    private var mediaRecorder: MediaRecorder? = null
    private var recorderController: Long? = null
    private var recordingId: String? = null
    private var recordingFile: File? = null
    private var recordingStartedAt = 0L
    private var lastRecording: Recording? = null

    init {
        val files = recordingsDirectory().listFiles().orEmpty()
        files.filter { it.name.endsWith(PARTIAL_SUFFIX) }.forEach(File::delete)
        files.filter { it.extension == "m4a" }
            .maxByOrNull(File::lastModified)
            ?.let { file ->
                val id = file.nameWithoutExtension
                if (RECORDING_ID.matches(id)) {
                    val duration = runCatching {
                        val retriever = MediaMetadataRetriever()
                        try {
                            retriever.setDataSource(file.absolutePath)
                            retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)
                                ?.toLongOrNull() ?: 0
                        } finally {
                            retriever.release()
                        }
                    }.getOrDefault(0)
                    lastRecording = Recording(id, duration)
                }
            }
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
        if (mediaRecorder != null) {
            stopRecorder()
        }
        enabled.toList().forEach { controller ->
            disable(controller)
            controllers[controller]?.let { updateController(controller, idle(it)) }
        }
    }

    override fun stop() {
        cancelRecorder(false)
        playback.stop()
        enabled.toList().forEach(::disable)
        controllers.forEach { (controller, kind) ->
            if (kind in SUPPORTED_PROCESSORS) {
                deactivateProcessor(controller)
            }
        }
        controllers.clear()
        stopCapture()
        captureExecutor.shutdownNow()
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
            in SUPPORTED_PROCESSORS -> {
                if (activateProcessor(controller, kind, config)) {
                    NativeResult.Success("")
                } else {
                    protocol("Unknown audio processor: $kind")
                }
            }
            PLAYER -> playback.activate(controller, config)
            RECORDER -> {
                if (recorderController != null && recorderController != controller) {
                    protocol("Only one audio recorder can be active")
                } else {
                    recorderController = controller
                    publishRecorder(if (lastRecording == null) "idle" else "ready")
                    NativeResult.Success("")
                }
            }
            else -> protocol("Unknown audio controller: $kind")
        }
        if (result is NativeResult.Failure) {
            complete(result)
            return
        }
        controllers[controller] = kind
        if (kind in SUPPORTED_PROCESSORS) {
            updateController(controller, idle(kind))
        }
        complete(result)
    }

    private fun deactivate(controller: Long, complete: NativeResultHandler) {
        when (val kind = controllers.remove(controller)) {
            in SUPPORTED_PROCESSORS -> {
                disable(controller)
                deactivateProcessor(controller)
            }
            PLAYER -> playback.deactivate(controller)
            RECORDER -> {
                if (mediaRecorder != null) {
                    stopRecorder()
                }
                recorderController = null
            }
            null -> Unit
        }
        complete(NativeResult.Success(""))
    }

    private fun executeActiveController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        when (controllers[controller]) {
            in SUPPORTED_PROCESSORS -> when (operation) {
                START -> start(controller, complete)
                STOP -> {
                    disable(controller)
                    controllers[controller]?.let { updateController(controller, idle(it)) }
                    complete(NativeResult.Success(""))
                }
                else -> complete(protocol("Unknown audio analyser operation: $operation"))
            }
            PLAYER -> {
                val result = if (operation == PLAY_RECORDING) {
                    val recording = lastRecording
                    if (recording == null) {
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
                            .put("src", "$RECORDING_PREFIX${recording.id}")
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
            RECORDER -> complete(executeRecorder(operation))
            null -> complete(protocol("Audio controller is not active"))
            else -> complete(protocol("Unknown audio controller"))
        }
    }

    private fun start(controller: Long, complete: NativeResultHandler) {
        val kind = controllers[controller]
        if (kind == null) {
            complete(protocol("Audio controller is not active"))
            return
        }
        if (mediaRecorder != null) {
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

    private fun executeRecorder(operation: String): NativeResult = when (operation) {
        START -> startRecorder()
        STOP -> stopRecorder()
        CANCEL -> cancelRecorder(true)
        DELETE -> deleteRecording()
        else -> protocol("Unknown audio recorder operation: $operation")
    }

    private fun startRecorder(): NativeResult {
        val controller = recorderController ?: return protocol("Audio recorder is not active")
        if (
            activity.checkSelfPermission(Manifest.permission.RECORD_AUDIO) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            publishRecorderError("permission-denied", "Microphone permission is not granted", true)
            return NativeResult.Failure(
                NativeErrorKind.PERMISSION_DENIED,
                "Microphone permission is not granted",
                true,
            )
        }
        if (mediaRecorder != null) {
            return NativeResult.Success("")
        }
        if (enabled.isNotEmpty()) {
            publishRecorderError("unavailable", "Microphone is being used by an analyser", true)
            return NativeResult.Failure(
                NativeErrorKind.UNAVAILABLE,
                "Microphone is being used by an analyser",
                true,
            )
        }
        val id = UUID.randomUUID().toString()
        val output = File(recordingsDirectory(), "$id$PARTIAL_SUFFIX")
        val recorder = runCatching {
            MediaRecorder(activity).apply {
                setAudioSource(MediaRecorder.AudioSource.MIC)
                setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
                setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
                setAudioChannels(1)
                setAudioSamplingRate(SAMPLE_RATE)
                setAudioEncodingBitRate(RECORDING_BIT_RATE)
                setOutputFile(output.absolutePath)
                prepare()
                start()
            }
        }.getOrElse { error ->
            output.delete()
            val message = error.message ?: "Microphone is unavailable"
            publishRecorderError("unavailable", message, true)
            return NativeResult.Failure(NativeErrorKind.UNAVAILABLE, message, true)
        }
        mediaRecorder = recorder
        recordingId = id
        recordingFile = output
        recordingStartedAt = SystemClock.elapsedRealtime()
        handler.removeCallbacks(recorderProgress)
        handler.post(recorderProgress)
        updateController(controller, recorderState("recording"))
        return NativeResult.Success("")
    }

    private fun stopRecorder(): NativeResult {
        if (mediaRecorder == null) {
            return NativeResult.Success("")
        }
        publishRecorder("stopping")
        handler.removeCallbacks(recorderProgress)
        val duration = elapsedRecordingTime()
        val recorder = mediaRecorder
        mediaRecorder = null
        val stopped = runCatching { recorder?.stop() }
        recorder?.release()
        val partial = recordingFile
        val id = recordingId
        recordingFile = null
        recordingId = null
        recordingStartedAt = 0
        if (stopped.isFailure || partial == null || id == null) {
            partial?.delete()
            val message = stopped.exceptionOrNull()?.message ?: "Recording could not be finalised"
            publishRecorderError("output", message, true)
            return NativeResult.Failure(NativeErrorKind.UNEXPECTED, message, true)
        }
        val output = File(recordingsDirectory(), "$id.m4a")
        if (!partial.renameTo(output)) {
            partial.delete()
            publishRecorderError("output", "Recording could not be saved", true)
            return NativeResult.Failure(
                NativeErrorKind.UNEXPECTED,
                "Recording could not be saved",
                true,
            )
        }
        lastRecording?.takeIf { it.id != id }?.let { previous ->
            File(recordingsDirectory(), "${previous.id}.m4a").delete()
        }
        lastRecording = Recording(id, duration)
        publishRecorder("ready")
        return NativeResult.Success("")
    }

    private fun cancelRecorder(publish: Boolean): NativeResult {
        handler.removeCallbacks(recorderProgress)
        mediaRecorder?.runCatching { reset() }
        mediaRecorder?.release()
        mediaRecorder = null
        recordingFile?.delete()
        recordingFile = null
        recordingId = null
        recordingStartedAt = 0
        if (publish) {
            publishRecorder(if (lastRecording == null) "idle" else "ready")
        }
        return NativeResult.Success("")
    }

    private fun deleteRecording(): NativeResult {
        val recording = lastRecording ?: return NativeResult.Success("")
        val file = File(recordingsDirectory(), "${recording.id}.m4a")
        if (file.exists() && !file.delete()) {
            publishRecorderError("output", "Recording could not be deleted", true)
            return NativeResult.Failure(
                NativeErrorKind.UNEXPECTED,
                "Recording could not be deleted",
                true,
            )
        }
        lastRecording = null
        publishRecorder("idle")
        return NativeResult.Success("")
    }

    private fun publishRecorder(status: String) {
        val controller = recorderController ?: return
        updateController(controller, recorderState(status))
    }

    private fun publishRecorderError(kind: String, message: String, retryable: Boolean) {
        val controller = recorderController ?: return
        updateController(
            controller,
            recorderState(
                status = "error",
                error = inkError(kind, message, retryable),
            ),
        )
    }

    private fun recorderState(
        status: String,
        error: JSONObject = inkError(),
    ): String {
        val recording = lastRecording
        val duration = if (status == "recording" || status == "stopping") {
            elapsedRecordingTime()
        } else {
            0
        }
        return JSONObject()
            .put("status", status)
            .put("durationMs", duration)
            .put("id", recording?.id.orEmpty())
            .put("src", recording?.let { "$RECORDING_PREFIX${it.id}" }.orEmpty())
            .put("recordingDurationMs", recording?.durationMs ?: 0)
            .put("error", error)
            .toString()
    }

    private fun elapsedRecordingTime(): Long = if (recordingStartedAt == 0L) {
        0
    } else {
        SystemClock.elapsedRealtime() - recordingStartedAt
    }

    private fun recordingsDirectory() = File(activity.filesDir, "recordings").apply { mkdirs() }

    private fun disable(controller: Long) {
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
        private const val START = "start"
        private const val STOP = "stop"
        private const val CANCEL = "cancel"
        private const val DELETE = "delete"
        private const val PLAY_RECORDING = "playRecording"
        private const val PLAYER = "player"
        private const val RECORDER = "recorder"
        private const val KIND = "kind"
        private const val CONFIG = "config"
        private const val PREFERENCES = "ink-audio"
        private const val REQUESTED = "microphone-permission-requested"
        private const val SAMPLE_RATE = 48_000
        private const val FRAME_SAMPLES = 4_096
        private const val RECORDING_BIT_RATE = 96_000
        private const val PROGRESS_INTERVAL_MS = 250L
        private const val PARTIAL_SUFFIX = ".partial.m4a"
        private const val RECORDING_PREFIX = "ink://audio/recordings/"
        private val RECORDING_ID = Regex("[0-9a-f-]{36}")
        private val SUPPORTED_PROCESSORS = setOf("level", "pitch")

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

    private data class Recording(val id: String, val durationMs: Long)
}
