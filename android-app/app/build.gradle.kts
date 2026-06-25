// ══════════════════════════════════════════════════════════════════════════════
//  :app — BehaviorGuard Demo
//
//  Before building, run the native library build script from the repo root:
//    ./build-android.sh
//  This compiles libbehavior_guard.so for all ABIs and copies it into :lib.
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
    implementation(project(":lib"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
}
