package com.vandam.ink

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.os.Looper
import org.json.JSONObject

internal fun Location.toJson(): JSONObject = JSONObject()
    .put("latitude", latitude)
    .put("longitude", longitude)
    .put("accuracy", accuracy.toDouble())
    .put("provider", provider ?: "unknown")
    .put("timestamp", time)

internal class LocationUpdates(
    private val context: Context,
    private val options: JSONObject,
    private val update: (Location) -> Unit,
    private val failure: (NativeResult.Failure) -> Unit,
) : LocationListener {
    private val manager = context.getSystemService(LocationManager::class.java)
    private var provider: String? = null

    fun start() {
        if (provider != null) return
        val precise = options.optString("accuracy") == "precise"
        val fine = context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED
        val coarse = context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED
        if (!fine && (precise || !coarse)) throw SecurityException("Location permission has not been granted")
        check(manager.isLocationEnabled) { "Location is switched off" }
        val candidates = if (precise) listOf(LocationManager.GPS_PROVIDER, LocationManager.FUSED_PROVIDER, LocationManager.NETWORK_PROVIDER)
            else listOf(LocationManager.FUSED_PROVIDER, LocationManager.NETWORK_PROVIDER)
        val selected = candidates.firstOrNull { runCatching { manager.isProviderEnabled(it) }.getOrDefault(false) }
            ?: error("No location provider is available")
        manager.requestLocationUpdates(selected, options.getLong("intervalMs"), options.getDouble("distanceMetres").toFloat(), this, Looper.getMainLooper())
        provider = selected
    }

    override fun onLocationChanged(location: Location) = update(location)

    override fun onProviderDisabled(provider: String) {
        if (provider == this.provider) failure(NativeResult.Failure(NativeErrorKind.LOCATION_DISABLED, "Location provider was disabled", false))
    }

    fun stop() {
        manager.removeUpdates(this)
        provider = null
    }
}

internal fun validateLocationUpdates(options: JSONObject) {
    require(options.optString("accuracy") in setOf("approximate", "precise")) { "Invalid location accuracy" }
    require(options.optLong("intervalMs", -1) in 1000..3_600_000) { "Location interval must be between one second and one hour" }
    val distance = options.optDouble("distanceMetres", Double.NaN)
    require(distance.isFinite() && distance in 0.0..100_000.0) { "Invalid location distance" }
}
