package com.vandam.ink

import android.net.Uri

internal data class StagedRingtone(
    val relativePath: String,
    val uri: Uri,
)

internal interface RingtoneFiles {
    fun stage(source: String): StagedRingtone
    fun commit(kind: String, staged: StagedRingtone)
    fun discard(staged: StagedRingtone)
}
