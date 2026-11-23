param(
    [switch]$RequireDevice
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Issues = New-Object System.Collections.Generic.List[string]
$Warnings = New-Object System.Collections.Generic.List[string]

function Write-Section([string]$Title) {
    Write-Host ""
    Write-Host "== $Title ==" -ForegroundColor Cyan
}

function Test-Command([string]$Name, [string]$VersionCommand) {
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) {
        $Issues.Add("Missing command: $Name")
        Write-Host "  [MISSING] $Name" -ForegroundColor Red
        return
    }
    try {
        $version = Invoke-Expression $VersionCommand 2>&1 | Select-Object -First 1
        Write-Host "  [OK] $Name ($version)" -ForegroundColor Green
    } catch {
        Write-Host "  [OK] $Name" -ForegroundColor Green
    }
}

Write-Host "Tomiko environment verification" -ForegroundColor Magenta
Write-Host "Repository: $RepoRoot"

Write-Section "Host toolchain"
Test-Command "java" "java -version"
Test-Command "rustc" "rustc --version"
Test-Command "cargo" "cargo --version"
Test-Command "cargo-ndk" "cargo ndk --version"
Test-Command "adb" "adb version"

Write-Section "Android SDK"
$sdkRoot = $env:ANDROID_SDK_ROOT
if (-not $sdkRoot) { $sdkRoot = $env:ANDROID_HOME }
if (-not $sdkRoot) {
    $localProperties = Join-Path $RepoRoot "local.properties"
    if (Test-Path $localProperties) {
        $line = Get-Content $localProperties | Where-Object { $_ -match '^sdk\.dir=' } | Select-Object -First 1
        if ($line) {
            $sdkRoot = $line.Substring("sdk.dir=".Length).Replace("\:", ":").Replace("\\", "\")
        }
    }
}
if (-not $sdkRoot) { $sdkRoot = Join-Path $env:LOCALAPPDATA "Android\Sdk" }

if (Test-Path $sdkRoot) {
    Write-Host "  [OK] SDK: $sdkRoot" -ForegroundColor Green
} else {
    $Issues.Add("Android SDK not found at $sdkRoot")
    Write-Host "  [MISSING] SDK: $sdkRoot" -ForegroundColor Red
}

$platform = Join-Path $sdkRoot "platforms\android-37.1"
if (Test-Path $platform) {
    Write-Host "  [OK] Platform: android-37.1" -ForegroundColor Green
} else {
    $Issues.Add("Missing SDK platform android-37.1 (required by Compose 1.13)")
    Write-Host "  [MISSING] Platform: android-37.1" -ForegroundColor Red
}

$ndkRoot = Join-Path $sdkRoot "ndk\28.2.13676358"
if (Test-Path $ndkRoot) {
    Write-Host "  [OK] NDK: 28.2.13676358" -ForegroundColor Green
} else {
    $Issues.Add("Missing NDK 28.2.13676358")
    Write-Host "  [MISSING] NDK: 28.2.13676358" -ForegroundColor Red
}

Write-Section "Rust Android target"
$targets = rustup target list --installed 2>&1
if ($targets -match "aarch64-linux-android") {
    Write-Host "  [OK] aarch64-linux-android" -ForegroundColor Green
} else {
    $Issues.Add("Rust target aarch64-linux-android is not installed")
    Write-Host "  [MISSING] aarch64-linux-android" -ForegroundColor Red
}

Write-Section "Project"
$wrapperProperties = Join-Path $RepoRoot "gradle\wrapper\gradle-wrapper.properties"
if (Test-Path $wrapperProperties) {
    $distribution = Get-Content $wrapperProperties | Where-Object { $_ -match '^distributionUrl=' } | Select-Object -First 1
    Write-Host "  [OK] $distribution" -ForegroundColor Green
} else {
    $Issues.Add("Gradle wrapper properties missing")
    Write-Host "  [MISSING] Gradle wrapper" -ForegroundColor Red
}

$bevyApp = Join-Path $RepoRoot "rust\crates\bevy_app\Cargo.toml"
if (Test-Path $bevyApp) {
    Write-Host "  [OK] rust/crates/bevy_app" -ForegroundColor Green
} else {
    $Warnings.Add("rust/crates/bevy_app is not created yet (Phase 3)")
    Write-Host "  [PENDING] rust/crates/bevy_app (Phase 3)" -ForegroundColor Yellow
}

$blender = Get-Command blender -ErrorAction SilentlyContinue
if ($blender) {
    Write-Host "  [OK] Blender: $($blender.Source)" -ForegroundColor Green
} else {
    $Warnings.Add("Blender not found; required for the original character asset (Phase 4)")
    Write-Host "  [OPTIONAL] Blender not found (Phase 4)" -ForegroundColor Yellow
}

Write-Section "Device"
$devices = @()
try {
    $devices = (adb devices | Select-String -Pattern "^\S+\s+device$")
} catch {
    $devices = @()
}
if ($devices.Count -gt 0) {
    Write-Host "  [OK] $($devices.Count) device(s) connected" -ForegroundColor Green
    $devices | ForEach-Object { Write-Host "       $_" }
} else {
    if ($RequireDevice) {
        $Issues.Add("No Android device connected")
    } else {
        $Warnings.Add("No Android device connected; audio/render verification is blocked")
    }
    Write-Host "  [WARN] No Android device connected" -ForegroundColor Yellow
}

Write-Section "Disk"
$drive = Get-PSDrive -Name ($RepoRoot.Substring(0, 1))
$freeGb = [math]::Round($drive.Free / 1GB, 1)
if ($freeGb -lt 20) {
    $Warnings.Add("Low free disk space: ${freeGb} GB")
    Write-Host "  [WARN] ${freeGb} GB free (Bevy builds need headroom)" -ForegroundColor Yellow
} else {
    Write-Host "  [OK] ${freeGb} GB free" -ForegroundColor Green
}

Write-Section "Summary"
if ($Warnings.Count -gt 0) {
    Write-Host "Warnings:" -ForegroundColor Yellow
    $Warnings | ForEach-Object { Write-Host "  - $_" -ForegroundColor Yellow }
}
if ($Issues.Count -gt 0) {
    Write-Host "Blocking issues:" -ForegroundColor Red
    $Issues | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host "Environment is ready for the current phase." -ForegroundColor Green
exit 0
