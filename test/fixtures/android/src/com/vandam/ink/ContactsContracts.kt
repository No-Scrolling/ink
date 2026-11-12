package com.vandam.ink

import android.Manifest
import android.app.Instrumentation
import android.content.ContentProviderOperation
import android.content.ContentUris
import android.content.ContentValues
import android.os.Looper
import android.provider.ContactsContract
import android.provider.ContactsContract.AggregationExceptions
import android.provider.ContactsContract.CommonDataKinds.Email
import android.provider.ContactsContract.CommonDataKinds.Event
import android.provider.ContactsContract.CommonDataKinds.Note
import android.provider.ContactsContract.CommonDataKinds.Organization
import android.provider.ContactsContract.CommonDataKinds.Phone
import android.provider.ContactsContract.CommonDataKinds.StructuredName
import android.provider.ContactsContract.CommonDataKinds.StructuredPostal
import android.provider.ContactsContract.CommonDataKinds.Website
import android.provider.ContactsContract.Data
import android.provider.ContactsContract.RawContacts
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

// Contacts are real rows in the device's ContactsProvider; the adapter under test is production code.
internal class ContactsContracts(private val instrumentation: Instrumentation) {
    private val context = instrumentation.targetContext
    private val resolver = context.contentResolver
    private val rawContacts = mutableListOf<Long>()

    fun run(test: (String, () -> Unit) -> Unit) {
        test("contacts list named contacts in localized order and filter them", ::list)
        test("contact details use LightOS labels and drop blank and duplicate values", ::details)
        test("contact IDs survive re-aggregation and resolve to null after deletion", ::identity)
        test("contacts beyond the message limit list in pages without gaps or duplicates", ::paging)
    }

    private fun withContacts(test: (ContactsAdapter) -> Unit) {
        for (permission in listOf(Manifest.permission.READ_CONTACTS, Manifest.permission.WRITE_CONTACTS)) {
            instrumentation.uiAutomation.grantRuntimePermission(context.packageName, permission)
        }
        val adapter = createContactsAdapter(context)
        try {
            test(adapter)
        } finally {
            adapter.stop()
            for (ids in rawContacts.chunked(400)) {
                resolver.applyBatch(ContactsContract.AUTHORITY, ArrayList(ids.map { id ->
                    ContentProviderOperation.newDelete(
                        ContentUris.withAppendedId(RawContacts.CONTENT_URI, id).buildUpon()
                            .appendQueryParameter(ContactsContract.CALLER_IS_SYNCADAPTER, "true").build(),
                    ).build()
                }))
            }
            rawContacts.clear()
        }
    }

    private fun call(adapter: ContactsAdapter, operation: String, payload: JSONObject): String {
        val results = LinkedBlockingQueue<Pair<NativeResult, Boolean>>()
        instrumentation.runOnMainSync {
            adapter.execute(1, operation, payload.toString()) { results.add(it to (Looper.myLooper() == Looper.getMainLooper())) }
        }
        val (result, onMain) = checkNotNull(results.poll(10, TimeUnit.SECONDS)) { "Contacts completion did not arrive" }
        check(!onMain) { "Contacts queried on the main thread" }
        return (result as? NativeResult.Success)?.value ?: error("Contacts $operation failed: $result")
    }

    // Pages the way @ink/contacts does, checking that each page fits in one bridge message.
    private fun pages(adapter: ContactsAdapter, query: String? = null, limit: Int = 2000): List<List<Pair<String, String>>> {
        val pages = mutableListOf<List<Pair<String, String>>>()
        var offset: Int? = 0
        while (offset != null) {
            val value = call(adapter, "list", JSONObject().put("query", query).put("offset", offset).put("limit", limit))
            check(javascriptResultFits(value)) { "A contacts page of ${value.toByteArray().size} bytes exceeds the message limit" }
            val page = JSONObject(value)
            val contacts = page.getJSONArray("contacts")
            val next = if (page.isNull("next")) null else page.getInt("next")
            check(contacts.length() <= limit && (next == null || next > offset && contacts.length() > 0)) { "Invalid page at $offset: $next" }
            pages += (0 until contacts.length()).map { contacts.getJSONObject(it) }.map { it.getString("name") to it.getString("id") }
            offset = next
        }
        return pages
    }

