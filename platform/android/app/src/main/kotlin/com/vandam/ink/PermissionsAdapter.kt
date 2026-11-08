package com.vandam.ink

import android.app.Activity
import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import org.json.JSONObject

internal class PermissionsAdapter(
    private val activity: Activity,
    private val lightSdk: LightSdkAdapter,
) : NativeAdapter {
    private data class Permission(val name: String, val android: Array<String>)
    private data class Pending(val id: Long, val permission: Permission, val complete: NativeResultHandler, var cancelled: Boolean = false, var routing: Boolean = true)
    private var pending: Pending? = null
    private val history = activity.getSharedPreferences("ink-permissions", Context.MODE_PRIVATE)
    private val usesLightOs = try {
        activity.packageManager.getPackageInfo(BuildConfig.INK_LIGHT_SERVER_PACKAGE, 0)
        true
    } catch (_: PackageManager.NameNotFoundException) {
        false
    }

    override fun execute(id: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val requestPayload = runCatching { JSONObject(payload) }.getOrNull()
        val permission = requestPayload?.optString("permission")?.let(::permission)
        if (permission == null || operation !in setOf("status", "request")) {
            complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid permission request", false))
            return
        }
        if (operation == "status") {
            status(id, permission, complete)
            return
        }
        if (pending != null) {
            complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "A permission request is already open", true))
            return
        }
        val request = Pending(id, permission, complete)
        pending = request
        usesLightPermission(id, permission) { useLightPermission ->
            if (pending !== request) return@usesLightPermission
            request.routing = false
            if (useLightPermission) {
                lightSdk.execute(id, "request-permission", permission.name) { result ->
                    if (result is NativeResult.Failure || result is NativeResult.Success && result.value == "granted") activity.runOnUiThread {
                        if (pending === request) finish(result)
                    }
                }
            } else {
                val permissions = permission.android
                if (permissions.all { activity.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }) {
                    finish(NativeResult.Success("granted"))
                    return@usesLightPermission
                }
                history.edit().apply { permissions.forEach { putBoolean(it, true) } }.apply()
                try {
                    activity.requestPermissions(permissions, REQUEST_CODE)
                } catch (error: Exception) {
                    finish(NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Could not request permission", false))
                }
            }
        }
    }

    private fun usesLightPermission(id: Long, permission: Permission, complete: (Boolean) -> Unit) {
        if (!BuildConfig.INK_LIGHT_SDK_ENABLED || !usesLightOs) {
            complete(false)
            return
        }
        if (permission.name == "notifications") {
            // POST_NOTIFICATIONS joined the server's default grants after 0.2.3.
            lightSdk.execute(id, "version", "") { result ->
                val version = (result as? NativeResult.Success)?.value
                val parts = version?.takeIf { it.matches(Regex("[0-9]+\\.[0-9]+\\.[0-9]+(?:\\+[0-9A-Za-z.-]+)?")) }
                    ?.substringBefore('+')?.split('.')?.mapNotNull { it.toIntOrNull() }
                val supported = parts != null && parts.size == 3 &&
                    (parts[0] > 0 || parts[1] > 2 || parts[1] == 2 && parts[2] >= 4)
                activity.runOnUiThread { complete(supported) }
            }
            return
        }
        // Older emulator SDK versions cannot grant location permissions.
        complete(permission.name in setOf("camera", "microphone", "audio-files", "location-approximate", "location-precise") &&
            (BuildConfig.INK_LIGHT_SERVER_PACKAGE != "com.thelightphone.sdk.emulator" ||
                Manifest.permission.ACCESS_COARSE_LOCATION !in permission.android))
    }

    private fun status(id: Long, permission: Permission, complete: NativeResultHandler) {
        usesLightPermission(id, permission) { useLightPermission ->
            if (useLightPermission) {
                lightSdk.execute(id, "permission-status", permission.name, complete)
                return@usesLightPermission
            }
            val permissions = permission.android
            val denied = permissions.filter { activity.checkSelfPermission(it) != PackageManager.PERMISSION_GRANTED }
            val status = when {
                denied.isEmpty() -> "granted"
                denied.any {
                    (history.getBoolean(it, false) || history.getBoolean(permission.name, false)) &&
                        !activity.shouldShowRequestPermissionRationale(it)
                } -> "blocked"
                else -> "denied"
            }
            complete(NativeResult.Success(status))
        }
    }

    fun result(requestCode: Int) {
        if (requestCode != REQUEST_CODE && requestCode != LIGHT_OS_REQUEST_CODE) return
        val request = pending ?: return
        status(request.id, request.permission) { result -> activity.runOnUiThread {
            if (pending === request) finish(result)
        } }
    }

    private fun finish(result: NativeResult) {
        val request = pending ?: return
        pending = null
        if (!request.cancelled) request.complete(result)
    }

    override fun cancel(requestId: Long) {
        pending?.takeIf { it.id == requestId }?.let { request ->
            request.cancelled = true
            if (request.routing) pending = null
        }
        if (usesLightOs) lightSdk.cancel(requestId)
    }

    fun stop() {
        pending?.let { cancel(it.id) }
        pending = null
    }

    private fun permission(name: String): Permission? {
        val android = when (name) {
            "camera" -> arrayOf(Manifest.permission.CAMERA)
            "microphone" -> arrayOf(Manifest.permission.RECORD_AUDIO)
            "notifications" -> arrayOf(Manifest.permission.POST_NOTIFICATIONS)
            "audio-files" -> arrayOf(Manifest.permission.READ_MEDIA_AUDIO)
            "photos" -> arrayOf(Manifest.permission.READ_MEDIA_IMAGES)
            "videos" -> arrayOf(Manifest.permission.READ_MEDIA_VIDEO)
            "photos-and-videos" -> arrayOf(Manifest.permission.READ_MEDIA_IMAGES, Manifest.permission.READ_MEDIA_VIDEO)
            "location-approximate" -> arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION)
            "location-precise" -> arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION, Manifest.permission.ACCESS_FINE_LOCATION)
            else -> return null
        }
        return Permission(name, android)
    }

    private companion object {
        const val REQUEST_CODE = 10203
        const val LIGHT_OS_REQUEST_CODE = 10101
    }
}
