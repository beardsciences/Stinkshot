<#
.SYNOPSIS
  Renders the website's clips and screenshots and the Store screenshots from one commit of the
  app into staging folders. Publishes nothing.

.DESCRIPTION
  The pictures on stinkshot.com and in the Microsoft Store must show the release people
  download. This script checks out the release (tag v<Version>, or -Ref) into a scratch
  worktree, runs the demo harness there (crates/app/src/overlay/demo_clips.rs: the app's own
  UI code, driven by scripted input and rendered offscreen with demo History content and
  default settings) and writes:

    -OutDir (default <site repo>\media-staging, git-ignored, not served)
      video\           the site's clips and posters
      img\             the Settings crops the site shows (tools\crop_shots.py)
      shots\           whole-window pictures, dark and light, with shots.json
      media-source.json  version and commit they were rendered from
    -StoreDir (default dist\store\listing, or listing-<Lang> with -Lang)
      01-*.png ...     Microsoft Store screenshots, 2560x1440 PNG

  The site's tools\update-site.ps1 -Apply copies video\ and img\ into site\ and writes
  media.json; nothing here touches site\. release.ps1 runs this script on every release.

  Nothing touches the real screen, pointer, windows, or the installed app and its settings:
  the harness is a test binary that never opens a window, and the paths the pages show come
  from a neutral demo user folder. Needs a GPU, an installed Windows OCR language, Python with
  Pillow, and an ffmpeg build with libx264 and libwebp (not needed with -ShotsOnly).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\site-media.ps1 -Version 0.1.6

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\site-media.ps1 -Ref HEAD -ShotsOnly

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\site-media.ps1 -Version 0.1.6 -StoreOnly -Lang es
#>
[CmdletBinding()]
param(
    # Version the media is labelled with. Default: the version in Cargo.toml.
    [string]$Version,
    # Commit to render. Default: tag v<Version>. release.ps1 passes the commit it built.
    [string]$Ref,
    [string]$SiteDir,
    # Staging folder for the site media (see above).
    [string]$OutDir,
    # Folder for the Store screenshots.
    [string]$StoreDir,
    # Scratch folder for the worktree, demo inputs and lossless masters.
    [string]$WorkDir,
    [string]$TargetDir,
    [string]$Ffmpeg = $env:STINKSHOT_FFMPEG,
    # Render only these clips (for example select,history). Stills are always rendered.
    [string]$Only = '',
    # Window screenshots only (site shots and Store): no clips, no Settings crops.
    [switch]$ShotsOnly,
    # Store screenshots only; the staged site media is left as it is.
    [switch]$StoreOnly,
    # UI language of the Store screenshots (en, es). Needs -StoreOnly unless en; the default
    # -StoreDir becomes dist\store\listing-<Lang>. Site media is always English.
    [string]$Lang = 'en',
    # Extra files (repo-relative) to copy from this checkout over the commit, for trying an
    # uncommitted harness hook. Never for a release.
    [string[]]$Overlay = @(),
    [int]$Crf = 20,
    # Caps parallel rustc jobs for the harness build.
    [int]$Jobs = 2
)
Set-StrictMode -Version 3
$ErrorActionPreference = 'Stop'