    private fun list(adapter: ContactsAdapter, query: String? = null): List<Pair<String, String>> =
        pages(adapter, query).flatten().filter { it.first.endsWith(MARKER) }

    private fun insert(vararg rows: Pair<String, ContentValues>): Long {
        val operations = arrayListOf(
            ContentProviderOperation.newInsert(RawContacts.CONTENT_URI)
                .withValue(RawContacts.ACCOUNT_TYPE, null).withValue(RawContacts.ACCOUNT_NAME, null).build(),
        )
        for ((mimeType, values) in rows) {
            operations += ContentProviderOperation.newInsert(Data.CONTENT_URI)
                .withValueBackReference(Data.RAW_CONTACT_ID, 0).withValue(Data.MIMETYPE, mimeType).withValues(values).build()
        }
        val id = ContentUris.parseId(checkNotNull(resolver.applyBatch(ContactsContract.AUTHORITY, operations)[0].uri))
        rawContacts += id
        return id
    }

    private fun insertNames(names: List<String>) {
        val operations = ArrayList<ContentProviderOperation>()
        for (name in names) {
            val rawContact = operations.size
            operations += ContentProviderOperation.newInsert(RawContacts.CONTENT_URI)
                .withValue(RawContacts.ACCOUNT_TYPE, null).withValue(RawContacts.ACCOUNT_NAME, null).build()
            operations += ContentProviderOperation.newInsert(Data.CONTENT_URI)
                .withValueBackReference(Data.RAW_CONTACT_ID, rawContact).withValue(Data.MIMETYPE, StructuredName.CONTENT_ITEM_TYPE)
                .withValue(StructuredName.DISPLAY_NAME, name).build()
        }
        val results = resolver.applyBatch(ContactsContract.AUTHORITY, operations)
        for (index in results.indices step 2) rawContacts += ContentUris.parseId(checkNotNull(results[index].uri))
    }

    private fun name(name: String) = StructuredName.CONTENT_ITEM_TYPE to ContentValues().apply { put(StructuredName.DISPLAY_NAME, name) }
    private fun row(mimeType: String, vararg values: Pair<String, Any?>) = mimeType to ContentValues().apply {
        for ((key, value) in values) when (value) {
            is Int -> put(key, value)
            else -> put(key, value as String?)
        }
    }

    private fun contactOf(rawContact: Long): Long = checkNotNull(resolver.query(
        ContentUris.withAppendedId(RawContacts.CONTENT_URI, rawContact), arrayOf(RawContacts.CONTACT_ID), null, null, null,
    )?.use { if (it.moveToFirst()) it.getLong(0) else null })

    private fun list() = withContacts { adapter ->
        for (contact in listOf("Zoë", "adam", "Émile", "Bob", "Mary Allen")) insert(name("$contact $MARKER"))
        val blank = contactOf(insert(row(Note.CONTENT_ITEM_TYPE, Note.NOTE to "No name")))

        // Localized collation ignores case and accents; byte order would put Bob, Mary and Zoë before adam and Émile.
        check(list(adapter).map { it.first } == listOf("adam", "Bob", "Émile", "Mary Allen", "Zoë").map { "$it $MARKER" }) { list(adapter).toString() }
        for ((name, id) in pages(adapter).flatten()) {
            check(name.isNotBlank() && !id.startsWith("$blank:")) { "Listed a contact without a name: $id" }
        }
        check(list(adapter, "al").map { it.first } == listOf("Mary Allen $MARKER")) { list(adapter, "al").toString() }
        check(list(adapter, "émi").map { it.first } == listOf("Émile $MARKER")) { list(adapter, "émi").toString() }
        check(list(adapter, "ob").isEmpty()) { "The filter matched inside a word" }
        check(list(adapter, "  ").size == 5) { "A blank query filtered contacts" }
    }

