package com.vandam.ink

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Base64
import java.security.MessageDigest
import java.security.SecureRandom
import org.json.JSONObject

internal class ExternalAdapter(private val activity: Activity) : NativeAdapter, AutoCloseable {
    private data class Pending(val id: Long, val complete: NativeResultHandler, val redirect: String? = null, val state: String? = null, val verifier: String? = null, var left: Boolean = false)
    private var pending: Pending? = null
    private var lightAppearance = true
    private val handler = Handler(Looper.getMainLooper())
    fun setLightAppearance(light: Boolean) { lightAppearance = light }
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        if (pending != null) { complete(NativeResult.Failure(NativeErrorKind.BUSY, "An external window is already open", true)); return }
        try {
            val input = JSONObject(payload)
            val intent: Intent
            if (operation == "share") {
                intent = Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, input.getString("text")), null)
                pending = Pending(requestId, complete)
            } else {
                var url = input.optString("url")
                if (operation == "browser-auth") {
                    val config = input.getJSONObject("config")
                    val redirect = config.getString("redirectUri")
                    val redirectUri = Uri.parse(redirect)
                    require(!redirectUri.scheme.isNullOrBlank() && redirectUri.fragment == null && redirectUri.query == null) { "Invalid OAuth redirect URI" }
                    val state = random(); val verifier = random()
                    val challenge = Base64.encodeToString(MessageDigest.getInstance("SHA-256").digest(verifier.toByteArray()), Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING)
                    val scopes = config.optJSONArray("scopes")
                    url = Uri.parse(config.getString("authorizationEndpoint")).buildUpon()
                        .appendQueryParameter("response_type", "code").appendQueryParameter("client_id", config.getString("clientId"))
                        .appendQueryParameter("redirect_uri", redirect).appendQueryParameter("state", state)
                        .appendQueryParameter("code_challenge", challenge).appendQueryParameter("code_challenge_method", "S256")
                        .appendQueryParameter("scope", scopes?.let { (0 until it.length()).joinToString(" ") { index -> it.getString(index) } } ?: "").build().toString()
                    pending = Pending(requestId, complete, redirect, state, verifier)
                } else {
                    require(operation == "open-url") { "Unknown external operation" }
                    pending = Pending(requestId, complete)
                }
                require(url.length <= 16_384 && url.none { it.code < 32 }) { "Invalid URL" }
                java.net.URI(url)
                val uri = Uri.parse(url)
                val scheme = uri.scheme?.lowercase() ?: throw IllegalArgumentException("URL requires a scheme")
                if (scheme in listOf("http", "https")) {
                    require(!uri.host.isNullOrBlank() && uri.userInfo == null) { "Invalid web URL" }
                    if (operation == "browser-auth") require(scheme == "https" || (BuildConfig.DEBUG && scheme == "http" && uri.host in setOf("localhost", "127.0.0.1", "10.0.2.2"))) { "OAuth authorisation must use HTTPS" }
                    val browser = activity.packageManager.queryIntentServices(Intent("android.support.customtabs.action.CustomTabsService"), 0)
                        .map { it.serviceInfo.packageName }.firstOrNull { name -> activity.packageManager.resolveActivity(Intent(Intent.ACTION_VIEW, uri).setPackage(name), 0) != null }
                    if (browser == null && operation == "browser-auth") throw UnsupportedOperationException("Custom Tabs unavailable")
                    intent = Intent(Intent.ACTION_VIEW, uri).addCategory(Intent.CATEGORY_BROWSABLE)
                    if (browser != null) {
                        intent.setPackage(browser)
                        intent.putExtras(Bundle().apply { putBinder("android.support.customtabs.extra.SESSION", null) })
                        intent.putExtra("android.support.customtabs.extra.TOOLBAR_COLOR", if (lightAppearance) android.graphics.Color.WHITE else android.graphics.Color.BLACK)
                        intent.putExtra("androidx.browser.customtabs.extra.COLOR_SCHEME", if (lightAppearance) 1 else 2)
                        intent.putExtra("android.support.customtabs.extra.TITLE_VISIBILITY", 1)
                        intent.putExtra("androidx.browser.customtabs.extra.SHARE_STATE", 2)
                        intent.putExtra("android.support.customtabs.extra.SHARE_MENU_ITEM", false)
                        intent.putExtra("org.chromium.chrome.browser.customtabs.EXTRA_DISABLE_STAR_BUTTON", true)
                        intent.putExtra("org.chromium.chrome.browser.customtabs.EXTRA_DISABLE_DOWNLOAD_BUTTON", true)
                    }
                } else {
                    require(operation == "open-url") { "OAuth authorisation must use HTTPS" }
                    require(scheme !in setOf("javascript", "data", "file", "content", "intent", "about", "blob")) { "Unsupported URL scheme" }
                    require(!uri.schemeSpecificPart.isNullOrBlank()) { "URL requires a destination" }
                    val action = when (scheme) {
                        "tel" -> Intent.ACTION_DIAL
                        "mailto", "sms", "smsto", "mms", "mmsto" -> Intent.ACTION_SENDTO
                        else -> Intent.ACTION_VIEW
                    }
                    intent = Intent(action, uri).addCategory(Intent.CATEGORY_BROWSABLE)
                }
            }
            activity.startActivity(intent)
        } catch (error: Exception) {
            pending = null
            complete(NativeResult.Failure(if (error is UnsupportedOperationException || error is android.content.ActivityNotFoundException) NativeErrorKind.UNAVAILABLE else NativeErrorKind.UNEXPECTED,
                if (error is android.content.ActivityNotFoundException) "No app can open this URL" else error.message ?: "Could not open external window", false))
        }
    }
    fun onPause() { pending?.left = true }
    fun onResume() {
        val current = pending ?: return
        if (!current.left) return
        handler.postDelayed({
            if (pending === current) {
                pending = null
                current.complete(if (current.redirect == null) NativeResult.Success("null") else NativeResult.Failure(NativeErrorKind.UNEXPECTED, "Sign-in cancelled", false))
            }
        }, 250)
    }
    fun handleIntent(intent: Intent) {
        val current = pending ?: return
        val expected = current.redirect ?: return
        val uri = intent.data ?: return
        val redirect = Uri.parse(expected)
        if (uri.scheme != redirect.scheme || uri.authority != redirect.authority || uri.path != redirect.path) return
        pending = null
        if (uri.getQueryParameter("state") != current.state) { current.complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "OAuth state did not match", false)); return }
        val code = uri.getQueryParameter("code")
        if (code.isNullOrEmpty()) { current.complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED, "Authorisation was not granted", false)); return }
        current.complete(NativeResult.Success(JSONObject().put("grant_type", "authorization_code").put("code", code)
            .put("redirect_uri", expected).put("code_verifier", current.verifier).toString()))
    }
    override fun cancel(requestId: Long) { if (pending?.id == requestId) pending = null }
    override fun close() { pending = null; handler.removeCallbacksAndMessages(null) }
    private fun random(): String = Base64.encodeToString(ByteArray(32).also { SecureRandom().nextBytes(it) }, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING)
}
