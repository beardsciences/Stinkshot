<#
.SYNOPSIS
  Runs every check a change must pass: formatting, lints, tests, UI string rules and
  dependency advisories. Run it before you push.

.DESCRIPTION
  The same checks scripts\release.ps1 and the CI workflow run, in one place.
  -Desktop also runs the tests that need a real desktop session (capture, hotkeys, windows).
  They open windows and take a few minutes; leave the PC alone while they run.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\check.ps1
#>
[CmdletBinding()]
param(
    [switch]$Desktop,
    # Skip the advisory check (needs cargo-audit and a network connection).
    [switch]$NoAudit
)
Set-StrictMode -Version 3
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = Split-Path -Parent $here

$failed = @()
function Check([string]$name, [scriptblock]$body) {
    Write-Host "== $name" -ForegroundColor Cyan
    $global:LASTEXITCODE = 0
    & $body
    if ($LASTEXITCODE -ne 0) {
        Write-Host "FAILED: $name" -ForegroundColor Red
        $script:failed += $name
    }
}

Push-Location $repo
$savedProfile = $env:STINKSHOT_PROFILE
try {
    # The tests check default paths, so no dev profile may leak in.
    $env:STINKSHOT_PROFILE = $null
    Check 'cargo fmt' { cargo fmt --all -- --check }
    Check 'cargo clippy' { cargo clippy --workspace --all-targets --locked -- -D warnings }
    Check 'cargo test' { cargo test --workspace --locked }
    Check 'UI strings' { powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $here 'check-strings.ps1') }
    if (-not $NoAudit) {
        if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
            # Fails on vulnerabilities. Unmaintained-crate notices are printed, not fatal.
            Check 'cargo audit' { cargo audit }
        } else {
            Write-Host 'cargo-audit is not installed (cargo install cargo-audit --locked): advisories not checked' -ForegroundColor Yellow
        }
    }
    if ($Desktop) {
        Check 'desktop tests' { cargo test --workspace --locked -- --ignored --test-threads=1 --skip demo_clips }
    }
} finally {
    $env:STINKSHOT_PROFILE = $savedProfile
    Pop-Location
}

if ($failed.Count) {
    Write-Host "FAILED: $($failed -join ', ')" -ForegroundColor Red
    exit 1
}
Write-Host 'All checks passed.' -ForegroundColor Green
