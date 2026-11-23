param(
    [switch]$SkipBuild,
    [switch]$Logcat,
    [string]$Serial
)

$ErrorActionPreference = "Stop"
if ($Serial) {
    $env:ANDROID_SERIAL = $Serial
}
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Apk = Join-Path $RepoRoot "app\build\outputs\apk\debug\app-debug.apk"
$Package = "com.vedica.labs.ind.app.talkin.tomiko.debug"
$Activity = "com.vedica.labs.ind.app.talkin.tomiko.game.CharacterGameActivity"
$Component = "$Package/$Activity"

$devices = @(adb devices | Select-String -Pattern "^\S+\s+device$")
if ($devices.Count -eq 0) {
    throw "No Android device connected. Connect an arm64 device with USB debugging enabled."
}

if (-not $SkipBuild) {
    & (Join-Path $PSScriptRoot "build_android.ps1") -Mode debug
    if ($LASTEXITCODE -ne 0) { throw "Build failed." }
}

if (-not (Test-Path $Apk)) {
    throw "APK not found: $Apk"
}

Write-Host "Installing $Apk" -ForegroundColor Cyan
adb install -r $Apk
if ($LASTEXITCODE -ne 0) { throw "adb install failed." }

Write-Host "Launching $Component" -ForegroundColor Cyan
adb shell am force-stop $Package | Out-Null
adb shell am start -n $Component
if ($LASTEXITCODE -ne 0) { throw "adb am start failed." }

Write-Host "Launched. Useful commands:" -ForegroundColor Green
Write-Host "  adb logcat -s Tomiko:* bevy:* wgpu:* AndroidRuntime:E"
Write-Host "  adb shell dumpsys gfxinfo $Package"
Write-Host "  adb shell dumpsys meminfo $Package"

if ($Logcat) {
    adb logcat -c
    adb logcat -s Tomiko:* bevy:* wgpu:* AndroidRuntime:E
}
