# Builds the Microsoft Store package (MSIX) from packaging\msix. See packaging\msix\README.md.
#
#   Store upload (identity values from Partner Center > Product identity):
#   powershell -ExecutionPolicy Bypass -File scripts\store-package.ps1 `
#     -IdentityName <Package/Identity/Name> -Publisher <CN=...> -PublisherDisplayName <name>
#
#   Local test package (runs beside an installed copy with its own settings and History):
#   powershell -ExecutionPolicy Bypass -File scripts\store-package.ps1 -Test
#
# Output: dist\store\<version>\ with the .msix and the unpacked layout. The Store signs the
# upload. A -Test layout is installed for trying out with
#   Add-AppxPackage -Register dist\store\<version>-test\layout\AppxManifest.xml
# which needs Windows Developer Mode (Settings > System > For developers).
param(
    [string]$Version,
    [string]$IdentityName,
    [string]$Publisher,
    [string]$PublisherDisplayName,
    [switch]$Test
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
function Fail($msg) { Write-Host "FAILED: $msg"; exit 1 }

if (-not $Version) {
    $m = [regex]::Match((Get-Content -Raw (Join-Path $repo 'Cargo.toml')), '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    if (-not $m.Success) { Fail 'workspace.package version not found in Cargo.toml' }
    $Version = $m.Groups[1].Value
}
if ($Version -notmatch '^\d+\.\d+\.\d+$') { Fail "version '$Version' is not X.Y.Z" }
# MSIX versions have four parts; the Store requires the last one to be 0.
$msixVersion = "$Version.0"

if ($Test) {
    if (-not $IdentityName) { $IdentityName = 'Stinkshot.LocalTest' }
    if (-not $Publisher) { $Publisher = 'CN=Stinkshot Local Test' }
    if (-not $PublisherDisplayName) { $PublisherDisplayName = 'Stinkshot (Local Test)' }
    $displayName = 'Stinkshot (Test)'
    $outName = "$Version-test"
    $targetDir = Join-Path $repo 'target-store-test'
}
else {
    if (-not ($IdentityName -and $Publisher -and $PublisherDisplayName)) {
        Fail 'pass -IdentityName, -Publisher and -PublisherDisplayName from Partner Center (or -Test)'
    }
    $displayName = 'Stinkshot'
    $outName = $Version
    $targetDir = Join-Path $repo 'target'
}

$sdk = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Directory -ErrorAction SilentlyContinue |
    Where-Object { Test-Path (Join-Path $_.FullName 'x64\makeappx.exe') } |
    Sort-Object { [version]$_.Name } | Select-Object -Last 1
if (-not $sdk) { Fail 'makeappx.exe not found (install the Windows 10/11 SDK)' }
$makeappx = Join-Path $sdk.FullName 'x64\makeappx.exe'

$out = Join-Path $repo "dist\store\$outName"
$layout = Join-Path $out 'layout'
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $layout, (Join-Path $layout 'Assets') | Out-Null

$saved = @{}
foreach ($n in 'CARGO_TARGET_DIR', 'CARGO_ENCODED_RUSTFLAGS', 'STINKSHOT_BUILD_PROFILE') { $saved[$n] = [Environment]::GetEnvironmentVariable($n) }
try {
    $env:CARGO_TARGET_DIR = $targetDir
    # Keep this PC's paths out of the exe, as scripts\release.ps1 does.
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
    $env:CARGO_ENCODED_RUSTFLAGS = @(
        "--remap-path-prefix=$cargoHome=cargo",
        "--remap-path-prefix=$rustupHome=rustup",
        "--remap-path-prefix=$repo=stinkshot",
        "--remap-path-prefix=$targetDir=target"
    ) -join [string][char]0x1f
    if ($Test) { $env:STINKSHOT_BUILD_PROFILE = 'storetest' } else { Remove-Item Env:STINKSHOT_BUILD_PROFILE -ErrorAction SilentlyContinue }

    Write-Host '== cargo build --release'
    cargo build --release --locked -p stinkshot
    if ($LASTEXITCODE -ne 0) { Fail 'cargo build failed' }
    Write-Host '== store images'
    cargo run --release --locked -p stinkshot --example store_assets -- (Join-Path $layout 'Assets')
    if ($LASTEXITCODE -ne 0) { Fail 'store_assets failed' }
}
finally {
    foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }
}

Copy-Item (Join-Path $targetDir 'release\stinkshot.exe') (Join-Path $layout 'stinkshot.exe')
Copy-Item (Join-Path $repo 'LICENSE') (Join-Path $layout 'LICENSE.txt')

$manifest = Get-Content -Raw (Join-Path $repo 'packaging\msix\AppxManifest.xml')
$escape = { param($s) [System.Security.SecurityElement]::Escape($s) }
$values = @{
    IDENTITY_NAME          = $IdentityName
    PUBLISHER              = $Publisher
    PUBLISHER_DISPLAY_NAME = $PublisherDisplayName
    DISPLAY_NAME           = $displayName
    VERSION                = $msixVersion
}
foreach ($k in $values.Keys) { $manifest = $manifest.Replace("{{$k}}", (& $escape $values[$k])) }
if ($manifest -match '\{\{[A-Z_]+\}\}') { Fail "unfilled token in the manifest: $($Matches[0])" }
[IO.File]::WriteAllText((Join-Path $layout 'AppxManifest.xml'), $manifest, (New-Object Text.UTF8Encoding $false))

$msix = Join-Path $out "Stinkshot-$Version-x64.msix"
Write-Host '== makeappx pack'
& $makeappx pack /d $layout /p $msix /o | Out-Host
if ($LASTEXITCODE -ne 0) { Fail 'makeappx pack failed' }

Write-Host ''
Write-Host "Package: $msix"
if ($Test) {
    Write-Host "Try it (needs Developer Mode): Add-AppxPackage -Register `"$layout\AppxManifest.xml`""
    Write-Host 'Remove it again:               Get-AppxPackage Stinkshot.LocalTest | Remove-AppxPackage'
}
else {
    Write-Host 'Upload the .msix in Partner Center. The Store signs it.'
}
