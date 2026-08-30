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
val inkUsesTextInput = providers.gradleProperty("inkUsesTextInput").orElse("false")

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
        getByName("main").java.srcDir(
            if (inkUsesTextInput.get().toBoolean()) {
                "src/textInput/kotlin"
            } else {
                "src/noTextInput/kotlin"
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
    commandLine(
        "cargo",
        "ndk",
        "-t",
        "arm64-v8a",
        "-o",
        generatedJniRoot.get().dir("debug").asFile.absolutePath,
        "build",
        "-p",
        "ink-android",
    )
    environment("INK_APP_RS", inkGeneratedSource.get())
}

val cargoBuildRelease by tasks.registering(Exec::class) {
    group = "rust"
    description = "Builds the Ink runtime for the LP3 arm64 ABI."
    workingDir(repositoryRoot)
    commandLine(
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
    )
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
}
