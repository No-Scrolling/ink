package com.vandam.ink

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import org.json.JSONObject

internal class ClipboardAdapter(context: Context) : NativeAdapter {
    private val clipboard = context.getSystemService(ClipboardManager::class.java)

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val result = when (operation) {
            "write-text" -> {
                val text = JSONObject(payload).getString("text")
                clipboard.setPrimaryClip(ClipData.newPlainText("", text))
                NativeResult.Success("")
            }
            "read-text" -> {
                val clip = clipboard.primaryClip
                val text = if (clip != null && clip.itemCount > 0) clip.getItemAt(0).text else null
                NativeResult.Success(text?.let { JSONObject.quote(it.toString()) } ?: "null")
            }
            else -> NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown clipboard operation: $operation", false)
        }
        complete(result)
    }

    override fun cancel(requestId: Long) = Unit
}
