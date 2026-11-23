param(
    [switch]$NoGradle
)

$ErrorActionPreference = "Stop"
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$RepoRoot = Split-Path -Parent $PSScriptRoot

$paths = @(
    (Join-Path $RepoRoot "build"),
    (Join-Path $RepoRoot "app\build"),
    (Join-Path $RepoRoot "app\.cxx"),
    (Join-Path $RepoRoot "rust\target"),
    (Join-Path $RepoRoot "app\src\main\jniLibs")
)

foreach ($path in $paths) {
    if (Test-Path $path) {
        Write-Host "Removing $path" -ForegroundColor DarkGray
        Remove-Item -Recurse -Force $path
    }
}

if (-not $NoGradle) {
    Push-Location $RepoRoot
    try {
        & .\gradlew.bat clean
    } finally {
        Pop-Location
    }
}

Write-Host "Clean complete." -ForegroundColor Green
