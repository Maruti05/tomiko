plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

android {
    namespace = "com.vedica.labs.ind.app.talkin.tomiko"
    compileSdk = 37
    compileSdkMinor = 1
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "com.vedica.labs.ind.app.talkin.tomiko"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    signingConfigs {
        val keystore = System.getenv("TOMIKO_KEYSTORE")
        if (!keystore.isNullOrBlank()) {
            create("release") {
                storeFile = file(keystore)
                storePassword = System.getenv("TOMIKO_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("TOMIKO_KEY_ALIAS")
                keyPassword = System.getenv("TOMIKO_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.findByName("release")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isMinifyEnabled = false
            applicationIdSuffix = ".debug"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    sourceSets {
        getByName("main") {
            // Repository-level assets are the source of truth for characters,
            // environments, audio, and fonts. Existing app assets remain until
            // the legacy SceneView assets are migrated in Phase 3.
            assets.srcDir(rootProject.file("assets"))
        }
    }

    androidResources {
        noCompress += listOf("glb", "gltf", "bin", "ktx2", "wav", "ogg")
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
        jniLibs {
            useLegacyPackaging = false
        }
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.compose.ui)
    implementation(libs.compose.ui.graphics)
    implementation(libs.compose.ui.tooling.preview)
    implementation(libs.compose.foundation)
    implementation(libs.compose.animation)
    implementation(libs.compose.material.icons.core)
    implementation(libs.activity.compose)
    implementation(libs.appcompat)
    implementation(libs.window)
    implementation(libs.window.core)
    implementation(libs.navigation.compose)
    implementation(libs.coroutines.core)
    implementation(libs.coroutines.android)
    implementation(libs.serialization.json)
    implementation(libs.core.ktx)
    implementation(libs.lifecycle.viewmodel.compose)
    implementation(libs.lifecycle.runtime.compose)
    implementation(libs.datastore.preferences)
    implementation(libs.games.activity)

    testImplementation(libs.junit)
    testImplementation(libs.turbine)
    testImplementation(libs.coroutines.test)

    androidTestImplementation(platform(libs.compose.bom))
    androidTestImplementation(libs.compose.ui.test)
    androidTestImplementation(libs.junit)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.junit)

    debugImplementation(libs.compose.ui.tooling)
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
