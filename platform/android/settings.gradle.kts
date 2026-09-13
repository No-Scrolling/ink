pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "InkCounter"

include(":app")

providers.gradleProperty("inkAppAndroid").orNull?.let { path ->
    if (file("$path/build.gradle.kts").isFile) {
        include(":app-native")
        project(":app-native").projectDir = file(path)
    }
}
