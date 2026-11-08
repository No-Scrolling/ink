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
        buildConfigField("boolean", "INK_LIGHT_SDK_ENABLED", "true")
        buildConfigField("String", "INK_LIGHT_SERVER_PACKAGE", "\"com.vandam.ink.contracttests\"")
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
            "../../../platform/android/app/src/audioPlayback/kotlin", "../../../platform/android/app/src/audioDetached/kotlin", "../../../platform/android/app/src/lightSdkPush/kotlin",
            "../../../platform/android/app/src/notifications/kotlin", "../../../platform/android/app/src/crypto/kotlin",
            "../../../platform/android/app/src/noBackground/kotlin", "../../../platform/android/app/src/lightSdkToolManager/kotlin"))
        include(
            "**/ContractInstrumentation.kt", "**/ContractTests.kt", "**/NativeAdapter.kt",
            "**/PermissionsAdapter.kt", "**/LightSdkAdapter.kt", "**/PermissionContracts.kt",
            "**/NativeRequests.kt", "**/InkFetchStreams.kt", "**/InkManagedFiles.kt", "**/SecureStoreAdapter.kt",
            "**/SplineFling.kt", "**/SplineFlingContracts.kt",
            "**/AudioUsage.kt", "**/AudioQueue.kt", "**/AudioQueuePersistence.kt", "**/AudioEffects.kt", "**/AudioContracts.kt",
            "**/InkAudioService.kt", "**/AudioReadiness.kt", "**/AudioOffload.kt", "**/AudioMediaButtons.kt", "**/InkAudioRenderersFactory.kt", "**/VoiceAudioProcessor.kt", "**/QuietAudioProcessor.kt",
            "**/AudioSource.kt", "**/InkAudioMediaSource.kt", "**/InkMediaSources.kt", "**/AudioContractProvider.kt",
            "**/PushContracts.kt", "**/InkLightPushAdapter.kt", "**/LightPushAdapter.kt",
            "**/InkNotificationsAdapter.kt", "**/NotificationsAdapter.kt", "**/InkCipher.kt",
            "**/ToolManagerContracts.kt", "**/ToolManagerAdapter.kt", "**/InkSharedFiles.kt", "**/InkToolManagerAdapter.kt",
            "**/NoBackgroundAdapter.kt", "**/BackgroundAdapter.kt",
        )
    }
}

dependencies {
    implementation("org.unifiedpush.android:connector:3.3.2")
    implementation("androidx.media3:media3-exoplayer:1.10.1")
    implementation("androidx.media3:media3-session:1.10.1")
}
