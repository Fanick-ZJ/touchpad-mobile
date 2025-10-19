import org.gradle.internal.declarativedsl.parsing.main
import org.gradle.kotlin.dsl.main
import com.android.build.gradle.api.BaseVariant
import com.google.protobuf.gradle.id
import com.google.protobuf.gradle.proto

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.google.protobuf)
}

android {
    namespace = "com.fanickzz.touchpad"
    compileSdk {
        version = release(36)
    }

    defaultConfig {
        applicationId = "com.fanickzz.touchpad"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    kotlinOptions {
        jvmTarget = "11"
    }
    buildFeatures {
        compose = true
    }

    sourceSets {
        getByName("main") {
            proto {
                srcDir("../../protos")
            }
        }
    }
}

protobuf {
    protoc {
        artifact = "com.google.protobuf:protoc:${libs.versions.googleProtoBuf.get()}"
    }
    plugins {
        // 如果后面要用 gRPC 再打开，目前只用官方 Kotlin 生成器
        id("kotlin") {
            artifact = "com.google.protobuf:protoc-gen-kotlin:${libs.versions.googleProtoBuf.get()}"
        }
        // 没有这个生成不了java文件
        id("javalite") {
            artifact = "com.google.protobuf:protoc-gen-javalite:3.0.0"
        }
    }
    generateProtoTasks {
        all().forEach { task ->
            task.builtins {
                // Java 用 lite
                id("java") {
                    option("lite")
                }
                // Kotlin 也用 lite
                id("kotlin") {
                    option("lite")
                }
            }
        }
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
    debugImplementation(libs.androidx.compose.ui.tooling)
    debugImplementation(libs.androidx.compose.ui.test.manifest)
    implementation(libs.ktor)
    implementation(libs.google.protobuf)
}