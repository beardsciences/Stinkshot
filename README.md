<p align="center"><img src="docs/logo-anim.webp" width="128" height="128" alt="Stinkshot logo: a glossy green gel orb"></p>

# Stinkshot

A local-first screenshot and screen-recording tool for Windows. It freezes the screen, lets you
select and annotate in place, and saves every capture to a plain folder you can use without the app.

## Features

- Region capture on a frozen snapshot of all monitors, one selection across mixed-DPI displays.
  Shift+drag adds more regions; they combine into one picture you can annotate across.
- Magnifier loupe with pixel coordinates and a hex color picker.
- Window and UI-element snapping through UI Automation. Aspect and size presets, arrow-key nudging.
- Inline annotation: arrow, line, rectangle, ellipse, pen, highlighter, text, step counters,
  callout, magnifier inset, spotlight.
- Real redaction: blur, pixelate and solid box destroy the pixels in the output.
- Annotations stay editable. Any history item reopens in the same editor.
- Window capture with Windows 11 rounded corners and alpha, optional drop shadow and padded
  background, cursor hidden by default.
- Scrolling capture: auto-scroll and stitch, with sticky header and footer handling. Waits out
  smooth scrolling, spinners and lazy-loading placeholders on its own, and crops the scrollbar.
  Full web pages: start a scrolling capture and click the page; Stinkshot snaps to the page
  itself, without the browser's tabs and toolbar (no extension, nothing leaves the PC).
- OCR with Windows.Media.Ocr: select a region, get the text on the clipboard.
- Pin any capture to the screen as an always-on-top reference.
- Screen recording to MP4 (hardware H.264 where available, AAC audio) at 30, 60 or 120 FPS, or
  size-optimized GIF.
  System audio and microphone, pause/resume, click and key visualizer, webcam bubble,
  3-2-1 countdown, trim before saving.
- History: plain image files plus a small index, background OCR, full-text search, filters by app,
  window title and date, retention by age or size (starred items are kept).
- Optional Snipping Tool replacement: Win+Shift+S, Win+Shift+R and PrtSc open Stinkshot.
- Fluent look with dark and light themes, tray icon, portable single exe.

## Privacy

Everything stays on your machine. There are no upload targets and no telemetry.

The only network call Stinkshot can make is an update check that fetches signed
release info from stinkshot.com. It is off until you turn it on in Settings, Privacy, and you can
run it manually with Check Now. It sends only what any web request carries (your IP address and a
`Stinkshot/<version>` user agent), nothing about your captures. Release info is trusted only after its
minisign signature checks out against the key built into the app. When an update is out, Update
Now (only when you click it) downloads the installer from stinkshot.com, runs it only if it matches
the SHA-256 in the signed release info, and restarts Stinkshot on the new version. See SECURITY.md.

## Install

Download: https://stinkshot.com/

Requirements: 64-bit Windows 10 version 2004 (build 19041) or later, or Windows 11. Windows 10
is less tested than Windows 11.

