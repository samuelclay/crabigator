import java.util.Properties
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}
val firebase = Properties().apply {
    rootProject.file("firebase.properties").takeIf { it.exists() }?.inputStream()?.use { load(it) }
}
val releaseStore = providers.environmentVariable("CRABIGATOR_ANDROID_KEYSTORE").orNull
val releasePassword = providers.environmentVariable("CRABIGATOR_ANDROID_STORE_PASSWORD").orNull
android {
    namespace = "com.crabigator.app"
    compileSdk = 36
    defaultConfig {
        applicationId = "com.crabigator.app"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        for (key in listOf("google_app_id", "google_api_key", "gcm_defaultSenderId", "project_id")) {
            resValue("string", key, firebase.getProperty(key, ""))
        }
    }
    signingConfigs {
        create("release") {
            if (releaseStore != null && releasePassword != null) {
                storeFile = file(releaseStore)
                storePassword = releasePassword
                keyAlias = "crabigator-upload"
                keyPassword = releasePassword
            }
        }
    }
    buildTypes {
        getByName("release") { signingConfig = signingConfigs.getByName("release") }
        create("profile") {
            initWith(getByName("release"))
            signingConfig = signingConfigs.getByName("debug")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            matchingFallbacks += "release"
        }
    }
    buildFeatures { compose = true; buildConfig = true }
    // The flow scenes (crates/flow-render), built for each ABI by the task below.
    sourceSets["main"].jniLibs.srcDir(layout.buildDirectory.dir("flowJni"))
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
}
tasks.matching { it.name == "validateSigningRelease" || it.name == "bundleRelease" || it.name == "assembleRelease" }.configureEach {
    doFirst {
        check(!releaseStore.isNullOrBlank() && !releasePassword.isNullOrBlank()) {
            "Set CRABIGATOR_ANDROID_KEYSTORE and CRABIGATOR_ANDROID_STORE_PASSWORD for a signed store build."
        }
        check(listOf("google_app_id", "google_api_key", "gcm_defaultSenderId", "project_id").all { !firebase.getProperty(it).isNullOrBlank() }) {
            "Add the Firebase Android client configuration to firebase.properties before building a store release."
        }
    }
}
// Builds the flow scenes' native library. Needs rustup with the Android targets,
// cargo-ndk, and the NDK (see crates/flow-render/build.sh). Cargo skips the work
// when nothing changed.
val buildFlowScenes by tasks.registering(Exec::class) {
    val out = layout.buildDirectory.dir("flowJni").get().asFile
    commandLine(rootProject.file("../../crates/flow-render/build.sh").path, "android", out.path)
    environment("ANDROID_HOME", android.sdkDirectory.path)
}
tasks.named("preBuild") { dependsOn(buildFlowScenes) }
dependencies {
    implementation("androidx.fragment:fragment-ktx:1.8.9")
    implementation("androidx.activity:activity-compose:1.11.0")
    implementation("androidx.compose.ui:ui:1.9.3")
    "profileImplementation"("androidx.compose.runtime:runtime-tracing:1.9.3")
    implementation("androidx.compose.foundation:foundation:1.9.3")
    implementation("androidx.compose.material3:material3:1.4.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.google.firebase:firebase-messaging:25.0.1")
    implementation("androidx.work:work-runtime-ktx:2.10.5")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4:1.9.3")
    debugImplementation("androidx.compose.ui:ui-test-manifest:1.9.3")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20250517")
}
