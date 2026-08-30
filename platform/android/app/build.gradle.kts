import org.gradle.api.tasks.Exec
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val repositoryRoot = rootProject.layout.projectDirectory.dir("../..")
val generatedJniRoot = layout.buildDirectory.dir("generated/jniLibs")
val inkAppName = providers.gradleProperty("inkAppName").orElse("Ink")
val inkApplicationId = providers.gradleProperty("inkApplicationId").orElse("com.vandam.ink")
val inkVersionName = providers.gradleProperty("inkVersionName").orElse("0.1.0")
val inkVersionCode = providers.gradleProperty("inkVersionCode").orElse("1")
val inkSigning = providers.gradleProperty("inkSigning").orElse("development")
val inkGeneratedSource = providers.gradleProperty("inkGeneratedSource")
val inkAndroidResources = providers.gradleProperty("inkAndroidResources")
val inkAndroidAssets = providers.gradleProperty("inkAndroidAssets")
val inkUsesLightSdk = providers.gradleProperty("inkUsesLightSdk").orElse("false")
val inkUsesNetwork = providers.gradleProperty("inkUsesNetwork").orElse("false")
val inkUsesAudio = providers.gradleProperty("inkUsesAudio").orElse("false")
val inkUsesAudioPlayback = providers.gradleProperty("inkUsesAudioPlayback").orElse("false")
val inkUsesDetachedAudio = providers.gradleProperty("inkUsesDetachedAudio").orElse("false")
val inkUsesCameraPermission = providers.gradleProperty("inkUsesCameraPermission").orElse("false")
val inkUsesMicrophonePermission = providers.gradleProperty("inkUsesMicrophonePermission").orElse("false")
val inkLightServerPackage = providers.gradleProperty("inkLightServerPackage").orElse("com.lightos")
val inkUsesTextInput = providers.gradleProperty("inkUsesTextInput").orElse("false")
val inkLightSdkVersion = "0.1.1"
val inkPermissionManifest = when {
    inkUsesDetachedAudio.get().toBoolean() && inkUsesCameraPermission.get().toBoolean() &&
        inkUsesMicrophonePermission.get().toBoolean() ->
        "src/detachedCameraAudioPermission/AndroidManifest.xml"
    inkUsesDetachedAudio.get().toBoolean() && inkUsesMicrophonePermission.get().toBoolean() ->
        "src/detachedAudioPermission/AndroidManifest.xml"
    inkUsesDetachedAudio.get().toBoolean() && inkUsesCameraPermission.get().toBoolean() ->
        "src/detachedCameraPermission/AndroidManifest.xml"
    inkUsesDetachedAudio.get().toBoolean() -> "src/detachedPermission/AndroidManifest.xml"
    inkUsesNetwork.get().toBoolean() && inkUsesCameraPermission.get().toBoolean() &&
        inkUsesMicrophonePermission.get().toBoolean() ->
        "src/networkCameraAudioPermission/AndroidManifest.xml"
    inkUsesNetwork.get().toBoolean() && inkUsesMicrophonePermission.get().toBoolean() ->
        "src/networkAudioPermission/AndroidManifest.xml"
    inkUsesCameraPermission.get().toBoolean() && inkUsesMicrophonePermission.get().toBoolean() ->
        "src/cameraAudioPermission/AndroidManifest.xml"
    inkUsesNetwork.get().toBoolean() && inkUsesCameraPermission.get().toBoolean() ->
        "src/networkCameraPermission/AndroidManifest.xml"
    inkUsesMicrophonePermission.get().toBoolean() -> "src/audioPermission/AndroidManifest.xml"
    inkUsesNetwork.get().toBoolean() -> "src/network/AndroidManifest.xml"
    inkUsesCameraPermission.get().toBoolean() -> "src/cameraPermission/AndroidManifest.xml"
    else -> null
}
val inkLightSdkMarkerAction = if (inkUsesLightSdk.get().toBoolean()) {
    "com.thelightphone.sdk.ACTION_SDK_MARKER"
} else {
    "com.vandam.ink.NO_LIGHT_SDK"
}

