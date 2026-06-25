// ══════════════════════════════════════════════════════════════════════════════
//  app/build.gradle.kts — BehaviorGuard Demo
// ══════════════════════════════════════════════════════════════════════════════
//
//  INTEGRATION NOTES
//  ─────────────────
//  1. Build the native library before opening this project in Android Studio:
//
//       cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
//           -o android-app/app/src/main/jniLibs \
//           build --release --features jni
//
//     The output .so goes to:
//       app/src/main/jniLibs/arm64-v8a/libbehavior_guard.so
//       app/src/main/jniLibs/armeabi-v7a/libbehavior_guard.so
//       app/src/main/jniLibs/x86_64/libbehavior_guard.so
//
//  2. Copy the Kotlin wrapper alongside the demo Activity:
//
//       cp android/BehaviorGuard.kt \
//          android-app/app/src/main/java/com/example/behaviorgaurd/BehaviorGuard.kt
//
//  3. ProGuard / R8: see proguard-rules.pro.
//
// ══════════════════════════════════════════════════════════════════════════════

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace  = "com.example.behaviorgaurd.demo"
    compileSdk = 35

    defaultConfig {
        applicationId   = "com.example.behaviorgaurd.demo"
        minSdk          = 24
        targetSdk       = 35
        versionCode     = 1
        versionName     = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    splits {
        abi {
            isEnable       = true
            reset()
            include("arm64-v8a", "armeabi-v7a", "x86_64")
            isUniversalApk = false
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
}
