package com.vandam.ink

import android.content.Context
import android.graphics.SurfaceTexture
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.media.MediaPlayer
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.Surface
import android.view.TextureView
import android.view.View
import android.widget.FrameLayout
import org.json.JSONObject

internal fun createVideoAdapter(activity: MainActivity, root: FrameLayout, update: (Long, String) -> Unit): VideoAdapter =
    InkVideoAdapter(activity, root, update)

private class InkVideoAdapter(
    private val activity: MainActivity,
    private val root: FrameLayout,
    private val update: (Long, String) -> Unit,
) : VideoAdapter {
    private val sessions = mutableMapOf<Long, Session>()
    private var portal: VideoPortal? = null
    private var paused = false
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) =
        complete(failure("Unknown video operation"))
    override fun cancel(requestId: Long) = Unit
    override fun executeController(requestId: Long, controller: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            val data = JSONObject(payload)
            when (operation) {
                "activate" -> {
                    require(controller !in sessions) { "Video is already attached" }
                    val uri = Uri.parse(data.getString("src"))
                    require(uri.scheme in setOf("https", "content", "file", "asset") ||
                        (BuildConfig.DEBUG && uri.scheme == "http" && uri.host in setOf("localhost", "127.0.0.1", "10.0.2.2", "::1"))) { "Unsupported video URL" }
                    val session = Session(controller)
                    sessions[controller] = session
                    session.open(uri)
                    session.mount(portal?.takeIf { it.controller == controller })
                }
                "deactivate" -> sessions.remove(controller)?.close()
                "play" -> requireNotNull(sessions[controller]).play()
                "pause" -> requireNotNull(sessions[controller]).pause()
                "seek" -> requireNotNull(sessions[controller]).seek(data.getDouble("position"))
                else -> error("Unknown video operation")
            }
            complete(NativeResult.Success(""))
        } catch (error: Exception) {
            if (operation == "activate") sessions.remove(controller)?.close()
            complete(failure(error.message ?: "Could not play video"))
        }
    }
    override fun syncPortal(portal: VideoPortal?) {
        this.portal = portal
        sessions.forEach { (id, session) -> session.mount(portal?.takeIf { it.controller == id }) }
    }
    override fun pause() { paused = true; sessions.values.forEach { it.pause() } }
    override fun resume() { paused = false }
    override fun stop() { sessions.values.forEach { it.close() }; sessions.clear(); portal = null }
    private fun failure(message: String) = NativeResult.Failure(NativeErrorKind.UNEXPECTED, message, false)

    private inner class Session(private val controller: Long) : TextureView.SurfaceTextureListener {
        private val player = MediaPlayer()
        private val view = TextureView(activity).apply { surfaceTextureListener = this@Session; isOpaque = false; alpha = 0f }
        private val container = FrameLayout(activity).apply { addView(view) }
        private val playIcon = object : View(activity) {
            private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
            private val path = Path()
            override fun onDraw(canvas: Canvas) {
                val size = minOf(width, height) * 0.18f
                val x = width / 2f
                val y = height / 2f
                path.reset()
                path.moveTo(x - size * 0.35f, y - size / 2)
                path.lineTo(x + size * 0.45f, y)
                path.lineTo(x - size * 0.35f, y + size / 2)
                path.close()
                canvas.drawPath(path, paint)
            }
        }.apply { visibility = View.GONE }
        private val handler = Handler(Looper.getMainLooper())
        private val audio = activity.getSystemService(Context.AUDIO_SERVICE) as AudioManager
        private val attributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MOVIE).build()
        private val focus = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN).setAudioAttributes(attributes)
            .setOnAudioFocusChangeListener { change -> if (change < 0) pause() }.build()
        private var surface: Surface? = null
        private var ready = false
        private var rendered = false
        private var ended = false
        private var buffering = false
        private var closed = false
        private var mounted = false
        private var autoPlay = true
        private var width = 0
        private var height = 0
        private val tick = object : Runnable {
            override fun run() {
                if (closed || !ready) return
                updateClock()
                if (player.isPlaying) handler.postDelayed(this, 250)
            }
        }
        fun open(uri: Uri) {
            container.addView(playIcon, FrameLayout.LayoutParams(-1, -1))
            container.setOnClickListener {
                if (ready) { if (player.isPlaying) pause() else play() }
            }
            player.setAudioAttributes(attributes)
            player.setOnPreparedListener {
                if (closed) return@setOnPreparedListener
                ready = true
                fit()
                startIfReady()
                publish()
            }
            player.setOnVideoSizeChangedListener { _, _, _ -> fit() }
            player.setOnInfoListener { _, what, _ ->
                if (what == MediaPlayer.MEDIA_INFO_VIDEO_RENDERING_START && !closed && ready) {
                    rendered = true
                    view.alpha = 1f
                    publish()
                    true
                } else if (what == MediaPlayer.MEDIA_INFO_BUFFERING_START || what == MediaPlayer.MEDIA_INFO_BUFFERING_END) {
                    buffering = what == MediaPlayer.MEDIA_INFO_BUFFERING_START
                    updateClock()
                    true
                } else if (what == MediaPlayer.MEDIA_INFO_VIDEO_NOT_PLAYING) {
                    if (!closed && ready) player.pause()
                    playbackError("This video format isn't supported on this device.")
                    true
                } else false
            }
            player.setOnCompletionListener { ended = true; pause() }
            player.setOnSeekCompleteListener { publish() }
            player.setOnErrorListener { _, _, extra ->
                playbackError(if (extra == MediaPlayer.MEDIA_ERROR_UNSUPPORTED)
                    "This video format isn't supported on this device." else "Could not play this video.")
                true
            }
            if (uri.scheme == "asset") {
                player.setDataSource(activity.bundledMediaFile(uri.toString()).absolutePath)
            } else {
                player.setDataSource(activity, uri)
            }
            player.prepareAsync()
        }
        private fun playbackError(message: String) {
            if (closed) return
            ready = false
            autoPlay = false
            handler.removeCallbacks(tick)
            activity.removePlaybackClock(-controller)
            container.keepScreenOn = false
            audio.abandonAudioFocusRequest(focus)
            update(controller, JSONObject().put("error", message).toString())
        }
        fun mount(portal: VideoPortal?) {
            if (closed) return
            if (portal == null) {
                if (mounted) pause()
                mounted = false
                container.visibility = View.GONE
                return
            }
            mounted = true
            width = portal.width
            height = portal.height
            val params = FrameLayout.LayoutParams(width, height).apply { leftMargin = portal.x; topMargin = portal.y }
            if (container.parent == null) {
                root.addView(container, params)
            } else {
                val current = container.layoutParams as FrameLayout.LayoutParams
                if (current.width != width || current.height != height || current.leftMargin != portal.x || current.topMargin != portal.y) {
                    container.layoutParams = params
                }
            }
            container.visibility = View.VISIBLE
            fit()
            startIfReady()
        }
        private fun fit() {
            if (!ready || width <= 0 || height <= 0 || player.videoWidth <= 0 || player.videoHeight <= 0) return
            val scale = minOf(width.toFloat() / player.videoWidth, height.toFloat() / player.videoHeight)
            val w = (player.videoWidth * scale).toInt().coerceAtLeast(1)
            val h = (player.videoHeight * scale).toInt().coerceAtLeast(1)
            if (view.layoutParams.width != w || view.layoutParams.height != h) view.layoutParams = FrameLayout.LayoutParams(w, h, Gravity.CENTER)
        }
        private fun startIfReady() {
            if (autoPlay && ready && mounted && surface != null && !paused) { autoPlay = false; play() }
        }
        fun play() {
            if (!ready || closed || !mounted || paused) return
            if (audio.requestAudioFocus(focus) != AudioManager.AUDIOFOCUS_REQUEST_GRANTED) return
            if (ended || player.currentPosition >= player.duration) player.seekTo(0)
            ended = false
            player.start()
            container.keepScreenOn = true
            handler.removeCallbacks(tick)
            handler.post(tick)
            publish()
        }
        fun pause() {
            if (closed) return
            autoPlay = false
            if (ready && player.isPlaying) player.pause()
            handler.removeCallbacks(tick)
            container.keepScreenOn = false
            audio.abandonAudioFocusRequest(focus)
            if (ready) publish()
        }
        fun seek(seconds: Double) {
            require(seconds.isFinite()) { "Invalid video position" }
            if (ready) {
                ended = false
                player.seekTo((seconds * 1000).toLong().coerceIn(0, player.duration.toLong()), MediaPlayer.SEEK_CLOSEST)
            }
        }
        private fun publish() {
            updateClock()
            playIcon.visibility = if (ready && rendered && !player.isPlaying) View.VISIBLE else View.GONE
            if (!closed) update(controller, JSONObject().put("ready", ready).put("rendered", rendered)
                .put("position", if (ready) (if (ended) player.duration else player.currentPosition) / 1000.0 else 0).put("duration", if (ready) player.duration / 1000.0 else 0).toString())
        }
        private fun updateClock() {
            if (closed || !ready) return
            activity.updatePlaybackClock(-controller,
                (if (ended) player.duration else player.currentPosition).coerceAtLeast(0).toLong(),
                player.duration.coerceAtLeast(0).toLong(), player.isPlaying && !buffering, 1f)
        }
        fun close() {
            pause()
            closed = true
            activity.removePlaybackClock(-controller)
            ready = false
            player.release()
            surface?.release()
            surface = null
            root.removeView(container)
        }
        override fun onSurfaceTextureAvailable(texture: SurfaceTexture, width: Int, height: Int) {
            if (closed) return
            surface = Surface(texture)
            player.setSurface(surface)
            startIfReady()
        }
        override fun onSurfaceTextureDestroyed(texture: SurfaceTexture): Boolean {
            if (!closed) { pause(); player.setSurface(null) }
            surface?.release()
            surface = null
            return true
        }
        override fun onSurfaceTextureSizeChanged(texture: SurfaceTexture, width: Int, height: Int) = Unit
        override fun onSurfaceTextureUpdated(texture: SurfaceTexture) = Unit
    }
}
