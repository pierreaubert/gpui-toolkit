plugins {
    id("com.android.application")
}

// Release signing is env-driven so no secrets live in the repo:
//   GPUI_ANDROID_KEYSTORE / _KEYSTORE_PASSWORD / _KEY_ALIAS / _KEY_PASSWORD
// Without them, release builds stay unsigned (CI and local checks).
val gpuiKeystore: String? = System.getenv("GPUI_ANDROID_KEYSTORE")

android {
    namespace = "org.spinorama.gpui.pythonmobile"
    compileSdk = 36

    defaultConfig {
        applicationId = "org.spinorama.gpui.pythonmobile"
        minSdk = 26
        targetSdk = 36
        versionCode = (providers.gradleProperty("gpuiVersionCode").orNull ?: "1").toInt()
        versionName = providers.gradleProperty("gpuiVersionName").orNull ?: "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        manifestPlaceholders["nativeLibraryName"] = "python_mobile_android"
    }

    signingConfigs {
        if (gpuiKeystore != null) {
            create("release-env") {
                storeFile = file(gpuiKeystore)
                storePassword = System.getenv("GPUI_ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("GPUI_ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("GPUI_ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
        release {
            isMinifyEnabled = false
            if (gpuiKeystore != null) {
                signingConfig = signingConfigs.getByName("release-env")
            }
        }
    }

    androidResources {
        // The staged CPython tree is fully controlled by
        // stage-android-resources.sh; AGP's default ignore pattern would
        // silently drop stdlib dirs like `_pyrepl/` and `__phello__/`.
        ignoreAssetsPattern = ""
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
            // Java host (GpuiActivity et al.) is shared with every GPUI
            // Android app; the relative path reaches gpui-android's copy.
            java.srcDir("../../../../../gpui-android/android/src/main/java")
        }
    }

    packaging {
        jniLibs {
            keepDebugSymbols += listOf("*/arm64-v8a/libpython_mobile_android.so")
        }
    }
}
