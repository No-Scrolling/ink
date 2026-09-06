package com.vandam.ink

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import org.json.JSONObject

internal class ConnectivityAdapter(context: Context, private val emit: (String) -> Unit) : NativeAdapter, AutoCloseable {
    private val manager = context.getSystemService(ConnectivityManager::class.java)
    private var watching = false
    private var active = true
    private var registered = false
    private val callback = object : ConnectivityManager.NetworkCallback() {
        override fun onAvailable(network: Network) = changed()
        override fun onLost(network: Network) = changed()
        override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) = changed()
    }
    private fun snapshot(): JSONObject {
        val result = JSONObject()
        val service = manager ?: return result.put("status", "unknown")
        val network = service.activeNetwork ?: return result.put("status", "offline")
        val capabilities = service.getNetworkCapabilities(network) ?: return result.put("status", "unknown")
        val transport = when {
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN) -> "vpn"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> "ethernet"
            else -> "other"
        }
        return result.put("status", "connected").put("transport", transport).put("metered", service.isActiveNetworkMetered)
    }
    @Synchronized private fun changed() {
        if (active && watching) emit(JSONObject().put("type", "connectivity-changed").put("data", snapshot()).toString())
    }
    @Synchronized private fun update() {
        if (active && watching && !registered) { manager?.registerDefaultNetworkCallback(callback); registered = manager != null }
        else if ((!active || !watching) && registered) { manager?.unregisterNetworkCallback(callback); registered = false }
    }
    @Synchronized fun start() { active = true; update(); changed() }
    @Synchronized fun stop() { active = false; update() }
    @Synchronized override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            when (operation) {
                "watch" -> { watching = true; update() }
                "unwatch" -> { watching = false; update() }
                "snapshot" -> Unit
                else -> throw IllegalArgumentException("Unknown connectivity operation")
            }
            complete(NativeResult.Success(snapshot().toString()))
        } catch (error: Exception) { complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Could not read network state", false)) }
    }
    override fun cancel(requestId: Long) = Unit
    @Synchronized fun reset() { watching = false; update() }
    override fun close() { reset(); stop() }
}
