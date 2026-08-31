package com.vandam.ink

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.Binder
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import android.webkit.MimeTypeMap
import java.io.File

class InkLightFileProvider : ContentProvider() {
    override fun onCreate(): Boolean = true

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        checkCaller()
        require(mode == "r") { "Shared Light files are read-only" }
        val file = resolve(uri)
        require(file.isFile) { "Shared Light file does not exist" }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor {
        checkCaller()
        val file = resolve(uri)
        val columns = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        return MatrixCursor(columns, 1).apply {
            addRow(columns.map { column ->
                when (column) {
                    OpenableColumns.DISPLAY_NAME -> file.name
                    OpenableColumns.SIZE -> file.length()
                    else -> null
                }
            })
        }
    }

    override fun getType(uri: Uri): String? {
        checkCaller()
        return MimeTypeMap.getSingleton().getMimeTypeFromExtension(
            resolve(uri).extension.lowercase(),
        ) ?: "application/octet-stream"
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?): Int = 0

    private fun checkCaller() {
        if (Binder.getCallingUid() != android.os.Process.SYSTEM_UID) {
            throw SecurityException("Shared Light files are available only to the system")
        }
    }

    private fun resolve(uri: Uri): File {
        val root = File(requireNotNull(context).filesDir, "shared").canonicalFile
        val relative = uri.pathSegments.joinToString(File.separator)
        val file = File(root, relative).canonicalFile
        if (!file.toPath().startsWith(root.toPath()) || file == root) {
            throw SecurityException("Shared Light file path is invalid")
        }
        return file
    }
}
