package com.vandam.ink

import android.view.Surface
import java.nio.ByteBuffer

internal interface InkJavaScriptHost {
    fun onJavaScriptReady()
}

internal object InkNative {
    init {
        System.loadLibrary("ink_android")
    }

    @JvmStatic
    external fun nativeCreate(cacheDirectory: String, mediaPreviews: Boolean): Long

    @JvmStatic
    external fun nativeSetViewportScale(handle: Long, scale: Float)

    @JvmStatic
    external fun nativeTakeJavaScriptError(handle: Long): String?

    @JvmStatic
    external fun nativeRefreshJavaScript(handle: Long, source: String): Boolean

    @JvmStatic
    external fun nativeStartJavaScript(
        handle: Long, source: String?, icons: ByteArray?, assets: android.content.res.AssetManager,
        storePath: String, filesPath: String, host: InkJavaScriptHost, bundle: String,
    ): Boolean

    @JvmStatic
    external fun nativeDrainJavaScript(handle: Long): Boolean

    @JvmStatic
    external fun nativeIsLightAppearance(handle: Long): Boolean

    @JvmStatic
    external fun nativeTakeJavaScriptCalls(handle: Long): String?

    @JvmStatic
    external fun nativeTakeJavaScriptBytes(handle: Long, id: Long): ByteArray?

    @JvmStatic
    external fun nativeJavaScriptReceiveBytes(handle: Long, message: String, bytes: ByteArray): Boolean

    @JvmStatic
    external fun nativePlaybackClock(handle: Long, controller: Long, position: Float, duration: Float, playing: Boolean, speed: Float): Boolean
    @JvmStatic
    external fun nativeCaptureState(handle: Long, controller: Long, value: String): Boolean
    @JvmStatic
    external fun nativeRemovePlaybackClock(handle: Long, controller: Long)

    @JvmStatic
    external fun nativeJavaScriptReceive(handle: Long, message: String)

    @JvmStatic
    external fun nativePublicSans(): ByteBuffer

    @JvmStatic
    external fun nativeAttachSurface(
        handle: Long,
        surface: Surface,
        width: Int,
        height: Int,
    ): Boolean

    @JvmStatic
    external fun nativeSetKeyboardInset(handle: Long, height: Int): Boolean

    @JvmStatic
    external fun nativeSetDisplayCutouts(handle: Long, bounds: IntArray): Boolean

    @JvmStatic
    external fun nativeResize(handle: Long, width: Int, height: Int)

    @JvmStatic
    external fun nativeImageWaitRemaining(handle: Long): Long

    @JvmStatic
    external fun nativeHasSceneAnimations(handle: Long): Boolean
    /** Milliseconds until the scene changes on its own, zero for every frame, or -1 when idle. */
    @JvmStatic
    external fun nativeNextSceneFrame(handle: Long): Long
    @JvmStatic
    external fun nativeAnimateScene(handle: Long): Boolean
    @JvmStatic
    external fun nativeFrameReady(handle: Long): Boolean

    @JvmStatic
    external fun nativeCanPresentPointerMove(handle: Long, id: Int, y: Float): Boolean

    @JvmStatic
    external fun nativePointer(
        handle: Long,
        action: Int,
        id: Int,
        x: Float,
        y: Float,
    ): Int

    @JvmStatic
    external fun nativeImagePinchBegin(handle: Long, x: Float, y: Float): Int

    @JvmStatic
    external fun nativeImagePinchUpdate(
        handle: Long,
        scale: Float,
        x: Float,
        y: Float,
    ): Int

    @JvmStatic
    external fun nativeImagePinchEnd(handle: Long)

    @JvmStatic
    external fun nativeImageZoomTarget(handle: Long, x: Float, y: Float): Long

    @JvmStatic
    external fun nativeImageDoubleTap(
        handle: Long,
        x: Float,
        y: Float,
    ): Int

    @JvmStatic
    external fun nativeScrollBy(handle: Long, delta: Float): Int

    @JvmStatic
    external fun nativeRender(handle: Long): String

    @JvmStatic
    external fun nativeInstallSystemGlyph(
        handle: Long,
        requestId: Long,
        pixels: IntArray,
    ): Boolean

    @JvmStatic
    external fun nativeBack(handle: Long): Boolean

    @JvmStatic
    external fun nativeCameraPortal(handle: Long): String?

    @JvmStatic
    external fun nativeCompletePixels(handle: Long, requestId: Long, width: Int, height: Int, rgba: ByteArray): Boolean

    @JvmStatic
    external fun nativeCameraReviewReady(handle: Long): Boolean

    @JvmStatic
    external fun nativeVideoPortal(handle: Long): String?
    @JvmStatic
    external fun nativeMapPortal(handle: Long): String?

    @JvmStatic
    external fun nativeTextInputState(handle: Long): Int

    @JvmStatic
    external fun nativeTextInputContext(handle: Long): String

    @JvmStatic
    external fun nativeTextInput(handle: Long, action: Int, value: String?): Boolean

    @JvmStatic
    external fun nativeNextRequest(handle: Long): String?

    @JvmStatic
    external fun nativeCompleteFile(
        handle: Long,
        requestId: Long,
        path: String,
    ): Int

    @JvmStatic
    external fun nativeCompleteAsset(
        handle: Long,
        requestId: Long,
        assets: android.content.res.AssetManager,
        path: String,
    ): Int

    @JvmStatic
    external fun nativeCompleteEncoded(
        handle: Long,
        requestId: Long,
        bytes: ByteArray,
        size: Int,
    ): Int

    @JvmStatic
    external fun nativeFailRequest(
        handle: Long,
        requestId: Long,
        kind: Int,
        message: String,
        retryable: Boolean,
    ): Boolean

    @JvmStatic
    external fun nativeCompleteAction(handle: Long, requestId: Long): Boolean

    @JvmStatic
    external fun nativeMediaPreviewNeeded(url: String): Boolean

    @JvmStatic
    external fun nativeMediaPreviewStore(url: String, bytes: ByteArray): Boolean

    @JvmStatic
    external fun nativeAudioActivate(
        handle: Long,
        controller: Long,
        kind: String,
        config: String,
    ): Boolean

    @JvmStatic
    external fun nativeAudioDeactivate(handle: Long, controller: Long)

    @JvmStatic
    external fun nativeAudioSetEnabled(
        handle: Long,
        controller: Long,
        enabled: Boolean,
    ): Boolean

    @JvmStatic
    external fun nativeAudioObserve(handle: Long, controller: Long, observed: Boolean): Boolean

    @JvmStatic
    external fun nativeAudioSamples(
        handle: Long,
        samples: ShortArray,
        sampleRate: Int,
    ): Boolean

    @JvmStatic
    external fun nativeDetachSurface(handle: Long)

    @JvmStatic
    external fun nativeDestroy(handle: Long)

}
