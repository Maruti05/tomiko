param(
    [string]$Blender = ""
)

$ErrorActionPreference = "Stop"
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Script = Join-Path $RepoRoot "tools\blender\create_tomiko.py"

if (-not $Blender) {
    $Blender = (Get-Command blender -ErrorAction SilentlyContinue).Source
}
if (-not $Blender) {
    $candidate = "C:\Program Files\Blender Foundation\Blender 5.3\blender.exe"
    if (Test-Path $candidate) { $Blender = $candidate }
}
if (-not $Blender -or -not (Test-Path $Blender)) {
    throw "Blender was not found. Install Blender 5.3 or pass -Blender <path>."
}

Write-Host "Generating Tomiko character with $Blender" -ForegroundColor Cyan
& $Blender --background --python $Script 2>&1 | ForEach-Object { Write-Host $_ }
if ($LASTEXITCODE -ne 0) {
    throw "Blender character generation failed with exit code $LASTEXITCODE"
}

& (Join-Path $PSScriptRoot "validate_character.ps1")
if ($LASTEXITCODE -ne 0) {
    throw "Character validation failed."
}

Write-Host "Character generation complete." -ForegroundColor Green
