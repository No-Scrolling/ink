package com.vandam.ink

import android.app.Activity
import android.content.Intent

internal fun createFilesAdapter(activity: Activity): FilesAdapter = NoFilesAdapter

private object NoFilesAdapter : FilesAdapter {
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Files are not included in this app", false))
    }
    override fun cancel(requestId: Long) = Unit
    override fun onActivityResult(requestCode: Int, resultCode: Int, intent: Intent?) = false
    override fun onRequestPermissionsResult(requestCode: Int) = false
    override fun stop() = Unit
}
