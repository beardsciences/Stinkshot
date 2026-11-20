<#
.SYNOPSIS
  Renders the website's clips and screenshots from a released version of the app.

.DESCRIPTION
  The pictures on stinkshot.com must show the release people download. This script checks out
  the release tag into a scratch worktree, runs the demo harness there
  (crates/app/src/overlay/demo_clips.rs: the app's own UI code, driven by scripted input and
  rendered offscreen), encodes the clips into the site's video folder, crops the stills into
  its img folder and writes media.json with the version, commit and a hash of every file.
  The site's tools/check_site.py fails when media.json is older than the release.

  Nothing touches the real screen, pointer or windows. Needs a GPU, an installed Windows OCR
  language, Python with Pillow, and an ffmpeg build with libx264 and libwebp.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\site-media.ps1 -Version 0.1.5
#>
[CmdletBinding()]
param(
    # Release to render (tag v<Version>). Default: the version in Cargo.toml.
    [string]$Version,
    [string]$SiteDir,
    # Scratch folder for the worktree, demo inputs and lossless masters.
    [string]$WorkDir,
    [string]$TargetDir,
    [string]$Ffmpeg = $env:STINKSHOT_FFMPEG,
    # Render only these clips (for example select,history). Stills are always rendered.
    [string]$Only = '',
    [int]$Crf = 20
)
Set-StrictMode -Version 3
$ErrorActionPreference = 'Stop'

