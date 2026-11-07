package com.vandam.ink

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.util.Log
import android.view.KeyEvent
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.WindowManager
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import java.io.File

class MainActivity : Activity() {
    private lateinit var runtime: InkUiRuntime
    private lateinit var bindings: InkActivityBindings
    private var developmentError: AlertDialog? = null
    private val backCallback = OnBackInvokedCallback { handleBack() }

    override fun onCreate(savedInstanceState: Bundle?) {
        installSplashScreen().setOnExitAnimationListener { it.remove() }
        super.onCreate(savedInstanceState)
        window.setWindowAnimations(0)
        window.setDecorFitsSystemWindows(false)
        window.attributes = window.attributes.apply {
            layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
        }
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK

        runtime = InkUiRuntime(this, InkBundle(this, "app"), { runtime ->
            InkActivityBindings(this, runtime) { createAppNativeAdapters(this) }.also { bindings = it }
        }, onFailure = ::runtimeFailed)
        setContentView(bindings.root)
        onBackInvokedDispatcher.registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT, backCallback)
        enterFullscreen()
        startBundle(null)
        if (BuildConfig.DEBUG) intent.getStringExtra("ink.dev.generation")?.let(::activateBundle)
        bindings.intent(intent)
    }

    private fun startBundle(directory: File?) {
        developmentError?.dismiss()
        try {
            if (BuildConfig.DEBUG && directory == null) runtime.bundle.clearActive()
            runtime.start(directory)
        } catch (error: Exception) {
            if (!BuildConfig.DEBUG) throw error
            showDevelopmentError(error.message) { startBundle(directory) }
        }
    }

    private fun activateBundle(generation: String) {
        if (!BuildConfig.DEBUG || !generation.matches(Regex("[0-9]+"))) return
        developmentError?.dismiss()
        try { runtime.activate(generation) }
        catch (error: Exception) { showDevelopmentError(error.message) { activateBundle(generation) } }
    }

    private fun runtimeFailed(error: String) {
        if (BuildConfig.DEBUG) {
            val mapped = DevelopmentErrors.map(error) { runtime.bundle.open("app.js.map").bufferedReader().use { it.readText() } }
            showDevelopmentError(mapped) { startBundle(runtime.bundle.directory) }
        } else {
            Log.e("Ink", error)
            AlertDialog.Builder(this).setTitle("App stopped")
                .setMessage("Close the app and open it again to try again.")
                .setPositiveButton("Close") { _, _ -> finish() }.setCancelable(false).show()
        }
    }

    private fun showDevelopmentError(message: String?, reload: () -> Unit) {
        Log.e("Ink", message ?: "Bundle activation failed")
        developmentError = AlertDialog.Builder(this).setTitle("Ink development error")
            .setMessage(message).setPositiveButton("Reload") { _, _ -> reload() }
            .setNegativeButton("Close", null).show()
    }

    internal fun openBundleAsset(path: String) = runtime.bundle.open(path)
    internal fun emit(message: String) = runtime.send(message)

    private fun handleBack() { if (!bindings.back()) finish() }

    @Suppress("DEPRECATION")
    override fun onBackPressed() = handleBack()

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        onUserInteraction()
        if (window.superDispatchKeyEvent(event)) return true
        if (bindings.forwardDeviceKey(event)) return true
        return event.dispatch(this, window.decorView.keyDispatcherState, this)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (BuildConfig.DEBUG) intent.getStringExtra("ink.dev.generation")?.let(::activateBundle)
        bindings.intent(intent)
    }

    override fun onResume() {
        super.onResume()
        bindings.resume()
    }

    override fun onPause() {
        bindings.pause()
        super.onPause()
    }

    @Deprecated("Android activity result callback")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        bindings.activityResult(requestCode, resultCode, data)
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        bindings.permissionResult(requestCode)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) enterFullscreen()
    }

    private fun enterFullscreen() {
        window.insetsController?.apply {
            hide(WindowInsets.Type.systemBars())
            systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }

    override fun onDestroy() {
        developmentError?.dismiss()
        onBackInvokedDispatcher.unregisterOnBackInvokedCallback(backCallback)
        runtime.close()
        super.onDestroy()
    }
}
