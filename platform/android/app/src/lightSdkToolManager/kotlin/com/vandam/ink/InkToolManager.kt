package com.vandam.ink

import android.content.Context
import android.util.Log
import com.thelightphone.toolmanager.ClientLeafNode
import com.thelightphone.toolmanager.ClientToolManifest
import com.thelightphone.toolmanager.FileBrowserSpec
import org.json.JSONObject

internal class InkToolManager(private val context: Context) {
    private val config = JSONObject(context.assets.open("ink-tool-manager.json").bufferedReader().use { it.readText() })
    val manifest = ClientToolManifest(config.getString("title"), config.getJSONArray("folders").let { folders ->
        (0 until folders.length()).map { index ->
            val folder = folders.getJSONObject(index)
            val path = folder.getString("path")
            val header = folder.optString("header_text").takeUnless { folder.isNull("header_text") }
            ClientLeafNode(FileBrowserSpec(folder.getString("label"), path, header), basePath = path,
                readOnly = folder.getBoolean("read_only"), showHiddenFiles = folder.getBoolean("show_hidden_files"))
        }
    })

    fun changed() {
        val preferences = context.getSharedPreferences("ink-tool-manager", Context.MODE_PRIVATE)
        check(preferences.edit().putLong("revision", preferences.getLong("revision", 0) + 1).commit())
        val task = context.getSharedPreferences("ink-worker-state", Context.MODE_PRIVATE).getString("shared-files-task", null) ?: return
        try {
            InkWorkerJobs.enqueue(context, JSONObject().put("task", task).put("key", "ink.shared-files.change")
                .put("input", JSONObject.NULL))
        } catch (error: Exception) {
            Log.e("InkToolManager", "Could not schedule shared-files task", error)
        }
    }
}
