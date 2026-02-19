# TunnelHub Windows Build Script
# Produces:
#   - TunnelHub-v{VERSION}-Windows.msi          (installer)
#   - TunnelHub-v{VERSION}-Windows-Portable.zip  (portable / green version)
#
# Usage:
#   .\scripts\build-windows.ps1
#   .\scripts\build-windows.ps1 -SkipBuild   # only re-package from existing build

param(
    [switch]$SkipBuild
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
$BundleDir    = Join-Path $ReleaseDir  'bundle'
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
    Write-Host "`n>>> Running: npm run tauri build" -ForegroundColor Yellow
    Push-Location $ProjectRoot
    try {
        npm run tauri -- build
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

# ─── 1) MSI Installer ──────────────────────────────────────
Write-Host "`n>>> Packaging MSI installer..." -ForegroundColor Yellow

$Msi = Get-ChildItem -Path $BundleDir -Recurse -Include '*.msi' -ErrorAction SilentlyContinue |
       Select-Object -First 1

if ($null -ne $Msi) {
    $MsiDest = "$AppName-$Version-Windows.msi"
    Copy-Item $Msi.FullName (Join-Path $OutputDir $MsiDest)
    Write-Host "  MSI: $MsiDest" -ForegroundColor Green
} else {
    Write-Warning "No MSI found under $BundleDir"
}

# ─── 2) NSIS Installer (if available) ──────────────────────
$Nsis = Get-ChildItem -Path $BundleDir -Recurse -Include '*.exe' -ErrorAction SilentlyContinue |
        Where-Object { $_.Directory.Name -eq 'nsis' } |
        Select-Object -First 1

if ($null -ne $Nsis) {
    $NsisDest = "$AppName-$Version-Windows-Setup.exe"
    Copy-Item $Nsis.FullName (Join-Path $OutputDir $NsisDest)
    Write-Host "  NSIS: $NsisDest" -ForegroundColor Green
}

# ─── 3) Portable ZIP ───────────────────────────────────────
Write-Host "`n>>> Packaging Portable ZIP..." -ForegroundColor Yellow

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
    $PortableTempDir = Join-Path $OutputDir "$AppName-Portable"
    New-Item -ItemType Directory -Force -Path $PortableTempDir | Out-Null

    # Copy exe
    Copy-Item $ExePath $PortableTempDir

    # Create portable marker file (cc-switch pattern)
    @(
        "# $AppName portable build marker",
        "portable=true"
    ) | Set-Content -Path (Join-Path $PortableTempDir 'portable.ini') -Encoding UTF8

    # Create the zip
    $PortableZip = "$AppName-$Version-Windows-Portable.zip"
    $PortableZipPath = Join-Path $OutputDir $PortableZip
    Compress-Archive -Path "$PortableTempDir\*" -DestinationPath $PortableZipPath -Force

    # Cleanup temp
    Remove-Item -Recurse -Force $PortableTempDir

    Write-Host "  Portable: $PortableZip" -ForegroundColor Green
} else {
    Write-Warning "No executable found for portable packaging"
}

# ─── Summary ────────────────────────────────────────────────
Write-Host "`n=== Build Complete ===" -ForegroundColor Cyan
Write-Host "Output directory: $OutputDir" -ForegroundColor Cyan
Get-ChildItem $OutputDir | ForEach-Object {
    $size = if ($_.Length -ge 1MB) { "{0:N1} MB" -f ($_.Length / 1MB) }
            else { "{0:N0} KB" -f ($_.Length / 1KB) }
    Write-Host "  $($_.Name)  ($size)" -ForegroundColor White
}
