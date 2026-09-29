package com.vandam.ink

import android.content.ContentValues
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteOpenHelper
import org.json.JSONObject
import java.util.concurrent.Executors

internal class StoreAdapter(context: Context, private val onChanged: ((String) -> Unit)? = null) : NativeAdapter {
    private val appContext = context.applicationContext
    private val changedAction = "${context.packageName}.ink.STORE_CHANGED"
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.getIntExtra("process", -1) == android.os.Process.myPid()) return
            intent.getStringExtra("key")?.let { onChanged?.invoke(it) }
        }
    }
    init {
        if (onChanged != null) {
            listeners.add(onChanged)
            appContext.registerReceiver(receiver, IntentFilter(changedAction), Context.RECEIVER_NOT_EXPORTED)
        }
    }
    private val executor = Executors.newSingleThreadExecutor()
    private val database = object : SQLiteOpenHelper(context, "ink-store.db", null, 1) {
        override fun onCreate(db: SQLiteDatabase) {
            db.execSQL("CREATE TABLE entries (key TEXT PRIMARY KEY, revision INTEGER NOT NULL, version INTEGER NOT NULL, value TEXT NOT NULL)")
        }
        override fun onUpgrade(db: SQLiteDatabase, oldVersion: Int, newVersion: Int) {
            error("Unsupported store database version")
        }
    }

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        executor.execute {
            var changedKey: String? = null
            val result = try {
                val input = JSONObject(payload)
                val key = input.getString("key")
                require(key.isNotEmpty() && key.length <= 256) { "Invalid store key" }
                val db = database.writableDatabase
                when (operation) {
                    "get" -> NativeResult.Success(read(db, key)?.toString() ?: "null")
                    "write", "replace" -> {
                        db.beginTransaction()
                        try {
                            val current = read(db, key, includeValue = false)
                            val revision = current?.getLong("revision") ?: 0L
                            if (operation == "write" && revision != input.getLong("revision")) {
                                NativeResult.Success(JSONObject().put("committed", false).toString())
                            } else {
                                val version = input.getLong("version")
                                require(version in 1..9_007_199_254_740_991L) { "Invalid store version" }
                                require((current?.getLong("version") ?: 0L) <= version) { "Store $key uses a newer version" }
                                require(revision < 9_007_199_254_740_991L) { "Store revision is exhausted" }
                                val value = input.getString("value")
                                // Reserve the widest revision so later writes cannot make this value unreadable.
                                val entry = JSONObject().put("revision", 9_007_199_254_740_991L)
                                    .put("version", version).put("value", value)
                                require(javascriptResultFits(entry.toString())) { "Store value is too large" }
                                val values = ContentValues().apply {
                                    put("key", key)
                                    put("revision", revision + 1)
                                    put("version", version)
                                    put("value", value)
                                }
                                db.insertWithOnConflict("entries", null, values, SQLiteDatabase.CONFLICT_REPLACE).also {
                                    check(it != -1L) { "Could not persist store value" }
                                }
                                db.setTransactionSuccessful()
                                changedKey = key
                                NativeResult.Success(JSONObject().put("committed", true).toString())
                            }
                        } finally {
                            db.endTransaction()
                        }
                    }
                    else -> NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown store operation: $operation", false)
                }
            } catch (error: Exception) {
                NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Store operation failed", false)
            }
            complete(result)
            if (result is NativeResult.Success) changedKey?.let { key ->
                listeners.forEach { if (it !== onChanged) it(key) }
                appContext.sendBroadcast(Intent(changedAction).setPackage(appContext.packageName)
                    .putExtra("key", key).putExtra("process", android.os.Process.myPid()))
            }
        }
    }

    private fun read(db: SQLiteDatabase, key: String, includeValue: Boolean = true): JSONObject? {
        val columns = if (includeValue) arrayOf("revision", "version", "value") else arrayOf("revision", "version")
        db.query("entries", columns, "key = ?", arrayOf(key), null, null, null).use { cursor ->
            if (!cursor.moveToFirst()) return null
            val entry = JSONObject()
                .put("revision", cursor.getLong(0))
                .put("version", cursor.getLong(1))
            if (includeValue) entry.put("value", cursor.getString(2))
            return entry
        }
    }

    override fun cancel(requestId: Long) {}

    fun stop() {
        if (onChanged != null) {
            listeners.remove(onChanged)
            appContext.unregisterReceiver(receiver)
        }
        executor.execute { database.close() }
        executor.shutdown()
    }

    companion object {
        private val listeners = java.util.concurrent.CopyOnWriteArraySet<(String) -> Unit>()
    }
}
