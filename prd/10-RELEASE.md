# Tomiko — Play Store Release Checklist

## 1. Pre-Release Checklist

### 1.1 Code & Build

- [ ] Version bump: update `versionCode` and `versionName` in `app/build.gradle.kts`
- [ ] `buildConfig` fields final: `BUILD_TYPE = "release"`, `DEBUG = false`
- [ ] `minifyEnabled = true`, `shrinkResources = true` in release build type
- [ ] R8 full mode enabled: `android.enableR8.fullMode = true` in `gradle.properties`
- [ ] No `Log.d/e/w` calls in release builds (use `if (BuildConfig.DEBUG)` guard)
- [ ] ProGuard mapping file saved for crash deobfuscation
- [ ] Build Android App Bundle (`./gradlew bundleRelease`)
- [ ] Universal APK generated for local testing (`./gradlew assembleRelease`)
- [ ] App bundle size verified: `≤ 40 MB`

### 1.2 Signing

- [ ] App signing key generated and stored securely (not in repo)
- [ ] `signingConfigs` in `build.gradle.kts` references keystore via environment variables:
  ```kotlin
  signingConfigs {
      create("release") {
          storeFile = file(System.getenv("TOMIKO_STORE_FILE") ?: "")
          storePassword = System.getenv("TOMIKO_STORE_PASSWORD")
          keyAlias = System.getenv("TOMIKO_KEY_ALIAS")
          keyPassword = System.getenv("TOMIKO_KEY_PASSWORD")
      }
  }
  ```
- [ ] Play App Signing enabled in Play Console (preferred)

### 1.3 Assets & Store Listing

- [ ] App icon: 512×512 px adaptive icon (foreground + background layers)
- [ ] Feature graphic: 1024×500 px (JPEG 24-bit, no text in bottom 10 %)
- [ ] Phone screenshots (minimum 2): 1080×1920 or 1080×2340 px
- [ ] Tablet screenshots: 1920×1080 px (landscape), 1080×1920 px (portrait)
- [ ] App description: COPPA/GDPR-K compliant, no "AI" or "cloud" claims
- [ ] Content rating: submit IARC questionnaire → "Everyone"
- [ ] Privacy policy: explicitly state "No data collection, no internet access"
- [ ] App category: "Educational" or "Fun"

---

## 2. Manifest Final Checks

```xml
<!-- AndroidManifest.xml release checks -->
<uses-permission android:name="android.permission.RECORD_AUDIO"
    android:maxSdkVersion="36" />
<!-- No INTERNET permission -->
<!-- No ACCESS_NETWORK_STATE permission -->
<!-- No AD_ID permission (remove if present) -->
<uses-feature android:name="android.hardware.microphone"
    android:required="true" />
<uses-feature android:name="android.hardware.opengles"
    android:version="0x00030000" android:required="true" />
```

### Manifest Removals Checklist

- [ ] Remove `android.permission.INTERNET` (rejected: kids app without internet is safer)
- [ ] Remove `android.permission.ACCESS_NETWORK_STATE`
- [ ] Remove any `<queries>` block for browser / app launch intents
- [ ] Ensure `android:networkSecurityConfig` blocks ALL cleartext and HTTPS (optional)

---

## 3. Safety & Privacy Declarations

### For Play Console "Data Safety" section:

| Question | Answer |
|---|---|
| Does your app collect data? | **No** |
| Is data encrypted? | N/A |
| Is data shared with third parties? | **No** |
| Does it require account creation? | **No** |
| Target audience | **Children (under 13)** |
| COPPA compliance | **Yes** — zero data collection, zero ads |

---

## 4. Post-Release Monitoring

Since no analytics / crash reporting is included:

| Monitoring Method | Implementation |
|---|---|
| **Play Console ANR & Crashes** | Android vitals (opt-out not possible — Google collects anonymously) |
| **Manual dogfooding** | Internal test track with trusted testers |
| **User reviews** | Respond within 48 hours; fix crash reports within 1 week |
| **No logs or telemetry** | By design — privacy first |

---

## 5. Versioning Strategy

```kotlin
// app/build.gradle.kts
android {
    defaultConfig {
        versionCode = 1
        versionName = "1.0.0"
    }
}
```

| Release Type | Version Bump | Example |
|---|---|---|
| Initial release | 1.0.0 (code 1) | — |
| Minor bug fix | 1.0.1 (code 2) | Crash fix, performance |
| Feature addition | 1.1.0 (code 3) | New character, new effect |
| Major update | 2.0.0 (code 4) | Rewrite, new architecture |

---

## 6. Build Command Reference

```bash
# Clean build
./gradlew clean

# Build release app bundle (for Play Store)
./gradlew :app:bundleRelease

# Build release universal APK (for local testing)
./gradlew :app:assembleRelease

# Check APK size
./gradlew :app:assembleRelease && `
    Get-ChildItem app/build/outputs/apk/release/*.apk | `
    Select-Object Name, @{N="SizeMB";E={$_.Length / 1MB}}

# Verify no internet permission in final binary
./gradlew :app:bundleRelease
aapt2 dump permissions app/build/outputs/bundle/release/app.aab
# Should NOT show: android.permission.INTERNET
```
