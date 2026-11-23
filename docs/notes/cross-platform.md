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
panic. The error is `stinkshot_platform::Unsupported` (`unsupported("what")`), recognized by
`is_unsupported` and, after crossing a thread as text, `is_unsupported_text`. Types and pure
logic used by both live in portable modules, so they are defined once.

Each module says what it implements: `pub const SUPPORTED: bool` (true in the Windows file,
false in the stand-in). `system` has one per feature (`AUTOSTART_SUPPORTED`,
`PRINTSCREEN_SUPPORTED`, `SINGLE_INSTANCE_SUPPORTED`, `SESSION_EVENTS_SUPPORTED`,
`OPEN_SUPPORTED`), `clipboard` adds `HISTORY_MARKERS_SUPPORTED`, and `record` sums its modules
up as `RECORDING_SUPPORTED`, `VIDEO_FILES_SUPPORTED`, `MICROPHONES_SUPPORTED` and
`WEBCAMS_SUPPORTED`.

To add a real implementation for a module on macOS: write `macos/<module>.rs` with
`SUPPORTED = true`, then change that module's attribute to pick it with
`cfg_attr(target_os = "macos", path = ...)`. The rest of the app does not change.

## Contract tests

`crates/platform/tests/contract.rs` and `crates/record/tests/contract.rs` hold the boundary
contract and run against whatever implementation the OS compiles:

- `shape`: every per-OS public function and method pinned as a fn pointer, so a signature
  change fails to compile on that OS.
- `everywhere`: safe with any implementation: bad input (empty rect, null window, missing
  file, 0x0 recording region) gives an error or nothing and writes no file; monitor lists are
  consistent with the virtual bounds; fixed key labels match. On Windows nothing captures,
  touches the clipboard, installs hooks, writes the registry or opens windows.
- `stand_ins`: for every `SUPPORTED` that is false, each call gives an `Unsupported` error or
  the documented default. A real implementation flips its constant and these checks step
  aside for it.

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

## Smoke test

```
stinkshot --smoke-test
```

Hidden from `--help` (`crates/app/src/shell/smoke.rs`). Starts the app as far as is safe
without a desktop session: a fresh temporary profile (`APPDATA` and `USERPROFILE` pointed
at a temp folder, `STINKSHOT_PROFILE=smoke`), config, language, fonts, history, the wgpu
device from `gpu.rs`, eframe with the hidden root viewport, and the Settings window as an
immediate viewport. It passes after 5 frames painted in Settings' own native window and
exits 0. A failure, panic or more than 20 s exits 1 with the reason on stderr. It skips the
single-instance lock, IPC, tray, hotkeys, PrintScreen takeover, autostart, capture warm-up
and background workers, so it never touches an installed instance and is safe to run
locally from a debug build.

CI runs it on Linux under `xvfb-run` with lavapipe (`WGPU_BACKEND=vulkan`). The macOS step
is `continue-on-error` until it has passed once.

## What the stand-ins do at run time

Off Windows the app would start but cannot capture yet. Every capture and clipboard write
returns "not supported on this OS yet", the monitor list is empty, recording fails at start,
video probing and export fail, and device lists are empty. Single instance is not enforced.
Start-with-Windows, PrintScreen takeover, the IPC listener and the session watcher return
errors. In the app, `sysutil` has a stand-in (`src/unsupported/sysutil.rs`): message boxes
print to stderr, questions answer No, and files are never trashed. The Save As dialog
returns an error. Window icons, cloaking and captions do nothing.

How the app handles that:

- An attempt that fails with `Unsupported` shows its usual failure toast once, with the body
  "This is not supported on this system yet" (`errors::Cause::Unsupported`,
  `shell.error.unsupported`) and no log hint. No text recognition says Text Recognition Not
  Available instead of the Windows language hint.
- Settings hides what has no implementation: Start With Windows, the Windows Integration
  group (Replace Snipping Tool, Take Over PrintScreen, the Win+Shift+S and Win+Shift+R rows)
  and Add Copies to Clipboard History. Hotkeys treats Replace Snipping Tool and the
  PrintScreen takeover as off there, whatever the config says, so no hotkey error appears at
  start.
- Start-up steps that are expected to fail there (IPC listener, session watcher, OCR, tray
  icon, element snapping) log at info, not warn or error.
- Linux has no tray icon yet: tray-icon builds GTK menus, which needs GTK initialized and a
  GTK main loop. `Tray::new` returns `Unsupported` there instead of panicking, and the app
  shows its Tray Icon Unavailable toast.

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
winit loads `libxkbcommon-x11.so` at run time on X11 and panics without it, so a Linux
package must depend on `libxkbcommon-x11-0` (found by the CI smoke test).
