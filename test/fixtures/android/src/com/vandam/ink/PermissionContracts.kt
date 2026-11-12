package com.vandam.ink

import android.Manifest
import android.app.Activity
import android.app.Instrumentation
import android.content.Intent
import android.content.pm.PackageManager
import android.view.KeyEvent
import android.view.View
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

class PermissionContractActivity : Activity()

internal class PermissionContracts(private val instrumentation: Instrumentation) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("notification access uses LightOS from server 0.2.4", ::supportedServer)
        test("notification access keeps Android on older or unknown servers", ::olderServer)
        test("cancelled notification version checks cannot open a request", ::cancelledVersionCheck)
        test("photo and video access uses LightOS from server 0.2.6", ::mediaSupportedServer)
        test("photo and video access keeps Android on older or unknown servers", ::mediaOlderServer)
        test("photos and videos open one LightOS screen per permission", ::mediaScreens)
        test("photos and videos combine LightOS statuses", ::mediaStatus)
        test("Light SDK caches the server version and maps media and contact permissions", ::lightSdkMedia)
        test("contact access uses LightOS from server 0.2.7", ::contactsSupportedServer)
        test("contact access keeps Android on older or unknown servers", ::contactsOlderServer)
    }

    private fun withActivity(test: (Activity) -> Unit) {
        val context = instrumentation.targetContext
        for (permission in listOf(Manifest.permission.POST_NOTIFICATIONS, Manifest.permission.READ_MEDIA_IMAGES, Manifest.permission.READ_MEDIA_VIDEO, Manifest.permission.READ_CONTACTS)) {
            instrumentation.uiAutomation.grantRuntimePermission(context.packageName, permission)
        }
        val activity = instrumentation.startActivitySync(
            Intent(context, PermissionContractActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )
        try {
            check(activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED)
            test(activity)
        } finally {
            instrumentation.runOnMainSync { activity.finish() }
        }
    }

    private fun call(adapter: PermissionsAdapter, operation: String, permission: String = "notifications"): NativeResult {
        val results = LinkedBlockingQueue<NativeResult>()
        instrumentation.runOnMainSync {
            adapter.execute(1, operation, """{"permission":"$permission"}""") { results.add(it) }
        }
        return checkNotNull(results.poll(5, TimeUnit.SECONDS)) { "Permission completion did not arrive" }
    }

    private fun supportedServer() = withActivity { activity ->
        for (version in listOf("0.2.4", "0.2.10", "0.3.0", "0.10.0", "1.0.0", "0.2.4+build.1")) {
            val sdk = Server(NativeResult.Success(version))
            val adapter = PermissionsAdapter(activity, sdk)
            check(call(adapter, "status") == NativeResult.Success("blocked")) { "Status bypassed LightOS on $version" }
            check(call(adapter, "request") == sdk.denied) { "Request bypassed LightOS on $version" }
            check(sdk.calls == listOf("version:", "permission-status:notifications", "version:", "request-permission:notifications"))
        }
    }

    private fun olderServer() = withActivity { activity ->
        val versions = listOf("0.1.1", "0.1.2", "0.1.99", "0.2.3", "0.2.4-preview", "", "invalid", "0.2.99999999999999")
            .map { NativeResult.Success(it) } + NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Version unavailable", true)
        for (version in versions) {
            val sdk = Server(version)
            val adapter = PermissionsAdapter(activity, sdk)
            check(call(adapter, "status") == NativeResult.Success("granted")) { "Status did not read Android on $version" }
            check(call(adapter, "request") == NativeResult.Success("granted")) { "Request did not use Android on $version" }
            check(sdk.calls == listOf("version:", "version:")) { "Unsupported server received a permission operation: ${sdk.calls}" }
        }
    }

    private fun cancelledVersionCheck() = withActivity { activity ->
        val sdk = Server(NativeResult.Success("0.2.4"), deferVersion = true)
        val adapter = PermissionsAdapter(activity, sdk)
        var completed = false
        var nextResult: NativeResult? = null
        instrumentation.runOnMainSync {
            adapter.execute(1, "request", """{"permission":"notifications"}""") { completed = true }
            val cancelledComplete = checkNotNull(sdk.versionComplete)
            adapter.cancel(1)
            adapter.execute(2, "request", """{"permission":"notifications"}""") { nextResult = it }
            check(nextResult == null) { "A cancelled version check blocked the next request" }
            cancelledComplete(NativeResult.Success("0.2.4"))
            checkNotNull(sdk.versionComplete)(NativeResult.Success("0.2.4"))
        }
        check(!completed)
        check(nextResult == sdk.denied)
        check(sdk.calls == listOf("version:", "version:", "request-permission:notifications"))
    }

    private fun mediaSupportedServer() = withActivity { activity ->
        for (version in listOf("0.2.6", "0.2.7", "0.2.10", "0.3.0", "1.0.0", "0.2.6+build.1")) {
            for (permission in listOf("photos", "videos")) {
                val sdk = Server(NativeResult.Success(version))
                val adapter = PermissionsAdapter(activity, sdk)
                check(call(adapter, "status", permission) == NativeResult.Success("blocked")) { "Status bypassed LightOS for $permission on $version" }
                check(call(adapter, "request", permission) == sdk.denied) { "Request bypassed LightOS for $permission on $version" }
                check(sdk.calls == listOf("version:", "permission-status:$permission", "version:", "request-permission:$permission")) { sdk.calls.toString() }
            }
            val sdk = Server(NativeResult.Success(version))
            val adapter = PermissionsAdapter(activity, sdk)
            check(call(adapter, "status", "photos-and-videos") == NativeResult.Success("blocked"))
            check(call(adapter, "request", "photos-and-videos") == sdk.denied)
            check(sdk.calls == listOf("version:", "permission-status:photos", "permission-status:videos", "version:", "request-permission:photos")) { sdk.calls.toString() }
        }
    }

    private fun mediaOlderServer() = withActivity { activity ->
        val versions = listOf("0.2.3", "0.2.4", "0.2.5", "0.2.6-preview", "", "invalid")
            .map { NativeResult.Success(it) } + NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Version unavailable", true)
        for (version in versions) {
            for (permission in listOf("photos", "videos", "photos-and-videos")) {
                val sdk = Server(version)
                val adapter = PermissionsAdapter(activity, sdk)
                check(call(adapter, "status", permission) == NativeResult.Success("granted")) { "Status did not read Android for $permission on $version" }
                check(call(adapter, "request", permission) == NativeResult.Success("granted")) { "Request did not use Android for $permission on $version" }
                check(sdk.calls == listOf("version:", "version:")) { "Unsupported server received a permission operation: ${sdk.calls}" }
            }
        }
    }

    private fun contactsSupportedServer() = withActivity { activity ->
        for (version in listOf("0.2.7", "0.2.10", "0.3.0", "1.0.0", "0.2.7+build.1")) {
            val sdk = Server(NativeResult.Success(version))
            val adapter = PermissionsAdapter(activity, sdk)
            check(call(adapter, "status", "contacts") == NativeResult.Success("blocked")) { "Status bypassed LightOS on $version" }
            check(call(adapter, "request", "contacts") == sdk.denied) { "Request bypassed LightOS on $version" }
            check(sdk.calls == listOf("version:", "permission-status:contacts", "version:", "request-permission:contacts")) { sdk.calls.toString() }
        }
    }

    private fun contactsOlderServer() = withActivity { activity ->
        val versions = listOf("0.2.3", "0.2.6", "0.2.6+build.1", "0.2.7-preview", "", "invalid")
            .map { NativeResult.Success(it) } + NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Version unavailable", true)
        for (version in versions) {
            val sdk = Server(version)
            val adapter = PermissionsAdapter(activity, sdk)
            check(call(adapter, "status", "contacts") == NativeResult.Success("granted")) { "Status did not read Android on $version" }
            check(call(adapter, "request", "contacts") == NativeResult.Success("granted")) { "Request did not use Android on $version" }
            check(sdk.calls == listOf("version:", "version:")) { "Unsupported server received a permission operation: ${sdk.calls}" }
        }
    }

    private fun mediaScreens() = withActivity { activity ->
        fun request(sdk: Server, screens: List<String>): NativeResult? {
            val adapter = PermissionsAdapter(activity, sdk)
            var result: NativeResult? = null
            instrumentation.runOnMainSync {
                adapter.execute(1, "request", """{"permission":"photos-and-videos"}""") { result = it }
            }
            for (screen in screens) {
                check(result == null) { "Request finished before the $screen screen closed" }
                check(sdk.screens.lastOrNull() == screen) { "Expected the $screen screen, saw ${sdk.screens}" }
                sdk.close()
                instrumentation.runOnMainSync { adapter.result(10101) }
            }
            return result
        }

        val both = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to "denied", "videos" to "denied"), grant = true)
        check(request(both, listOf("photos", "videos")) == NativeResult.Success("granted"))
        check(both.screens == listOf("photos", "videos"))

        val denied = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to "denied", "videos" to "denied"))
        check(request(denied, listOf("photos")) == NativeResult.Success("denied"))
        check(denied.screens == listOf("photos")) { "Request continued after a denial: ${denied.screens}" }

        val blocked = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to "denied", "videos" to "blocked"))
        check(request(blocked, listOf("photos")) == NativeResult.Success("blocked"))
        check(blocked.screens == listOf("photos"))

        val photos = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to "granted", "videos" to "denied"), grant = true)
        check(request(photos, listOf("videos")) == NativeResult.Success("granted"))
        check(photos.screens == listOf("videos")) { "Granted photos opened a screen: ${photos.screens}" }

        val cancelled = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to "denied", "videos" to "denied"), grant = true)
        val adapter = PermissionsAdapter(activity, cancelled)
        var result: NativeResult? = null
        instrumentation.runOnMainSync { adapter.execute(1, "request", """{"permission":"photos-and-videos"}""") { result = it } }
        instrumentation.runOnMainSync { adapter.cancel(1) }
        cancelled.close()
        instrumentation.runOnMainSync { adapter.result(10101) }
        check(cancelled.screens == listOf("photos")) { "A cancelled request opened another screen: ${cancelled.screens}" }
        check(result == null)

        val granted = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to "granted", "videos" to "granted"))
        check(request(granted, emptyList()) == NativeResult.Success("granted"))
        check(granted.screens.isEmpty())
    }

    private fun mediaStatus() = withActivity { activity ->
        val cases = listOf(
            listOf("granted", "granted", "granted"),
            listOf("granted", "denied", "denied"),
            listOf("denied", "granted", "denied"),
            listOf("denied", "blocked", "blocked"),
            listOf("blocked", "granted", "blocked"),
        )
        for ((photos, videos, expected) in cases) {
            val sdk = Server(NativeResult.Success("0.2.6"), mutableMapOf("photos" to photos, "videos" to videos))
            val result = call(PermissionsAdapter(activity, sdk), "status", "photos-and-videos")
            check(result == NativeResult.Success(expected)) { "photos=$photos videos=$videos returned $result" }
        }
    }

    private fun lightSdkMedia() = withActivity { activity ->
        val sdk = createLightSdkAdapter(activity) { _, _ -> }
        fun call(operation: String, payload: String = ""): NativeResult {
            val results = LinkedBlockingQueue<NativeResult>()
            instrumentation.runOnMainSync { sdk.execute(1, operation, payload) { results.add(it) } }
            return checkNotNull(results.poll(5, TimeUnit.SECONDS)) { "Light SDK completion did not arrive" }
        }
        try {
            VibrationLightService.requests.clear()
            check(call("version") == NativeResult.Success("0.2.3"))
            check(call("version") == NativeResult.Success("0.2.3"))
            check(VibrationLightService.requests.count { it.startsWith("GetVersion:") } == 1) { "Version was not cached: ${VibrationLightService.requests}" }
            check(call("permission-status", "photos") == NativeResult.Success("granted"))
            check(call("permission-status", "videos") == NativeResult.Success("granted"))
            check(call("permission-status", "contacts") == NativeResult.Success("granted"))
            val permissions = VibrationLightService.requests.filter { it.startsWith("GetPermission:") }
            check(permissions == listOf(
                """GetPermission:{"permissionName":"android.permission.READ_MEDIA_IMAGES"}""",
                """GetPermission:{"permissionName":"android.permission.READ_MEDIA_VIDEO"}""",
                """GetPermission:{"permissionName":"android.permission.READ_CONTACTS"}""",
            )) { permissions.toString() }
            check(call("permission-status", "photos-and-videos") is NativeResult.Failure)
        } finally {
            instrumentation.runOnMainSync { sdk.stop() }
        }
    }

    // The endpoint is controlled; routing and Android permission checks use production code.
    private class Server(
        private val version: NativeResult,
        private val statuses: MutableMap<String, String>? = null,
        private val grant: Boolean = false,
        private val deferVersion: Boolean = false,
    ) : LightSdkAdapter {
        override val hapticsEnabled = true
        val calls = mutableListOf<String>()
        val denied = NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, "Host denied notification access", false)
        var versionComplete: NativeResultHandler? = null
        val screens = mutableListOf<String>()

        // Mirrors the LightOS screen closing, with the user accepting only when grant is set.
        fun close() {
            if (grant) statuses!![screens.last()] = "granted"
        }

        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
            calls += "$operation:$payload"
            when (operation) {
                "version" -> if (deferVersion) versionComplete = complete else complete(version)
                "permission-status" -> complete(NativeResult.Success(statuses?.getValue(payload) ?: "blocked"))
                "request-permission" -> when {
                    statuses == null -> complete(denied)
                    statuses[payload] == "granted" -> complete(NativeResult.Success("granted"))
                    else -> {
                        screens += payload
                        complete(NativeResult.Success(""))
                    }
                }
                else -> error("Unexpected operation: $operation")
            }
        }

        override fun start() = Unit
        override fun refresh() = Unit
        override fun stop() = Unit
        override fun cancel(requestId: Long) = Unit
        override fun performHaptic(view: View) = Unit
        override fun forwardDeviceKey(event: KeyEvent) = false
        override fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler) = error("Unexpected controller")
    }
}
