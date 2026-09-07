package com.vandam.ink

import android.Manifest
import android.content.pm.PackageManager
import android.media.MediaMetadataRetriever
import android.media.MediaRecorder
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import java.io.File
import java.util.UUID
import org.json.JSONObject

internal fun createAudioRecording(
    activity: MainActivity,
    updateController: (Long, String) -> Unit,
    analysisActive: () -> Boolean,
): AudioRecording = InkAudioRecording(activity, updateController, analysisActive)

private class InkAudioRecording(
    private val activity: MainActivity,
    private val updateController: (Long, String) -> Unit,
    private val analysisActive: () -> Boolean,
) : AudioRecording {
    private val handler = Handler(Looper.getMainLooper())
    private val recorderProgress = object : Runnable {
        override fun run() {
            publishRecorder("recording")
            if (mediaRecorder != null) {
                handler.postDelayed(this, PROGRESS_INTERVAL_MS)
            }
        }
    }
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
                    InkManagedFiles(activity).adopt(file, "audio/mp4", id = id, duration = duration)
                }
            }
    }

    override val active get() = mediaRecorder != null
    override val source get() = lastRecording?.let { "$RECORDING_PREFIX${it.id}" }

    override fun activate(controller: Long): NativeResult {
        if (recorderController != null && recorderController != controller) {
            return protocol("Only one audio recorder can be active")
        }
        recorderController = controller
        publishRecorder(if (lastRecording == null) "idle" else "ready")
        return NativeResult.Success("")
    }

    override fun deactivate() {
        pause()
        recorderController = null
    }

    override fun pause() {
        if (active) stopRecorder()
    }

    override fun stop() {
        cancelRecorder(false)
        recorderController = null
    }

    override fun execute(operation: String): NativeResult = when (operation) {
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
        if (analysisActive()) {
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
        try {
            InkManagedFiles(activity).adopt(output, "audio/mp4", id = id, duration = duration)
        } catch (error: Exception) {
            output.delete()
            publishRecorderError("output", error.message ?: "Recording could not be saved", true)
            return NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Recording could not be saved", true)
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
        InkManagedFiles(activity).remove(recording.id)
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
            .put("name", recording?.let { "${it.id}.m4a" }.orEmpty())
            .put("mimeType", "audio/mp4")
            .put("size", recording?.let { File(recordingsDirectory(), "${it.id}.m4a").length() } ?: 0)
            .put("error", error)
            .toString()
    }

    private fun elapsedRecordingTime(): Long = if (recordingStartedAt == 0L) {
        0
    } else {
        SystemClock.elapsedRealtime() - recordingStartedAt
    }

    private fun recordingsDirectory() = File(activity.filesDir, "recordings").apply { mkdirs() }

    private fun protocol(message: String) = NativeResult.Failure(NativeErrorKind.PROTOCOL, message, false)

    private companion object {
        private const val START = "start"
        private const val STOP = "stop"
        private const val CANCEL = "cancel"
        private const val DELETE = "delete"
        private const val SAMPLE_RATE = 48_000
        private const val RECORDING_BIT_RATE = 96_000
        private const val PROGRESS_INTERVAL_MS = 250L
        private const val PARTIAL_SUFFIX = ".partial.m4a"
        private const val RECORDING_PREFIX = "ink://audio/recordings/"
        private val RECORDING_ID = Regex("[0-9a-f-]{36}")
    }

    private data class Recording(val id: String, val durationMs: Long)
}
