package com.vandam.ink

import org.json.JSONObject
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import java.util.concurrent.FutureTask

internal class AssetsAdapter(private val activity: MainActivity) : NativeAdapter {
    private val executor = Executors.newSingleThreadExecutor()
    private val requests = ConcurrentHashMap<Long, FutureTask<Unit>>()

    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        val path = runCatching { JSONObject(payload).getString("source") }.getOrNull()
        if (operation != "image" || path == null || !path.matches(Regex("ink-assets/[a-f0-9]{64}\\.(png|jpe?g|webp)"))) {
            complete(NativeResult.Failure(NativeErrorKind.PROTOCOL, "Invalid bundled image", false))
            return
        }
        val task = FutureTask<Unit> {
            var file: File? = null
            val result = try {
                val target = File.createTempFile("ink-asset-", ".image", activity.cacheDir)
                file = target
                activity.openBundleAsset(path).use { input -> target.outputStream().use(input::copyTo) }
                NativeResult.File(target.absolutePath)
            } catch (error: Exception) {
                file?.delete()
                NativeResult.Failure(NativeErrorKind.UNEXPECTED, error.message ?: "Could not read bundled image", false)
            }
            activity.runOnUiThread {
                if (requests.remove(requestId) != null) complete(result)
                else file?.delete()
            }
        }
        requests[requestId] = task
        executor.execute(task)
    }

    override fun cancel(requestId: Long) {
        requests.remove(requestId)?.cancel(true)
    }

    fun stop() {
        requests.keys.toList().forEach(::cancel)
        executor.shutdown()
    }
}
