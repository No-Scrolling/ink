package com.vandam.ink

import android.graphics.BitmapFactory
import java.time.OffsetDateTime
import java.text.DateFormat
import java.util.Date
import java.util.TimeZone
import android.app.Activity
import android.content.ClipData
import android.content.ComponentName
import android.content.Intent
import android.net.Uri
import androidx.core.content.FileProvider
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.zip.ZipFile

internal class InkPassFiles(private val activity: Activity) {
    private val files = InkManagedFiles(activity)
    private val artworkNames = listOf("logo", "strip", "thumbnail", "background", "footer", "icon")
    private val root get() = File(activity.cacheDir, "ink-shares/passes").apply { mkdirs() }
    private fun archive(id: String): File {
        require(id.matches(Regex("[a-f0-9]{64}"))) { "Invalid pass identity" }
        return File(root, "$id/pass.pkpass")
    }
    private fun copy(input: InputStream, output: File, cancelled: () -> Boolean) {
        output.outputStream().use { sink ->
            val buffer = ByteArray(32768)
            var total = 0
            while (true) {
                check(!cancelled()) { "Pass loading cancelled" }
                val count = input.read(buffer)
                if (count < 0) break
                total += count
                require(total <= 10 * 1024 * 1024) { "Pass exceeds 10 MB" }
                sink.write(buffer, 0, count)
            }
        }
    }
    @Synchronized
    fun preview(source: String, cancelled: () -> Boolean): JSONObject {
        check(!cancelled()) { "Pass loading cancelled" }
        val sourceKey = MessageDigest.getInstance("SHA-256").digest(source.toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it) }
        val cached = root.listFiles()?.firstOrNull { it.isDirectory && File(it, "source-$sourceKey").exists() && File(it, "pass.pkpass").isFile }
        if (cached != null) {
            val metadata = File(cached, "preview-v2.json")
            val result = try { JSONObject(metadata.readText()).takeIf { pass ->
                val artwork = pass.optJSONObject("artwork")
                artworkNames.all { name -> artwork?.optJSONObject(name)?.let { files.open("${cached.name}-$name") != null } ?: true }
            } } catch (_: Exception) { null }
                ?: read(File(cached, "pass.pkpass"), cached.name).put("id", cached.name).also { metadata.writeText(it.toString()) }
            check(!cancelled()) { "Pass loading cancelled" }
            cached.setLastModified(System.currentTimeMillis())
            return result
        }
        val temporary = File.createTempFile("pass-", ".pkpass", activity.cacheDir)
        var previewId: String? = null
        try {
            val uri = Uri.parse(source)
            when (uri.scheme) {
                "ink-file" -> InkManagedFiles(activity).resolve(source).inputStream().use { copy(it, temporary, cancelled) }
                "http", "https" -> {
                    var url = URL(source)
                    var loaded = false
                    for (redirect in 0..5) {
                        require(url.protocol in listOf("http", "https") && url.userInfo == null) { "Invalid pass URL" }
                        val connection = url.openConnection() as HttpURLConnection
                        try {
                            connection.connectTimeout = 10_000
                            connection.readTimeout = 10_000
                            connection.instanceFollowRedirects = false
                            val status = connection.responseCode
                            if (status in listOf(301, 302, 303, 307, 308)) {
                                url = URL(url, requireNotNull(connection.getHeaderField("Location")))
                                continue
                            }
                            require(status in 200..299) { "Could not download pass (HTTP $status)" }
                            require(connection.contentLengthLong <= 10 * 1024 * 1024) { "Pass exceeds 10 MB" }
                            connection.inputStream.use { copy(it, temporary, cancelled) }
                            loaded = true
                            break
                        } finally { connection.disconnect() }
                    }
                    check(loaded) { "Too many pass redirects" }
                }
                else -> error("Unsupported pass source")
            }
            val digest = MessageDigest.getInstance("SHA-256")
            temporary.inputStream().use { input ->
                val bytes = ByteArray(32768)
                while (true) { val count = input.read(bytes); if (count < 0) break; digest.update(bytes, 0, count) }
            }
            val id = digest.digest().joinToString("") { "%02x".format(it) }
            previewId = id
            val result = try { read(temporary, id).put("id", id) }
                catch (error: java.util.zip.ZipException) { error("This file is not a valid Wallet pass") }
            check(!cancelled()) { "Pass loading cancelled" }
            val destination = archive(id)
            destination.parentFile!!.mkdirs()
            if (!destination.exists()) check(temporary.renameTo(destination)) { "Could not cache pass" }
            File(destination.parentFile, "preview-v2.json").writeText(result.toString())
            File(destination.parentFile, "source-$sourceKey").writeText("")
            destination.parentFile!!.listFiles()?.filter { it.name.startsWith("source-") }
                ?.sortedByDescending { it.lastModified() }?.drop(8)?.forEach { it.delete() }
            destination.parentFile!!.setLastModified(System.currentTimeMillis())
            root.listFiles()?.filter { it.name != id }?.sortedByDescending { it.lastModified() }
                ?.drop(31)?.forEach { directory ->
                    for (name in artworkNames) files.remove("${directory.name}-$name")
                    directory.deleteRecursively()
                }
            return result
        } finally {
            temporary.delete()
            previewId?.let { id ->
                if (!archive(id).exists()) for (name in artworkNames) files.remove("$id-$name")
            }
        }
    }
    private fun formatValue(field: JSONObject): String {
        val value = field.optString("value")
        if (!field.has("dateStyle") && !field.has("timeStyle")) return value
        fun style(key: String): Int? = when (field.optString(key)) {
            "PKDateStyleShort" -> DateFormat.SHORT
            "PKDateStyleMedium" -> DateFormat.MEDIUM
            "PKDateStyleLong" -> DateFormat.LONG
            "PKDateStyleFull" -> DateFormat.FULL
            else -> null
        }
        val dateStyle = style("dateStyle")
        val timeStyle = style("timeStyle")
        val formatter = when {
            dateStyle != null && timeStyle != null -> DateFormat.getDateTimeInstance(dateStyle, timeStyle)
            dateStyle != null -> DateFormat.getDateInstance(dateStyle)
            timeStyle != null -> DateFormat.getTimeInstance(timeStyle)
            else -> return value
        }
        return try {
            val date = OffsetDateTime.parse(value)
            if (field.optBoolean("ignoresTimeZone")) formatter.timeZone = TimeZone.getTimeZone(date.offset)
            formatter.format(Date.from(date.toInstant()))
        } catch (_: java.time.format.DateTimeParseException) { value }
    }

    private fun read(file: File, id: String): JSONObject = ZipFile(file).use { zip ->
        require(zip.size() <= 256) { "Pass archive has too many entries" }
        val entry = requireNotNull(zip.getEntry("pass.json")) { "This file is not a Wallet pass" }
        val bytes = zip.getInputStream(entry).use { it.readNBytes(256 * 1024 + 1) }
        require(bytes.size <= 256 * 1024) { "Pass data is too large" }
        val pass = JSONObject(bytes.toString(Charsets.UTF_8))
        require(pass.optInt("formatVersion") == 1) { "Unsupported pass version" }
        val styleName = listOf("boardingPass", "coupon", "eventTicket", "generic", "storeCard")
            .firstOrNull { pass.optJSONObject(it) != null } ?: error("Unsupported pass style")
        val style = pass.getJSONObject(styleName)
        val fields = JSONArray()
        for (group in listOf("headerFields", "primaryFields", "secondaryFields", "auxiliaryFields", "backFields")) {
            val values = style.optJSONArray(group) ?: continue
            require(values.length() <= 64) { "Too many pass fields" }
            for (i in 0 until values.length()) {
                val field = values.getJSONObject(i)
                val label = field.optString("label")
                val value = field.optString("value")
                val formatted = formatValue(field)
                fields.put(JSONObject().put("group", group).put("label", label).put("value", value)
                    .put("displayValue", formatted))
            }
        }
        val result = JSONObject().put("serialNumber", pass.getString("serialNumber"))
            .put("passTypeIdentifier", pass.getString("passTypeIdentifier"))
            .put("title", pass.optString("logoText").ifBlank { pass.optString("description", "Pass") })
            .put("organisation", pass.optString("organizationName"))
            .put("description", pass.optString("description")).put("style", styleName).put("fields", fields)
        val barcodes = pass.optJSONArray("barcodes") ?: JSONArray().apply { pass.optJSONObject("barcode")?.let { put(it) } }
        val formats = mapOf("PKBarcodeFormatQR" to "qr", "PKBarcodeFormatPDF417" to "pdf417",
            "PKBarcodeFormatAztec" to "aztec", "PKBarcodeFormatCode128" to "code-128")
        for (i in 0 until barcodes.length()) {
            val barcode = barcodes.getJSONObject(i)
            val format = formats[barcode.optString("format")] ?: continue
            val value = barcode.optString("message")
            val encoding = barcode.optString("messageEncoding").lowercase()
            if (value.isEmpty() || value.toByteArray(Charsets.UTF_8).size > 1024) continue
            if (format == "code-128" && value.any { it.code > 127 }) continue
            // The barcode renderer uses UTF-8; ASCII is identical in the common legacy encodings.
            if (encoding !in listOf("utf-8", "utf8", "us-ascii", "iso-8859-1", "latin1", "windows-1252")) continue
            if (encoding !in listOf("utf-8", "utf8") && value.any { it.code > 127 }) continue
            result.put("barcode", JSONObject().put("format", format).put("value", value).put("altText", barcode.optString("altText")))
            break
        }
        val artwork = JSONObject()
        for (name in artworkNames) {
            val image = listOf("$name@3x.png", "$name@2x.png", "$name.png")
                .firstNotNullOfOrNull { zip.getEntry(it) } ?: continue
            val data = zip.getInputStream(image).use { it.readNBytes(2 * 1024 * 1024 + 1) }
            require(data.size <= 2 * 1024 * 1024) { "Pass artwork is too large" }
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(data, 0, data.size, bounds)
            require(bounds.outMimeType == "image/png" && bounds.outWidth in 1..4096 && bounds.outHeight in 1..4096
                && bounds.outWidth.toLong() * bounds.outHeight <= 8_000_000) { "Invalid pass artwork" }
            val imageId = "$id-$name"
            val target = File(activity.filesDir, "ink-files/$imageId.data")
            target.parentFile!!.mkdirs()
            target.writeBytes(data)
            val stored = files.adopt(target, "image/png", "$name.png", imageId, bounds.outWidth, bounds.outHeight)
            val scale = if (image.name.endsWith("@3x.png")) 3 else if (image.name.endsWith("@2x.png")) 2 else 1
            artwork.put(name, JSONObject().put("src", stored.getString("src"))
                .put("width", (bounds.outWidth / scale).coerceAtLeast(1)).put("height", (bounds.outHeight / scale).coerceAtLeast(1)))
        }
        result.put("artwork", artwork)
    }
    fun retain(pass: JSONObject): JSONObject {
        val id = pass.getString("id")
        archive(id)
        val artwork = pass.optJSONObject("artwork") ?: return pass
        val copied = mutableListOf<String>()
        try {
            for (name in artworkNames) {
                val image = artwork.optJSONObject(name) ?: continue
                val source = image.getString("src")
                require(source == "ink-file://$id-$name" || source == "ink-file://$id-s-$name") { "Invalid pass artwork source" }
                val savedId = "$id-s-$name"
                if (source != "ink-file://$savedId" && files.open(savedId) == null) {
                    val target = File(activity.filesDir, "ink-files/$savedId.data")
                    files.resolve(source).copyTo(target, overwrite = true)
                    files.adopt(target, "image/png", "$name.png", savedId, image.getInt("width"), image.getInt("height"))
                    copied.add(savedId)
                }
                image.put("src", "ink-file://$savedId")
            }
            return pass
        } catch (error: Exception) {
            for (imageId in copied) files.remove(imageId)
            throw error
        }
    }
    fun details(pass: JSONObject): JSONArray {
        val fields = pass.getJSONArray("fields")
        val result = JSONArray()
        for (index in 0 until fields.length()) {
            val field = fields.getJSONObject(index)
            if (field.optString("group") != "backFields") continue
            val value = field.getString("value")
            val text = android.text.SpannableStringBuilder(
                if (Regex("</?(?:a|p|br|div|span|b|i|strong|em)\\b", RegexOption.IGNORE_CASE).containsMatchIn(value))
                    android.text.Html.fromHtml(value, android.text.Html.FROM_HTML_MODE_COMPACT)
                else value)
            if (text.getSpans(0, text.length, android.text.style.URLSpan::class.java).isEmpty())
                android.text.util.Linkify.addLinks(text, android.text.util.Linkify.WEB_URLS or android.text.util.Linkify.EMAIL_ADDRESSES)
            val parts = JSONArray()
            var offset = 0
            for (span in text.getSpans(0, text.length, android.text.style.URLSpan::class.java).sortedBy { text.getSpanStart(it) }) {
                val start = text.getSpanStart(span)
                val end = text.getSpanEnd(span)
                if (start < offset) continue
                if (start > offset) parts.put(JSONObject().put("text", text.subSequence(offset, start).toString()))
                val part = JSONObject().put("text", text.subSequence(start, end).toString())
                if (Uri.parse(span.url).scheme?.lowercase() in listOf("http", "https", "mailto", "tel")) part.put("url", span.url)
                parts.put(part)
                offset = end
            }
            if (offset < text.length) parts.put(JSONObject().put("text", text.subSequence(offset, text.length).toString()))
            result.put(JSONObject().put("label", field.optString("label")).put("parts", parts))
        }
        return result
    }
    private fun intent(packageName: String) = Intent(Intent.ACTION_VIEW)
        .setComponent(ComponentName(packageName, "com.vandam.ink.PassImportActivity"))
        .setType("application/vnd.apple.pkpass")
    fun canOpen(packageName: String): Boolean = activity.packageManager.resolveActivity(intent(packageName), 0) != null
    fun open(id: String, packageName: String) {
        val file = archive(id)
        check(file.isFile) { "Pass preview has expired; reopen it" }
        val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.ink.files", file)
        activity.startActivity(intent(packageName).setDataAndType(uri, "application/vnd.apple.pkpass")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION).apply {
                clipData = ClipData.newUri(activity.contentResolver, "Pass", uri)
            })
    }
}