function Fail([string]$m) { Write-Host "FAILED: $m" -ForegroundColor Red; exit 1 }
function Step([string]$m) { Write-Host "== $m" -ForegroundColor Cyan }
function Run([string]$exe, [string[]]$a) {
    & $exe @a
    if ($LASTEXITCODE -ne 0) { Fail "$exe $($a -join ' ') exited with $LASTEXITCODE" }
}

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = (Resolve-Path (Join-Path $here '..')).Path
$side = Split-Path -Parent $repo
# Defaults: the site repo and a scratch folder next to this repo.
if (-not $SiteDir) { $SiteDir = Join-Path $side 'stinkshot-site' }
if (-not $WorkDir) { $WorkDir = Join-Path $side ('stinkshot-bench' + [IO.Path]::DirectorySeparatorChar + 'site-media') }
if (-not $TargetDir) { $TargetDir = Join-Path $side ('stinkshot-bench' + [IO.Path]::DirectorySeparatorChar + 'target-demo-clips') }
if (-not $Version) {
    $m = [regex]::Match((Get-Content -Raw (Join-Path $repo 'Cargo.toml')), '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    if (-not $m.Success) { Fail 'workspace.package version not found in Cargo.toml' }
    $Version = $m.Groups[1].Value
}
$tag = "v$Version"
if (-not $Ffmpeg) { $Ffmpeg = (Get-Command ffmpeg -ErrorAction SilentlyContinue).Source }
if (-not $Ffmpeg -or -not (Test-Path $Ffmpeg)) { Fail 'ffmpeg not found: pass -Ffmpeg or set STINKSHOT_FFMPEG' }
if (-not (Test-Path (Join-Path $SiteDir 'site\index.html'))) { Fail "site repo not found at $SiteDir" }
$SiteDir = (Resolve-Path $SiteDir).Path
$commit = (& git -C $repo rev-parse -q --verify "$tag^{commit}")
if ($LASTEXITCODE -ne 0) { Fail "tag $tag does not exist: tag the release first" }
$commit = "$commit".Trim()

New-Item -ItemType Directory -Force $WorkDir | Out-Null
$WorkDir = (Resolve-Path $WorkDir).Path
$src = Join-Path $WorkDir 'src'
$demo = Join-Path $WorkDir 'demo'
New-Item -ItemType Directory -Force $demo | Out-Null

Step "Check out $tag ($($commit.Substring(0, 8)))"
if (Test-Path $src) {
    Run git @('-C', $repo, 'worktree', 'remove', '--force', $src)
}
Run git @('-C', $repo, 'worktree', 'add', '--detach', $src, $commit)
try {
    # Releases before the harness was merged: lay the current harness over the tag's code.
    # The app code that gets rendered is still exactly the tag's.
    $harness = 'crates\app\src\overlay\demo_clips.rs'
    $overlaid = $false
    if (-not (Test-Path (Join-Path $src $harness))) {
        $overlaid = $true
        Copy-Item (Join-Path $repo $harness) (Join-Path $src $harness)
        $modPath = Join-Path $src 'crates\app\src\overlay\mod.rs'
        $mod = [IO.File]::ReadAllText($modPath)
        if ($mod -notmatch 'mod demo_clips;') {
            $mod = $mod -replace '(?m)^mod beautify_panel;\r?$', "mod beautify_panel;`n#[cfg(test)]`nmod demo_clips;"
            [IO.File]::WriteAllText($modPath, $mod)
        }
        Write-Host "  $tag has no demo harness; using the one from this checkout"
    }

    Step 'Demo inputs'
    $py = (Get-Command python -ErrorAction Stop).Source
    # Fictional pictures the demo desktop and History are built from.
    $DemoArt = Join-Path $demo 'art'
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\gen_demo.py'), $DemoArt)
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\gen_desktop.py'), $DemoArt, (Join-Path $demo 'desktop.png'))
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\seed_history.py'), $DemoArt, (Join-Path $demo 'history'))

    Step 'Render (the app UI, offscreen)'
    $saved = @{}
    foreach ($n in 'CARGO_TARGET_DIR', 'STINKSHOT_DEMO_DIR', 'STINKSHOT_FFMPEG', 'STINKSHOT_DEMO_ONLY', 'STINKSHOT_PROFILE') {
        $saved[$n] = [Environment]::GetEnvironmentVariable($n)
    }
    $clips = @('select', 'annotate', 'redact', 'history', 'steps', 'magnify', 'measure', 'picker', 'beautify', 'sensitive', 'multi')
    if ($Only) { $clips = @($Only.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
    try {
        $env:CARGO_TARGET_DIR = $TargetDir
        $env:STINKSHOT_DEMO_DIR = $demo
        $env:STINKSHOT_FFMPEG = $Ffmpeg
        $env:STINKSHOT_PROFILE = $null
        $env:STINKSHOT_DEMO_ONLY = (@($clips) + 'site') -join ','
        Push-Location $src
        try {
            Run cargo @('test', '--locked', '-p', 'stinkshot', 'demo_clips', '--', '--ignored', '--nocapture')
        } finally { Pop-Location }
    } finally {
        foreach ($n in $saved.Keys) { [Environment]::SetEnvironmentVariable($n, $saved[$n]) }
    }

    Step 'Encode clips'
    $video = Join-Path $SiteDir 'site\video'
    $lanczos = 'flags=lanczos'
    # Framing per clip, in the 1920x1200 px of the masters.
    $filters = @{
        select    = "scale=1200:-2:$lanczos"
        annotate  = "scale=1200:-2:$lanczos"
        steps     = "scale=1200:-2:$lanczos"
        magnify   = "scale=1200:-2:$lanczos"
        measure   = "scale=1200:-2:$lanczos"
        picker    = "scale=1200:-2:$lanczos"
        beautify  = "scale=1200:-2:$lanczos"
        multi     = "scale=1200:-2:$lanczos"
        redact    = 'crop=1200:540:100:640'
        history   = "crop=1920:840:0:0,scale=1200:-2:$lanczos"
        sensitive = "crop=1720:500:100:640,scale=1200:-2:$lanczos"
    }
    foreach ($c in $clips) {
        if (-not $filters.ContainsKey($c)) { Fail "no framing defined for clip '$c'" }
        $master = Join-Path $demo "$c.mkv"
        if (-not (Test-Path $master)) { Fail "the harness did not write $master" }
        Run $Ffmpeg @('-y', '-hide_banner', '-loglevel', 'error', '-i', $master,
            '-vf', "$($filters[$c]),scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
            '-c:v', 'libx264', '-preset', 'veryslow', '-tune', 'animation', '-crf', "$Crf", '-g', '300',
            '-colorspace', 'bt709', '-color_primaries', 'bt709', '-color_trc', 'bt709',
            '-movflags', '+faststart', '-an', (Join-Path $video "$c-1200.mp4"))
        # Poster: the last frame, also what reduced-motion visitors see.
        Run $Ffmpeg @('-y', '-hide_banner', '-loglevel', 'error', '-sseof', '-0.2', '-i', $master,
            '-frames:v', '1', '-vf', $filters[$c], '-c:v', 'libwebp', '-quality', '80',
            (Join-Path $video "$c-1200.webp"))
        Write-Host "  $c"
    }

    Step 'Crop stills'
    Run $py @('-I', (Join-Path $SiteDir 'tools\crop_shots.py'), '--src', (Join-Path $demo 'stills'))

    Step 'Write media.json'
    $wargs = @('-I', (Join-Path $SiteDir 'tools\write_media_manifest.py'), '--app-version', $Version, '--app-commit', $commit)
    if ($Only) { $wargs += @('--partial') }
    Run $py $wargs
} finally {
    & git -C $repo worktree remove --force $src 2>$null | Out-Null
}

Step 'Check the site'
Run $py @('-I', (Join-Path $SiteDir 'tools\check_site.py'), '--app', $repo)
Write-Host "Site media rendered from $tag. Look at the pages before committing the site repo." -ForegroundColor Green
