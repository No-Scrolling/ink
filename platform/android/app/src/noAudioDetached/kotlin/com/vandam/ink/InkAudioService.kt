package com.vandam.ink

import android.app.Service
import android.content.Intent
import android.os.IBinder

internal class InkAudioService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null
}
