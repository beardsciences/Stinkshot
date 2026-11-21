<p align="center"><img src="docs/logo-anim.webp" width="128" height="128" alt="Stinkshot logo: a glossy green gel orb"></p>

# Stinkshot

A local-first screenshot and screen-recording tool for Windows. It freezes the screen, lets you
select and annotate in place, and saves every capture to a plain folder you can use without the app.

## Features

Capture

- Region capture on a frozen snapshot of all monitors, with one selection across mixed-DPI
  displays. Shift+drag adds regions that combine into one picture.
- Window and UI-element snapping through UI Automation. Aspect and size presets, arrow-key
  nudging.
- Magnifier loupe with pixel coordinates. Pick Color copies hex, RGB or HSL.
- Window capture with Windows 11 rounded corners and alpha, optional drop shadow and padded
  background. The cursor is left out by default.
- Region After Delay: waits a few seconds before freezing, so open menus and tooltips can be
  captured. From the tray, a hotkey or `stinkshot region --delay N`.
- Scrolling capture: auto-scroll and stitch, with sticky header and footer handling. It starts
  from the top of the page, waits out smooth scrolling, spinners and lazy-loading placeholders,
  and crops the scrollbar.
- Full web pages: start a scrolling capture and click the page. Stinkshot snaps to the page
  without the browser's tabs and toolbar. No extension needed.
- Copy Text (OCR) with Windows.Media.Ocr: select a region, get the text on the clipboard.
- Optional Snipping Tool replacement: Win+Shift+S, Win+Shift+R and PrtSc open Stinkshot.

Editor

- Annotation in place: arrow, line, rectangle, ellipse, pen, highlighter, text, step counters,
  callout, magnifier inset, spotlight.
- Redaction: Blur, Pixelate and Redact boxes destroy the pixels in the output. Find Sensitive
  Info spots emails, keys and numbers to redact.
- Measure: a ruler for pixel distances and angles.
- Select Text: select words in a capture and copy them.
- Codes and Text: reads QR codes and barcodes, and copies OCR text as a table.
- Beautify: frame a capture with a background, padding, corners and shadow.
- Editor Window: any History item, toast or pin reopens in a normal window with its annotations
  editable. Several can be open at once.
- Canvas: Add to Canvas in History arranges captures side by side. Ctrl+V pastes a picture from
  any app.
- Pin any capture to the screen as an always-on-top reference.

Recording

- MP4 at 30, 60 or 120 FPS with hardware H.264 where available and AAC audio, or size-optimized
  GIF.
- System audio and microphone, pause and resume, click and key visualizer, webcam bubble, 3-2-1
  countdown, trim before saving.

History and App

- History: plain image files plus a small index. Full-text search through background OCR, with
  filters by app, window title and date.
- Retention by age or size. Starred items are kept.
- Update Now installs a new release when you click it. Update checks are off by default.
- Fluent look with dark and light themes, tray icon, portable single exe.

## Privacy

Captures never leave your PC. There are no uploads, accounts or telemetry.

The only network request is the update check, off until you turn it on in Settings > Privacy. It
fetches signed release info from stinkshot.com at most once a day. It sends only what any web
request carries: your IP address and a `Stinkshot/<version>` user agent. Update Now runs only when
you click it, and runs the installer only if it matches the SHA-256 in the signed release info.
`SECURITY.md` has the full promise.

## Install

Download: https://stinkshot.com/

Requirements: 64-bit Windows 10 version 2004 (build 19041) or later, or Windows 11. Windows 10
is less tested than Windows 11.

- Installer: `Stinkshot-<version>-x64-setup.exe`. Installs for your user only, without admin
  rights, to `%LOCALAPPDATA%\Programs\Stinkshot`. Silent: `/VERYSILENT /SUPPRESSMSGBOXES
  /NORESTART`, plus `/TASKS=autostart` to start with Windows.
- Portable: unzip `Stinkshot-<version>-x64-portable.zip` anywhere and run `stinkshot.exe`. No
  install needed. Settings, log and cache still go to `%APPDATA%\Stinkshot` and
  `%LOCALAPPDATA%\Stinkshot`, and captures to the history folder. For updates it opens the
  download page.
- Winget: `winget install Stinkshot.Stinkshot`. Winget can lag a few days behind a release.

Releases are not code-signed yet, so Windows SmartScreen may show "Windows protected your PC"
the first time you run the installer or the portable exe. Verify the download first: see
Verifying a Download in `SECURITY.md`. Then choose More info, then Run anyway.

