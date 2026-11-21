# Cross-platform (macOS and Linux)

Started Nov 21 2026. Windows stays the reference platform. The goal of step 1 is a clean
boundary: everything that is not Windows-specific builds and is tested on all three systems,
and every Windows-only piece sits behind a module whose API is the same everywhere.

## Where the code stands

| Crate | Off Windows |
| --- | --- |
| `core`, `stitch`, `history`, `logo` | Portable. Tested on Linux and macOS in CI. |
| `platform` | Windows code in place. Other systems compile `src/unsupported/` stand-ins. |
| `record` | Same pattern as `platform`. |
| `app` | Compiles everywhere. Windows-only calls are gated or have stand-ins. |

`core` notes:

- `Fonts::load_system` uses Segoe UI from the Windows fonts folder on Windows, and the first
  installed of Arial (macOS), DejaVu, Noto or Liberation (Linux) elsewhere. CJK and emoji
  fallbacks are Windows-only for now.
- Settings and History folders come from the `directories` crate off Windows
  (`~/.config/Stinkshot`, `~/Pictures/Stinkshot` on Linux; the Application Support folder on
  macOS).

## The boundary

Each `platform` and `record` module keeps one API. Windows implementations stay in their
files. On other systems `lib.rs` points the module at a stand-in:

```rust
#[cfg_attr(not(windows), path = "unsupported/capture.rs")]
pub mod capture;
```

Stand-ins return an error ("not supported on this OS yet") or a harmless default, never
panic. Types and pure logic used by both live in portable modules, so they are defined once.

To add a real implementation for a module on macOS: write `macos/<module>.rs`, then change
that module's attribute to pick it with `cfg_attr(target_os = "macos", path = ...)`. The rest of
the app does not change.

## Checking it from Windows

```
powershell -ExecutionPolicy Bypass -File scripts\xcheck.ps1
```

Runs clippy (`-D warnings`) on the whole workspace for `x86_64-unknown-linux-gnu` and
`aarch64-apple-darwin`. Nothing links or runs. It needs both rustup targets and LLVM in
`C:\Program Files\LLVM\bin`. Three shortcuts make a type check possible without a Linux or
macOS SDK, and none of them affect real builds:

- `LIBSQLITE3_SYS_USE_PKG_CONFIG=1` skips compiling the bundled SQLite.
- `ring` (SHA-256 for updates) is compiled with clang against the tiny headers in
  `scripts/xcheck-include/`.
- The GTK libraries behind the Linux tray icon skip their pkg-config probes.

CI (`portable` job in `.github/workflows/ci.yml`) builds and tests on real Linux and macOS
runners.

## What the stand-ins do at run time

Off Windows the app would start but cannot capture yet. Every capture and clipboard write
returns "not supported on this OS yet", the monitor list is empty, recording fails at start,
video probing and export fail, and device lists are empty. Single instance is not enforced.
Start-with-Windows, PrintScreen takeover, the IPC listener and the session watcher return
errors. In the app, `sysutil` has a stand-in (`src/unsupported/sysutil.rs`): message boxes
print to stderr, questions answer No, and files are never trashed. The Save As dialog
returns an error. Window icons, cloaking and captions do nothing.

## What each system needs next

| Area | macOS | Linux |
| --- | --- | --- |
| Screen capture, monitors | ScreenCaptureKit | X11: XGetImage/XShm. Wayland: xdg-desktop-portal Screenshot, asks each time |
| Window and element snapping | Accessibility API (needs permission) | AT-SPI. Wayland hides window positions |
| Text recognition | Vision framework | No system engine: bundle Tesseract |
| Recording | ScreenCaptureKit + AVFoundation | PipeWire + GStreamer or ffmpeg |
| Clipboard, scroll input | AppKit, CGEvent | X11 or portal; Wayland limits synthetic input |
| Overlay over all windows | NSWindow at screen-saver level | X11 override-redirect. Wayland: layer-shell where supported |

Windows-only features with no counterpart: Replace Snipping Tool, PrintScreen takeover,
Win+V and Cloud Clipboard markers, Windows 11 rounded-corner window capture.

Release needs per system: macOS builds need an Apple Developer account for signing and
notarization. Linux would ship as an AppImage or Flatpak (Flatpak fits the portal model).
