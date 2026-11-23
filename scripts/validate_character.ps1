param(
    [string]$Path = "assets\characters\tomiko\character.glb"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$fullPath = if ([System.IO.Path]::IsPathRooted($Path)) { $Path } else { Join-Path $RepoRoot $Path }
if (-not (Test-Path $fullPath)) {
    throw "Character GLB not found: $fullPath"
}

$fs = [System.IO.File]::OpenRead($fullPath)
try {
    $br = New-Object System.IO.BinaryReader($fs)
    $magic = $br.ReadUInt32()
    $version = $br.ReadUInt32()
    $length = $br.ReadUInt32()
    $chunkLength = $br.ReadUInt32()
    $chunkType = $br.ReadUInt32()
    $jsonBytes = $br.ReadBytes($chunkLength)
} finally {
    $fs.Close()
}

if ($magic -ne 0x46546C67) { throw "Not a GLB file (bad magic)." }
if ($version -ne 2) { throw "Expected glTF 2.0, got version $version." }

$json = [System.Text.Encoding]::UTF8.GetString($jsonBytes).TrimEnd([char]0x20, [char]0)
$gltf = $json | ConvertFrom-Json

$errors = New-Object System.Collections.Generic.List[string]

$meshCount = if ($gltf.meshes) { $gltf.meshes.Count } else { 0 }
if ($meshCount -ne 1) { $errors.Add("Expected 1 mesh, found $meshCount") }

$skinCount = if ($gltf.skins) { $gltf.skins.Count } else { 0 }
if ($skinCount -ne 1) { $errors.Add("Expected 1 skin, found $skinCount") }

$joints = if ($gltf.skins) { $gltf.skins[0].joints.Count } else { 0 }
if ($joints -lt 20 -or $joints -gt 45) { $errors.Add("Joint count $joints outside 20..45") }

$animations = if ($gltf.animations) { $gltf.animations.Count } else { 0 }
if ($animations -lt 10) { $errors.Add("Expected at least 10 animations, found $animations") }

$primitive = $gltf.meshes[0].primitives[0]
$targets = if ($primitive.targets) { $primitive.targets.Count } else { 0 }
if ($targets -lt 15) { $errors.Add("Expected at least 15 morph targets, found $targets") }

$targetNames = @()
if ($gltf.meshes[0].extras -and $gltf.meshes[0].extras.targetNames) {
    $targetNames = @($gltf.meshes[0].extras.targetNames)
}
$required = @(
    "viseme_AA", "viseme_E", "viseme_I", "viseme_O", "viseme_U",
    "viseme_M", "viseme_F", "viseme_L", "viseme_R", "viseme_S",
    "viseme_SH", "viseme_TH", "viseme_W", "viseme_REST", "viseme_BLINK"
)
foreach ($name in $required) {
    if ($targetNames -notcontains $name) {
        $errors.Add("Missing morph target name: $name")
    }
}

if ($gltf.meshes[0].weights) {
    foreach ($weight in $gltf.meshes[0].weights) {
        if ([math]::Abs([double]$weight) -gt 0.0001) {
            $errors.Add("Default morph weight is nonzero: $weight")
            break
        }
    }
}

$sizeMb = [math]::Round((Get-Item $fullPath).Length / 1MB, 2)
if ($sizeMb -gt 10) { $errors.Add("GLB is ${sizeMb} MB, above the 10 MB limit") }

$clipNames = @()
if ($gltf.animations) { $clipNames = @($gltf.animations | ForEach-Object { $_.name }) }

Write-Host "Character: $fullPath" -ForegroundColor Cyan
Write-Host "  size: ${sizeMb} MB"
Write-Host "  meshes: $meshCount, primitives: $($gltf.meshes[0].primitives.Count)"
Write-Host "  skin joints: $joints"
Write-Host "  morph targets: $targets"
Write-Host "  animations: $animations [$($clipNames -join ', ')]"
Write-Host "  morph names: $($targetNames -join ', ')"

if ($errors.Count -gt 0) {
    Write-Host ""
    Write-Host "Validation failed:" -ForegroundColor Red
    $errors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}

Write-Host ""
Write-Host "Character validation passed." -ForegroundColor Green
exit 0
