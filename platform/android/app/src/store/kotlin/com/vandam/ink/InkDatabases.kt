package com.vandam.ink

import android.content.ContentValues
import android.content.res.AssetManager
import android.database.Cursor
import android.database.sqlite.SQLiteCursor
import android.database.sqlite.SQLiteDatabase
import android.os.CancellationSignal
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.util.concurrent.atomic.AtomicLong

/** Called synchronously from Rust database workers, never through the activity handler. */
internal class InkDatabases(
    private val assets: AssetManager,
    private val storePath: String,
    private val filesPath: String,
) {
    fun connect(store: Boolean) = InkDatabaseConnection(assets, storePath, filesPath, store)
}

internal class InkDatabaseConnection(
    private val assets: AssetManager,
    private val storePath: String,
    private val filesPath: String,
    private val isStore: Boolean,
) {
    private var store: SQLiteDatabase? = null
    private val databases = mutableMapOf<String, SQLiteDatabase>()

    /** Value, error kind and error message; the Rust responder builds the result envelope. */
    fun execute(operation: String, payload: String, signal: CancellationSignal?): Array<String?> = try {
        signal?.throwIfCanceled()
        val input = JSONObject(payload)
        val value = if (isStore) executeStore(operation, input) else executeSqlite(operation, input, signal)
        arrayOf(value, null, null)
    } catch (error: DatabaseProtocol) {
        arrayOf(null, "protocol", error.message)
    } catch (error: Exception) {
        arrayOf(null, "unexpected", error.message ?: "Database operation failed")
    }

    fun discard(id: String) { databases.remove(id)?.close() }

    fun close() {
        store?.close()
        store = null
        databases.values.forEach { it.close() }
        databases.clear()
    }

    private fun storeDatabase(): SQLiteDatabase {
        store?.let { return it }
        File(storePath).parentFile?.mkdirs()
        val options = SQLiteDatabase.OpenParams.Builder()
            .addOpenFlags(SQLiteDatabase.CREATE_IF_NECESSARY or SQLiteDatabase.NO_LOCALIZED_COLLATORS)
            .setJournalMode("WAL")
            .setSynchronousMode("FULL")
            .build()
        val database = SQLiteDatabase.openDatabase(File(storePath), options)
        try {
            when (database.version) {
                0 -> {
                    database.beginTransactionNonExclusive()
                    try {
                        database.execSQL("CREATE TABLE IF NOT EXISTS entries (key TEXT PRIMARY KEY, revision INTEGER NOT NULL, version INTEGER NOT NULL, value TEXT NOT NULL)")
                        database.version = 1
                        database.setTransactionSuccessful()
                    } finally { database.endTransaction() }
                }
                1 -> Unit
                else -> error("Unsupported store database version")
            }
            store = database
            return database
        } catch (error: Exception) {
            database.close()
            throw error
        }
    }

    private fun executeStore(operation: String, input: JSONObject): String {
        val key = input.getString("key")
        require(key.isNotEmpty() && key.length <= 256) { "Invalid store key" }
        val db = storeDatabase()
        return when (operation) {
            "get" -> readEntry(db, key)?.toString() ?: "null"
            "write", "replace" -> {
                db.beginTransactionNonExclusive()
                try {
                    val current = readEntry(db, key, includeValue = false)
                    val revision = current?.getLong("revision") ?: 0L
                    if (operation == "write" && revision != input.getLong("revision")) {
                        "{\"committed\":false}"
                    } else {
                        val version = input.getLong("version")
                        require(version in 1..MAX_SAFE_INTEGER) { "Invalid store version" }
                        require((current?.getLong("version") ?: 0) <= version) { "Store $key uses a newer version" }
                        require(revision < MAX_SAFE_INTEGER) { "Store revision is exhausted" }
                        val value = input.getString("value")
                        val widest = JSONObject().put("revision", MAX_SAFE_INTEGER).put("version", version).put("value", value)
                        require(javascriptResultFits(widest.toString())) { "Store value is too large" }
                        val values = ContentValues().apply {
                            put("key", key)
                            put("revision", revision + 1)
                            put("version", version)
                            put("value", value)
                        }
                        check(db.insertWithOnConflict("entries", null, values, SQLiteDatabase.CONFLICT_REPLACE) != -1L) {
                            "Could not persist store value"
                        }
                        db.setTransactionSuccessful()
                        "{\"committed\":true}"
                    }
                } finally { db.endTransaction() }
            }
            else -> throw DatabaseProtocol("Unknown store operation: $operation")
        }
    }

    private fun readEntry(db: SQLiteDatabase, key: String, includeValue: Boolean = true): JSONObject? {
        val columns = if (includeValue) arrayOf("revision", "version", "value") else arrayOf("revision", "version")
        return db.query("entries", columns, "key = ?", arrayOf(key), null, null, null).use { cursor ->
            if (!cursor.moveToFirst()) null else JSONObject()
                .put("revision", cursor.getLong(0)).put("version", cursor.getLong(1)).also {
                    if (includeValue) it.put("value", cursor.getString(2))
                }
        }
    }

    private fun executeSqlite(operation: String, input: JSONObject, signal: CancellationSignal?): String = when (operation) {
        "open" -> {
            require(databases.size < 8) { "Too many open databases" }
            val asset = input.getString("asset").removePrefix("asset://")
            require(asset.matches(Regex("ink-assets/[a-zA-Z0-9._-]+\\.db"))) { "Expected an imported database asset" }
            val destination = File(filesPath, asset)
            synchronized(copyLock) {
                if (!destination.exists()) {
                    destination.parentFile!!.mkdirs()
                    val temporary = File(destination.path + ".partial")
                    try {
                        assets.open(asset).use { source -> FileOutputStream(temporary).use { target ->
                            source.copyTo(target)
                            target.fd.sync()
                        } }
                        check(temporary.renameTo(destination)) { "Could not install database asset" }
                    } finally { temporary.delete() }
                }
            }
            signal?.throwIfCanceled()
            val id = nextDatabase.getAndIncrement().toString()
            databases[id] = SQLiteDatabase.openDatabase(destination.path, null,
                SQLiteDatabase.OPEN_READONLY or SQLiteDatabase.NO_LOCALIZED_COLLATORS)
            id
        }
        "query" -> {
            val database = databases[input.getString("id")] ?: error("Database is closed")
            val sql = input.getString("sql")
            require(sql.length <= 65_536) { "SQL is too long" }
            require(readOnly.containsMatchIn(sql)) { "Read-only queries must start with SELECT, WITH or EXPLAIN" }
            val parameters = input.getJSONArray("parameters")
            val args = Array<Any>(parameters.length()) { index ->
                parameters.get(index).also { value ->
                    require(value == JSONObject.NULL || value is String || value is Number) { "Invalid SQL parameter" }
                    if (value is Number) require(value.toDouble().isFinite()) { "SQL numbers must be finite" }
                }
            }
            val result = JSONArray()
            var bytes = 0
            database.rawQueryWithFactory({ _, driver, table, query ->
                args.forEachIndexed { index, value -> when (value) {
                    JSONObject.NULL -> query.bindNull(index + 1)
                    is String -> query.bindString(index + 1, value)
                    is Int -> query.bindLong(index + 1, value.toLong())
                    is Long -> query.bindLong(index + 1, value)
                    is Number -> query.bindDouble(index + 1, value.toDouble())
                } }
                SQLiteCursor(driver, table.orEmpty(), query)
            }, sql, emptyArray(), "", signal).use { cursor ->
                while (cursor.moveToNext()) {
                    signal?.throwIfCanceled()
                    require(result.length() < 10_000) { "Query returns too many rows; add a LIMIT" }
                    val row = JSONObject()
                    for (column in 0 until cursor.columnCount) {
                        val value = when (cursor.getType(column)) {
                            Cursor.FIELD_TYPE_NULL -> JSONObject.NULL
                            Cursor.FIELD_TYPE_INTEGER -> cursor.getLong(column).also {
                                require(it in -MAX_SAFE_INTEGER..MAX_SAFE_INTEGER) { "Integer exceeds JavaScript precision; cast it to text" }
                            }
                            Cursor.FIELD_TYPE_FLOAT -> cursor.getDouble(column).also {
                                require(it.isFinite()) { "SQL numbers must be finite" }
                            }
                            Cursor.FIELD_TYPE_STRING -> cursor.getString(column)
                            else -> error("Binary SQL values are unsupported; select text or numeric columns")
                        }
                        row.put(cursor.getColumnName(column), value)
                    }
                    bytes += row.toString().toByteArray(Charsets.UTF_8).size
                    require(bytes < 400_000) { "Query result is too large; add a LIMIT" }
                    result.put(row)
                }
            }
            result.toString()
        }
        "close" -> { discard(input.getString("id")); "" }
        else -> throw DatabaseProtocol("Unknown database operation")
    }

    private class DatabaseProtocol(message: String) : RuntimeException(message)
    private companion object {
        const val MAX_SAFE_INTEGER = 9_007_199_254_740_991L
        val nextDatabase = AtomicLong(1)
        val copyLock = Any()
        val readOnly = Regex("(?is)^\\s*(?:(?:--[^\\n]*(?:\\n|$)|/\\*.*?\\*/)\\s*)*(SELECT|WITH|EXPLAIN)\\b")
    }
}
