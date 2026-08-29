plugins {
    id("com.android.application")
}

android {
    namespace = "org.spinorama.gpui.showcase"
    compileSdk = 35

    defaultConfig {
        applicationId = "org.spinorama.gpui.showcase"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }

        manifestPlaceholders["nativeLibraryName"] = "showcase_android"
    }

    buildTypes {
        debug {
            isDebuggable = true
            isJniDebuggable = true
        }
        release {
            isMinifyEnabled = false
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
            java.srcDir("../../../../gpui-android/android/src/main/java")
        }
    }

    packaging {
        jniLibs {
            keepDebugSymbols += listOf("*/arm64-v8a/libshowcase_android.so")
        }
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}
