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
    }

    private fun withActivity(test: (Activity) -> Unit) {
        val context = instrumentation.targetContext
        instrumentation.uiAutomation.grantRuntimePermission(context.packageName, Manifest.permission.POST_NOTIFICATIONS)
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

    private fun call(adapter: PermissionsAdapter, operation: String): NativeResult {
        val results = LinkedBlockingQueue<NativeResult>()
        instrumentation.runOnMainSync {
            adapter.execute(1, operation, """{"permission":"notifications"}""") { results.add(it) }
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

    // The endpoint is controlled; routing and Android permission checks use production code.
    private class Server(private val version: NativeResult, private val deferVersion: Boolean = false) : LightSdkAdapter {
        val calls = mutableListOf<String>()
        val denied = NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, "Host denied notification access", false)
        var versionComplete: NativeResultHandler? = null

        override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
            calls += "$operation:$payload"
            when (operation) {
                "version" -> if (deferVersion) versionComplete = complete else complete(version)
                "permission-status" -> complete(NativeResult.Success("blocked"))
                "request-permission" -> complete(denied)
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
