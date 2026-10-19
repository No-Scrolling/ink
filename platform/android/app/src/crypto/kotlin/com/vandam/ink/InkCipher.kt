package com.vandam.ink

import android.util.Base64
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

internal class InkCipher(key: String) {
    private val key = SecretKeySpec(Base64.decode(key, Base64.NO_WRAP).also {
        require(it.size == 32) { "Encryption key must contain 32 bytes" }
    }, "AES")

    fun encrypt(text: String): String {
        val nonce = ByteArray(12).also { random.nextBytes(it) }
        val cipher = cipher(Cipher.ENCRYPT_MODE, nonce)
        return PREFIX + Base64.encodeToString(nonce + cipher.doFinal(text.toByteArray(Charsets.UTF_8)), Base64.NO_WRAP)
    }

    fun decrypt(text: String): String {
        require(text.startsWith(PREFIX)) { "Unsupported encrypted format" }
        val bytes = Base64.decode(text.substring(PREFIX.length), Base64.NO_WRAP)
        require(bytes.size >= 28) { "Invalid encrypted data" }
        val cipher = cipher(Cipher.DECRYPT_MODE, bytes.copyOfRange(0, 12))
        return cipher.doFinal(bytes, 12, bytes.size - 12).toString(Charsets.UTF_8)
    }

    private fun cipher(mode: Int, nonce: ByteArray) = Cipher.getInstance("AES/GCM/NoPadding").apply {
        init(mode, key, GCMParameterSpec(128, nonce))
        updateAAD(PREFIX.toByteArray(Charsets.UTF_8))
    }

    private companion object {
        const val PREFIX = "ink1."
        val random = SecureRandom()
    }
}
