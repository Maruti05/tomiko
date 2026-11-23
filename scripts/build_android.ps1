param(
    [ValidateSet("debug", "release", "bundle")]
    [string]$Mode = "debug",
    [switch]$SkipRust,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$RepoRoot = Split-Path -Parent $PSScriptRoot

if (-not $SkipRust) {
    $rustProfile = if ($Mode -eq "debug") { "debug" } else { "release" }
    & (Join-Path $PSScriptRoot "build_rust.ps1") -Profile $rustProfile -Clean:$Clean
    if ($LASTEXITCODE -ne 0) {
        throw "Rust build failed."
    }
}

Push-Location $RepoRoot
try {
    if ($Clean) {
        & .\gradlew.bat clean
        if ($LASTEXITCODE -ne 0) { throw "Gradle clean failed." }
    }

    switch ($Mode) {
        "debug" { $task = ":app:assembleDebug" }
        "release" { $task = ":app:assembleRelease" }
        "bundle" { $task = ":app:bundleRelease" }
    }

    Write-Host "Running $task" -ForegroundColor Cyan
    & .\gradlew.bat $task
    if ($LASTEXITCODE -ne 0) {
        throw "Gradle task $task failed."
    }
} finally {
    Pop-Location
}

Write-Host "Android build complete ($Mode)." -ForegroundColor Green
