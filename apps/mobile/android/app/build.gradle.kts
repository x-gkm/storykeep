plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

// The Nix devshell's Android SDK is read-only, so Gradle can't download the exact
// platform, build-tools and NDK Flutter asks for. Use the newest ones the SDK ships instead.
val androidSdk = System.getenv("ANDROID_HOME")?.let(::File)

fun newestInstalled(dir: String, prefix: String = ""): List<Int>? =
    androidSdk?.resolve(dir)?.list().orEmpty()
        .map { it.removePrefix(prefix).split('.').map(String::toInt) }
        .maxWithOrNull { a, b -> a.zip(b).map { (x, y) -> x.compareTo(y) }.firstOrNull { it != 0 } ?: 0 }

val sdkPlatform = newestInstalled("platforms", prefix = "android-")
val sdkBuildTools = newestInstalled("build-tools")
val sdkNdk = newestInstalled("ndk")

android {
    namespace = "app.storykeep.storykeep"
    if (sdkPlatform != null) {
        compileSdk {
            version = release(sdkPlatform[0]) { minorApiLevel = sdkPlatform.getOrElse(1) { 0 } }
        }
    } else {
        compileSdk = flutter.compileSdkVersion
    }
    sdkBuildTools?.let { buildToolsVersion = it.joinToString(".") }
    ndkVersion = sdkNdk?.joinToString(".") ?: flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "app.storykeep.storykeep"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