    private fun details() = withContacts { adapter ->
        insert(
            name("Ada $MARKER"),
            row(Phone.CONTENT_ITEM_TYPE, Phone.NUMBER to "+44 20 7946 0000", Phone.TYPE to Phone.TYPE_MOBILE),
            row(Phone.CONTENT_ITEM_TYPE, Phone.NUMBER to "+44 20 7946 0001", Phone.TYPE to Phone.TYPE_CUSTOM, Phone.LABEL to "Boat"),
            row(Phone.CONTENT_ITEM_TYPE, Phone.NUMBER to "   ", Phone.TYPE to Phone.TYPE_HOME),
            row(Email.CONTENT_ITEM_TYPE, Email.ADDRESS to "ada@example.com", Email.TYPE to Email.TYPE_WORK),
            row(StructuredPostal.CONTENT_ITEM_TYPE, StructuredPostal.FORMATTED_ADDRESS to "12 St James's Square, London", StructuredPostal.TYPE to StructuredPostal.TYPE_HOME),
            row(Website.CONTENT_ITEM_TYPE, Website.URL to "https://example.com/ada", Website.TYPE to Website.TYPE_BLOG),
            row(Website.CONTENT_ITEM_TYPE, Website.URL to "https://example.com/work", Website.TYPE to Website.TYPE_CUSTOM, Website.LABEL to "Portfolio"),
            row(Event.CONTENT_ITEM_TYPE, Event.START_DATE to "1815-12-10", Event.TYPE to Event.TYPE_BIRTHDAY),
            row(Organization.CONTENT_ITEM_TYPE, Organization.TITLE to "Analyst", Organization.COMPANY to "Analytical Engines", Organization.DEPARTMENT to " "),
            row(Organization.CONTENT_ITEM_TYPE, Organization.COMPANY to "Royal Society"),
            row(Note.CONTENT_ITEM_TYPE, Note.NOTE to "First note"),
            row(Note.CONTENT_ITEM_TYPE, Note.NOTE to "Second note"),
        )
        // Another account's copy of the same person is aggregated into the contact. The provider drops identical
        // rows itself, so these differ in storage but show the same label and value.
        insert(
            name("Ada $MARKER"),
            row(Phone.CONTENT_ITEM_TYPE, Phone.NUMBER to "+44 20 7946 0000", Phone.TYPE to Phone.TYPE_CUSTOM, Phone.LABEL to "Mobile"),
            row(Email.CONTENT_ITEM_TYPE, Email.ADDRESS to "ada@example.com", Email.TYPE to Email.TYPE_CUSTOM, Email.LABEL to "Work"),
            row(Website.CONTENT_ITEM_TYPE, Website.URL to "https://example.com/ada", Website.TYPE to Website.TYPE_CUSTOM, Website.LABEL to "Blog"),
            row(Organization.CONTENT_ITEM_TYPE, Organization.TITLE to "Analyst", Organization.COMPANY to "Analytical Engines"),
            row(Note.CONTENT_ITEM_TYPE, Note.NOTE to "First note"),
        )
        val id = list(adapter).single().second
        val details = JSONObject(call(adapter, "get", JSONObject().put("id", id)))
        fun values(key: String) = details.getJSONArray(key).let { array ->
            (0 until array.length()).map { array.getJSONObject(it).let { value -> value.getString("label") to value.getString("value") } }
        }
        check(details.getString("id") == id && details.getString("name") == "Ada $MARKER") { details.toString() }
        check(values("phones") == listOf("Mobile" to "+44 20 7946 0000", "Boat" to "+44 20 7946 0001")) { values("phones").toString() }
        check(values("emails") == listOf("Work" to "ada@example.com")) { values("emails").toString() }
        check(values("addresses") == listOf("Home" to "12 St James's Square, London")) { values("addresses").toString() }
        check(values("websites").size == 2 && values("websites").toSet() == setOf("Blog" to "https://example.com/ada", "Portfolio" to "https://example.com/work")) { values("websites").toString() }
        check(values("events") == listOf("Birthday" to "1815-12-10")) { values("events").toString() }
        check(details.getString("organization").lines().size == 2 && details.getString("organization").lines().toSet() == setOf("Analyst, Analytical Engines", "Royal Society")) { details.toString() }
        check(details.getString("note").split("\n\n").size == 2 && details.getString("note").split("\n\n").toSet() == setOf("First note", "Second note")) { details.toString() }

        insert(name("Sparse $MARKER"))
        val sparse = JSONObject(call(adapter, "get", JSONObject().put("id", list(adapter, "sparse").single().second)))
        for (key in listOf("phones", "emails", "addresses", "websites", "events")) check(sparse.getJSONArray(key).length() == 0) { sparse.toString() }
        check(sparse.isNull("organization") && sparse.isNull("note")) { sparse.toString() }
    }

