package com.vandam.ink

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import org.json.JSONObject

internal class SecureValues(context: Context) {
    private val directory = File(context.noBackupFilesDir, "credentials").apply { mkdirs() }
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey("ink.credentials", null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder("ink.credentials", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    private fun file(name: String): android.util.AtomicFile {
        require(name.isNotBlank() && name.length <= 256) { "Invalid secure store key" }
        check(directory.isDirectory) { "Secure storage is unavailable" }
        val hash = MessageDigest.getInstance("SHA-256").digest(name.toByteArray()).joinToString("") { "%02x".format(it) }
        return android.util.AtomicFile(File(directory, hash))
    }
    fun get(name: String): String? = synchronized(lock) {
        val file = file(name)
        if (!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return@synchronized null
        val data = JSONObject(file.readFully().toString(Charsets.UTF_8))
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, Base64.decode(data.getString("iv"), Base64.NO_WRAP)))
        cipher.updateAAD(name.toByteArray())
        cipher.doFinal(Base64.decode(data.getString("value"), Base64.NO_WRAP)).toString(Charsets.UTF_8)
    }
    fun set(name: String, value: String) = synchronized(lock) {
        require(value.toByteArray().size <= 256 * 1024) { "Secure value is too large" }
        val file = file(name)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(name.toByteArray())
        val data = JSONObject().put("iv", Base64.encodeToString(cipher.iv, Base64.NO_WRAP))
            .put("value", Base64.encodeToString(cipher.doFinal(value.toByteArray()), Base64.NO_WRAP)).toString().toByteArray()
        val stream = file.startWrite()
        try { stream.write(data); file.finishWrite(stream) } catch (error: Exception) { file.failWrite(stream); throw error }
    }
    fun remove(name: String) = synchronized(lock) {
        val file = file(name)
        file.delete()
        check(!file.baseFile.exists()) { "Could not remove secure value" }
    }
    companion object { private val lock = Any() }
}

internal class SecureStoreAdapter(context: Context) : NativeAdapter {
    private val values = SecureValues(context.applicationContext)
    override fun execute(requestId: Long, operation: String, payload: String, complete: NativeResultHandler) {
        try {
            val input = JSONObject(payload)
            val key = input.getString("key")
            val value = when (operation) {
                "get" -> values.get(key)?.let { JSONObject.quote(it) } ?: "null"
                "set" -> { values.set(key, input.getString("value")); "null" }
                "remove" -> { values.remove(key); "null" }
                else -> throw IllegalArgumentException("Unknown secure store operation")
            }
            complete(NativeResult.Success(value))
        } catch (error: Exception) { complete(NativeResult.Failure(NativeErrorKind.UNEXPECTED, "Secure storage failed: ${error.javaClass.simpleName}", false)) }
    }
    override fun cancel(requestId: Long) = Unit
}
