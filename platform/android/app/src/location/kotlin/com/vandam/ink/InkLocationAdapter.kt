package com.vandam.ink

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.CancellationSignal
import android.os.SystemClock
import org.json.JSONObject

internal fun createLocationAdapter(activity: MainActivity): LocationAdapter =
    InkLocationAdapter(activity)

internal fun createWorkerLocationAdapter(context: Context): LocationAdapter =
    InkLocationAdapter(context, allowUpdates = false)

private class InkLocationAdapter(private val context: Context, private val allowUpdates: Boolean = true) : LocationAdapter {
    private val manager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
    private val requests = mutableMapOf<Long, PendingLocation>()
    private val watches = mutableMapOf<Long, Watch>()
    private var nextWatch = 1L
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        if (operation.startsWith("watch-") || operation.startsWith("tracking-")) {
            if (!allowUpdates) {
                complete(protocol("Workers support one-off location only"))
                return
            }
            executeUpdates(requestId, operation, payload, complete)
            return
        }
        if (operation != CURRENT_OPERATION) {
            complete(protocol("Unknown location operation: $operation"))
            return
        }
        val options = runCatching { JSONObject(payload) }.getOrElse {
            complete(protocol("Ink produced invalid location options"))
            return
        }
        val accuracy = options.optString(ACCURACY_KEY)
        if (accuracy != APPROXIMATE && accuracy != PRECISE) {
            complete(protocol("Unknown location accuracy: $accuracy"))
            return
        }
        val maxAgeMs = options.optLong(MAX_AGE_KEY, -1)
        if (maxAgeMs !in 0..MAX_CACHE_AGE_MS) {
            complete(protocol("Invalid location cache age"))
            return
        }
        if (!hasPermission(accuracy)) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.PERMISSION_DENIED,
                    "Location permission has not been granted",
                    false,
                ),
            )
            return
        }
        if (!manager.isLocationEnabled) {
            complete(
                NativeResult.Failure(
                    NativeErrorKind.LOCATION_DISABLED,
                    "Location is switched off",
                    false,
                ),
            )
            return
        }

        val cached = cachedLocation(accuracy, maxAgeMs)
        if (cached != null) {
            complete(cached.location.result(cached.provider))
            return
        }

        val providers = freshProviders(accuracy)
        if (providers.isEmpty()) {
            complete(unavailable("No location provider is available"))
            return
        }
        val pending = PendingLocation(requestId, providers.size, complete)
        requests[requestId] = pending
        for (provider in providers) {
            val cancellation = CancellationSignal()
            pending.cancellations += cancellation
            runCatching {
                manager.getCurrentLocation(
                    provider,
                    cancellation,
                    context.mainExecutor,
                ) { location ->
                    if (location == null) {
                        pending.providerFinished()
                    } else {
                        pending.finish(location.result(provider))
                    }
                }
            }.onFailure {
                pending.providerFinished()
            }
        }
    }

    override fun cancel(requestId: Long) {
        requests.remove(requestId)?.cancel()
        watches.values.forEach { if (it.requestId == requestId) it.cancelPending() }
        watches.filterValues { it.createdBy == requestId }.keys.toList().forEach { watches.remove(it)?.stop() }
        if (allowUpdates) InkLocationService.cancel(requestId)
    }

    override fun requiredPermission(payload: String): String? = runCatching {
        when (JSONObject(payload).getString(ACCURACY_KEY)) {
            APPROXIMATE -> LOCATION_APPROXIMATE_PERMISSION
            PRECISE -> LOCATION_PRECISE_PERMISSION
            else -> null
        }
    }.getOrNull()

    override fun pause() {
        watches.values.forEach { it.updates.stop() }
    }

    override fun resume() {
        watches.values.forEach { watch ->
            runCatching { watch.updates.start() }.onFailure {
                watch.error = unavailable(it.message ?: "Could not resume location watch")
                watch.deliver()
            }
        }
    }

    override fun stop() {
        requests.values.toList().forEach(PendingLocation::cancel)
        requests.clear()
        watches.values.forEach(Watch::stop)
        watches.clear()
    }

    private fun executeUpdates(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            val options = JSONObject(payload)
            when (operation) {
                "watch-start", "tracking-start" -> {
                    validateLocationUpdates(options)
                    if (!hasPermission(options.getString("accuracy"))) {
                        complete(NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, "Location permission has not been granted", false))
                        return
                    }
                    if (operation == "tracking-start") {
                        InkLocationService.start(context as? MainActivity ?: error("Tracking requires a visible app"), requestId, options, complete)
                    } else {
                        check(watches.size < 16) { "Too many location watches" }
                        val id = nextWatch++
                        val watch = Watch(requestId)
                        watch.updates = LocationUpdates(context, options, { location ->
                            watch.version++
                            watch.fix = location.toJson()
                            watch.deliver()
                        }, { error -> watch.error = error; watch.deliver() })
                        watch.updates.start()
                        watches[id] = watch
                        complete(NativeResult.Success(id.toString()))
                    }
                }
                "watch-next" -> {
                    val watch = watches[options.getLong("watch")] ?: error("Location watch has stopped")
                    check(watch.complete == null) { "Location watch already has a pending read" }
                    watch.requestId = requestId
                    watch.complete = complete
                    watch.seen = options.getLong("version")
                    watch.deliver()
                    if (watch.complete != null) handler.postDelayed(watch.heartbeat, 20_000)
                }
                "watch-stop" -> {
                    watches.remove(options.getLong("watch"))?.stop()
                    complete(NativeResult.Success("stopped"))
                }
                "tracking-status" -> complete(NativeResult.Success(InkLocationService.status(context).toString()))
                "tracking-stop" -> {
                    InkLocationService.stop(context)
                    complete(NativeResult.Success("stopped"))
                }
                else -> complete(protocol("Unknown location operation: $operation"))
            }
        } catch (error: SecurityException) {
            complete(NativeResult.Failure(NativeErrorKind.PERMISSION_DENIED, error.message ?: "Location permission denied", false))
        } catch (error: Exception) {
            complete(unavailable(error.message ?: "Location updates failed"))
        }
    }

    private inner class Watch(val createdBy: Long) {
        lateinit var updates: LocationUpdates
        var version = 0L
        var fix: JSONObject? = null
        var error: NativeResult.Failure? = null
        var seen = 0L
        var requestId = 0L
        var complete: NativeResultHandler? = null
        val heartbeat = Runnable { finish(NativeResult.Success("null")) }

        fun deliver() {
            error?.let { finish(it); return }
            if (version > seen) finish(NativeResult.Success(JSONObject().put("version", version).put("fix", fix).toString()))
        }

        private fun finish(result: NativeResult) {
            val callback = complete ?: return
            cancelPending()
            callback(result)
        }

        fun cancelPending() {
            handler.removeCallbacks(heartbeat)
            complete = null
        }

        fun stop() {
            updates.stop()
            finish(unavailable("Location watch stopped"))
        }
    }

    private fun hasPermission(accuracy: String): Boolean {
        val fine = context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED
        return fine || accuracy == APPROXIMATE &&
            context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED
    }

    private fun cachedLocation(accuracy: String, maxAgeMs: Long): LocatedFix? {
        val now = SystemClock.elapsedRealtimeNanos()
        return cacheProviders(accuracy)
            .mapNotNull { provider ->
                runCatching { manager.getLastKnownLocation(provider) }
                    .getOrNull()
                    ?.let { location -> LocatedFix(location, provider) }
            }
            .filter { fix ->
                val age = now - fix.location.elapsedRealtimeNanos
                age >= 0 && age <= maxAgeMs * NANOS_PER_MILLISECOND
            }
            .maxByOrNull { fix -> fix.location.elapsedRealtimeNanos }
    }

    private fun cacheProviders(accuracy: String): List<String> = buildList {
        add(LocationManager.FUSED_PROVIDER)
        if (accuracy == PRECISE) {
            add(LocationManager.GPS_PROVIDER)
        }
        add(LocationManager.NETWORK_PROVIDER)
        add(LocationManager.PASSIVE_PROVIDER)
    }

    private fun freshProviders(accuracy: String): List<String> = buildList {
        if (accuracy == APPROXIMATE && providerEnabled(LocationManager.FUSED_PROVIDER)) {
            add(LocationManager.FUSED_PROVIDER)
            return@buildList
        }
        if (accuracy == PRECISE && providerEnabled(LocationManager.GPS_PROVIDER)) {
            add(LocationManager.GPS_PROVIDER)
        }
        if (providerEnabled(LocationManager.NETWORK_PROVIDER)) {
            add(LocationManager.NETWORK_PROVIDER)
        }
    }

    private fun providerEnabled(provider: String): Boolean =
        runCatching { manager.isProviderEnabled(provider) }.getOrDefault(false)

    private fun Location.result(sourceProvider: String): NativeResult.Bytes = NativeResult.Bytes(
        JSONObject()
            .put("latitude", latitude)
            .put("longitude", longitude)
            .put("accuracy", accuracy.toDouble())
            .put("provider", sourceProvider)
            .put("timestamp", time)
            .toString()
            .toByteArray(Charsets.UTF_8),
    )

    private data class LocatedFix(val location: Location, val provider: String)

    private inner class PendingLocation(
        private val requestId: Long,
        private var remainingProviders: Int,
        private val complete: NativeResultHandler,
    ) {
        val cancellations = mutableListOf<CancellationSignal>()
        private var finished = false

        fun providerFinished() {
            if (finished) {
                return
            }
            remainingProviders -= 1
            if (remainingProviders == 0) {
                finish(unavailable("Could not determine the current location"))
            }
        }

        fun finish(result: NativeResult) {
            if (finished) {
                return
            }
            finished = true
            requests.remove(requestId)
            cancellations.forEach(CancellationSignal::cancel)
            complete(result)
        }

        fun cancel() {
            if (finished) {
                return
            }
            finished = true
            cancellations.forEach(CancellationSignal::cancel)
        }
    }

    private companion object {
        private const val CURRENT_OPERATION = "current"
        private const val ACCURACY_KEY = "accuracy"
        private const val MAX_AGE_KEY = "maxAgeMs"
        private const val APPROXIMATE = "approximate"
        private const val PRECISE = "precise"
        private const val LOCATION_APPROXIMATE_PERMISSION = "location-approximate"
        private const val LOCATION_PRECISE_PERMISSION = "location-precise"
        private const val MAX_CACHE_AGE_MS = 3_600_000L
        private const val NANOS_PER_MILLISECOND = 1_000_000L

        private fun protocol(message: String) = NativeResult.Failure(
            NativeErrorKind.PROTOCOL,
            message,
            false,
        )

        private fun unavailable(message: String) = NativeResult.Failure(
            NativeErrorKind.UNAVAILABLE,
            message,
            true,
        )
    }
}
