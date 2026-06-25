// ══════════════════════════════════════════════════════════════════════════════
//  :lib — BehaviorGuard Android library
//
//  Produces behavior-guard-<version>.aar containing:
//    • BehaviorGuard.kt + BehaviorGuardManager.kt compiled classes
//    • libbehavior_guard.so for arm64-v8a, armeabi-v7a, x86_64
//    • consumer-rules.pro forwarded to consuming apps
//
//  Build the native .so before assembling the AAR:
//
//    cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
//        -o android-app/lib/src/main/jniLibs \
//        build --release --features jni
//
//  Publish to Maven Local:
//    ./gradlew :lib:publishToMavenLocal
//
//  Publish to a remote repository:
//    Add `MAVEN_*` credentials to local.properties and run:
//    ./gradlew :lib:publish
// ══════════════════════════════════════════════════════════════════════════════

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    `maven-publish`
}

android {
    namespace  = "com.behaviorgaurd"
    compileSdk = 35

    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false  // minification done by consuming app
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.security.crypto)
}

// ── Maven publish ─────────────────────────────────────────────────────────────

afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("release") {
                from(components["release"])

                groupId    = "com.behaviorgaurd"
                artifactId = "behavior-guard"
                version    = "0.1.0"

                pom {
                    name        = "BehaviorGuard"
                    description = "On-device behavioral biometrics for Android. " +
                            "Models typing rhythm, touch patterns, and device motion " +
                            "to score user authenticity — entirely on-device."
                    url         = "https://github.com/rukmaldias/BehaviorGuard"
                    licenses {
                        license {
                            name = "GPL-3.0-only"
                            url  = "https://www.gnu.org/licenses/gpl-3.0.html"
                        }
                    }
                    developers {
                        developer {
                            id   = "rukmaldias"
                            name = "Rukmal Dias"
                        }
                    }
                    scm {
                        url = "https://github.com/rukmaldias/BehaviorGuard"
                    }
                }
            }
        }

        repositories {
            // `./gradlew :lib:publishToMavenLocal` installs to ~/.m2/
            maven { url = uri("${System.getProperty("user.home")}/.m2/repository") }
        }
    }
}