android {
    namespace = "com.vandam.ink"
    compileSdk = 36
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = inkApplicationId.get()
        minSdk = 34
        targetSdk = 36
        versionCode = inkVersionCode.get().toInt()
        versionName = inkVersionName.get()
        resValue("string", "app_name", inkAppName.get())
        manifestPlaceholders["inkLightSdkEnabled"] = inkUsesLightSdk.get()
        manifestPlaceholders["inkLightSdkMarkerAction"] = inkLightSdkMarkerAction
        manifestPlaceholders["inkLightSdkVersion"] = inkLightSdkVersion
        manifestPlaceholders["inkLightServerPackage"] = inkLightServerPackage.get()
        manifestPlaceholders["inkDetachedAudioEnabled"] = inkUsesDetachedAudio.get()
        buildConfigField(
            "String",
            "INK_LIGHT_SERVER_PACKAGE",
            "\"${inkLightServerPackage.get()}\"",
        )
        buildConfigField("String", "INK_LIGHT_SDK_VERSION", "\"$inkLightSdkVersion\"")
    }

    signingConfigs {
        if (inkSigning.get() == "release") {
            create("inkRelease") {
                storeFile = file(
                    providers.gradleProperty("inkStoreFile").orNull
                        ?: error("Ink release signing requires inkStoreFile"),
                )
                keyAlias = providers.gradleProperty("inkKeyAlias").orNull
                    ?: error("Ink release signing requires inkKeyAlias")
                storePassword = providers.environmentVariable("INK_KEYSTORE_PASSWORD").orNull
                    ?: error("Ink release signing requires INK_KEYSTORE_PASSWORD")
                keyPassword = providers.environmentVariable("INK_KEY_PASSWORD").orElse(
                    providers.environmentVariable("INK_KEYSTORE_PASSWORD"),
                ).orNull ?: error("Ink release signing requires INK_KEY_PASSWORD")
            }
        }
    }

    sourceSets {
        getByName("main").res.srcDir(inkAndroidResources)
        getByName("main").assets.srcDir(inkAndroidAssets)
        if (inkPermissionManifest != null) {
            getByName("debug").manifest.srcFile(inkPermissionManifest)
            getByName("release").manifest.srcFile(inkPermissionManifest)
        }
        getByName("main").java.srcDir(
            if (inkUsesTextInput.get().toBoolean()) {
                "src/textInput/kotlin"
            } else {
                "src/noTextInput/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesAudio.get().toBoolean()) {
                "src/audio/kotlin"
            } else {
                "src/noAudio/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesAudioPlayback.get().toBoolean()) {
                "src/audioPlayback/kotlin"
            } else {
                "src/noAudioPlayback/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesDetachedAudio.get().toBoolean()) {
                "src/audioDetached/kotlin"
            } else {
                "src/noAudioDetached/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesNetwork.get().toBoolean()) {
                "src/network/kotlin"
            } else {
                "src/noNetwork/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesLightSdk.get().toBoolean()) {
                "src/lightSdk/kotlin"
            } else {
                "src/noLightSdk/kotlin"
            },
        )
        if (inkUsesTextInput.get().toBoolean()) {
            getByName("main").res.srcDir("src/textInput/res")
        }
        getByName("debug").jniLibs.srcDir(generatedJniRoot.map { it.dir("debug") })
        getByName("release").jniLibs.srcDir(generatedJniRoot.map { it.dir("release") })
    }

    buildTypes {
        debug {
            ndk {
                abiFilters += "arm64-v8a"
            }
        }

        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            ndk {
                abiFilters += "arm64-v8a"
            }
            if (inkSigning.get() == "release") {
                signingConfig = signingConfigs.getByName("inkRelease")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        buildConfig = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

val cargoBuildDebug by tasks.registering(Exec::class) {
    group = "rust"
    description = "Builds the Ink runtime for the arm64 LP3 emulator."
    workingDir(repositoryRoot)
    commandLine(buildList {
        addAll(
            listOf(
                "cargo",
                "ndk",
                "-t",
                "arm64-v8a",
                "-o",
                generatedJniRoot.get().dir("debug").asFile.absolutePath,
                "build",
                "-p",
                "ink-android",
                "--profile",
                "ink-dev",
            ),
        )
        if (inkUsesNetwork.get().toBoolean()) {
            addAll(listOf("--features", "network"))
        }
        if (inkUsesAudio.get().toBoolean()) {
            addAll(listOf("--features", "audio"))
        }
    })
    environment("INK_APP_RS", inkGeneratedSource.get())
}

val cargoBuildRelease by tasks.registering(Exec::class) {
    group = "rust"
    description = "Builds the Ink runtime for the LP3 arm64 ABI."
    workingDir(repositoryRoot)
    commandLine(buildList {
        addAll(
            listOf(
                "cargo",
                "ndk",
                "-t",
                "arm64-v8a",
                "-o",
                generatedJniRoot.get().dir("release").asFile.absolutePath,
                "build",
                "-p",
                "ink-android",
                "--release",
            ),
        )
        if (inkUsesNetwork.get().toBoolean()) {
            addAll(listOf("--features", "network"))
        }
        if (inkUsesAudio.get().toBoolean()) {
            addAll(listOf("--features", "audio"))
        }
    })
    environment("INK_APP_RS", inkGeneratedSource.get())
}

tasks.configureEach {
    when (name) {
        "mergeDebugJniLibFolders" -> dependsOn(cargoBuildDebug)
        "mergeReleaseJniLibFolders" -> dependsOn(cargoBuildRelease)
    }
}

dependencies {
    implementation("androidx.core:core-splashscreen:1.0.1")
    if (inkUsesAudioPlayback.get().toBoolean()) {
        implementation("androidx.media3:media3-exoplayer:1.10.1")
    }
    if (inkUsesDetachedAudio.get().toBoolean()) {
        implementation("androidx.media3:media3-session:1.10.1")
    }
}
