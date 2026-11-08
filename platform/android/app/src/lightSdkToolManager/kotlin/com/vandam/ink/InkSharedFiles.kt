package com.vandam.ink

import android.content.Context
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.RandomAccessFile
import java.io.OutputStream
import java.nio.file.Files
import java.nio.file.StandardCopyOption

internal class InkSharedFiles(private val context: Context) {
    private val root = File(context.filesDir, "shared").canonicalFile.also { it.mkdirs() }

    fun resolve(path: String, allowRoot: Boolean = false): File {
        require(!path.startsWith('/') && !path.contains('\\') && !path.contains('\u0000')) { "Expected a relative shared path" }
        val file = File(root, path).canonicalFile
        require(file.toPath().startsWith(root.toPath()) && (allowRoot || file != root)) { "Path escapes shared files" }
        return file
    }

    fun list(path: String): JSONArray {
        val directory = resolve(path, true)
        require(directory.isDirectory) { "Shared directory does not exist: $path" }
        return JSONArray().apply {
            directory.listFiles()?.sortedBy { it.name }?.filterNot { Files.isSymbolicLink(it.toPath()) }?.forEach { file ->
                val relative = file.relativeTo(root).path
                put(JSONObject().put("name", file.name).put("path", relative)
                    .put("uri", Uri.Builder().scheme("content").authority("${context.packageName}.lightfileprovider").path(relative).build().toString())
                    .put("isFile", file.isFile).put("size", if (file.isFile) file.length() else 0)
                    .put("lastModified", file.lastModified()))
            }
        }
    }

    fun read(path: String, offset: Long, length: Int): ByteArray {
        require(offset >= 0 && length in 0..MAX_BYTES) { "Invalid shared file range" }
        return RandomAccessFile(resolve(path), "r").use { file ->
            file.seek(offset)
            val bytes = ByteArray(minOf(length.toLong(), (file.length() - offset).coerceAtLeast(0)).toInt())
            file.readFully(bytes)
            bytes
        }
    }

    fun write(path: String, bytes: ByteArray) = writeStream(path) { it.write(bytes) }

    fun copyFrom(path: String, source: String) {
        val input = if (source.startsWith("ink-file://")) InkManagedFiles(context).resolve(source).inputStream()
        else {
            val uri = Uri.parse(source)
            require(uri.scheme == "content") { "Expected a managed file or content URI" }
            requireNotNull(context.contentResolver.openInputStream(uri)) { "Could not open shared file source" }
        }
        input.use { stream ->
            writeStream(path) { output ->
                val buffer = ByteArray(64 * 1024)
                while (true) {
                    check(!Thread.currentThread().isInterrupted) { "Shared file copy cancelled" }
                    val count = stream.read(buffer)
                    if (count < 0) break
                    output.write(buffer, 0, count)
                }
            }
        }
    }

    private fun writeStream(path: String, write: (OutputStream) -> Unit) {
        val file = resolve(path)
        val parent = file.parentFile!!
        check(parent.isDirectory || parent.mkdirs()) { "Could not create shared directory" }
        val temporary = File.createTempFile(".${file.name}.", ".tmp", parent)
        try {
            temporary.outputStream().use(write)
            check(temporary.setLastModified(System.currentTimeMillis())) { "Could not update shared file timestamp" }
            Files.move(temporary.toPath(), file.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
        } finally {
            temporary.delete()
        }
    }

    companion object { const val MAX_BYTES = 512 * 1024 }
}
