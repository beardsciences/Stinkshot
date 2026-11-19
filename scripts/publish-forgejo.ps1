# Create the Forgejo release for a tag and upload the dist files to it.
# Asks for a Forgejo access token (scope: write:repository). The token is kept in memory only
# and reaches curl through a config on stdin, never on a command line.
#
#   powershell -ExecutionPolicy Bypass -File scripts/publish-forgejo.ps1 -Version 0.1.5
#
# Before anything is created it checks that the tag exists on the server and points at the
# commit in stinkshot-release.json. Exit code: 0 when every file is on the release with the
# right size, 1 otherwise. Safe to run again: files already uploaded are skipped.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    # Default: dist\<version> in the repo, the same as scripts\release.ps1.
    [string]$DistDir,
    [string]$Repo = 'christian/stinkshot',
    [string]$Server = 'https://code.mccullough.dev'
)
Set-StrictMode -Version 3
$ErrorActionPreference = 'Stop'

if (-not $DistDir) { $DistDir = Join-Path (Split-Path -Parent $PSScriptRoot) "dist\$Version" }
$DistDir = [IO.Path]::GetFullPath($DistDir).TrimEnd('\')
$tag = "v$Version"
$api = "$Server/api/v1/repos/$Repo"
$notes = Join-Path $DistDir 'RELEASE_NOTES.md'
$manifestPath = Join-Path $DistDir 'stinkshot-release.json'

if (-not (Test-Path -LiteralPath $notes)) { throw "No release in $DistDir (run scripts\release.ps1 first)." }
if ($DistDir -match '-dirty$') { throw "$DistDir is a dry-run build (-AllowDirty). Never publish it." }
if (-not (Test-Path -LiteralPath $manifestPath)) { throw "stinkshot-release.json is missing in $DistDir." }
foreach ($sig in 'stinkshot-release.json.minisig', 'SHA256SUMS.minisig') {
    if (-not (Test-Path -LiteralPath (Join-Path $DistDir $sig))) { throw "$sig is missing in ${DistDir}: this build is not signed. Never publish it." }
}
$manifest = [IO.File]::ReadAllText($manifestPath) | ConvertFrom-Json
if ($manifest.version -ne $Version) { throw "stinkshot-release.json is for version $($manifest.version), not $Version." }
if ($manifest.tag -ne $tag) { throw "stinkshot-release.json has tag $($manifest.tag), expected $tag." }
$wantCommit = "$($manifest.commit)".Trim().ToLowerInvariant()
if ($wantCommit -notmatch '^[0-9a-f]{40}$') { throw "stinkshot-release.json has no usable commit ('$wantCommit')." }

$uploads = @(Get-ChildItem -File -LiteralPath $DistDir | Where-Object { $_.Name -ne 'RELEASE_NOTES.md' } | Sort-Object Name)
if (-not $uploads.Count) { throw "Nothing to upload in $DistDir." }
foreach ($f in $uploads) {
    # curl -F splits its value on these.
    if ($f.FullName -match '[",;]') { throw "Path not usable with curl -F (quote, comma or semicolon): $($f.FullName)" }
}

$script:CurlConfig = $null

# Runs curl with the auth header fed through a config on stdin. Returns Status (HTTP code) and
# Body. Throws when curl itself fails (DNS, TLS, connection, unreadable file).
function Invoke-Api {
    param([string[]]$CurlArgs)
    # Windows PowerShell 5.1 turns native stderr into error records; judge by the exit code.
    $ErrorActionPreference = 'Continue'
    $out = @($script:CurlConfig | & curl.exe --config - --silent --show-error --write-out '\n%{http_code}' @CurlArgs)
    $code = $LASTEXITCODE
    if ($code -ne 0) { throw "curl exited with $code." }
    if (-not $out.Count) { throw 'curl returned nothing.' }
    $status = 0
    if (-not [int]::TryParse("$($out[-1])".Trim(), [ref]$status)) { throw "curl returned no HTTP status: $($out[-1])" }
    $body = ''
    if ($out.Count -gt 1) { $body = ($out[0..($out.Count - 2)] -join "`n") }
    return [pscustomobject]@{ Status = $status; Body = $body }
}

function ConvertFrom-JsonOrNull([string]$Text) {
    if (-not $Text -or -not $Text.Trim()) { return $null }
    try { return ($Text | ConvertFrom-Json) } catch { return $null }
}

function Get-Prop($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    $p = $Object.PSObject.Properties[$Name]
    if ($p) { return $p.Value }
    return $null
}

$failed = 0
$secure = Read-Host "Forgejo token for $Server (Settings > Applications, scope write:repository)" -AsSecureString
$bstr = [IntPtr]::Zero
$tmp = $null
try {
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    $token = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
    if (-not $token -or $token -notmatch '^[\x21-\x7e]+$') { throw 'The token is empty or has spaces or non-ASCII characters.' }
    $script:CurlConfig = "header = `"Authorization: token $token`""
    $token = $null

    # The tag must already be on the server and point at the commit that was built.
    $r = Invoke-Api @("$api/tags/$tag")
    if ($r.Status -eq 404) { throw "Tag $tag is not on $Server. Push it first: git push origin $tag" }
    if ($r.Status -eq 401 -or $r.Status -eq 403) { throw "The server refused the token (HTTP $($r.Status))." }
    if ($r.Status -ne 200) { throw "Reading tag $tag failed (HTTP $($r.Status)): $($r.Body)" }
    $tagInfo = ConvertFrom-JsonOrNull $r.Body
    $tagCommit = "$(Get-Prop (Get-Prop $tagInfo 'commit') 'sha')".Trim().ToLowerInvariant()
    if (-not $tagCommit) { throw "The server's answer for tag $tag has no commit: $($r.Body)" }
    if ($tagCommit -ne $wantCommit) {
        throw "Tag $tag on the server is commit $tagCommit, but $DistDir was built from $wantCommit. Nothing was published."
    }
    Write-Host "Tag $tag is on the server at $tagCommit (matches the build)."

    # Reuse the release if the tag already has one, otherwise create it.
    $have = @{}
    $r = Invoke-Api @("$api/releases/tags/$tag")
    if ($r.Status -eq 200) {
        $existing = ConvertFrom-JsonOrNull $r.Body
        $id = Get-Prop $existing 'id'
        if (-not $id) { throw "Unexpected answer for release $tag`: $($r.Body)" }
        Write-Host "Release $tag exists (id $id); uploading missing files."
    } elseif ($r.Status -eq 404) {
        $body = @{ tag_name = $tag; name = "Stinkshot $Version"; body = [IO.File]::ReadAllText($notes); draft = $false; prerelease = $false } | ConvertTo-Json
        $tmp = [IO.Path]::GetTempFileName()
        [IO.File]::WriteAllText($tmp, $body, (New-Object Text.UTF8Encoding $false))
        $r = Invoke-Api @('--header', 'Content-Type: application/json', '--data-binary', "@$tmp", "$api/releases")
        $created = ConvertFrom-JsonOrNull $r.Body
        $id = Get-Prop $created 'id'
        if ($r.Status -lt 200 -or $r.Status -ge 300 -or -not $id) { throw "Creating the release failed (HTTP $($r.Status)): $($r.Body)" }
        Write-Host "Created release $tag (id $id)."
    } else {
        throw "Looking up release $tag failed (HTTP $($r.Status)): $($r.Body)"
    }

    $r = Invoke-Api @("$api/releases/$id/assets")
    if ($r.Status -ne 200) { throw "Listing the release's files failed (HTTP $($r.Status)): $($r.Body)" }
    foreach ($a in @(ConvertFrom-JsonOrNull $r.Body)) {
        $n = Get-Prop $a 'name'
        if ($n) { $have[$n] = Get-Prop $a 'size' }
    }

    foreach ($f in $uploads) {
        if ($have.ContainsKey($f.Name)) {
            if ([int64]$have[$f.Name] -eq $f.Length) {
                Write-Host "  skip $($f.Name) (already uploaded, size matches)"
            } else {
                Write-Warning "$($f.Name) is already on the release with $($have[$f.Name]) bytes, local file has $($f.Length). Delete it on the release page and run again."
                $failed++
            }
            continue
        }
        Write-Host "  uploading $($f.Name) ($([math]::Round($f.Length / 1MB, 1)) MB)..."
        try {
            $r = Invoke-Api @('--form', "attachment=@$($f.FullName)", "$api/releases/$id/assets?name=$([uri]::EscapeDataString($f.Name))")
        } catch {
            Write-Warning "upload failed: $($f.Name): $($_.Exception.Message)"
            $failed++
            continue
        }
        $asset = ConvertFrom-JsonOrNull $r.Body
        $size = Get-Prop $asset 'size'
        if ($r.Status -lt 200 -or $r.Status -ge 300 -or -not (Get-Prop $asset 'name')) {
            Write-Warning "upload failed: $($f.Name) (HTTP $($r.Status)): $($r.Body.Trim())"
            $failed++
        } elseif ([int64]$size -ne $f.Length) {
            Write-Warning "$($f.Name): the server stored $size bytes, local file has $($f.Length). Delete it on the release page and run again."
            $failed++
        } else {
            Write-Host "  uploaded $($f.Name) ($size bytes)"
        }
    }
}
finally {
    $script:CurlConfig = $null
    $token = $null
    if ($bstr -ne [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr) }
    if ($secure) { $secure.Dispose() }
    if ($tmp -and (Test-Path -LiteralPath $tmp)) { Remove-Item -LiteralPath $tmp -Force }
}

if ($failed) {
    Write-Host "FAILED: $failed file(s) are missing or wrong on $Server/$Repo/releases/tag/$tag. Fix and run again." -ForegroundColor Red
    exit 1
}
Write-Host "Done: $Server/$Repo/releases/tag/$tag"
exit 0
