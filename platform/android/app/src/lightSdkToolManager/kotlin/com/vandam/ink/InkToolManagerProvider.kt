package com.vandam.ink

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.content.pm.ProviderInfo
import android.net.Uri
import android.os.Bundle
import android.os.CancellationSignal
import com.thelightphone.toolmanager.LightFileProvider

class InkToolManagerProvider : ContentProvider() {
    private val provider = LightFileProvider()

    override fun attachInfo(context: Context, info: ProviderInfo) {
        val manager = InkToolManager(context.applicationContext)
        LightFileProvider.manifest = { manager.manifest }
        LightFileProvider.onToolManagerDataUpdate = manager::changed
        provider.attachInfo(context, info)
        super.attachInfo(context, info)
    }

    override fun onCreate() = true
    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?) =
        provider.query(uri, projection, selection, selectionArgs, sortOrder)
    override fun getType(uri: Uri) = provider.getType(uri)
    override fun insert(uri: Uri, values: ContentValues?) = provider.insert(uri, values)
    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) =
        provider.update(uri, values, selection, selectionArgs)
    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = provider.delete(uri, selection, selectionArgs)
    override fun openFile(uri: Uri, mode: String) = provider.openFile(uri, mode)
    override fun openTypedAssetFile(uri: Uri, mimeTypeFilter: String, opts: Bundle?, signal: CancellationSignal?) =
        provider.openTypedAssetFile(uri, mimeTypeFilter, opts, signal)
    override fun call(method: String, arg: String?, extras: Bundle?) = provider.call(method, arg, extras)
}
