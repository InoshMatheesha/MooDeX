# build.ps1 - Local release build: frontend + MooDeX.exe + NSIS/MSI installers (+ signed updater feed).
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
# A MooDeX running from the build output folder locks MooDeX.exe and breaks linking/bundling.
$locked = Get-Process -Name MooDeX -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($rel, [StringComparison]::OrdinalIgnoreCase) }
if ($locked) {
    throw "MooDeX is running - quit it from the tray icon (Quit MooDeX), then build again."
}
$exe = Join-Path $rel "MooDeX.exe"
$conf = Get-Content (Join-Path $root "app\src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
$version = $conf.version

# Updater packages are minisign-signed with the private key kept outside the repo (spec 13.7).
$keyFile = Join-Path $env:USERPROFILE ".tauri\moodex.key"
$bundleArgs = @()
if (Test-Path $keyFile) {
    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $keyFile -Raw
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "" }
} else {
    Write-Warning "No updater key at $keyFile - building without updater packages."
    $bundleArgs = @("--config", '{"bundle":{"createUpdaterArtifacts":false}}')
}

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
        npx tauri bundle @bundleArgs
        if ($LASTEXITCODE -ne 0) { throw "tauri bundle failed" }
    }
} finally { Pop-Location }

# Updater feed for GitHub Releases: latest.json points at the signed NSIS installer.
$setup = Join-Path $rel "bundle\nsis\MooDeX_${version}_x64-setup.exe"
$sig = "$setup.sig"
if (Test-Path $sig) {
    $changelog = Get-Content (Join-Path $root "CHANGELOG.md") -Raw
    $m = [regex]::Match($changelog, "(?s)## $([regex]::Escape($version))[^\n]*\n(.*?)(?=\n## |\z)")
    $notes = if ($m.Success) { $m.Groups[1].Value.Trim() } else { "MooDeX $version" }
    $url = "https://github.com/InoshMatheesha/MooDeX/releases/download/v$version/MooDeX_${version}_x64-setup.exe"
    $entry = @{ signature = (Get-Content $sig -Raw).Trim(); url = $url }
    $feed = [ordered]@{
        version   = $version
        notes     = $notes
        pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        platforms = [ordered]@{ "windows-x86_64" = $entry; "windows-x86_64-nsis" = $entry }
    }
    $feedPath = Join-Path $rel "bundle\latest.json"
    # No BOM: the updater parses this with a strict JSON reader.
    [IO.File]::WriteAllText($feedPath, ($feed | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
    Write-Host "Updater feed: $feedPath"
}

if (Test-Path $exe) {
    $h = (Get-FileHash $exe -Algorithm SHA256).Hash
    Write-Host "Built $exe ($([math]::Round((Get-Item $exe).Length / 1MB, 1)) MB)`nSHA-256 $h"
    Write-Warning "Not Authenticode-signed: Windows SmartScreen may warn until a code-signing certificate is used (spec 13.5)."
}
