buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        // AGP 9 has built-in Kotlin, but the KGP version on the classpath
        // determines which Kotlin compiler is used.
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:2.4.21")
    }
}

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.kotlin.serialization) apply false
}
