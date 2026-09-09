package com.vandam.ink

import android.content.ClipData
import android.content.ClipboardManager
import com.vandam.ink.keyboard.InkKeyboardView
import com.vandam.ink.keyboard.KeyboardAction
import org.json.JSONObject

internal class InputActions(
    private val activity: MainActivity,
    private val keyboard: InkKeyboardView,
    private val onEdit: TextEditHandler,
) {
    private var context: JSONObject? = null

    init {
        keyboard.onDismissActions = { dismiss() }
        keyboard.onEditAction = { action ->
            val current = context
            if (current != null) {
                val text = current.getString("text")
                when (action) {
                    "Copy" -> if (text.isNotEmpty()) {
                        activity.getSystemService(ClipboardManager::class.java)
                            .setPrimaryClip(ClipData.newPlainText("", text))
                    }
                    "Paste" -> {
                        val clip = activity.getSystemService(ClipboardManager::class.java).primaryClip
                        val pasted = if (clip != null && clip.itemCount > 0) clip.getItemAt(0).text?.toString() else null
                        if (pasted != null) {
                            val normalised = pasted.replace("\r\n", "\n").replace('\r', '\n')
                            val replacement = if (keyboard.numeric) {
                                normalised.filter { it in '0'..'9' }
                            } else {
                                normalised.replace('\t', ' ')
                                    .let { if (keyboard.action == KeyboardAction.Return) it else it.replace('\n', ' ') }
                                    .filter { !it.isISOControl() || it == '\n' }
                            }
                            if (replacement.isNotEmpty()) {
                                val cursor = current.getInt("cursor")
                                replace(current, cursor, cursor, replacement)
                            }
                        }
                    }
                    "Clear" -> replace(current, 0, text.length, "")
                }
            }
            dismiss()
        }
    }

    fun sync(encoded: String) {
        context = if (encoded.isEmpty()) null else JSONObject(encoded)
        keyboard.editActions = context?.optBoolean("menu") == true
    }

    fun close() {
        context = null
        keyboard.editActions = false
    }

    fun dismiss(): Boolean {
        if (!keyboard.editActions) return false
        keyboard.editActions = false
        context?.let { onEdit(TextEdit.Assistance(
            JSONObject().put("id", it.getInt("id")).put("text", it.getString("text"))
                .put("dismiss", true).toString(),
        )) }
        return true
    }

    private fun replace(current: JSONObject, start: Int, end: Int, replacement: String) {
        onEdit(TextEdit.Assistance(
            JSONObject().put("id", current.getInt("id")).put("text", current.getString("text"))
                .put("cursor", current.getInt("cursor")).put("start", start).put("end", end)
                .put("replacement", replacement).toString(),
        ))
    }
}
