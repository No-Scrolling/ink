package com.vandam.ink

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import android.os.Parcel
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.View
import org.json.JSONObject
import java.util.concurrent.Executors

internal fun createLightSdkAdapter(
    activity: MainActivity,
    onHapticsChanged: HapticsChangedHandler,
): LightSdkAdapter = InkLightSdkAdapter(activity, onHapticsChanged)

private class InkLightSdkAdapter(
    private val activity: MainActivity,
    private val onHapticsChanged: HapticsChangedHandler,
) : LightSdkAdapter, ServiceConnection {
    private val context = activity.applicationContext
    private val executor = Executors.newSingleThreadExecutor()
    private var binder: IBinder? = null
    private var bound = false
    private var token = DEFAULT_TOKEN

    @Volatile
    private var hapticsEnabled = false

    override fun start() {
        onHapticsChanged(false)
        bind()
    }

    override fun refresh() {
        if (binder != null) {
            executor.execute(::refreshServerState)
        }
    }

    override fun stop() {
        if (bound) {
            runCatching { context.unbindService(this) }
        }
        bound = false
        binder = null
        token = DEFAULT_TOKEN
        executor.shutdownNow()
    }

    override fun performHaptic(view: View) {
        if (hapticsEnabled) {
            view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
        }
    }

    override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
        if (!bound || service == null) {
            return
        }
        binder = service
        token = DEFAULT_TOKEN
        executor.execute(::refreshServerState)
    }

    override fun onServiceDisconnected(name: ComponentName?) {
        binder = null
        token = DEFAULT_TOKEN
    }

    override fun onBindingDied(name: ComponentName?) {
        binder = null
        token = DEFAULT_TOKEN
        if (!bound) {
            return
        }
        runCatching { context.unbindService(this) }
        bound = false
        bind()
    }

    override fun onNullBinding(name: ComponentName?) {
        Log.e(TAG, "Light SDK server returned a null binding")
    }

    private fun bind() {
        if (bound) {
            return
        }
        val intent = Intent(ACTION_BIND_SDK_SERVICE).setPackage(BuildConfig.INK_LIGHT_SERVER_PACKAGE)
        bound = context.bindService(intent, this, Context.BIND_AUTO_CREATE)
        if (!bound) {
            Log.w(TAG, "Could not bind to ${BuildConfig.INK_LIGHT_SERVER_PACKAGE}")
        }
    }

    private fun refreshServerState() {
        if (!ensureToken()) {
            return
        }

        when (val version = authenticatedRequest(GET_VERSION, UNIT_JSON)) {
            is Response.Success -> {
                val serverVersion = JSONObject(version.data).optString("version")
                if (serverVersion != BuildConfig.INK_LIGHT_SDK_VERSION) {
                    Log.w(
                        TAG,
                        "Light SDK server is $serverVersion; Ink supports ${BuildConfig.INK_LIGHT_SDK_VERSION}",
                    )
                } else {
                    Log.i(TAG, "Connected to Light SDK $serverVersion")
                }
            }
            is Response.Error -> Log.w(TAG, "Could not read Light SDK version: ${version.message}")
        }

        when (val preferences = authenticatedRequest(GET_USER_PREFERENCES, UNIT_JSON)) {
            is Response.Success -> updateHaptics(
                JSONObject(preferences.data).optBoolean("hapticsEnabled", false),
            )
            is Response.Error -> Log.w(TAG, "Could not read Light SDK preferences: ${preferences.message}")
        }
    }

    private fun ensureToken(): Boolean {
        if (token != DEFAULT_TOKEN) {
            return true
        }
        return when (val response = request(GET_TOKEN, UNIT_JSON)) {
            is Response.Success -> {
                token = JSONObject(response.data).getString("token")
                true
            }
            is Response.Error -> {
                Log.w(TAG, "Could not authenticate with Light SDK: ${response.message}")
                false
            }
        }
    }

    private fun authenticatedRequest(method: String, payload: String): Response {
        var response = request(method, payload)
        if (response is Response.Error && response.code == INVALID_TOKEN) {
            token = DEFAULT_TOKEN
            if (ensureToken()) {
                response = request(method, payload)
            }
        }
        return response
    }

    private fun request(method: String, payload: String): Response {
        val service = binder ?: return Response.Error(UNKNOWN_ERROR, "not connected")
        val request = Parcel.obtain()
        val reply = Parcel.obtain()
        return try {
            request.writeInterfaceToken(ACTION_BIND_SDK_SERVICE)
            request.writeString(method)
            request.writeString(payload)
            request.writeString(token)
            service.transact(TRANSACTION_REQUEST, request, reply, 0)
            reply.readException()
            val errorCode = reply.readInt()
            if (errorCode == SUCCESS) {
                Response.Success(reply.readString().orEmpty())
            } else {
                Response.Error(errorCode, reply.readString())
            }
        } catch (error: Exception) {
            Log.e(TAG, "Light SDK request failed: $method", error)
            Response.Error(UNKNOWN_ERROR, error.message)
        } finally {
            request.recycle()
            reply.recycle()
        }
    }

    private fun updateHaptics(enabled: Boolean) {
        hapticsEnabled = enabled
        Log.i(TAG, "Haptics enabled: $enabled")
        activity.runOnUiThread { onHapticsChanged(enabled) }
    }

    private sealed interface Response {
        data class Success(val data: String) : Response
        data class Error(val code: Int, val message: String?) : Response
    }

    private companion object {
        const val TAG = "InkLightSdk"
        const val ACTION_BIND_SDK_SERVICE = "com.thelightphone.sdk.action.BIND_SERVICE"
        const val TRANSACTION_REQUEST = 1
        const val SUCCESS = -1
        const val UNKNOWN_ERROR = 0
        const val INVALID_TOKEN = 4
        const val DEFAULT_TOKEN = "no_auth"
        const val UNIT_JSON = "{}"
        const val GET_TOKEN = "GetToken"
        const val GET_VERSION = "GetVersion"
        const val GET_USER_PREFERENCES = "GetUserPreferences"
    }
}
