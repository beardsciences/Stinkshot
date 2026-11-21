# Type-checks the workspace for Linux and macOS from Windows (clippy, -D warnings). Nothing
# is linked or run: real builds and tests run on CI (`portable` job). See
# docs/notes/cross-platform.md.
#
#   powershell -ExecutionPolicy Bypass -File scripts\xcheck.ps1 [-Target linux|macos|all]
#
# Needs: rustup targets x86_64-unknown-linux-gnu and aarch64-apple-darwin, and LLVM (clang,
# llvm-ar) in C:\Program Files\LLVM\bin. C dependencies (ring) are compiled against the tiny
# headers in scripts\xcheck-include, which are good enough for a type check only.
param(
    [ValidateSet('linux', 'macos', 'all')]
    [string]$Target = 'all'
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$include = (Join-Path $repo 'scripts\xcheck-include') -replace '\\', '/'
$llvm = 'C:\Program Files\LLVM\bin'
if (-not (Test-Path (Join-Path $llvm 'clang.exe'))) { throw "clang not found in $llvm" }

$saved = @{}
function Set-Var($name, $value) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name)
    [Environment]::SetEnvironmentVariable($name, $value)
}

Set-Var 'PATH' "$env:PATH;$llvm"
Set-Var 'CARGO_TARGET_DIR' (Join-Path $repo 'target-xcheck')
# Skip building the bundled SQLite; it needs a full C toolchain for the target.
Set-Var 'LIBSQLITE3_SYS_USE_PKG_CONFIG' '1'
foreach ($t in 'x86_64_unknown_linux_gnu', 'aarch64_apple_darwin') {
    Set-Var "CC_$t" 'clang'
    Set-Var "AR_$t" 'llvm-ar'
    Set-Var "CFLAGS_$t" "-ffreestanding -isystem $include"
}
# GTK libraries behind the Linux tray icon: skip their pkg-config probes.
foreach ($k in 'ATK', 'CAIRO', 'CAIRO_GOBJECT', 'GDK_3_0', 'GDK_PIXBUF_2_0', 'GIO_2_0', 'GLIB_2_0', 'GOBJECT_2_0', 'PANGO', 'GTK_3_0') {
    Set-Var "SYSTEM_DEPS_${k}_NO_PKG_CONFIG" '1'
    Set-Var "SYSTEM_DEPS_${k}_LIB" 'x'
}

$targets = switch ($Target) {
    'linux' { @('x86_64-unknown-linux-gnu') }
    'macos' { @('aarch64-apple-darwin') }
    default { @('x86_64-unknown-linux-gnu', 'aarch64-apple-darwin') }
}
$failed = @()
try {
    foreach ($t in $targets) {
        Write-Host "== $t"
        cargo clippy --locked --workspace --all-targets --target $t -- -D warnings
        if ($LASTEXITCODE -ne 0) { $failed += $t }
    }
}
finally {
    foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k]) }
}
if ($failed) {
    Write-Host "FAILED: $($failed -join ', ')"
    exit 1
}
Write-Host 'Linux and macOS type-check passed.'