Uninstall: Settings > Apps > Installed apps > Stinkshot, or run
`%LOCALAPPDATA%\Programs\Stinkshot\unins000.exe`. Add `/VERYSILENT` for no prompts. It keeps
your captures, history and settings. Portable: quit Stinkshot and delete the folder.

## Hotkeys

These are the defaults. All can be changed in Settings.

| Action | Default |
| --- | --- |
| Region | PrintScreen |
| Window | Alt+PrintScreen |
| Full Screen | Ctrl+PrintScreen |
| Repeat Last Region | Shift+PrintScreen |
| Scrolling | Ctrl+Alt+S |
| Copy Text (OCR) | Ctrl+Alt+T |
| Record MP4 | Ctrl+Alt+R |
| Record GIF | Ctrl+Alt+G |

Take Over PrintScreen is a setting. Turning it off gives the key back to Windows.

### Replace Snipping Tool

Off by default, in Settings > Hotkeys. Win+Shift+S opens region capture, Win+Shift+R starts
Record MP4 and PrtSc opens Stinkshot. Only those key presses are intercepted. Turning it off or
quitting Stinkshot gives the shortcuts back to Snipping Tool and restores the Take Over
PrintScreen setting. The Snip tile in Quick Settings, the pen button and `ms-screenclip:` links
still open Snipping Tool. Windows has no supported way to redirect them.

## CLI

With no arguments Stinkshot starts in the tray. Subcommands act on the running instance and start
it if needed.

```
stinkshot [--tray]
stinkshot region [--delay SECONDS]
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
  follows the file extension: png, jpg or webp.
- `--monitor N` is 1-based, in system order. Without it, `full` takes the monitor under the
  cursor.
- `text` captures the text selected in the focused app. `--paragraph` grows the selection to
  whole paragraphs first.
- `pick-color` picks a color anywhere on screen and copies it.
- `record` and `gif` toggle: run again to stop.
- `settings --page NAME` opens Settings on a page: general, hotkeys, capture, editor,
  recording, history, privacy, help or about.
- `update` checks for a newer release and installs it. It needs Check for Updates on in
  Settings > Privacy.
- `quit` asks the running instance to exit and waits for it. It never starts one.
- `--tray` starts in the tray only. Start With Windows uses it. Plain `stinkshot` while it is
  already running opens History.

All of these work from scripts, AutoHotkey and Stream Deck.

## Files and Config

- Config: `%APPDATA%\Stinkshot\config.toml`. The settings window writes the same file.
- Log: `%APPDATA%\Stinkshot\stinkshot.log`.
- Cache: `%LOCALAPPDATA%\Stinkshot\cache`. A few MB for the About page's logo animation. Safe to
  delete. It is rebuilt in the background.
- Captures: the history folder set in Settings, `%USERPROFILE%\Pictures\Stinkshot` by default.
  Annotation data lives beside each image in a `.stinkshot` subfolder, together with the
  search index and thumbnails.
- Update downloads: `%TEMP%\Stinkshot-update`, where Update Now puts the installer.
- Installed program: `%LOCALAPPDATA%\Programs\Stinkshot`.

## Redaction and Editable Originals

- Blur, Pixelate and Redact boxes destroy the pixels in the exported image.
- With Keep Editable Originals on, the default, Stinkshot also keeps an editable copy of every
  annotated capture in `<history folder>\.stinkshot\edit` so you can change the annotations
  later. That copy has the redactions applied too. Whatever sits under a Blur, Pixelate or
  Redact box, including pasted pictures and text, is flattened and never stored.
- The copy does keep what other annotations cover. An arrow or a filled rectangle is not a
  redaction, so use a Redact box for anything secret.
- Copies written by 0.1.4 and earlier can still hold a pasted picture under a redaction until
  the capture is saved again.
- To keep no editable copies, turn off Settings > History > Keep Editable Originals and remove
  the `edit` folder.

## Build

Requirements:

- Windows 10 version 2004 or later, or Windows 11.
- Rust 1.95 or newer with the MSVC toolchain.
- Visual Studio Build Tools with the C++ workload and a Windows SDK.

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

The workspace crates are `core`, `stitch`, `history`, `platform`, `record`, `logo` and `app`.
`docs/ARCHITECTURE.md` describes them.

Release builds: `scripts\release.ps1`, described in `docs/RELEASING.md`. See also
`CONTRIBUTING.md` and `CHANGELOG.md`.

Source and issues live at https://code.mccullough.dev/christian/stinkshot. The GitHub
repository is a read-only mirror. Report issues and send changes on code.mccullough.dev.

## License

GPL-3.0-or-later. See `LICENSE`.
