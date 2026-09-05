import groovy.json.JsonSlurper
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
val inkAndroidResources = providers.gradleProperty("inkAndroidResources")
val inkAndroidAssets = providers.gradleProperty("inkAndroidAssets")
val inkCapabilitiesManifest = providers.gradleProperty("inkCapabilitiesManifest")
val inkCapabilitiesFile = inkCapabilitiesManifest.map(::file)
val inkBenchmark = providers.environmentVariable("INK_BENCHMARK")
    .map { it == "1" }
    .orElse(false)
val inkBenchmarkRevision = providers.environmentVariable("INK_BENCHMARK_REVISION")
    .orElse("unknown")
val supportedInkCapabilities = setOf(
    "audio",
    "audio-detached",
    "audio-playback",
    "background",
    "barcode-generate",
    "camera-permission",
    "code-scanner",
    "image",
    "light-sdk",
    "light-sdk-push",
    "light-sdk-ringtone",
    "location",
    "microphone-permission",
    "network",
    "nfc",
    "notification-permission",
    "notifications",
    "photo-capture",
    "text-input",
)
val inkCapabilities = providers.provider {
    val source = inkCapabilitiesFile.get()
    if (!source.isFile) {
        error("Ink capability manifest does not exist: $source")
    }
    val manifest = JsonSlurper().parse(source) as? Map<*, *>
        ?: error("Ink capability manifest must be a JSON object")
    if (manifest.keys != setOf("version", "capabilities")) {
        error("Ink capability manifest has unknown fields")
    }
    val version = (manifest["version"] as? Number)?.toInt()
        ?: error("Ink capability manifest version must be a number")
    if (version != 1) {
        error("Ink capability manifest version $version is unsupported; expected 1")
    }
    val values = manifest["capabilities"] as? List<*>
        ?: error("Ink capability manifest capabilities must be a list")
    val names = values.map { value ->
        value as? String ?: error("Ink capabilities must be strings")
    }
    if (names != names.sorted() || names.size != names.distinct().size) {
        error("Ink capabilities must be sorted and unique")
    }
    val unknown = names.filterNot(supportedInkCapabilities::contains)
    if (unknown.isNotEmpty()) {
        error("Unknown Ink capabilities: ${unknown.joinToString(", ")}")
    }
    names.toSet()
}
fun inkUses(capability: String) = inkCapabilities.map { capability in it }
val inkUsesLightSdk = inkUses("light-sdk")
val inkUsesLightSdkRingtone = inkUses("light-sdk-ringtone")
val inkUsesLightSdkPush = inkUses("light-sdk-push")
val inkUsesNetwork = inkUses("network")
val inkUsesAudio = inkUses("audio")
val inkUsesAudioPlayback = inkUses("audio-playback")
val inkUsesDetachedAudio = inkUses("audio-detached")
val inkUsesCameraPermission = inkUses("camera-permission")
val inkUsesPhotoCapture = inkUses("photo-capture")
val inkUsesCodeScanner = inkUses("code-scanner")
val inkUsesBarcodeGenerate = inkUses("barcode-generate")
val inkUsesImage = inkUses("image")
val inkUsesMicrophonePermission = inkUses("microphone-permission")
val inkUsesLocation = inkUses("location")
val inkUsesNfc = inkUses("nfc")
val inkUsesBackground = inkUses("background")
val inkUsesNotifications = inkUses("notifications")
val inkUsesNotificationPermission = inkUses("notification-permission")
val inkLightServerPackage = providers.gradleProperty("inkLightServerPackage").orElse("com.lightos")
val inkUsesTextInput = inkUses("text-input")
val inkLightSdkVersion = "0.1.1"
val inkCapabilityFingerprint = inkCapabilities.map { it.sorted().joinToString(",") }
val inkPermissions = buildList {
    if (
        inkUsesNetwork.get() ||
        inkUsesDetachedAudio.get() ||
        inkUsesBackground.get() ||
        inkUsesLightSdkPush.get()
    ) {
        add("android.permission.ACCESS_NETWORK_STATE")
        add("android.permission.INTERNET")
    }
    if (inkUsesDetachedAudio.get()) {
        add("android.permission.FOREGROUND_SERVICE")
        add("android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK")
    }
    if (inkUsesCameraPermission.get()) {
        add("android.permission.CAMERA")
    }
    if (inkUsesMicrophonePermission.get()) {
        add("android.permission.RECORD_AUDIO")
    }
    if (inkUsesLocation.get()) {
        add("android.permission.FOREGROUND_SERVICE")
        add("android.permission.FOREGROUND_SERVICE_LOCATION")
        add("android.permission.ACCESS_COARSE_LOCATION")
        add("android.permission.ACCESS_FINE_LOCATION")
    }
    if (inkUsesNfc.get()) {
        add("android.permission.NFC")
    }
    if (inkUsesNotificationPermission.get() || inkUsesLightSdkPush.get() || inkUsesLocation.get()) {
        add("android.permission.POST_NOTIFICATIONS")
    }
    if (inkUsesBackground.get() || inkUsesNotifications.get()) {
        add("android.permission.RECEIVE_BOOT_COMPLETED")
    }
    if (inkUsesNotifications.get()) {
        add("android.permission.SCHEDULE_EXACT_ALARM")
    }
}
val inkFeatures = buildList {
    if (inkUsesCameraPermission.get()) {
        add("android.hardware.camera")
    }
}
val inkPermissionManifest = layout.buildDirectory.file("generated/ink/AndroidManifest.xml")
val generateInkPermissionManifest by tasks.registering {
    inputs.file(inkCapabilitiesFile)
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
            if (inkUsesNfc.get()) {
                appendLine(
                    "    <uses-feature android:name=\"android.hardware.nfc\" android:required=\"false\" />",
                )
            }
            val hasInkComponents = inkUsesBackground.get() ||
                inkUsesNotifications.get() ||
                inkUsesLightSdkRingtone.get() || inkUsesLightSdkPush.get() ||
                inkUsesLocation.get() || inkUsesNfc.get() || inkUsesNetwork.get()
            if (hasInkComponents) {
                val networkSecurity = if (inkUsesLightSdkPush.get() || inkUsesNetwork.get()) {
                    " android:networkSecurityConfig=\"@xml/ink_network_security\""
                } else {
                    ""
                }
                appendLine("    <application$networkSecurity>")
            }
            if (inkUsesBackground.get()) {
                appendLine("        <service android:name=\".InkWorkerJobService\" android:exported=\"true\" android:permission=\"android.permission.BIND_JOB_SERVICE\" />")
            }
            if (inkUsesLocation.get()) {
                appendLine("        <service android:name=\"com.vandam.ink.InkLocationService\" android:exported=\"false\" android:foregroundServiceType=\"location\" />")
            }
            if (inkUsesNfc.get()) {
                appendLine("        <service android:name=\".InkHostApduService\" android:exported=\"true\" android:permission=\"android.permission.BIND_NFC_SERVICE\">")
                appendLine("            <intent-filter><action android:name=\"android.nfc.cardemulation.action.HOST_APDU_SERVICE\" /></intent-filter>")
                appendLine("            <meta-data android:name=\"android.nfc.cardemulation.host_apdu_service\" android:resource=\"@xml/ink_host_apdu_service\" />")
                appendLine("        </service>")
            }
            if (inkUsesNotifications.get()) {
                appendLine("        <receiver android:name=\"com.vandam.ink.InkNotificationAlarmReceiver\" android:exported=\"false\" />")
                appendLine("        <receiver android:name=\"com.vandam.ink.InkNotificationDismissReceiver\" android:exported=\"false\" />")
                appendLine("        <receiver android:name=\"com.vandam.ink.InkNotificationBootReceiver\" android:exported=\"true\">")
                appendLine("            <intent-filter>")
                appendLine("                <action android:name=\"android.intent.action.BOOT_COMPLETED\" />")
                appendLine("                <action android:name=\"android.app.action.SCHEDULE_EXACT_ALARM_PERMISSION_STATE_CHANGED\" />")
                appendLine("            </intent-filter>")
                appendLine("        </receiver>")
            }
            if (inkUsesLightSdkRingtone.get()) {
                appendLine("        <provider android:name=\"com.vandam.ink.InkLightFileProvider\" android:authorities=\"${inkApplicationId.get()}.lightfiles\" android:exported=\"true\" />")
            }
            if (inkUsesLightSdkPush.get()) {
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
val inkLightSdkMarkerAction = if (inkUsesLightSdk.get()) {
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
        ndk { abiFilters += "arm64-v8a" }
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
        if (inkUsesNfc.get()) getByName("main").res.srcDir("src/nfc/res")
        getByName("main").assets.srcDir(inkAndroidAssets)
        getByName("debug").manifest.srcFile(inkPermissionManifest)
        getByName("release").manifest.srcFile(inkPermissionManifest)
        getByName("main").java.srcDir(
            if (inkUsesTextInput.get()) {
                "src/textInput/kotlin"
            } else {
                "src/noTextInput/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesNotifications.get() || inkUsesLightSdkPush.get()) {
                "src/notifications/kotlin"
            } else {
                "src/noNotifications/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesAudio.get()) {
                "src/audio/kotlin"
            } else {
                "src/noAudio/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesLightSdkRingtone.get()) {
                "src/lightSdkRingtone/kotlin"
            } else {
                "src/noLightSdkRingtone/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesLightSdkPush.get()) {
                "src/lightSdkPush/kotlin"
            } else {
                "src/noLightSdkPush/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesAudioPlayback.get()) {
                "src/audioPlayback/kotlin"
            } else {
                "src/noAudioPlayback/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesDetachedAudio.get()) {
                "src/audioDetached/kotlin"
            } else {
                "src/noAudioDetached/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesNetwork.get()) {
                "src/network/kotlin"
            } else {
                "src/noNetwork/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesLightSdk.get()) {
                "src/lightSdk/kotlin"
            } else {
                "src/noLightSdk/kotlin"
            },
        )
        getByName("main").java.srcDir(if (inkUsesBarcodeGenerate.get()) "src/barcode/kotlin" else "src/noBarcode/kotlin")
        getByName("main").java.srcDir(
            if (inkUsesLocation.get()) {
                "src/location/kotlin"
            } else {
                "src/noLocation/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesNfc.get()) {
                "src/nfc/kotlin"
            } else {
                "src/noNfc/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesBackground.get()) {
                "src/background/kotlin"
            } else {
                "src/noBackground/kotlin"
            },
        )
        getByName("main").java.srcDir(
            if (inkUsesPhotoCapture.get() || inkUsesCodeScanner.get()) {
                "src/cameraSession/kotlin"
            } else if (inkUsesCameraPermission.get()) {
                "src/cameraPermission/kotlin"
            } else {
                "src/noCamera/kotlin"
            },
        )
        if (inkUsesPhotoCapture.get() || inkUsesCodeScanner.get()) {
            getByName("main").java.srcDir(
                if (inkUsesPhotoCapture.get()) {
                    "src/photoCapture/kotlin"
                } else {
                    "src/noPhotoCapture/kotlin"
                },
            )
            getByName("main").java.srcDir(
                if (inkUsesCodeScanner.get()) {
                    "src/codeScanner/kotlin"
                } else {
                    "src/noCodeScanner/kotlin"
                },
            )
        }
        if (inkUsesTextInput.get()) {
            getByName("main").res.srcDir("src/textInput/res")
        }
        if (inkUsesLightSdkPush.get() || inkUsesNetwork.get()) {
            getByName("main").res.srcDir("src/networkSecurity/res")
        }
        getByName("debug").jniLibs.srcDir(generatedJniRoot.map { it.dir("debug") })
        getByName("release").jniLibs.srcDir(generatedJniRoot.map { it.dir("release") })
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
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
    inputs.property("inkCapabilityFingerprint", inkCapabilityFingerprint)
}

fun registerCargoBuild(variant: String, profile: List<String>) = tasks.register<Exec>(
    "cargoBuild" + variant.replaceFirstChar(Char::uppercaseChar),
) {
    group = "rust"
    description = "Builds the Ink runtime for the LP3 arm64 ABI."
    workingDir(repositoryRoot)
    inputs.property("inkBenchmark", inkBenchmark)
    inputs.property("inkBenchmarkRevision", inkBenchmarkRevision)
    environment("INK_BENCHMARK_REVISION", inkBenchmarkRevision.get())
    commandLine(buildList {
        addAll(listOf(
            "cargo", "ndk", "-t", "arm64-v8a",
            "-o", generatedJniRoot.get().dir(variant).asFile.absolutePath,
            "build", "-p", "ink-android",
        ))
        addAll(profile)
        for ((feature, enabled) in listOf(
            "network" to inkUsesNetwork,
            "image" to inkUsesImage,
            "audio" to inkUsesAudio,
            "background" to inkUsesBackground,
            "camera-photo" to inkUsesPhotoCapture,
            "benchmark" to inkBenchmark,
        )) {
            if (enabled.get()) addAll(listOf("--features", feature))
        }
    })
    doFirst {
        delete(generatedJniRoot.get().dir(variant))
        val prebuilt = android.ndkDirectory.resolve("toolchains/llvm/prebuilt")
            .listFiles()!!.single { it.isDirectory }
        val sysroot = prebuilt.resolve("sysroot")
        environment(
            "BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android",
            "--sysroot=$sysroot -I$sysroot/usr/include/aarch64-linux-android",
        )
    }
}

val cargoBuildDebug = registerCargoBuild("debug", listOf("--profile", "ink-dev"))
val cargoBuildRelease = registerCargoBuild("release", listOf("--release"))

tasks.configureEach {
    when (name) {
        "processDebugMainManifest", "processReleaseMainManifest" ->
            dependsOn(generateInkPermissionManifest)
        "mergeDebugJniLibFolders" -> dependsOn(cargoBuildDebug)
        "mergeReleaseJniLibFolders" -> dependsOn(cargoBuildRelease)
    }
}

dependencies {
    if (inkUsesNetwork.get()) {
        implementation("com.squareup.okhttp3:okhttp:5.4.0")
    }
    implementation("androidx.core:core-splashscreen:1.0.1")
    if (inkUsesLightSdkPush.get()) {
        implementation("org.unifiedpush.android:connector:3.3.2")
    }
    if (inkUsesAudioPlayback.get() || inkUsesDetachedAudio.get()) {
        implementation("androidx.media3:media3-exoplayer:1.10.1")
        implementation("androidx.media3:media3-session:1.10.1")
    }
    if (inkUsesPhotoCapture.get() || inkUsesCodeScanner.get()) {
        implementation("androidx.camera:camera-core:1.5.0")
        implementation("androidx.camera:camera-camera2:1.5.0")
        implementation("androidx.camera:camera-lifecycle:1.5.0")
        implementation("androidx.camera:camera-view:1.5.0")
    }
    if (inkUsesCodeScanner.get() || inkUsesBarcodeGenerate.get()) {
        implementation("com.google.zxing:core:3.5.4")
    }
}
