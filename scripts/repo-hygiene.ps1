# repo-hygiene.ps1 - Fail if build output, installers, secrets or large blobs are tracked (spec 13.1).
$ErrorActionPreference = "Stop"
$bad = @()
$patterns = '(^|/)(target|dist|node_modules|\.vs|\.vercel)/|\.(exe|msi|pdb|dll|zip|7z)$|(^|/)\.env'
$files = git ls-files
foreach ($f in $files) {
    if ($f -match $patterns) { $bad += "artifact: $f" }
    if (Test-Path -LiteralPath $f) {
        $size = (Get-Item -LiteralPath $f).Length
        if ($size -gt 2MB) { $bad += "too large ($([math]::Round($size / 1MB, 1)) MB): $f" }
    }
}
# Known-compromised v1 key must never reappear.
$hits = git grep -n -I -E "3e485ee2[0-9a-f]{24}" -- . ':!scripts/repo-hygiene.ps1' 2>$null
if ($hits) { $bad += "v1 RAWG key found:`n$hits" }
if ($bad.Count -gt 0) {
    $bad | ForEach-Object { Write-Host "::error::$_" }
    exit 1
}
Write-Host "repo hygiene OK ($($files.Count) tracked files)"
