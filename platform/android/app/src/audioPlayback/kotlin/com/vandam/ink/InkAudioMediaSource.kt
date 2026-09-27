package com.vandam.ink

import android.content.Context
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultDataSource
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory

@UnstableApi
internal fun audioMediaSourceFactory(context: Context): DefaultMediaSourceFactory {
    val http = DefaultHttpDataSource.Factory()
        .setAllowCrossProtocolRedirects(BuildConfig.INK_CLEARTEXT_NETWORK_ENABLED)
    return DefaultMediaSourceFactory(DefaultDataSource.Factory(context, http))
}
