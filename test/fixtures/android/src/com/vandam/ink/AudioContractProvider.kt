package com.vandam.ink

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import java.io.File
import java.io.FileNotFoundException

class AudioContractProvider : ContentProvider() {
    override fun onCreate() = true
    override fun getType(uri: Uri) = "audio/wav"
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        when (uri.lastPathSegment) {
            "denied" -> throw SecurityException("Audio permission revoked")
            "missing" -> throw FileNotFoundException("Audio item deleted")
        }
        return ParcelFileDescriptor.open(File(requireNotNull(context).cacheDir, "audio-content-contract.wav"), ParcelFileDescriptor.MODE_READ_ONLY)
    }
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0
}
