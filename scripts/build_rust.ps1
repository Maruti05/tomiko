param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "release",
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$RepoRoot = Split-Path -Parent $PSScriptRoot
$RustRoot = Join-Path $RepoRoot "rust"
$BevyApp = Join-Path $RustRoot "crates\bevy_app\Cargo.toml"
# cargo-ndk appends the ABI directory itself, so point it at the jniLibs root
# (not at arm64-v8a) or it produces jniLibs\arm64-v8a\arm64-v8a and Gradle keeps
# packaging a stale library.
$JniDir = Join-Path $RepoRoot "app\src\main\jniLibs"
$AbiDir = Join-Path $JniDir "arm64-v8a"

if (-not (Test-Path $BevyApp)) {
    throw "rust/crates/bevy_app/Cargo.toml does not exist yet. The Bevy native crate is created in Phase 3."
}

$ndkRoot = $env:ANDROID_NDK_HOME
if (-not $ndkRoot) {
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
    $ndkRoot = Join-Path $sdkRoot "ndk\28.2.13676358"
}

if (-not (Test-Path $ndkRoot)) {
    throw "Android NDK 28.2.13676358 not found at $ndkRoot"
}
$env:ANDROID_NDK_HOME = $ndkRoot
$env:ANDROID_NDK_ROOT = $ndkRoot

New-Item -ItemType Directory -Force -Path $AbiDir | Out-Null

if ($Clean) {
    Remove-Item -Recurse -Force (Join-Path $RustRoot "target") -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $JniDir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $AbiDir | Out-Null
}

# Remove the nested layout produced by older script versions. If left behind,
# Gradle can merge the wrong library.
Remove-Item -Recurse -Force (Join-Path $AbiDir "arm64-v8a") -ErrorAction SilentlyContinue

Push-Location $RustRoot
try {
    # AAudio is API 26+; pin the link platform to minSdk so -laaudio resolves.
    $arguments = @(
        "ndk", "-t", "arm64-v8a", "-P", "30", "-o", $JniDir,
        "build", "-p", "tomiko_bevy"
    )
    if ($Profile -eq "release") {
        $arguments += "--release"
    }

    Write-Host "Building Rust native library ($Profile)..." -ForegroundColor Cyan
    # cargo-ndk writes progress to stderr; keep it visible without letting
    # PowerShell's strict error preference abort the script.
    $previousErrorPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    & cargo @arguments 2>&1 | ForEach-Object { Write-Host $_ }
    $cargoExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorPreference
    if ($cargoExitCode -ne 0) {
        throw "cargo ndk failed with exit code $cargoExitCode"
    }
} finally {
    Pop-Location
}

$nativeLib = Join-Path $AbiDir "libtomiko_game.so"
if (-not (Test-Path $nativeLib)) {
    throw "Expected native library was not produced: $nativeLib"
}

# Bevy/wgpu may require libc++_shared.so. Copy it from the pinned NDK if present.
$cxxShared = Join-Path $ndkRoot "toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\aarch64-linux-android\libc++_shared.so"
if (Test-Path $cxxShared) {
    Copy-Item -Force $cxxShared (Join-Path $AbiDir "libc++_shared.so")
    Write-Host "Copied libc++_shared.so" -ForegroundColor DarkGray
} else {
    Write-Warning "libc++_shared.so not found under $ndkRoot; verify whether Bevy needs it before release."
}

$sizeMb = [math]::Round((Get-Item $nativeLib).Length / 1MB, 2)
Write-Host "Native library ready: $nativeLib (${sizeMb} MB)" -ForegroundColor Green
