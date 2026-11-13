package com.vandam.ink

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import java.io.File
import java.io.FileNotFoundException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

class AudioContractProvider : ContentProvider() {
    override fun onCreate() = true
    override fun getType(uri: Uri) = "audio/wav"
    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        when (uri.lastPathSegment) {
            "denied" -> throw SecurityException("Audio permission revoked")
            "missing" -> throw FileNotFoundException("Audio item deleted")
            "held" -> hold?.await(10, TimeUnit.SECONDS)
        }
        return ParcelFileDescriptor.open(File(requireNotNull(context).cacheDir, "audio-content-contract.wav"), ParcelFileDescriptor.MODE_READ_ONLY)
    }
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = null
    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0

    companion object {
        // Keeps a "held" item loading until the contract releases it.
        @Volatile var hold: CountDownLatch? = null
    }
}
