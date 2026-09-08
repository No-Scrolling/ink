plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.ksp)
    alias(libs.plugins.light.sdk)
}

// The benchmark measures startup work, without the SDK's minimum splash duration.
val removeSplashDelay = tasks.register("removeSplashDelay") {
    doLast {
        val activity = project(":sdk:client").file(
            "src/main/kotlin/com/thelightphone/sdk/LightActivity.kt",
        )
        val source = activity.readText()
        val delay = "!contentReady || android.os.SystemClock.elapsedRealtime() - createdAt < 1000"
        check(source.contains(delay) || source.contains("setKeepOnScreenCondition {\n            !contentReady\n")) {
            "LightActivity's splash condition changed; review the benchmark startup setup."
        }
        val updated = source.replace(delay, "!contentReady").replace(
            "    private val createdAt = android.os.SystemClock.elapsedRealtime()\n",
            "",
        )
        if (updated != source) activity.writeText(updated)
    }
}

project(":sdk:client").tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile>().configureEach {
    dependsOn(removeSplashDelay)
}

android {
    compileSdk = rootProject.ext["compileSdk"] as Int

    defaultConfig {
        minSdk = rootProject.ext["minSdk"] as Int
        targetSdk = rootProject.ext["targetSdk"] as Int
        ndk { abiFilters += "arm64-v8a" }
        manifestPlaceholders["sdkVersion"] = property("sdkVersion") as String
    }

    signingConfigs {
        create("benchmark") {
            storeFile = file(System.getProperty("user.home") + "/.android/debug.keystore")
            storePassword = "android"
            keyAlias = "androiddebugkey"
            keyPassword = "android"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
            signingConfig = signingConfigs.getByName("benchmark")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.toVersion(rootProject.ext["jvmTarget"] as String)
        targetCompatibility = JavaVersion.toVersion(rootProject.ext["jvmTarget"] as String)
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.fromTarget(rootProject.ext["jvmTarget"] as String))
    }
}

dependencies {
    implementation(project(":sdk:client"))
}
