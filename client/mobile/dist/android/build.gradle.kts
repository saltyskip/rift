plugins {
    id("com.android.library") version "8.11.0"
    kotlin("android") version "2.2.0"
    `maven-publish`
}

android {
    namespace = "ink.riftl.sdk"
    compileSdk = 35

    defaultConfig {
        minSdk = 24
        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86", "x86_64")
        }
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir("jniLibs")
            kotlin.srcDir("kotlin/src")
        }
    }

    buildTypes {
        release {
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

    publishing {
        singleVariant("release") {
            withSourcesJar()
            withJavadocJar()
        }
    }
}

afterEvaluate {
    publishing {
        publications {
            register<MavenPublication>("release") {
                from(components["release"])
                groupId = "ink.riftl"
                artifactId = "sdk"
                version = "0.1.1"

                pom {
                    name.set("Rift SDK")
                    description.set("Rift deep linking SDK for Android")
                }
            }
        }
    }
}

dependencies {
    implementation("org.jetbrains.kotlin:kotlin-stdlib:2.2.0")
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}
