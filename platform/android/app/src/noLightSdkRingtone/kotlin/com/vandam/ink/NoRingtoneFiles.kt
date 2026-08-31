package com.vandam.ink

import android.content.Context

internal fun createRingtoneFiles(_context: Context): RingtoneFiles = NoRingtoneFiles

private object NoRingtoneFiles : RingtoneFiles {
    override fun stage(source: String): StagedRingtone =
        error("Ringtone installation is not packaged")

    override fun commit(kind: String, staged: StagedRingtone) = Unit

    override fun discard(staged: StagedRingtone) = Unit
}
