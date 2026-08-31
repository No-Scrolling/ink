package com.vandam.ink

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.ServiceConnection
import android.media.RingtoneManager
import android.os.IBinder
import android.os.Parcel
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.KeyEvent
import android.view.View
import org.json.JSONObject
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.Future
import java.util.concurrent.FutureTask

internal fun createLightSdkAdapter(
    activity: MainActivity,
    onKeyboardPreferencesChanged: KeyboardPreferencesChangedHandler,
    updateController: (Long, String) -> Unit,
): LightSdkAdapter = InkLightSdkAdapter(
    activity,
    onKeyboardPreferencesChanged,
    updateController,
)

private class InkLightSdkAdapter(
    private val activity: MainActivity,
    private val onKeyboardPreferencesChanged: KeyboardPreferencesChangedHandler,
    private val updateController: (Long, String) -> Unit,
) : LightSdkAdapter, ServiceConnection {
    private val context = activity.applicationContext
    private val executor = Executors.newSingleThreadExecutor()
    private var binder: IBinder? = null
    private var bound = false
    private var token = DEFAULT_TOKEN
    private val pendingRequests = mutableListOf<PendingRequest>()
    private val runningRequests = ConcurrentHashMap<Long, Future<*>>()
    private val ringtoneFiles = createRingtoneFiles(context)

    @Volatile
    private var hapticsEnabled = false
    @Volatile
    private var emojis: String? = null
    @Volatile
    private var keyAnimationEnabled = true

    override fun start() {
        updateKeyboardPreferences()
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
        pendingRequests.clear()
        runningRequests.values.forEach { it.cancel(true) }
        runningRequests.clear()
        executor.shutdownNow()
    }

    override fun performHaptic(view: View) {
        if (hapticsEnabled) {
            view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
        }
    }

    override fun forwardDeviceKey(event: KeyEvent): Boolean {
        if (event.keyCode !in DEVICE_KEY_CODES) return false
        if (binder != null) {
            val payload = JSONObject()
                .put("keyCode", event.keyCode)
                .put("repeatCount", event.repeatCount)
                .put("action", event.action)
                .put("unicodeChar", event.unicodeChar)
                .put("componentToRelaunch", activity.componentName.flattenToString())
                .toString()
            executor.execute {
                if (authenticatedRequest(DEVICE_KEY_EVENT, payload) is Response.Error) {
                    Log.w(TAG, "Could not forward LightOS device key ${event.keyCode}")
                }
            }
        }
        return true
    }

    override fun executeController(
        requestId: Long,
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation == "activate") {
            updateRingtone(controller, "idle")
            complete(NativeResult.Success(""))
            return
        }
        if (operation == "deactivate") {
            complete(NativeResult.Success(""))
            return
        }
        if (operation != "set") {
            updateRingtone(
                controller,
                "error",
                "protocol",
                "Unknown ringtone operation: $operation",
            )
            complete(NativeResult.Success(""))
            return
        }
        val request = runCatching { JSONObject(payload) }.getOrNull()
        val source = request?.optString("source").orEmpty()
        val kind = request?.optString("kind", "ringtone").orEmpty()
        if (source.isEmpty() || kind !in RINGTONE_TYPES) {
            updateRingtone(controller, "error", "protocol", "Invalid ringtone request")
            complete(NativeResult.Success(""))
            return
        }
        updateRingtone(controller, "installing")
        val task = FutureTask<Unit> {
            val staged = runCatching { ringtoneFiles.stage(source) }.getOrElse { error ->
                runningRequests.remove(requestId)
                updateRingtone(
                    controller,
                    "error",
                    "source",
                    error.message ?: "Could not stage ringtone",
                )
                complete(NativeResult.Success(""))
                return@FutureTask
            }
            val requestPayload = JSONObject()
                .put("type", RINGTONE_TYPES.getValue(kind))
                .put("uri", staged.uri.toString())
                .toString()
            when (val response = authenticatedRequest(SET_RINGTONE, requestPayload)) {
                is Response.Success -> {
                    ringtoneFiles.commit(kind, staged)
                    if (!Thread.currentThread().isInterrupted) {
                        updateRingtone(controller, "installed")
                    }
                }
                is Response.Error -> {
                    ringtoneFiles.discard(staged)
                    if (!Thread.currentThread().isInterrupted) {
                        updateRingtone(
                            controller,
                            "error",
                            if (response.code == INVALID_PARAMETERS) "protocol" else "unavailable",
                            response.message ?: "Could not install ringtone",
                            response.code != INVALID_PARAMETERS,
                        )
                    }
                }
            }
            runningRequests.remove(requestId)
            complete(NativeResult.Success(""))
        }
        runningRequests[requestId] = task
        executor.execute(task)
    }

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        val valid = when (operation) {
            VERSION_OPERATION -> payload.isEmpty()
            PERMISSION_STATUS_OPERATION, REQUEST_PERMISSION_OPERATION ->
                payload == CAMERA ||
                    payload == MICROPHONE ||
                    payload == LOCATION_APPROXIMATE ||
                    payload == LOCATION_PRECISE
            OPEN_DIALLER_OPERATION -> runCatching {
                val phoneNumber = JSONObject(payload).getString("phoneNumber")
                phoneNumber.trim().isNotEmpty() && phoneNumber.length <= 64 &&
                    phoneNumber.none { Character.isISOControl(it.code) }
            }.getOrDefault(false)
            else -> false
        }
        if (!valid) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.PROTOCOL,
                    "Invalid Light SDK operation: $operation",
                    false,
                ),
            )
            return
        }
        val request = PendingRequest(requestId, operation, payload, complete)
        if (binder == null) {
            pendingRequests += request
            bind()
            return
        }
        execute(request)
    }

    override fun cancel(requestId: Long) {
        pendingRequests.removeAll { it.id == requestId }
        runningRequests.remove(requestId)?.cancel(true)
    }

    override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
        if (!bound || service == null) {
            return
        }
        binder = service
        token = DEFAULT_TOKEN
        executor.execute(::refreshServerState)
        val requests = pendingRequests.toList()
        pendingRequests.clear()
        requests.forEach(::execute)
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
        failPending("Light SDK server returned a null binding")
    }

    private fun bind() {
        if (bound) {
            return
        }
        val intent = Intent(ACTION_BIND_SDK_SERVICE).setPackage(BuildConfig.INK_LIGHT_SERVER_PACKAGE)
        bound = context.bindService(intent, this, Context.BIND_AUTO_CREATE)
        if (!bound) {
            Log.w(TAG, "Could not bind to ${BuildConfig.INK_LIGHT_SERVER_PACKAGE}")
            failPending("Could not connect to Light SDK")
        }
    }

    private fun execute(request: PendingRequest) {
        val task = FutureTask<Unit> {
            val result = runCatching { executeOperation(request) }.getOrElse { error ->
                NativeResult.Failure(
                    NativeErrorKind.UNEXPECTED,
                    error.message ?: "Light SDK operation failed",
                    false,
                )
            }
            runningRequests.remove(request.id)
            request.complete(result)
            Unit
        }
        runningRequests[request.id] = task
        executor.execute(task)
    }

    private fun executeOperation(request: PendingRequest): NativeResult = when (request.operation) {
        VERSION_OPERATION -> when (val response = authenticatedRequest(GET_VERSION, UNIT_JSON)) {
            is Response.Success -> NativeResult.Success(
                JSONObject(response.data).getString("version"),
            )
            is Response.Error -> response.failure()
        }
        PERMISSION_STATUS_OPERATION -> permissionStatus(request.payload)
        REQUEST_PERMISSION_OPERATION -> requestPermission(request.payload)
        OPEN_DIALLER_OPERATION -> openDialler(request.payload)
        else -> NativeResult.Failure(
            NativeErrorKind.PROTOCOL,
            "Unknown Light SDK operation: ${request.operation}",
            false,
        )
    }

    private fun openDialler(payload: String): NativeResult {
        val phoneNumber = JSONObject(payload).getString("phoneNumber").trim()
        return when (
            val response = authenticatedRequest(
                OPEN_DIALLER,
                JSONObject().put("phoneNumber", phoneNumber).toString(),
            )
        ) {
            is Response.Success -> NativeResult.Success("")
            is Response.Error -> response.failure()
        }
    }

    private fun permissionStatus(permission: String): NativeResult {
        if (
            permission == LOCATION_APPROXIMATE &&
            activity.checkSelfPermission(android.Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return NativeResult.Success("granted")
        }
        val payload = JSONObject()
            .put(PERMISSION_NAME_KEY, androidPermission(permission))
            .toString()
        return when (val response = authenticatedRequest(GET_PERMISSION, payload)) {
            is Response.Success -> {
                val status = when (
                    JSONObject(response.data).getString(PERMISSION_RESULT_KEY)
                ) {
                    "Granted" -> "granted"
                    "Denied" -> "denied"
                    "BlockedByServer" -> "blocked"
                    else -> "unknown"
                }
                NativeResult.Success(status)
            }
            is Response.Error -> response.failure()
        }
    }

    private fun requestPermission(permission: String): NativeResult {
        return when (val response = authenticatedRequest(REQUEST_PERMISSION_COMPONENT, UNIT_JSON)) {
            is Response.Error -> response.failure()
            is Response.Success -> launchPermission(response.data, permission)
        }
    }

    private fun launchPermission(response: String, permission: String): NativeResult {
        val component = ComponentName.unflattenFromString(
            JSONObject(response).getString(COMPONENT_NAME_KEY),
        ) ?: return NativeResult.Failure(
            NativeErrorKind.PROTOCOL,
            "Light SDK returned an invalid permission component",
            false,
        )
        val launch = FutureTask {
            @Suppress("DEPRECATION")
            activity.startActivityForResult(
                Intent()
                    .setComponent(component)
                    .putExtra(PERMISSION_EXTRA, androidPermission(permission)),
                PERMISSION_REQUEST_CODE,
            )
            NativeResult.Success("")
        }
        activity.runOnUiThread(launch)
        return launch.get()
    }

    private fun androidPermission(permission: String): String = when (permission) {
        CAMERA -> android.Manifest.permission.CAMERA
        MICROPHONE -> android.Manifest.permission.RECORD_AUDIO
        LOCATION_APPROXIMATE -> android.Manifest.permission.ACCESS_COARSE_LOCATION
        LOCATION_PRECISE -> android.Manifest.permission.ACCESS_FINE_LOCATION
        else -> error("Unsupported permission: $permission")
    }

    private fun Response.Error.failure(): NativeResult.Failure {
        val kind = when (code) {
            INVALID_PARAMETERS -> NativeErrorKind.PROTOCOL
            NO_PERMISSION -> NativeErrorKind.PERMISSION_DENIED
            else -> NativeErrorKind.UNAVAILABLE
        }
        return NativeResult.Failure(
            kind,
            message ?: "Light SDK request failed",
            kind == NativeErrorKind.UNAVAILABLE,
        )
    }

    private fun failPending(message: String) {
        val requests = pendingRequests.toList()
        pendingRequests.clear()
        requests.forEach { request ->
            request.complete(
                NativeResult.Failure(NativeErrorKind.UNAVAILABLE, message, true),
            )
        }
    }

    private fun refreshServerState() {
        if (authenticate() != null) {
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
            is Response.Success -> {
                hapticsEnabled = JSONObject(preferences.data).optBoolean("hapticsEnabled", false)
            }
            is Response.Error -> Log.w(TAG, "Could not read Light SDK preferences: ${preferences.message}")
        }

        when (val options = authenticatedRequest(GET_KEYBOARD_OPTIONS, UNIT_JSON)) {
            is Response.Success -> {
                val value = JSONObject(options.data)
                emojis = value.optString("emojisAsString").takeIf(String::isNotEmpty)
                keyAnimationEnabled = value.optBoolean("enableKeyAnimation", true)
            }
            is Response.Error -> Log.w(TAG, "Could not read Light SDK keyboard options: ${options.message}")
        }
        updateKeyboardPreferences()
    }

    private fun authenticate(): Response.Error? {
        if (token != DEFAULT_TOKEN) {
            return null
        }
        return when (val response = request(GET_TOKEN, UNIT_JSON)) {
            is Response.Success -> {
                token = JSONObject(response.data).getString("token")
                null
            }
            is Response.Error -> {
                Log.w(TAG, "Could not authenticate with Light SDK: ${response.message}")
                response
            }
        }
    }

    private fun authenticatedRequest(method: String, payload: String): Response {
        authenticate()?.let { return it }
        var response = request(method, payload)
        if (response is Response.Error && response.code == INVALID_TOKEN) {
            token = DEFAULT_TOKEN
            response = authenticate() ?: request(method, payload)
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

    private fun updateKeyboardPreferences() {
        val preferences = KeyboardPreferences(hapticsEnabled, emojis, keyAnimationEnabled)
        activity.runOnUiThread { onKeyboardPreferencesChanged(preferences) }
    }

    private fun updateRingtone(
        controller: Long,
        status: String,
        errorKind: String = "",
        errorMessage: String = "",
        retryable: Boolean = false,
    ) {
        val value = JSONObject()
            .put("status", status)
            .put("errorKind", errorKind)
            .put("errorMessage", errorMessage)
            .put("errorRetryable", retryable)
            .toString()
        activity.runOnUiThread { updateController(controller, value) }
    }

    private sealed interface Response {
        data class Success(val data: String) : Response
        data class Error(val code: Int, val message: String?) : Response
    }

    private data class PendingRequest(
        val id: Long,
        val operation: String,
        val payload: String,
        val complete: NativeResultHandler,
    )

    private companion object {
        const val TAG = "InkLightSdk"
        const val ACTION_BIND_SDK_SERVICE = "com.thelightphone.sdk.action.BIND_SERVICE"
        const val TRANSACTION_REQUEST = 1
        const val SUCCESS = -1
        const val UNKNOWN_ERROR = 0
        const val INVALID_PARAMETERS = 2
        const val NO_PERMISSION = 3
        const val INVALID_TOKEN = 4
        const val DEFAULT_TOKEN = "no_auth"
        const val UNIT_JSON = "{}"
        const val GET_TOKEN = "GetToken"
        const val GET_VERSION = "GetVersion"
        const val GET_USER_PREFERENCES = "GetUserPreferences"
        const val GET_KEYBOARD_OPTIONS = "GetKeyboardOptions"
        const val DEVICE_KEY_EVENT = "DeviceKeyEvent"
        const val OPEN_DIALLER = "OpenDialer"
        const val SET_RINGTONE = "SetRingtone"
        const val GET_PERMISSION = "GetPermission"
        const val REQUEST_PERMISSION_COMPONENT = "RequestPermissionComponent"
        const val PERMISSION_NAME_KEY = "permissionName"
        const val PERMISSION_RESULT_KEY = "permissionResult"
        const val COMPONENT_NAME_KEY = "componentName"
        const val PERMISSION_EXTRA = "PermissionName"
        const val PERMISSION_REQUEST_CODE = 10101
        const val VERSION_OPERATION = "version"
        const val PERMISSION_STATUS_OPERATION = "permission-status"
        const val REQUEST_PERMISSION_OPERATION = "request-permission"
        const val OPEN_DIALLER_OPERATION = "open-dialler"
        const val CAMERA = "camera"
        const val MICROPHONE = "microphone"
        const val LOCATION_APPROXIMATE = "location-approximate"
        const val LOCATION_PRECISE = "location-precise"
        val DEVICE_KEY_CODES = setOf(24, 25, 27, 80, 317, 318, 319)
        val RINGTONE_TYPES = mapOf(
            "ringtone" to RingtoneManager.TYPE_RINGTONE,
            "notification" to RingtoneManager.TYPE_NOTIFICATION,
            "alarm" to RingtoneManager.TYPE_ALARM,
        )
    }
}
