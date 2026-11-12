package com.vandam.ink

import android.content.ContentUris
import android.content.Context
import android.database.Cursor
import android.net.Uri
import android.os.CancellationSignal
import android.provider.ContactsContract.CommonDataKinds.Email
import android.provider.ContactsContract.CommonDataKinds.Event
import android.provider.ContactsContract.CommonDataKinds.Note
import android.provider.ContactsContract.CommonDataKinds.Organization
import android.provider.ContactsContract.CommonDataKinds.Phone
import android.provider.ContactsContract.CommonDataKinds.StructuredPostal
import android.provider.ContactsContract.CommonDataKinds.Website
import android.provider.ContactsContract.Contacts
import android.provider.ContactsContract.Data
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

internal fun createContactsAdapter(context: Context): ContactsAdapter = InkContactsAdapter(context.applicationContext)

// Details follow LightContacts in Light SDK 0.2.7 so Ink apps show the same labels and values as LightOS.
private class InkContactsAdapter(private val context: Context) : ContactsAdapter {
    private val executor = Executors.newSingleThreadExecutor { task -> Thread(task, "ink-contacts") }
    private val pending = ConcurrentHashMap<Long, CancellationSignal>()
    @Volatile private var stopped = false

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        if (stopped) return
        val signal = CancellationSignal()
        pending[requestId] = signal
        executor.execute {
            val result = try {
                val data = JSONObject(payload)
                NativeResult.Success(when (operation) {
                    "list" -> list(data.optString("query").trim(), data.optInt("offset"), data.optInt("limit"), signal).toString()
                    "get" -> details(data.getString("id"), signal)?.toString() ?: "null"
                    else -> error("Unknown contacts operation")
                })
            } catch (error: SecurityException) {
                NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, "Allow contacts access to read contacts", false)
            } catch (error: IllegalArgumentException) {
                NativeResult.Failure(NativeErrorKind.PROTOCOL, error.message ?: "Invalid contacts request", false)
            } catch (error: Exception) {
                NativeResult.Failure(NativeErrorKind.UNAVAILABLE, error.message ?: "Contacts operation failed", true)
            }
            if (pending.remove(requestId) === signal && !stopped && !signal.isCanceled) complete(result)
        }
    }

    // Pages are cursor positions, so blank names still count towards `next`.
    private fun list(query: String, offset: Int, limit: Int, signal: CancellationSignal): JSONObject {
        require(offset >= 0 && limit > 0) { "Invalid contacts page" }
        val contacts = JSONArray()
        var next: Int? = null
        var bytes = 0
        val uri = if (query.isEmpty()) Contacts.CONTENT_URI else Uri.withAppendedPath(Contacts.CONTENT_FILTER_URI, Uri.encode(query))
        context.contentResolver.query(
            uri,
            arrayOf(Contacts._ID, Contacts.LOOKUP_KEY, Contacts.DISPLAY_NAME_PRIMARY),
            null,
            null,
            "${Contacts.DISPLAY_NAME_PRIMARY} COLLATE LOCALIZED ASC",
            signal,
        )?.use { cursor ->
            var position = offset
            while (cursor.moveToPosition(position)) {
                val name = cursor.getString(2).orEmpty()
                if (name.isNotBlank()) {
                    val contact = JSONObject().put("id", contactId(cursor.getLong(0), cursor.getString(1))).put("name", name)
                    // Counted as escaped inside the result envelope, which is what the message limit measures.
                    val size = JSONObject.quote(contact.toString()).toByteArray(Charsets.UTF_8).size - 1
                    if (contacts.length() == limit || contacts.length() > 0 && bytes + size > PAGE_BYTES) {
                        next = position
                        break
                    }
                    bytes += size
                    contacts.put(contact)
                }
                position++
            }
        }
        return JSONObject().put("contacts", contacts).put("next", next ?: JSONObject.NULL)
    }

    // IDs pair the row ID with the lookup key, so a contact can still be found after sync re-aggregates it.
    private fun contactId(id: Long, lookupKey: String?) = "$id:${lookupKey.orEmpty()}"

    private fun details(contactId: String, signal: CancellationSignal): JSONObject? {
        val id = contactId.substringBefore(':').toLongOrNull()
        val lookupKey = contactId.substringAfter(':', "")
        require(id != null && id > 0 && ':' in contactId) { "Invalid contact ID" }
        val uri = if (lookupKey.isEmpty()) ContentUris.withAppendedId(Contacts.CONTENT_URI, id) else Contacts.getLookupUri(id, lookupKey)
        val contact = context.contentResolver.query(uri, arrayOf(Contacts._ID, Contacts.LOOKUP_KEY, Contacts.DISPLAY_NAME_PRIMARY), null, null, null, signal)
            ?.use { cursor -> if (cursor.moveToFirst()) Triple(cursor.getLong(0), cursor.getString(1), cursor.getString(2).orEmpty()) else null }
            ?: return null
        val current = contact.first
        val resources = context.resources
        return JSONObject()
            .put("id", contactId(current, contact.second))
            .put("name", contact.third)
            .put("phones", labeled(Phone.CONTENT_URI, Phone.NUMBER, Phone.TYPE, Phone.LABEL, "${Phone.CONTACT_ID} = ?", arrayOf("$current"), signal) { type, label ->
                Phone.getTypeLabel(resources, type, label)
            })
            .put("emails", labeled(Email.CONTENT_URI, Email.ADDRESS, Email.TYPE, Email.LABEL, "${Email.CONTACT_ID} = ?", arrayOf("$current"), signal) { type, label ->
                Email.getTypeLabel(resources, type, label)
            })
            .put("addresses", addresses(current, signal))
            .put("websites", labeled(Data.CONTENT_URI, Website.URL, Website.TYPE, Website.LABEL, DATA_SELECTION, arrayOf("$current", Website.CONTENT_ITEM_TYPE), signal, ::websiteLabel))
            .put("events", labeled(Data.CONTENT_URI, Event.START_DATE, Event.TYPE, Event.LABEL, DATA_SELECTION, arrayOf("$current", Event.CONTENT_ITEM_TYPE), signal) { type, label ->
                Event.getTypeLabel(resources, type, label)
            })
            .put("organization", rows(arrayOf(Organization.TITLE, Organization.COMPANY, Organization.DEPARTMENT), Organization.CONTENT_ITEM_TYPE, current, signal)
                .map { it.joinToString(", ") }.distinct().joinToString("\n").ifBlank { null } ?: JSONObject.NULL)
            .put("note", rows(arrayOf(Note.NOTE), Note.CONTENT_ITEM_TYPE, current, signal)
                .map { it.single() }.distinct().joinToString("\n\n").ifBlank { null } ?: JSONObject.NULL)
    }

    private fun labeled(
        uri: Uri,
        valueColumn: String,
        typeColumn: String,
        labelColumn: String,
        selection: String,
        arguments: Array<String>,
        signal: CancellationSignal,
        typeLabel: (Int, String?) -> CharSequence,
    ): JSONArray {
        val rows = linkedSetOf<Pair<String, String>>()
        query(uri, arrayOf(valueColumn, typeColumn, labelColumn), selection, arguments, signal) { cursor ->
            val value = cursor.getString(0)
            if (!value.isNullOrBlank()) rows += typeLabel(cursor.getInt(1), cursor.getString(2)).toString() to value
        }
        return JSONArray(rows.map { (label, value) -> JSONObject().put("label", label).put("value", value) })
    }

    private fun addresses(id: Long, signal: CancellationSignal): JSONArray {
        val rows = linkedSetOf<Pair<String, String>>()
        val columns = arrayOf(
            StructuredPostal.FORMATTED_ADDRESS, StructuredPostal.STREET, StructuredPostal.CITY, StructuredPostal.REGION,
            StructuredPostal.POSTCODE, StructuredPostal.COUNTRY, StructuredPostal.TYPE, StructuredPostal.LABEL,
        )
        query(StructuredPostal.CONTENT_URI, columns, "${StructuredPostal.CONTACT_ID} = ?", arrayOf("$id"), signal) { cursor ->
            val formatted = cursor.getString(0)
            val value = if (!formatted.isNullOrBlank()) formatted else (1..5).mapNotNull { cursor.getString(it)?.takeIf(String::isNotBlank) }.joinToString(", ")
            if (value.isNotBlank()) {
                rows += StructuredPostal.getTypeLabel(context.resources, cursor.getInt(6), cursor.getString(7)).toString() to value
            }
        }
        return JSONArray(rows.map { (label, value) -> JSONObject().put("label", label).put("value", value) })
    }

    private fun rows(columns: Array<String>, mimeType: String, id: Long, signal: CancellationSignal): List<List<String>> {
        val rows = mutableListOf<List<String>>()
        query(Data.CONTENT_URI, columns, DATA_SELECTION, arrayOf("$id", mimeType), signal) { cursor ->
            val values = columns.indices.mapNotNull { cursor.getString(it)?.takeIf(String::isNotBlank) }
            if (values.isNotEmpty()) rows += values
        }
        return rows
    }

    private fun query(uri: Uri, columns: Array<String>, selection: String, arguments: Array<String>, signal: CancellationSignal, read: (Cursor) -> Unit) {
        context.contentResolver.query(uri, columns, selection, arguments, null, signal)?.use { cursor ->
            while (cursor.moveToNext()) read(cursor)
        }
    }

    private fun websiteLabel(type: Int, custom: String?): String {
        if (!custom.isNullOrBlank()) return custom
        return when (type) {
            Website.TYPE_HOME -> "Home"
            Website.TYPE_WORK -> "Work"
            Website.TYPE_HOMEPAGE -> "Homepage"
            Website.TYPE_BLOG -> "Blog"
            Website.TYPE_PROFILE -> "Profile"
            Website.TYPE_FTP -> "FTP"
            Website.TYPE_OTHER -> "Other"
            else -> ""
        }
    }

    override fun cancel(requestId: Long) {
        pending[requestId]?.cancel()
    }

    override fun stop() {
        stopped = true
        pending.values.forEach { it.cancel() }
        executor.shutdownNow()
    }

    private companion object {
        val DATA_SELECTION = "${Data.CONTACT_ID} = ? AND ${Data.MIMETYPE} = ?"
        // Half the 1 MiB message limit, so a page and its envelope always fit.
        const val PAGE_BYTES = 512 * 1024
    }
}
