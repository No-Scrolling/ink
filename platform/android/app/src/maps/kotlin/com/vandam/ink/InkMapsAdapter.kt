package com.vandam.ink

import android.graphics.Color
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import org.json.JSONArray
import org.json.JSONObject
import org.maplibre.android.MapLibre
import org.maplibre.android.annotations.Marker
import org.maplibre.android.annotations.MarkerOptions
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.camera.CameraUpdateFactory
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapLibreMapOptions
import org.maplibre.android.maps.MapView

internal fun createMapsAdapter(
    activity: MainActivity,
    root: FrameLayout,
    updateController: (Long, String) -> Unit,
): MapsAdapter = InkMapsAdapter(activity, root, updateController)

@Suppress("DEPRECATION")
private class InkMapsAdapter(
    private val activity: MainActivity,
    private val root: FrameLayout,
    private val updateController: (Long, String) -> Unit,
) : MapsAdapter {
    private val sessions = mutableMapOf<Long, Session>()
    private var portal: MapPortal? = null
    private var paused = false

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        complete(failure("Unknown maps operation: $operation"))
    }

    override fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            val data = JSONObject(payload)
            when (operation) {
                "activate" -> {
                    require(!sessions.containsKey(controller)) { "Map is already attached" }
                    val style = styleURL(data)
                    val initial = camera(data.getJSONObject("initialCentre"), data.getDouble("initialZoom"))
                    val markers = readMarkers(data.getJSONArray("markers"))
                    MapLibre.getInstance(activity)
                    val session = Session(controller, style, initial, markers, backgroundColour(data))
                    sessions[controller] = session
                    session.mount(portal?.takeIf { it.controller == controller })
                    complete(NativeResult.Success(""))
                }
                "deactivate" -> {
                    sessions.remove(controller)?.close()
                    complete(NativeResult.Success(""))
                }
                "update" -> {
                    val session = requireNotNull(sessions[controller]) { "Map is not attached" }
                    session.update(styleURL(data), readMarkers(data.getJSONArray("markers")), backgroundColour(data))
                    complete(NativeResult.Success(""))
                }
                "move-to" -> {
                    val session = requireNotNull(sessions[controller]) { "Map is not attached" }
                    session.move(requestId, camera(data.getJSONObject("centre"), data.getDouble("zoom")), complete)
                }
                else -> complete(failure("Unknown map controller operation: $operation"))
            }
        } catch (error: Exception) {
            complete(failure(error.message ?: "Invalid map request"))
        }
    }

    override fun cancel(requestId: Long) { sessions.values.forEach { it.pending.remove(requestId) } }
    override fun syncPortal(portal: MapPortal?) {
        this.portal = portal
        sessions.forEach { (id, session) -> session.mount(portal?.takeIf { it.controller == id }) }
    }
    override fun pause() {
        paused = true
        sessions.values.forEach { it.suspendView() }
    }
    override fun resume() {
        paused = false
        syncPortal(portal)
    }
    override fun stop() {
        sessions.values.forEach { it.close() }
        sessions.clear()
        portal = null
    }

    private inner class Session(
        private val controller: Long,
        private var styleURL: String,
        initial: CameraPosition,
        private var markerData: List<MarkerData>,
        private var backgroundColour: Int,
    ) {
        private val view = MapView(activity, MapLibreMapOptions.createFromAttributes(activity)
            .textureMode(true).foregroundLoadColor(backgroundColour))
        private val cover = View(activity).apply {
            setBackgroundColor(backgroundColour)
            isClickable = true
        }
        private val container = FrameLayout(activity).apply {
            addView(view, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
            addView(cover, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        }
        private var map: MapLibreMap? = null
        private var started = false
        private var closed = false
        private var mountedPortal: MapPortal? = null
        private var loadingStyle = true
        private var styleLoaded = false
        private val markers = mutableMapOf<String, Marker>()
        val pending = linkedMapOf<Long, Pair<CameraPosition, NativeResultHandler>>()

        init {
            view.setBackgroundColor(backgroundColour)
            view.foreground = null
            view.onCreate(null)
            view.addOnDidFinishRenderingFrameListener(MapView.OnDidFinishRenderingFrameWithStatsListener { fullyRendered, _ ->
                if (!closed && loadingStyle && styleLoaded && fullyRendered) {
                    cover.visibility = View.GONE
                    loadingStyle = false
                }
            })
            view.addOnDidFailLoadingMapListener { message ->
                if (!closed) emit(JSONObject().put("type", "error").put("error", inkError(message = message)))
            }
            view.getMapAsync { ready ->
                if (!closed) {
                    map = ready
                    ready.setMaxZoomPreference(22.0)
                    ready.cameraPosition = initial
                    ready.uiSettings.isAttributionEnabled = true
                    ready.setOnMarkerClickListener { marker ->
                        markers.entries.firstOrNull { it.value.id == marker.id }?.let {
                            emit(JSONObject().put("type", "marker-press").put("id", it.key))
                        }
                        false
                    }
                    ready.addOnCameraIdleListener {
                        val position = ready.cameraPosition
                        val target = position.target
                        if (target != null) emit(JSONObject().put("type", "camera-idle")
                            .put("centre", JSONObject().put("latitude", target.latitude).put("longitude", target.longitude))
                            .put("zoom", position.zoom))
                    }
                    loadStyle()
                    val commands = pending.values.toList()
                    pending.clear()
                    commands.forEach { (position, complete) ->
                        ready.moveCamera(CameraUpdateFactory.newCameraPosition(position))
                        complete(NativeResult.Success(""))
                    }
                }
            }
        }

        fun mount(portal: MapPortal?) {
            if (portal == null || paused) {
                container.visibility = View.GONE
                suspendView()
                return
            }
            if (container.parent == null) root.addView(container)
            if (mountedPortal != portal) {
                container.layoutParams = FrameLayout.LayoutParams(portal.width, portal.height).apply {
                    leftMargin = portal.x
                    topMargin = portal.y
                }
                mountedPortal = portal
            }
            container.visibility = View.VISIBLE
            if (!started) {
                view.onStart()
                view.onResume()
                started = true
            }
        }
        fun suspendView() {
            if (started) {
                view.onPause()
                view.onStop()
                started = false
            }
        }
        fun update(style: String, data: List<MarkerData>, colour: Int) {
            markerData = data
            if (backgroundColour != colour) {
                backgroundColour = colour
                view.setBackgroundColor(colour)
                cover.setBackgroundColor(colour)
            }
            if (styleURL != style) {
                styleURL = style
                loadStyle()
            } else syncMarkers()
        }
        private fun loadStyle() {
            loadingStyle = true
            styleLoaded = false
            cover.visibility = View.VISIBLE
            map?.setStyle(styleURL) {
                if (!closed) {
                    styleLoaded = true
                    syncMarkers()
                    emit(JSONObject().put("type", "ready"))
                }
            }
        }
        private fun syncMarkers() {
            val active = map ?: return
            val ids = markerData.map { it.id }.toSet()
            markers.keys.filter { it !in ids }.forEach { id -> markers.remove(id)?.let(active::removeMarker) }
            markerData.forEach { data ->
                val marker = markers[data.id]
                if (marker == null) {
                    markers[data.id] = active.addMarker(MarkerOptions().position(data.position).title(data.label))
                } else {
                    marker.position = data.position
                    marker.title = data.label
                    active.updateMarker(marker)
                }
            }
        }
        fun move(requestId: Long, position: CameraPosition, complete: NativeResultHandler) {
            val active = map
            if (active == null) pending[requestId] = position to complete
            else {
                active.moveCamera(CameraUpdateFactory.newCameraPosition(position))
                complete(NativeResult.Success(""))
            }
        }
        fun close() {
            closed = true
            pending.values.forEach { (_, complete) -> complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Map was detached", false)) }
            pending.clear()
            suspendView()
            (container.parent as? ViewGroup)?.removeView(container)
            view.onDestroy()
            map = null
        }
        private fun emit(value: JSONObject) { if (!closed) updateController(controller, value.toString()) }
    }

    private data class MarkerData(val id: String, val position: LatLng, val label: String?)
    private fun backgroundColour(value: JSONObject): Int = when (value.getString("colourScheme")) {
        "light" -> Color.WHITE
        "dark" -> Color.BLACK
        else -> throw IllegalArgumentException("Invalid map colour scheme")
    }
    private fun readMarkers(values: JSONArray): List<MarkerData> {
        val ids = mutableSetOf<String>()
        return (0 until values.length()).map { index ->
            val value = values.getJSONObject(index)
            val id = value.getString("id")
            require(id.isNotEmpty() && ids.add(id)) { "Markers require unique non-empty IDs" }
            MarkerData(id, coordinate(value), if (value.has("label")) value.getString("label") else null)
        }
    }
    private fun styleURL(value: JSONObject): String = value.getString("styleURL").also {
        require(it.startsWith("https://")) { "Map styleURL must use HTTPS" }
    }
    private fun coordinate(value: JSONObject): LatLng {
        val latitude = value.getDouble("latitude")
        val longitude = value.getDouble("longitude")
        require(latitude.isFinite() && latitude in -90.0..90.0 && longitude.isFinite() && longitude in -180.0..180.0) { "Invalid map coordinates" }
        return LatLng(latitude, longitude)
    }
    private fun camera(centre: JSONObject, zoom: Double): CameraPosition {
        require(zoom.isFinite() && zoom in 0.0..22.0) { "Map zoom must be between 0 and 22" }
        return CameraPosition.Builder().target(coordinate(centre)).zoom(zoom).build()
    }
    private fun failure(message: String) = NativeResult.Failure(NativeErrorKind.PROTOCOL, message, false)
}
