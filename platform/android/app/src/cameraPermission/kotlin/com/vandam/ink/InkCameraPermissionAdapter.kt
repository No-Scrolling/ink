package com.vandam.ink

import android.Manifest
import android.content.pm.PackageManager
import android.widget.FrameLayout

internal fun createCameraAdapter(
    activity: MainActivity,
    _root: FrameLayout,
    _updateController: (Long, String) -> Unit,
    _updateReview: (Long, String?) -> Unit,
    _requestOpen: (Long) -> Unit,
): CameraAdapter = InkCameraPermissionAdapter(activity)

private class InkCameraPermissionAdapter(
    private val activity: MainActivity,
) : CameraAdapter {
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
                activity.requestPermissions(arrayOf(Manifest.permission.CAMERA), REQUEST_CODE)
                complete(NativeResult.Success(""))
            }
            else -> complete(protocol("Unknown camera operation: $operation"))
        }
    }

    override fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) = complete(protocol("Camera sessions are not enabled"))

    override fun permissionDenied(
        requestId: Long,
        controller: Long,
        blocked: Boolean,
        complete: NativeResultHandler,
    ) = complete(protocol("Camera sessions are not enabled"))

    override fun cancel(requestId: Long) = Unit

    override fun syncPortal(portal: CameraPortal?) = Unit

    override fun pause() = Unit

    override fun resume() = Unit

    override fun stop() = Unit

    private fun permissionStatus(): String {
        if (activity.checkSelfPermission(Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return "granted"
        }
        val requested = activity.getSharedPreferences(PREFERENCES, 0)
            .getBoolean(REQUESTED, false)
        return when {
            !requested -> "unknown"
            activity.shouldShowRequestPermissionRationale(Manifest.permission.CAMERA) -> "denied"
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
        private const val PREFERENCES = "ink-camera"
        private const val REQUESTED = "camera-permission-requested"
        private const val REQUEST_CODE = 0xCA
    }
}
