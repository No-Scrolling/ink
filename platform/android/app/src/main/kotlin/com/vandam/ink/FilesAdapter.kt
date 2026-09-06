package com.vandam.ink

import android.content.Intent

internal interface FilesAdapter : NativeAdapter {
    fun onActivityResult(requestCode: Int, resultCode: Int, intent: Intent?): Boolean
    fun onRequestPermissionsResult(requestCode: Int): Boolean
    fun stop()
}
