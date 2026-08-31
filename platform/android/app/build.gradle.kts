import org.gradle.api.tasks.Exec
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

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
val inkUsesLightSdkRingtone = providers.gradleProperty("inkUsesLightSdkRingtone").orElse("false")
val inkUsesLightSdkPush = providers.gradleProperty("inkUsesLightSdkPush").orElse("false")
val inkUsesNetwork = providers.gradleProperty("inkUsesNetwork").orElse("false")
val inkUsesAudio = providers.gradleProperty("inkUsesAudio").orElse("false")
val inkUsesAudioPlayback = providers.gradleProperty("inkUsesAudioPlayback").orElse("false")
val inkUsesDetachedAudio = providers.gradleProperty("inkUsesDetachedAudio").orElse("false")
val inkUsesCameraPermission = providers.gradleProperty("inkUsesCameraPermission").orElse("false")
val inkUsesPhotoCapture = providers.gradleProperty("inkUsesPhotoCapture").orElse("false")
val inkUsesCodeScanner = providers.gradleProperty("inkUsesCodeScanner").orElse("false")
val inkUsesMicrophonePermission = providers.gradleProperty("inkUsesMicrophonePermission").orElse("false")
val inkUsesLocation = providers.gradleProperty("inkUsesLocation").orElse("false")
val inkUsesNfc = providers.gradleProperty("inkUsesNfc").orElse("false")
val inkUsesBackground = providers.gradleProperty("inkUsesBackground").orElse("false")
val inkUsesNotifications = providers.gradleProperty("inkUsesNotifications").orElse("false")
val inkUsesNotificationPermission = providers.gradleProperty("inkUsesNotificationPermission").orElse("false")
val inkLightServerPackage = providers.gradleProperty("inkLightServerPackage").orElse("com.lightos")
val inkUsesTextInput = providers.gradleProperty("inkUsesTextInput").orElse("false")
val inkLightSdkVersion = "0.1.1"
val inkConditionalSources = providers.provider {
    listOf(
        inkUsesTextInput.get(),
        inkUsesAudio.get(),
        inkUsesAudioPlayback.get(),
        inkUsesDetachedAudio.get(),
        inkUsesNetwork.get(),
        inkUsesLightSdk.get(),
        inkUsesLightSdkRingtone.get(),
        inkUsesLightSdkPush.get(),
        inkUsesLocation.get(),
        inkUsesNfc.get(),
        inkUsesBackground.get(),
        inkUsesNotifications.get(),
    ).joinToString(",")
}
val inkPermissions = buildList {
    if (
        inkUsesNetwork.get().toBoolean() ||
        inkUsesDetachedAudio.get().toBoolean() ||
        inkUsesBackground.get().toBoolean() ||
        inkUsesLightSdkPush.get().toBoolean()
    ) {
        add("android.permission.ACCESS_NETWORK_STATE")
        add("android.permission.INTERNET")
    }
    if (inkUsesDetachedAudio.get().toBoolean()) {
        add("android.permission.FOREGROUND_SERVICE")
        add("android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK")
    }
    if (inkUsesCameraPermission.get().toBoolean()) {
        add("android.permission.CAMERA")
    }
    if (inkUsesMicrophonePermission.get().toBoolean()) {
        add("android.permission.RECORD_AUDIO")
    }
    if (inkUsesLocation.get().toBoolean()) {
        add("android.permission.ACCESS_COARSE_LOCATION")
        add("android.permission.ACCESS_FINE_LOCATION")
    }
    if (inkUsesNfc.get().toBoolean()) {
        add("android.permission.NFC")
    }
    if (inkUsesNotificationPermission.get().toBoolean() || inkUsesLightSdkPush.get().toBoolean()) {
        add("android.permission.POST_NOTIFICATIONS")
    }
    if (inkUsesBackground.get().toBoolean() || inkUsesNotifications.get().toBoolean()) {
        add("android.permission.RECEIVE_BOOT_COMPLETED")
    }
}
val inkFeatures = buildList {
    if (inkUsesCameraPermission.get().toBoolean()) {
        add("android.hardware.camera")
    }
}
val inkPermissionManifest = layout.buildDirectory.file("generated/ink/AndroidManifest.xml")
val generateInkPermissionManifest by tasks.registering {
    inputs.property("permissions", inkPermissions.joinToString())
    inputs.property("features", inkFeatures.joinToString())
    inputs.property("nfc", inkUsesNfc)
    inputs.property("background", inkUsesBackground)
    inputs.property(
        "components",
        listOf(
            inkUsesNotifications.get(),
            inkUsesLightSdkRingtone.get(),
            inkUsesLightSdkPush.get(),
        ).joinToString(","),
    )
    outputs.file(inkPermissionManifest)
    doLast {
        val output = inkPermissionManifest.get().asFile
        output.parentFile.mkdirs()
        output.writeText(buildString {
            appendLine("<?xml version=\"1.0\" encoding=\"utf-8\"?>")
            appendLine("<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">")
            inkPermissions.forEach { permission ->
                appendLine("    <uses-permission android:name=\"$permission\" />")
            }
            inkFeatures.forEach { feature ->
                appendLine("    <uses-feature android:name=\"$feature\" android:required=\"false\" />")
            }
            if (inkUsesNfc.get().toBoolean()) {
                appendLine(
                    "    <uses-feature android:name=\"android.hardware.nfc\" android:required=\"false\" />",
                )
            }
            val hasInkComponents = inkUsesBackground.get().toBoolean() ||
                inkUsesNotifications.get().toBoolean() ||
                inkUsesLightSdkRingtone.get().toBoolean() || inkUsesLightSdkPush.get().toBoolean()
            if (hasInkComponents) {
                val networkSecurity = if (inkUsesLightSdkPush.get().toBoolean()) {
                    " android:networkSecurityConfig=\"@xml/ink_light_push_network_security\""
                } else {
                    ""
                }
                appendLine("    <application$networkSecurity>")
            }
            if (inkUsesBackground.get().toBoolean()) {
                appendLine("        <service")
                appendLine("            android:name=\".InkBackgroundJobService\"")
                appendLine("            android:exported=\"true\"")
                appendLine("            android:permission=\"android.permission.BIND_JOB_SERVICE\" />")
            }
            if (inkUsesNotifications.get().toBoolean()) {
                appendLine("        <receiver android:name=\"com.vandam.ink.InkNotificationAlarmReceiver\" android:exported=\"false\" />")
                appendLine("        <receiver android:name=\"com.vandam.ink.InkNotificationDismissReceiver\" android:exported=\"false\" />")
                appendLine("        <receiver android:name=\"com.vandam.ink.InkNotificationBootReceiver\" android:exported=\"true\">")
                appendLine("            <intent-filter>")
                appendLine("                <action android:name=\"android.intent.action.BOOT_COMPLETED\" />")
                appendLine("            </intent-filter>")
                appendLine("        </receiver>")
            }
            if (inkUsesLightSdkRingtone.get().toBoolean()) {
                appendLine("        <provider android:name=\"com.vandam.ink.InkLightFileProvider\" android:authorities=\"${inkApplicationId.get()}.lightfiles\" android:exported=\"true\" />")
            }
            if (inkUsesLightSdkPush.get().toBoolean()) {
                appendLine("        <receiver android:name=\"com.vandam.ink.InkLightPushReceiver\" android:enabled=\"true\" android:exported=\"true\">")
                appendLine("            <intent-filter>")
                appendLine("                <action android:name=\"org.unifiedpush.android.connector.MESSAGE\" />")
                appendLine("                <action android:name=\"org.unifiedpush.android.connector.NEW_ENDPOINT\" />")
                appendLine("                <action android:name=\"org.unifiedpush.android.connector.UNREGISTERED\" />")
                appendLine("                <action android:name=\"org.unifiedpush.android.connector.REGISTRATION_FAILED\" />")
                appendLine("                <action android:name=\"org.unifiedpush.android.connector.TEMP_UNAVAILABLE\" />")
                appendLine("            </intent-filter>")
                appendLine("        </receiver>")
                appendLine("        <receiver android:name=\"com.vandam.ink.InkLightPushDismissReceiver\" android:exported=\"false\" />")
            }
            if (hasInkComponents) {
                appendLine("    </application>")
            }
            appendLine("</manifest>")
        })
    }
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
        getByName("debug").manifest.srcFile(inkPermissionManifest)
        getByName("release").manifest.srcFile(inkPermissionManifest)
        getByName("main").java.srcDir(
            if (inkUsesTextInput.get().toBoolean()) {
                "src/textInput/kotlin"
            } else {
                "src/noTextInput/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesNotifications.get().toBoolean() || inkUsesLightSdkPush.get().toBoolean()) {
                "src/notifications/kotlin"
            } else {
                "src/noNotifications/kotlin"
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
            if (inkUsesLightSdkRingtone.get().toBoolean()) {
                "src/lightSdkRingtone/kotlin"
            } else {
                "src/noLightSdkRingtone/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesLightSdkPush.get().toBoolean()) {
                "src/lightSdkPush/kotlin"
            } else {
                "src/noLightSdkPush/kotlin"
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
        getByName("main").java.srcDir(
            if (inkUsesLocation.get().toBoolean()) {
                "src/location/kotlin"
            } else {
                "src/noLocation/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesNfc.get().toBoolean()) {
                "src/nfc/kotlin"
            } else {
                "src/noNfc/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesBackground.get().toBoolean()) {
                "src/background/kotlin"
            } else {
                "src/noBackground/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesPhotoCapture.get().toBoolean() || inkUsesCodeScanner.get().toBoolean()) {
                "src/cameraSession/kotlin"
            } else if (inkUsesCameraPermission.get().toBoolean()) {
                "src/cameraPermission/kotlin"
            } else {
                "src/noCamera/kotlin"
            },
        )
        if (inkUsesPhotoCapture.get().toBoolean() || inkUsesCodeScanner.get().toBoolean()) {
            getByName("main").java.srcDir(
                if (inkUsesPhotoCapture.get().toBoolean()) {
                    "src/photoCapture/kotlin"
                } else {
                    "src/noPhotoCapture/kotlin"
                },
            )
            getByName("main").java.srcDir(
                if (inkUsesCodeScanner.get().toBoolean()) {
                    "src/codeScanner/kotlin"
                } else {
                    "src/noCodeScanner/kotlin"
                },
            )
        }
        if (inkUsesTextInput.get().toBoolean()) {
            getByName("main").res.srcDir("src/textInput/res")
        }
        if (inkUsesLightSdkPush.get().toBoolean()) {
            getByName("main").res.srcDir("src/lightSdkPush/res")
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

tasks.withType<KotlinCompile>().configureEach {
    inputs.property("inkConditionalSources", inkConditionalSources)
    val marker = layout.buildDirectory.file("ink-source-features/$name.txt")
    doFirst {
        val file = marker.get().asFile
        val fingerprint = inkConditionalSources.get()
        if (!file.isFile || file.readText() != fingerprint) {
            project.delete(destinationDirectory)
        }
        file.parentFile.mkdirs()
        file.writeText(fingerprint)
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
        if (inkUsesBackground.get().toBoolean()) {
            addAll(listOf("--features", "background"))
        }
        if (inkUsesPhotoCapture.get().toBoolean()) {
            addAll(listOf("--features", "camera-photo"))
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
        if (inkUsesBackground.get().toBoolean()) {
            addAll(listOf("--features", "background"))
        }
        if (inkUsesPhotoCapture.get().toBoolean()) {
            addAll(listOf("--features", "camera-photo"))
        }
    })
    environment("INK_APP_RS", inkGeneratedSource.get())
}

tasks.configureEach {
    when (name) {
        "processDebugMainManifest", "processReleaseMainManifest" ->
            dependsOn(generateInkPermissionManifest)
        "mergeDebugJniLibFolders" -> dependsOn(cargoBuildDebug)
        "mergeReleaseJniLibFolders" -> dependsOn(cargoBuildRelease)
    }
}

dependencies {
    implementation("androidx.core:core-splashscreen:1.0.1")
    if (inkUsesLightSdkPush.get().toBoolean()) {
        implementation("org.unifiedpush.android:connector:3.3.2")
    }
    if (inkUsesAudioPlayback.get().toBoolean()) {
        implementation("androidx.media3:media3-exoplayer:1.10.1")
    }
    if (inkUsesDetachedAudio.get().toBoolean()) {
        implementation("androidx.media3:media3-session:1.10.1")
    }
    if (inkUsesPhotoCapture.get().toBoolean() || inkUsesCodeScanner.get().toBoolean()) {
        implementation("androidx.camera:camera-core:1.5.0")
        implementation("androidx.camera:camera-camera2:1.5.0")
        implementation("androidx.camera:camera-lifecycle:1.5.0")
        implementation("androidx.camera:camera-view:1.5.0")
    }
    if (inkUsesCodeScanner.get().toBoolean()) {
        implementation("com.google.zxing:core:3.5.4")
    }
}