    private fun identity() = withContacts { adapter ->
        val first = insert(name("Grace $MARKER"), row(Phone.CONTENT_ITEM_TYPE, Phone.NUMBER to "+1 555 0100", Phone.TYPE to Phone.TYPE_WORK))
        val second = insert(name("Hopper $MARKER"), row(Phone.CONTENT_ITEM_TYPE, Phone.NUMBER to "+1 555 0101", Phone.TYPE to Phone.TYPE_HOME))
        val ids = list(adapter).map { it.second }
        check(ids.size == 2 && contactOf(first) != contactOf(second))
        fun phones(id: String) = JSONObject(call(adapter, "get", JSONObject().put("id", id))).getJSONArray("phones").length()

        // Joining keeps one row ID, so an ID that only stored _ID would lose the other contact.
        aggregate(AggregationExceptions.TYPE_KEEP_TOGETHER, first, second)
        check(contactOf(first) == contactOf(second)) { "Raw contacts were not joined" }
        for (id in ids) check(phones(id) == 2) { "$id did not resolve to the joined contact" }
        val joined = list(adapter).single().second

        aggregate(AggregationExceptions.TYPE_KEEP_SEPARATE, first, second)
        check(contactOf(first) != contactOf(second)) { "Raw contacts were not split" }
        check(phones(joined) == 1) { "The joined ID did not resolve after the split" }
        for (id in ids) check(phones(id) == 1) { "$id did not resolve after the split" }

        resolver.delete(ContentUris.withAppendedId(RawContacts.CONTENT_URI, first), null, null)
        resolver.delete(ContentUris.withAppendedId(RawContacts.CONTENT_URI, second), null, null)
        for (id in ids + joined) check(call(adapter, "get", JSONObject().put("id", id)) == "null") { "$id resolved after deletion" }
    }

    private fun paging() = withContacts { adapter ->
        // Local contacts' lookup keys encode the name, so long names also make long IDs. The leading code sets the order.
        val names = (0 until 3500).map { index ->
            val code = listOf(index / 676, index / 26 % 26, index % 26).joinToString("") { ('a' + it).toString() }
            "Q$code Maximilian Alexander Featherstonehaugh-Montgomery (Lower Slaughter Parish Council) $MARKER"
        }
        for (chunk in names.chunked(200)) insertNames(chunk)

        val pages = pages(adapter)
        val all = pages.flatten()
        val single = JSONArray(all.map { (name, id) -> JSONObject().put("id", id).put("name", name) }).toString()
        check(pages.size > 1 && !javascriptResultFits(single)) { "${pages.size} pages, ${single.toByteArray().size} bytes" }
        check(all.map { it.second }.distinct().size == all.size) { "A contact was listed twice" }
        check(all.map { it.first }.filter { it.endsWith(MARKER) } == names) { "Paged contacts were missing or out of order" }

        val filtered = pages(adapter, "qb", limit = 300)
        check(filtered.size > 1) { "Filtered ${filtered.size} pages" }
        check(filtered.flatten().map { it.first }.filter { it.endsWith(MARKER) } == names.filter { it.startsWith("Qb") }) { "Filtered pages were missing or out of order" }
    }

    private fun aggregate(type: Int, first: Long, second: Long) {
        resolver.update(AggregationExceptions.CONTENT_URI, ContentValues().apply {
            put(AggregationExceptions.TYPE, type)
            put(AggregationExceptions.RAW_CONTACT_ID1, first)
            put(AggregationExceptions.RAW_CONTACT_ID2, second)
        }, null, null)
    }

    private companion object {
        const val MARKER = "Inkcontract"
    }
}
