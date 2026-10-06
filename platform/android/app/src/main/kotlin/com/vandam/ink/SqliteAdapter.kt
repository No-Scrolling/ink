package com.vandam.ink

import android.content.Context
import android.database.Cursor
import android.database.sqlite.SQLiteDatabase
import android.database.sqlite.SQLiteCursor
import android.os.CancellationSignal
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

internal class SqliteAdapter(private val context: Context) : NativeAdapter {
    private val executor = Executors.newSingleThreadExecutor()
    private val databases = mutableMapOf<String, SQLiteDatabase>()
    private val pending = ConcurrentHashMap<Long, CancellationSignal>()
    private val stopped = AtomicBoolean(false)

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        if (stopped.get()) { complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Database adapter is stopped", false)); return }
        val signal = CancellationSignal()
        pending[requestId] = signal
        try { executor.execute {
            var opened: String? = null
            val result = try {
                signal.throwIfCanceled()
                val input = JSONObject(payload)
                when (operation) {
                    "open" -> {
                        require(databases.size < 8) { "Too many open databases" }
                        val asset = input.getString("asset").removePrefix("asset://")
                        require(asset.matches(Regex("ink-assets/[a-zA-Z0-9._-]+\\.db"))) { "Expected an imported database asset" }
                        val destination = File(context.filesDir, asset)
                        synchronized(copyLock) {
                            if (!destination.exists()) {
                                destination.parentFile!!.mkdirs()
                                val temporary = File(destination.path + ".partial")
                                try {
                                    context.assets.open(asset).use { source -> temporary.outputStream().use(source::copyTo) }
                                    check(temporary.renameTo(destination)) { "Could not install database asset" }
                                } finally { temporary.delete() }
                            }
                        }
                        signal.throwIfCanceled()
                        val id = UUID.randomUUID().toString()
                        databases[id] = SQLiteDatabase.openDatabase(destination.path, null, SQLiteDatabase.OPEN_READONLY)
                        opened = id
                        NativeResult.Success(id) {
                            if (!stopped.get()) {
                                try { executor.execute { databases.remove(id)?.close() } }
                                catch (_: java.util.concurrent.RejectedExecutionException) { }
                            }
                        }
                    }
                    "query" -> {
                        val database = databases[input.getString("id")] ?: error("Database is closed")
                        val sql = input.getString("sql")
                        require(sql.length <= 65536) { "SQL is too long" }
                        require(Regex("(?is)^\\s*(?:(?:--[^\\n]*(?:\\n|$)|/\\*.*?\\*/)\\s*)*(SELECT|WITH|EXPLAIN)\\b").containsMatchIn(sql)) {
                            "Read-only queries must start with SELECT, WITH or EXPLAIN"
                        }
                        val parameters = input.getJSONArray("parameters")
                        val args = Array<Any>(parameters.length()) { index ->
                            val value = parameters.get(index)
                            require(value == JSONObject.NULL || value is String || value is Number) { "Invalid SQL parameter" }
                            if (value is Number) require(value.toDouble().isFinite()) { "SQL numbers must be finite" }
                            value
                        }
                        val rows = JSONArray()
                        var bytes = 0
                        database.rawQueryWithFactory({ _, driver, table, query ->
                            args.forEachIndexed { index, value ->
                                when (value) {
                                    JSONObject.NULL -> query.bindNull(index + 1)
                                    is String -> query.bindString(index + 1, value)
                                    is Int -> query.bindLong(index + 1, value.toLong())
                                    is Long -> query.bindLong(index + 1, value)
                                    is Number -> query.bindDouble(index + 1, value.toDouble())
                                }
                            }
                            SQLiteCursor(driver, table.orEmpty(), query)
                        }, sql, emptyArray(), "", signal).use { cursor ->
                            while (cursor.moveToNext()) {
                                signal.throwIfCanceled()
                                require(rows.length() < 10000) { "Query returns too many rows; add a LIMIT" }
                                val row = JSONObject()
                                for (column in 0 until cursor.columnCount) {
                                    val value = when (cursor.getType(column)) {
                                        Cursor.FIELD_TYPE_NULL -> JSONObject.NULL
                                        Cursor.FIELD_TYPE_INTEGER -> cursor.getLong(column).also {
                                            require(it in -9007199254740991L..9007199254740991L) { "Integer exceeds JavaScript precision; cast it to text" }
                                        }
                                        Cursor.FIELD_TYPE_FLOAT -> cursor.getDouble(column)
                                        Cursor.FIELD_TYPE_STRING -> cursor.getString(column)
                                        else -> error("Binary SQL values are unsupported; select text or numeric columns")
                                    }
                                    row.put(cursor.getColumnName(column), value)
                                }
                                bytes += row.toString().toByteArray().size
                                require(bytes < 400000) { "Query result is too large; add a LIMIT" }
                                rows.put(row)
                            }
                        }
                        NativeResult.Success(rows.toString())
                    }
                    "close" -> {
                        databases.remove(input.getString("id"))?.close()
                        NativeResult.Success("")
                    }
                    else -> NativeResult.Failure(NativeErrorKind.PROTOCOL, "Unknown database operation", false)
                }
            } catch (error: Exception) {
                NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Database operation failed", false)
            }
            if (signal.isCanceled || stopped.get()) opened?.let { databases.remove(it)?.close() }
            pending.remove(requestId, signal)
            if (!signal.isCanceled && !stopped.get()) complete(result)
        } } catch (error: java.util.concurrent.RejectedExecutionException) {
            pending.remove(requestId, signal)
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Database adapter is stopped", false))
        }
    }

    override fun cancel(requestId: Long) { pending[requestId]?.cancel() }
    fun reset() {
        pending.values.forEach { it.cancel() }
        executor.execute { databases.values.forEach { it.close() }; databases.clear() }
    }
    fun stop() {
        if (!stopped.compareAndSet(false, true)) return
        pending.values.forEach { it.cancel() }
        executor.execute { databases.values.forEach { it.close() }; databases.clear() }
        executor.shutdown()
    }
    companion object { private val copyLock = Any() }
}