function Fail([string]$m) { Write-Host "FAILED: $m" -ForegroundColor Red; exit 1 }
function Step([string]$m) { Write-Host "== $m" -ForegroundColor Cyan }
function Run([string]$exe, [string[]]$a) {
    # Windows PowerShell 5.1 turns native stderr (cargo progress) into errors under 'Stop'.
    $ErrorActionPreference = 'Continue'
    & $exe @a 2>&1 | ForEach-Object { if ($_ -is [System.Management.Automation.ErrorRecord]) { Write-Host $_.Exception.Message } else { $_ } }
    if ($LASTEXITCODE -ne 0) { Fail "$exe $($a -join ' ') exited with $LASTEXITCODE" }
}

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repo = (Resolve-Path (Join-Path $here '..')).Path
$side = Split-Path -Parent $repo
$bench = Join-Path $side 'stinkshot-bench'
if (-not $SiteDir) { $SiteDir = Join-Path $side 'stinkshot-site' }
if (-not $WorkDir) { $WorkDir = Join-Path $bench 'site-media' }
if (-not $TargetDir) { $TargetDir = Join-Path $bench 'target-demo-clips' }
$Lang = $Lang.ToLowerInvariant()
if ($Lang -notin 'en', 'es') { Fail "-Lang $Lang is not a shipped language (en, es)" }
if ($Lang -ne 'en' -and -not $StoreOnly) { Fail "-Lang $Lang needs -StoreOnly: the site media is English only" }
if (-not $StoreDir) {
    $StoreDir = Join-Path $repo $(if ($Lang -eq 'en') { 'dist\store\listing' } else { "dist\store\listing-$Lang" })
}
if (-not $Version) {
    $m = [regex]::Match((Get-Content -Raw (Join-Path $repo 'Cargo.toml')), '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"')
    if (-not $m.Success) { Fail 'workspace.package version not found in Cargo.toml' }
    $Version = $m.Groups[1].Value
}
if (-not $Ref) { $Ref = "v$Version" }
if ($StoreOnly) { $ShotsOnly = $true }
if (-not $ShotsOnly) {
    if (-not $Ffmpeg) { $Ffmpeg = (Get-Command ffmpeg -ErrorAction SilentlyContinue).Source }
    if (-not $Ffmpeg -or -not (Test-Path $Ffmpeg)) { Fail 'ffmpeg not found: pass -Ffmpeg, set STINKSHOT_FFMPEG, or use -ShotsOnly' }
}
if (-not (Test-Path (Join-Path $SiteDir 'tools\crop_shots.py'))) { Fail "site repo not found at $SiteDir" }
$SiteDir = (Resolve-Path $SiteDir).Path
if (-not $OutDir) { $OutDir = Join-Path $SiteDir 'media-staging' }
$OutDir = [IO.Path]::GetFullPath($OutDir).TrimEnd('\')
$live = Join-Path $SiteDir 'site'
if ($OutDir.StartsWith($live, [StringComparison]::OrdinalIgnoreCase)) { Fail "-OutDir is inside $live, which is live. Stage elsewhere." }
$commit = (& git -C $repo rev-parse -q --verify "$Ref^{commit}")
if ($LASTEXITCODE -ne 0) { Fail "$Ref is not a commit (tag the release first, or pass -Ref)" }
$commit = "$commit".Trim()

New-Item -ItemType Directory -Force $WorkDir | Out-Null
$WorkDir = (Resolve-Path $WorkDir).Path
$src = Join-Path $WorkDir 'src'
$demo = Join-Path $WorkDir 'demo'
$demoSite = Join-Path $WorkDir 'demo-site'
New-Item -ItemType Directory -Force $demo | Out-Null

Step "Check out $Ref ($($commit.Substring(0, 8)))"
if (Test-Path $src) {
    Run git @('-C', $repo, 'worktree', 'remove', '--force', $src)
}
Run git @('-C', $repo, 'worktree', 'add', '--detach', $src, $commit)
try {
    # The harness is test-only and not part of the media's sources (media.json excludes it),
    # so the current one is laid over the commit: older releases get the newer sets. The app
    # code that gets rendered is still exactly the commit's.
    $harness = 'crates\app\src\overlay\demo_clips.rs'
    $theirs = Join-Path $src $harness
    $had = Test-Path $theirs
    Copy-Item (Join-Path $repo $harness) $theirs -Force
    if (-not $had) {
        $modPath = Join-Path $src 'crates\app\src\overlay\mod.rs'
        $mod = [IO.File]::ReadAllText($modPath)
        if ($mod -notmatch 'mod demo_clips;') {
            $mod = $mod -replace '(?m)^mod beautify_panel;\r?$', "mod beautify_panel;`n#[cfg(test)]`nmod demo_clips;"
            [IO.File]::WriteAllText($modPath, $mod)
        }
    }
    foreach ($f in $Overlay) {
        Write-Host "  overlay $f (from this checkout, not $Ref)" -ForegroundColor Yellow
        Copy-Item (Join-Path $repo $f) (Join-Path $src $f) -Force
    }

    Step 'Demo inputs'
    $py = (Get-Command python -ErrorAction Stop).Source
    # Fictional pictures the demo desktop and History are built from.
    $DemoArt = Join-Path $demo 'art'
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\gen_demo.py'), $DemoArt)
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\gen_desktop.py'), $DemoArt, (Join-Path $demo 'desktop.png'))
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\seed_history.py'), $DemoArt, (Join-Path $demo 'history'))
    foreach ($d in (Join-Path $demo 'stills'), $demoSite) { if (Test-Path $d) { Remove-Item -Recurse -Force $d } }
    New-Item -ItemType Directory -Force $demoSite | Out-Null
    Copy-Item (Join-Path $demo 'desktop.png*') $demoSite
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\seed_history.py'), $DemoArt, (Join-Path $demoSite 'history'))

    Step 'Render (the app UI, offscreen)'
    $saved = @{}
    foreach ($n in 'CARGO_TARGET_DIR', 'CARGO_BUILD_JOBS', 'STINKSHOT_DEMO_DIR', 'STINKSHOT_FFMPEG', 'STINKSHOT_DEMO_ONLY', 'STINKSHOT_DEMO_SCALE', 'STINKSHOT_DEMO_HOME', 'STINKSHOT_DEMO_STORE', 'STINKSHOT_DEMO_LANG', 'STINKSHOT_PROFILE') {
        $saved[$n] = [Environment]::GetEnvironmentVariable($n)
    }
    $clips = @('select', 'annotate', 'redact', 'history', 'steps', 'magnify', 'measure', 'picker', 'beautify', 'sensitive', 'multi')
    if ($Only) { $clips = @($Only.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
    if ($ShotsOnly) { $clips = @() }
    $sets = @($clips) + 'shots'
    if (-not $ShotsOnly) { $sets += 'site' }
    try {
        $env:CARGO_TARGET_DIR = $TargetDir
        if ($Jobs -gt 0) { $env:CARGO_BUILD_JOBS = "$Jobs" }
        $env:STINKSHOT_FFMPEG = $Ffmpeg
        $env:STINKSHOT_PROFILE = $null
        # The user folder the pages show (Settings prints config and History paths).
        $env:STINKSHOT_DEMO_HOME = 'C:\Users\Demo'
        Push-Location $src
        try {
            # 150 %: clips, the Settings stills the site crops (installer build), and the Store
            # screenshots (the 'shots' set in the Store build's UI: no update check).
            $env:STINKSHOT_DEMO_DIR = $demo
            $env:STINKSHOT_DEMO_SCALE = $null
            $env:STINKSHOT_DEMO_STORE = '1'
            $env:STINKSHOT_DEMO_LANG = $Lang
            $env:STINKSHOT_DEMO_ONLY = $sets -join ','
            Run cargo @('test', '--locked', '-p', 'stinkshot', 'demo_clips', '--', '--ignored', '--nocapture')
            if (-not $StoreOnly) {
                # 125 %, installer build: the site's whole-window pictures (shots.json sizes).
                $env:STINKSHOT_DEMO_DIR = $demoSite
                $env:STINKSHOT_DEMO_SCALE = '0.8333333'
                $env:STINKSHOT_DEMO_STORE = $null
                $env:STINKSHOT_DEMO_LANG = 'en'
                $env:STINKSHOT_DEMO_ONLY = 'shots'
                Run cargo @('test', '--locked', '-p', 'stinkshot', 'demo_clips', '--', '--ignored', '--nocapture')
            }
        } finally { Pop-Location }
    } finally {
        foreach ($n in $saved.Keys) { [Environment]::SetEnvironmentVariable($n, $saved[$n]) }
    }

    if ($StoreOnly) {
        Step 'Store screenshots'
        Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\release_shots.py'),
            '--store-stills', (Join-Path $demo 'stills'), '--store-out', $StoreDir,
            '--version', $Version, '--commit', $commit, '--lang', $Lang)
        return
    }
    if (Test-Path $OutDir) { Remove-Item -Recurse -Force $OutDir }
    New-Item -ItemType Directory -Force $OutDir | Out-Null

    if ($clips.Count) {
        Step 'Encode clips'
        $video = Join-Path $OutDir 'video'
        New-Item -ItemType Directory -Force $video | Out-Null
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
    }

    if (-not $ShotsOnly) {
        Step 'Crop stills'
        Run $py @('-I', (Join-Path $SiteDir 'tools\crop_shots.py'), '--src', (Join-Path $demo 'stills'), '--out', (Join-Path $OutDir 'img'))
    }

    Step 'Window screenshots (site and Store)'
    Run $py @('-I', (Join-Path $repo 'scripts\demo-clips\release_shots.py'),
        '--site-stills', (Join-Path $demoSite 'stills'), '--store-stills', (Join-Path $demo 'stills'),
        '--site-out', (Join-Path $OutDir 'shots'), '--store-out', $StoreDir,
        '--version', $Version, '--commit', $commit)

    # update-site.ps1 publishes only a complete set rendered from the release commit.
    $source = [ordered]@{
        note        = 'Written by scripts/site-media.ps1 in the app repo. tools/update-site.ps1 publishes video/ and img/ from here.'
        app_version = $Version
        app_commit  = $commit
        ref         = $Ref
        rendered    = (Get-Date -Format 'yyyy-MM-ddTHH:mm:ssK')
        complete    = (-not $ShotsOnly -and -not $Only)
    }
    [IO.File]::WriteAllText((Join-Path $OutDir 'media-source.json'), (($source | ConvertTo-Json) + "`n"), (New-Object System.Text.UTF8Encoding($false)))
} finally {
    & git -C $repo worktree remove --force $src 2>$null | Out-Null
}

Write-Host "Rendered from $Ref ($($commit.Substring(0, 8))):" -ForegroundColor Green
Write-Host "  site media (staged, not live): $OutDir"
Write-Host "  Store screenshots:             $StoreDir"
Write-Host 'Look at them. In the site repo, tools/update-site.ps1 -Apply publishes the staged clips and crops.'
