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
    private data class Permission(val name: String, val android: Array<String>, val light: List<String> = listOf(name))
    private data class Pending(
        val id: Long,
        val permission: Permission,
        val complete: NativeResultHandler,
        var cancelled: Boolean = false,
        var routing: Boolean = true,
        var steps: List<String> = emptyList(),
    )
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
                requestLight(request, permission.light)
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
        val minimum = LIGHT_SERVER_VERSIONS[permission.name]
        if (minimum != null) {
            lightSdk.execute(id, "version", "") { result ->
                val supported = atLeast((result as? NativeResult.Success)?.value, minimum)
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
                lightStatus(id, permission.light, complete)
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
        val next = request.steps.drop(1)
        if (next.isEmpty()) {
            status(request.id, request.permission) { result -> activity.runOnUiThread {
                if (pending === request) finish(result)
            } }
            return
        }
        lightSdk.execute(request.id, "permission-status", request.steps.first()) { result -> activity.runOnUiThread {
            if (pending !== request) return@runOnUiThread
            if (result == NativeResult.Success("granted")) {
                requestLight(request, next)
            } else {
                status(request.id, request.permission) { status -> activity.runOnUiThread {
                    if (pending === request) finish(status)
                } }
            }
        } }
    }

    // LightOS opens one permission screen per launch, so combined permissions ask for each in turn.
    private fun requestLight(request: Pending, steps: List<String>) {
        if (steps.isEmpty() || request.cancelled) {
            finish(NativeResult.Success("granted"))
            return
        }
        request.steps = steps
        lightSdk.execute(request.id, "request-permission", steps.first()) { result -> activity.runOnUiThread {
            if (pending !== request) return@runOnUiThread
            if (result is NativeResult.Failure) finish(result)
            else if (result == NativeResult.Success("granted")) requestLight(request, steps.drop(1))
        } }
    }

    private fun lightStatus(id: Long, steps: List<String>, complete: NativeResultHandler) {
        lightSdk.execute(id, "permission-status", steps.first()) { first ->
            val rest = steps.drop(1)
            if (rest.isEmpty() || first !is NativeResult.Success) {
                complete(first)
                return@execute
            }
            lightStatus(id, rest) { others ->
                if (others !is NativeResult.Success) {
                    complete(others)
                    return@lightStatus
                }
                val statuses = listOf(first.value, others.value)
                complete(NativeResult.Success(when {
                    "blocked" in statuses -> "blocked"
                    statuses.all { it == "granted" } -> "granted"
                    else -> "denied"
                }))
            }
        }
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
            "photos-and-videos" -> return Permission(name, arrayOf(Manifest.permission.READ_MEDIA_IMAGES, Manifest.permission.READ_MEDIA_VIDEO), listOf("photos", "videos"))
            "location-approximate" -> arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION)
            "location-precise" -> arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION, Manifest.permission.ACCESS_FINE_LOCATION)
            else -> return null
        }
        return Permission(name, android)
    }

    private fun atLeast(version: String?, minimum: List<Int>): Boolean {
        val parts = version?.takeIf { it.matches(VERSION) }?.substringBefore('+')?.split('.')
            ?.map { it.toIntOrNull() ?: return false } ?: return false
        val difference = parts.zip(minimum).firstOrNull { (part, required) -> part != required }
        return difference == null || difference.first > difference.second
    }

    private companion object {
        const val REQUEST_CODE = 10203
        const val LIGHT_OS_REQUEST_CODE = 10101
        val VERSION = Regex("[0-9]+\\.[0-9]+\\.[0-9]+(?:\\+[0-9A-Za-z.-]+)?")
        // Servers before these versions do not grant the permission.
        val LIGHT_SERVER_VERSIONS = mapOf(
            "notifications" to listOf(0, 2, 4),
            "photos" to listOf(0, 2, 6),
            "videos" to listOf(0, 2, 6),
            "photos-and-videos" to listOf(0, 2, 6),
        )
    }
}
