# TunnelHub Windows Build Script
# Produces:
#   - TunnelHub-v{VERSION}-Windows.exe
#
# Usage:
#   .\scripts\build-windows.ps1
#   .\scripts\build-windows.ps1 -SkipBuild
#   .\scripts\build-windows.ps1 -KeepBuildArtifacts

param(
    [switch]$SkipBuild,
    [switch]$KeepBuildArtifacts
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# ─── Resolve paths ──────────────────────────────────────────
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not (Test-Path (Join-Path $ProjectRoot 'src-tauri'))) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$TauriDir     = Join-Path $ProjectRoot 'src-tauri'
$ReleaseDir   = Join-Path $TauriDir    'target\release'
$OutputDir    = Join-Path $ProjectRoot 'release-assets'

# ─── Read version from tauri.conf.json ──────────────────────
$TauriConf = Get-Content (Join-Path $TauriDir 'tauri.conf.json') -Raw | ConvertFrom-Json
$Version   = "v$($TauriConf.version)"
$AppName   = $TauriConf.productName   # "TunnelHub"

Write-Host "=== Building $AppName $Version for Windows ===" -ForegroundColor Cyan

# ─── Ensure cargo is on PATH ────────────────────────────────
$CargoHome = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $CargoHome) {
    $env:PATH = "$CargoHome;$env:PATH"
}

# ─── Build ──────────────────────────────────────────────────
if (-not $SkipBuild) {
    Write-Host "`n>>> Running: npm run tauri -- build --no-bundle" -ForegroundColor Yellow
    Push-Location $ProjectRoot
    try {
        npm run tauri -- build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
} else {
    Write-Host "`n>>> SkipBuild: using existing build artifacts" -ForegroundColor Yellow
}

# ─── Prepare output directory ───────────────────────────────
if (Test-Path $OutputDir) { Remove-Item -Recurse -Force $OutputDir }
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# ─── Package final executable only ──────────────────────────
Write-Host "`n>>> Collecting final executable..." -ForegroundColor Yellow

$ExeName = "$AppName.exe"
$ExeCandidates = @(
    (Join-Path $ReleaseDir $ExeName),
    (Join-Path $ReleaseDir (($AppName.ToLower() -replace ' ','-') + '.exe')),
    (Join-Path $ReleaseDir 'tunnel-hub.exe')
)
$ExePath = $ExeCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($null -eq $ExePath) {
    # Fallback: search for any .exe that looks like our app
    $ExePath = Get-ChildItem -Path $ReleaseDir -Filter '*.exe' -File |
               Where-Object { $_.Name -notmatch 'build-script|xtask' -and $_.Length -gt 1MB } |
               Sort-Object Length -Descending |
               Select-Object -First 1 -ExpandProperty FullName
}

if ($null -ne $ExePath) {
    $FinalExeName = "$AppName-$Version-Windows.exe"
    Copy-Item $ExePath (Join-Path $OutputDir $FinalExeName)
    Write-Host "  EXE: $FinalExeName" -ForegroundColor Green
} else {
    throw "No executable found under $ReleaseDir"
}

# ─── Cleanup intermediate artifacts ─────────────────────────
if (-not $KeepBuildArtifacts) {
    Write-Host "`n>>> Cleaning intermediate build artifacts..." -ForegroundColor Yellow
    $CleanupPaths = @(
        (Join-Path $ProjectRoot 'dist'),
        (Join-Path $TauriDir 'target')
    )

    foreach ($Path in $CleanupPaths) {
        if (Test-Path $Path) {
            Remove-Item -Recurse -Force $Path
            Write-Host "  Removed: $Path" -ForegroundColor DarkGray
        }
    }
}

# ─── Summary ────────────────────────────────────────────────
Write-Host "`n=== Build Complete ===" -ForegroundColor Cyan
Write-Host "Output directory: $OutputDir" -ForegroundColor Cyan
Get-ChildItem $OutputDir | ForEach-Object {
    $size = if ($_.Length -ge 1MB) { "{0:N1} MB" -f ($_.Length / 1MB) }
            else { "{0:N0} KB" -f ($_.Length / 1KB) }
    Write-Host "  $($_.Name)  ($size)" -ForegroundColor White
}
