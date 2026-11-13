# build.ps1 - Local release build: frontend + MooDeX.exe + NSIS/MSI installers.
# Usage: powershell -ExecutionPolicy Bypass -File scripts\build.ps1 [-NoBundle]
param([switch]$NoBundle)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

# GNU toolchain hosts need MinGW gcc for the bundled SQLite, ring and libwebp C sources.
$mingw = "C:\msys64\mingw64\bin"
$gnu = (rustup show active-toolchain) -match "gnu"
if ($gnu -and (Test-Path $mingw)) { $env:Path = "$mingw;$env:Path" }

$rel = Join-Path $root "target\release"
# A running MooDeX locks MooDeX.exe and breaks linking/bundling.
if (Get-Process -Name MooDeX -ErrorAction SilentlyContinue) {
    throw "MooDeX is running - quit it from the tray icon (Quit MooDeX), then build again."
}
$exe = Join-Path $rel "MooDeX.exe"
Push-Location (Join-Path $root "app")
try {
    if (-not (Test-Path "node_modules")) { npm ci }
    npx tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }

    # GNU embeds DWARF in the exe; archive it as MooDeX.debug, never ship it (spec 13.2).
    if ($gnu) {
        $full = Join-Path $rel "MooDeX.full.exe"
        Copy-Item $exe $full -Force
        & "$mingw\objcopy.exe" --only-keep-debug $full (Join-Path $rel "MooDeX.debug")
        & "$mingw\objcopy.exe" --strip-debug "--add-gnu-debuglink=$(Join-Path $rel 'MooDeX.debug')" $full $exe
        Remove-Item $full
    }

    if (-not $NoBundle) {
        npx tauri bundle
        if ($LASTEXITCODE -ne 0) { throw "tauri bundle failed" }
    }
} finally { Pop-Location }

if (Test-Path $exe) {
    $h = (Get-FileHash $exe -Algorithm SHA256).Hash
    Write-Host "Built $exe ($([math]::Round((Get-Item $exe).Length / 1MB, 1)) MB)`nSHA-256 $h"
    Write-Warning "Unsigned local build. Do not distribute it (spec 13.8); use the release workflow, which signs."
}
