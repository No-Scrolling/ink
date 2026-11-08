plugins {
    id("com.android.application") version "8.12.3"
    id("org.jetbrains.kotlin.android") version "2.3.20"
}

android {
    namespace = "com.vandam.ink"
    compileSdk = 36
    defaultConfig {
        applicationId = "com.vandam.ink.contracttests"
        minSdk = 34
        targetSdk = 36
        versionCode = 1
        versionName = "1"
        buildConfigField("boolean", "INK_CLEARTEXT_NETWORK_ENABLED", "false")
    }
    buildFeatures { buildConfig = true }
    sourceSets.getByName("main") { manifest.srcFile("AndroidManifest.xml") }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
kotlin {
    compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) }
    sourceSets.getByName("main").kotlin.apply {
        setSrcDirs(listOf("src", "../../../platform/android/app/src/main/kotlin", "../../../platform/android/app/src/network/kotlin",
            "../../../platform/android/app/src/audioPlayback/kotlin"))
        include(
            "**/ContractInstrumentation.kt", "**/ContractTests.kt", "**/NativeAdapter.kt",
            "**/NativeRequests.kt", "**/InkFetchStreams.kt", "**/InkManagedFiles.kt", "**/SecureStoreAdapter.kt",
            "**/SplineFling.kt", "**/SplineFlingContracts.kt",
            "**/AudioUsage.kt", "**/AudioQueue.kt", "**/AudioQueuePersistence.kt", "**/AudioEffects.kt", "**/AudioContracts.kt",
            "**/AudioOffload.kt", "**/AudioMediaButtons.kt", "**/InkAudioRenderersFactory.kt", "**/VoiceAudioProcessor.kt", "**/QuietAudioProcessor.kt",
            "**/AudioSource.kt", "**/InkAudioMediaSource.kt", "**/InkMediaSources.kt", "**/AudioContractProvider.kt",
        )
    }
}

dependencies {
    implementation("androidx.media3:media3-exoplayer:1.10.1")
    implementation("androidx.media3:media3-session:1.10.1")
}
