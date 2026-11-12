package com.vandam.ink

import android.app.Instrumentation
import android.app.Service
import android.content.Intent
import android.os.Binder
import android.os.IBinder
import android.os.Parcel
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.os.VibratorManager
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

// A controlled protocol peer; preference loading, gating and vibration use production adapters.
class VibrationLightService : Service() {
    override fun onBind(intent: Intent): IBinder = object : Binder() {
        override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
            if (code != 1) return super.onTransact(code, data, reply, flags)
            data.enforceInterface("com.thelightphone.sdk.action.BIND_SERVICE")
            val method = data.readString()
            val payload = data.readString()
            data.readString()
            requests += "$method:$payload"
            val response = when (method) {
                "GetToken" -> """{"token":"vibration-contract"}"""
                "GetVersion" -> """{"version":"0.2.3"}"""
                "GetUserPreferences" -> """{"hapticsEnabled":$enabled}"""
                "GetPermission" -> """{"permissionResult":"Granted"}"""
                else -> error("Unexpected Light SDK request: $method")
            }
            reply!!.writeNoException()
            reply.writeInt(-1)
            reply.writeString(response)
            return true
        }
    }

    companion object {
        @Volatile var enabled = false
        val requests = CopyOnWriteArrayList<String>()
    }
}

internal class VibrationContracts(private val instrumentation: Instrumentation) {
    fun run(test: (String, () -> Unit) -> Unit) {
        test("vibration uses Android effects and honours the LightOS haptics preference", ::vibration)
    }

    private fun vibration() {
        val context = instrumentation.targetContext
        check(context.getSystemService(VibratorManager::class.java).defaultVibrator.hasVibrator())
        val activity = instrumentation.startActivitySync(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        val sdk = createLightSdkAdapter(activity) { _, _ -> }
        val adapter = createVibrationAdapter(activity) { sdk.hapticsEnabled }
        fun call(operation: String, payload: String = "{}") {
            val results = LinkedBlockingQueue<NativeResult>()
            instrumentation.runOnMainSync { adapter.execute(1, operation, payload) { results.add(it) } }
            check(results.poll(5, TimeUnit.SECONDS) == NativeResult.Success(""))
        }
        fun records(): List<String> {
            val descriptor = instrumentation.uiAutomation.executeShellCommand("dumpsys vibrator_manager")
            return ParcelFileDescriptor.AutoCloseInputStream(descriptor).bufferedReader().use { it.readLines() }
                .filter { it.contains("opPkg=${context.packageName},") && it.contains("uid=${context.applicationInfo.uid},") }
        }
        fun awaitPreference(enabled: Boolean) {
            val deadline = SystemClock.uptimeMillis() + 5000
            while (sdk.hapticsEnabled != enabled && SystemClock.uptimeMillis() < deadline) SystemClock.sleep(10)
            check(sdk.hapticsEnabled == enabled) { "Light SDK did not load hapticsEnabled=$enabled" }
        }
        fun finishedRecord(usage: String = "TOUCH"): String {
            call("stop")
            SystemClock.sleep(100)
            return records().last { it.contains("Usage=$usage ") }
        }
        try {
            VibrationLightService.enabled = true
            instrumentation.runOnMainSync { sdk.start() }
            awaitPreference(true)
            call("vibrate", """{"segments":[{"ms":313,"strength":1}],"alarm":false}""")
            val touch = finishedRecord()
            check(touch.contains("Usage=TOUCH") && touch.contains("duration=313") && touch.contains("amplitude=1.0")) { touch }

            call("vibrate", """{"segments":[{"ms":3000}]}""")
            val default = finishedRecord()
            check(default.contains("Usage=TOUCH") && default.contains("duration=3000")) { default }
            check(default.contains("cancelled")) { "stopVibrating did not cancel the active vibration: $default" }

            VibrationLightService.enabled = false
            instrumentation.runOnMainSync { sdk.refresh() }
            awaitPreference(false)
            val before = records()
            call("vibrate", """{"segments":[{"ms":733,"strength":1}]}""")
            SystemClock.sleep(100)
            check(records() == before) { "Touch vibration reached Android with haptics disabled" }

            call("vibrate", """{"segments":[{"ms":317,"strength":1},{"ms":151,"strength":0},{"ms":419,"strength":0.4}],"alarm":true}""")
            val alarm = finishedRecord("ALARM")
            check(alarm.contains("Usage=ALARM")) { alarm }
            for (segment in listOf("duration=317", "duration=151", "duration=419", "amplitude=0.4")) {
                check(alarm.contains(segment)) { "Missing $segment: $alarm" }
            }
        } finally {
            instrumentation.runOnMainSync { adapter.stop(); sdk.stop(); activity.finish() }
            VibrationLightService.enabled = false
        }
    }
}