- Installer: `Stinkshot-<version>-x64-setup.exe`. Installs for your user only (no admin) to
  `%LOCALAPPDATA%\Programs\Stinkshot`. Silent: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART`, plus
  `/TASKS=autostart` to start with Windows.
- Portable: unzip `Stinkshot-<version>-x64-portable.zip` anywhere and run `stinkshot.exe`. It
  needs no install, but it is not self-contained: settings, log and cache still go to
  `%APPDATA%\Stinkshot` and `%LOCALAPPDATA%\Stinkshot`, and captures to the history folder.
  There is no Update Now: when an update is out, it opens the download page.
- Winget: `winget install Stinkshot.Stinkshot`. Winget can lag a few days behind a release.

Releases are not code-signed yet, so Windows SmartScreen may show "Windows protected your PC"
the first time you run the installer or the portable exe. Verify the download first (each
release has `SHA256SUMS` signed with minisign; see SECURITY.md, Verifying a download), then
choose More info, then Run anyway.

Uninstall: Settings > Apps > Installed apps > Stinkshot, or run
`%LOCALAPPDATA%\Programs\Stinkshot\unins000.exe` (add `/VERYSILENT` for no prompts). It keeps
your captures, history and settings. Portable: quit Stinkshot and delete the folder.

## Hotkeys

These are the defaults. All can be changed in Settings.

| Action | Default |
| --- | --- |
| Region capture | PrintScreen |
| Window capture | Alt+PrintScreen |
| Full screen | Ctrl+PrintScreen |
| Repeat last region | Shift+PrintScreen |
| Scrolling capture | Ctrl+Alt+S |
| OCR region | Ctrl+Alt+T |
| Record MP4 | Ctrl+Alt+R |
| Record GIF | Ctrl+Alt+G |

Taking over PrintScreen from Windows is a setting and is restored when you turn it off.

### Replace Snipping Tool

Settings > Hotkeys > Replace Snipping Tool (off by default) makes Stinkshot answer the Snipping
Tool shortcuts: Win+Shift+S opens region capture, Win+Shift+R starts Record MP4, and PrtSc is taken
over too. Only those exact key presses are intercepted; nothing else you type is touched. To undo
it, turn the setting off (or quit Stinkshot): the shortcuts go back to the Snipping Tool and the
PrintScreen setting is restored. The Snip tile in Quick Settings, the pen button and apps that open
the `ms-screenclip:` link still start the Snipping Tool, because Windows offers no supported way
to redirect them.

## CLI

With no arguments Stinkshot starts in the tray. Subcommands act on the running instance and start
it if needed.

```
stinkshot [--tray]
stinkshot region
stinkshot window [--out FILE] [--cursor]
stinkshot full [--out FILE] [--monitor N] [--cursor]
stinkshot scroll
stinkshot ocr [--file IMAGE]
stinkshot repeat
stinkshot text [--paragraph]
stinkshot pick-color
stinkshot record
stinkshot gif
stinkshot history
stinkshot settings [--page NAME]
stinkshot update
stinkshot quit
stinkshot --version
```

- `window --out`, `full --out` and `ocr --file` run headless and exit. The output format
  follows the file extension (png, jpg, webp).
- `--monitor N` is 1-based, in system order. Without it, `full` takes the monitor under the
  cursor.
- `text` captures the text selected in the focused app; `--paragraph` grows the selection to
  whole paragraphs first.
- `pick-color` picks a color anywhere on screen and copies it.
- `record` and `gif` toggle: run again to stop.
- `settings --page NAME` opens Settings on a page: general, hotkeys, capture, editor,
  recording, history, privacy, help or about.
- `update` checks for a newer release and installs it. It needs Settings > Privacy > Check for
  Updates on.
- `quit` asks the running instance to exit and waits for it; it never starts one.
- `--tray` starts in the tray only (start with Windows uses it). Plain `stinkshot` while it is
  already running opens History.

The others are handy for scripts, AutoHotkey and Stream Deck.

## Files and config

- Config: `%APPDATA%\Stinkshot\config.toml` (the settings window writes the same file).
- Log: `%APPDATA%\Stinkshot\stinkshot.log`.
- Cache: `%LOCALAPPDATA%\Stinkshot\cache` holds the About page's pre-rendered logo animation
  (a few MB, safe to delete; it is rebuilt in the background).
- Captures: the history folder set in Settings, `%USERPROFILE%\Pictures\Stinkshot` by default.
  Annotation data lives beside each image in a `.stinkshot` subfolder, together with the
  search index and thumbnails.
- Update downloads: `%TEMP%\Stinkshot-update` (the installer Update Now fetches).
- Program (installer): `%LOCALAPPDATA%\Programs\Stinkshot`.

## Redaction and Editable Originals

Redaction (Blur, Pixelate and Redact boxes) destroys the pixels in the exported image. With Keep
Editable Originals on (the default), Stinkshot also keeps an editable copy of every annotated
capture in `<history folder>\.stinkshot\edit` so you can change the annotations later. That copy
has the redactions applied too: whatever sits under a Blur, Pixelate or Redact box, including
pasted pictures and text, is flattened and never stored. It does keep what other annotations
cover (an arrow or a filled rectangle is not a redaction), so use a Redact box for anything
secret. Copies written by 0.1.4 and earlier can still hold a pasted picture under a redaction
until the capture is saved again. To keep no editable copies, turn off Settings > History >
Keep Editable Originals and remove the `edit` folder.

## Build

Requirements: Windows 10 version 2004 or later, or Windows 11; Rust 1.95 or newer (MSVC
toolchain); Visual Studio Build Tools with the C++ workload and a Windows SDK.

```
cargo build --release
```

The binary is `target\release\stinkshot.exe`. Tests:

```
cargo test --workspace
cargo test --workspace -- --ignored --test-threads=1
```

The second command runs desktop integration tests that capture the real screen, use the clipboard
and install input hooks, so run it on an interactive session.

The workspace is split into `core` (image, annotation render, config), `stitch`, `history`,
`platform` (Windows APIs), `record`, `logo` (icon and About animation renderer) and `app` (UI).
See `docs/ARCHITECTURE.md`.

Release builds (installer, zip, SBOM, checksums, signed update manifest): `scripts\release.ps1`,
see `docs/RELEASING.md`. Contributing: `CONTRIBUTING.md`. Changes: `CHANGELOG.md`.

Source and issues live at https://code.mccullough.dev/christian/stinkshot. The GitHub
repository is a read-only mirror: report issues and send changes on code.mccullough.dev.

## License

GPL-3.0-or-later. See `LICENSE`.
