# Releasing Stinkshot

Maintainer runbook. Nothing here runs automatically: every publish step is a command you
type. Background and design decisions: `docs/notes/release.md`.

Commands are for Windows PowerShell 5.1 (no `&&` or `||`; type `curl.exe`, not `curl`), run
from the app repo unless a step says otherwise. Paths in the examples use forward slashes;
PowerShell and both scripts accept them.

The order matters:

1. Changelog, version, commit.
2. Build: `scripts/release.ps1`.
3. Smoke test: `scripts/smoke-test.ps1`.
4. Tag and push.
5. Site: downloads first, update manifest last (`tools/update-site.ps1` in the site repo).
6. Check the live site (the site repo's `DEPLOY.md`, step 4).
7. Forgejo release: `scripts/publish-forgejo.ps1 -Version X.Y.Z`.
8. GitHub release on the mirror: `gh release create`.
9. winget.

Installed copies read `https://stinkshot.com/release/stinkshot-release.json` and download the
installer from `https://stinkshot.com/download/` (`crates/app/src/update.rs`). The site is what
ships a release. The Forgejo release is a second copy for people who look there; the app never
reads it.

## One-time setup

### Tools

`scripts/release.ps1` looks for each tool as a parameter, an environment variable, on PATH, then
under `C:\dev\stinkshot-bench\tools` (where the dev box has them, all portable, nothing
installed system-wide):

| Tool | Version | Where it came from | Override |
|---|---|---|---|
| Inno Setup | 7.1.1 | https://jrsoftware.org/isdl.php (installed with `/CURRENTUSER /PORTABLE=1 /DIR=...`) | `-Iscc`, `STINKSHOT_ISCC` |
| minisign | 0.12 | https://github.com/jedisct1/minisign/releases (signature checked against the author's key) | `-Minisign`, `STINKSHOT_MINISIGN` |
| cargo-cyclonedx | 0.5.10 | `cargo install cargo-cyclonedx --locked --root C:\dev\stinkshot-bench\tools\cargo` | `-Cyclonedx`, `STINKSHOT_CYCLONEDX` |
| signtool | Windows SDK 10.0.26100 | already installed with the Build Tools | `-SignTool` |

### Release signing key (update manifest and SHA256SUMS)

The minisign secret key is kept outside the repo, on the maintainer's PC only. `release.ps1`
finds it on its own; `-SigningKey` points it at another location. Never commit it.

The public key is committed as `crates/app/assets/release-key.pub` and compiled into the app.
Key id `A2920B49A99E16A0`, public key
`RWSgFp6pSQuSoo99V89nCiRKQ+SToPCxbnLeyViYxFzzfnbNKV77DFH3`.

- Keep a password on the key (`minisign -C -s <key file>` sets or changes it). minisign asks
  for it on every release.
- Keep two backups that are not this PC, for example an encrypted USB stick kept offline and
  a password manager attachment. Not on a synced share or a cloud folder in plain form.
- Planned rotation (old key still held): ship one release signed with the old key that embeds
  the new public key, then sign with the new key from the next release on.
- If the key is lost, nothing can be signed with it again, so installed copies can never trust
  a new release: users have to download the next version by hand.
- If it leaks, anyone can make installed copies show a fake "update available". Update Now only
  downloads from stinkshot.com/download and checks the file against the signed hash, so a
  leaked key alone cannot push an installer; a leaked key plus write access to the site could.
  Rotate at once (the leaked key still signs that one release) and say so in its notes.

### Code signing (Authenticode)

Releases are not code-signed yet. Windows SmartScreen shows "Windows protected your PC" on the
first run of the installer and of the portable exe; users click More info, then Run anyway. The
warning fades as a file builds reputation and starts over with every new unsigned build. The
README and the site say so.

SignPath Foundation declined the project in Nov 2026 (not enough public visibility yet). The
Microsoft Store build is signed by Microsoft and shows no warning (`packaging/msix/README.md`).
For the installer, publish the unsigned files `release.ps1` builds. The SignPath steps in
`.github/workflows/release-build.yml` stay dormant until its variables are set; if a
certificate is added later, `release.ps1` still needs a way to take CI-signed files as input
(the hashes, SBOM and minisign signatures are made here, because the minisign key never
leaves this PC).

`release.ps1` also has `-Sign signpath` and `-Sign signtool -CertThumbprint <sha1>`. They sign
a local build, which the policy above rules out for public releases; use them only for testing
the signing path. The SignPath token goes in an environment variable, typed at a prompt, never
pasted into a command:

```
$env:SIGNPATH_API_TOKEN = [Net.NetworkCredential]::new('', (Read-Host 'SignPath token' -AsSecureString)).Password
```

## Each release

### 1. Changelog, version, commit

1. `CHANGELOG.md`: move the `[Unreleased]` entries into `## [X.Y.Z] - yyyy-mm-dd` and fix the
   compare links at the bottom. The date must be the day you build: `release.ps1` stops when
   the heading says Unreleased, has TODO lines, or has a date that is not today.
2. Bump `version` under `[workspace.package]` in `Cargo.toml` and run `cargo check` so
   `Cargo.lock` follows. The winget templates in `packaging/winget/template` need no edit
   (the script fills `{{VERSION}}`, `{{INSTALLER_SHA256}}` and `{{RELEASE_DATE}}`).
3. Commit: `git commit -am "Release X.Y.Z"`.

### 2. Build

```
powershell -ExecutionPolicy Bypass -File scripts/release.ps1
```

Add `-Jobs 4` if memory is tight (other builds on the box can starve rustc). To build outside
the repo: `-TargetDir C:/dev/stinkshot-bench/target-dist -OutDir C:/dev/stinkshot-bench/dist/X.Y.Z`.

It refuses a dirty tree, checks fmt, clippy and tests, builds with `--locked` into
`target-dist/` (never `target/`), checks the exe's version resource, and writes to
`dist/X.Y.Z/`:

| File | Goes to |
|---|---|
| `Stinkshot-X.Y.Z-x64-setup.exe` | site `/download/`, Forgejo release |
| `Stinkshot-X.Y.Z-x64-portable.zip` | site `/download/`, Forgejo release |
| `Stinkshot-X.Y.Z.cdx.json` (CycloneDX 1.5 SBOM) | site `/download/`, Forgejo release |
| `SHA256SUMS`, `SHA256SUMS.minisig` | site `/download/`, Forgejo release |
| `stinkshot-release.json`, `stinkshot-release.json.minisig` | site `/release/` (the update check reads these), Forgejo release |
| `RELEASE_NOTES.md` | Forgejo release description |
| `winget/manifests/s/Stinkshot/Stinkshot/X.Y.Z/` | step 9 |
| `test/` | step 3 only. Never upload it. |

At the end it reads `SHA256SUMS` and the manifest back, hashes every listed file again and
verifies both signatures with the key the app embeds. About 4 minutes with warm caches;
30 minutes cold with `-Jobs 4`.

Then it runs `scripts/site-media.ps1` on the same commit (skip with `-SkipMedia`; dry runs
skip it). That renders the app's own windows offscreen from that commit with demo History
content and default settings (a debug test build of `overlay/demo_clips.rs`; nothing on your
screen, your installed copy or its settings is touched) and writes:

| Where | What |
|---|---|
| `C:\dev\stinkshot-site\media-staging\video`, `img` | the site's clips and Settings crops. Not live: step 5 publishes them |
| `C:\dev\stinkshot-site\media-staging\shots` | whole-window pictures (History, search, Settings pages, About, editor), dark and light, lossless WebP at 125 %, with `shots.json` |
| `dist\store\listing\01-*.png` | Microsoft Store screenshots, 2560x1440 PNG, in the Store build's UI (no update check), upload in name order (`README.txt` lists them) |
| `dist\store\listing-es\01-*.png` | the same set with the UI in Spanish (Latin America), for the Spanish Store listing (a second `-StoreOnly -Lang es` run) |

Look at them before step 5: a UI change can move what a scripted click was aimed at. To
render again without a release build:
`scripts/site-media.ps1 -Version X.Y.Z -Ref <commit>` (add `-ShotsOnly` for only the
window pictures and Store screenshots, or `-StoreOnly` for only the Store set; `update-site.ps1`
refuses a partial set). `-StoreOnly -Lang es` renders the Spanish Store set into `listing-es`.

`-AllowDirty` is for dry runs: the output folder gets a `-dirty` suffix and nothing in it is
signed. The publish scripts refuse such a folder.

### 3. Smoke test

```
powershell -ExecutionPolicy Bypass -File scripts/smoke-test.ps1
```

Pass `-DistDir` and `-TargetDir` if you built outside the repo. It installs the test installer
from `dist/X.Y.Z/test/` (own AppId, folder, Apps entry and Start menu shortcut) into a temp
folder under the `smoke` profile, so your running copy and a real install are never touched.
Then it checks version, start, IPC capture, headless capture, OCR, a 3 second recording, quit
and uninstall. The exit code is the number of failed checks.

- A 600x200 color swatch shows at the top left of the primary monitor for about 4 seconds
  during the recording check. The screen must be on.
- `-Mode portable` tests the zip instead of the installer. Run both.
- If `record` fails with other Stinkshot instances running (Desktop Duplication refuses a
  second client on some setups), close them and run again. `-SkipRecord` skips the check; then
  record something by hand with the installed build before publishing.
- The PrintScreen setting is shared between profiles. Never plant `PrevPrintScreenSnipping-*`
  values by hand while your own copy runs. Only Windows Sandbox or a VM isolates everything,
  which an upgrade test over a running copy needs.

#### Manual Pass

The smoke test cannot see the screen. Before tagging, with the test install:

- [ ] `scripts\check.ps1 -Desktop`: the tests that need a real desktop session.
- [ ] Region capture, annotate, copy. Paste it somewhere.
- [ ] Record an MP4 and a GIF. Play both.
- [ ] Settings and the overlay with the keyboard only, then with Narrator on
      (`docs/notes/accessibility.md`).
- [ ] One capture on each monitor: check scaling and, on the HDR monitor, colors.
- [ ] Update from the previous release with Update Now. Use Windows Sandbox or a spare PC,
      never your own install.

### 4. Tag and push

The tag must point at the commit `release.ps1` built. It prints the commit first, and
`stinkshot-release.json` has it as `commit`.

```
git tag -a vX.Y.Z -m "Stinkshot X.Y.Z"
git push origin main
git push origin vX.Y.Z
```

### 5. Publish to stinkshot.com

Everything under `site/` in the site repo (`C:\dev\stinkshot-site`) is live the moment it is
saved. The order is: downloads, then the page, then the update manifest last, so the app
never points anyone at a file that is not there yet. `tools/update-site.ps1` does it in that
order.

The clips and screenshots are already staged in the site repo's `media-staging\` (step 2). If
you skipped that, render them now from the tag, from this repo:

```
powershell -ExecutionPolicy Bypass -File scripts/site-media.ps1 -Version X.Y.Z
```

Then, from the site repo:

```
powershell -ExecutionPolicy Bypass -File tools/update-site.ps1 -DistDir C:/dev/stinkshot/dist/X.Y.Z
powershell -ExecutionPolicy Bypass -File tools/update-site.ps1 -DistDir C:/dev/stinkshot/dist/X.Y.Z -Apply
```

The first command is a dry run: it checks the dist folder and prints every copy and every
text change, and changes nothing. The second does it:

- copies the installer, zip, SBOM, `SHA256SUMS` and `SHA256SUMS.minisig` to `site/download/`
- copies the staged clips and crops from `media-staging\` to `site/video/` and `site/img/` and
  writes `media.json`. It stops before changing anything unless they were rendered from the
  commit the manifest names (`-SkipMedia` leaves the old media; `check_site.py` then fails)
- rewrites the version, file names, file size and SHA-256 in `site/index.html`
- copies `stinkshot-release.json` and its `.minisig` to `site/release/`
- verifies that every hash in the copied `SHA256SUMS` and manifest matches the copied files
  and that the hash shown on the page is the installer's
- regenerates the Releases page and runs `tools/check_site.py` (links, versions, hashes,
  copy rules, media freshness)

**Never delete old installers from `site/download/`.** The winget manifest of each version
points at `https://stinkshot.com/download/Stinkshot-<version>-x64-setup.exe` and pins its
hash. Removing or replacing a published installer breaks `winget install` for that version
and for anyone whose winget source has not caught up. `update-site.ps1` only adds files and
refuses to overwrite a published one with different content.

Then commit the page changes in the site repo (the binaries and manifests are git-ignored).

### 6. Check the live site

Run the checks in the site repo's `DEPLOY.md`, step 4: the manifest and `SHA256SUMS` answer
200 to non-browser clients, the live manifest carries the new version and verifies, and the
installer downloads with the right hash. Then, on an installed older build with update checks
on: Settings > Privacy > Check Now should offer the new version.

### 7. Forgejo release

```
powershell -ExecutionPolicy Bypass -File scripts/publish-forgejo.ps1 -Version X.Y.Z
```

Pass `-DistDir` if you built outside the repo. It prompts for a Forgejo access token (Settings >
Applications, scope `write:repository`) without echoing it. Do not put the token in a command,
a variable you type out, or a file. The script:

- checks that tag `vX.Y.Z` is on the server and points at the manifest's `commit`, and stops if not
- creates the release with `RELEASE_NOTES.md` as its description (or reuses an existing one)
- uploads every file in the dist folder except `RELEASE_NOTES.md`, and compares each stored
  size with the local file
- exits non-zero if any file is missing or wrong. Run it again after fixing the cause; files
  already uploaded are skipped.

code.mccullough.dev is behind Anubis. If the script gets HTML instead of JSON, Anubis is
challenging it: allow `/api/` and `/releases/download/` for non-browser clients in its policy.

### 8. GitHub release

The GitHub mirror gets code and tags from Forgejo on its own, but not releases. Once the tag
shows there, publish the same files so the Releases tab is not empty:

```
cd dist/X.Y.Z
gh release create vX.Y.Z --repo beardsciences/stinkshot --verify-tag --latest --title "Stinkshot X.Y.Z" --notes-file RELEASE_NOTES.md Stinkshot-X.Y.Z-x64-setup.exe Stinkshot-X.Y.Z-x64-portable.zip Stinkshot-X.Y.Z.cdx.json SHA256SUMS SHA256SUMS.minisig stinkshot-release.json stinkshot-release.json.minisig
```

`gh` uses its stored sign-in (`gh auth login` once); no token on the command line. This adds a
release, not a commit: the mirror stays read-only for code. Then download the installer from
the GitHub release and compare its SHA-256 with `SHA256SUMS`.

### 9. winget

After step 6, once the setup URL downloads. winget-pkgs reviews can take a few days, so
`winget install` lags behind the site.

```
winget validate --manifest dist/X.Y.Z/winget/manifests/s/Stinkshot/Stinkshot/X.Y.Z
wingetcreate submit dist/X.Y.Z/winget/manifests/s/Stinkshot/Stinkshot/X.Y.Z
```

`wingetcreate` (`winget install Microsoft.WingetCreate`) signs in to GitHub in the browser the
first time and remembers it; no token on the command line. It forks microsoft/winget-pkgs,
commits the folder and opens the pull request. Without it: fork
https://github.com/microsoft/winget-pkgs, copy the folder to the same path in the fork,
commit `New version: Stinkshot.Stinkshot version X.Y.Z`, open a pull request and answer the
bot. Their pipeline downloads and scans the installer; unsigned is allowed.

The manifest's `InstallerUrl` is the stinkshot.com download, which is why step 5 keeps every
old installer.
