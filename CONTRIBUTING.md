# Contributing

Thanks for helping. Stinkshot is a small project with firm rules. Reading them first saves a
round trip.

## Ground Rules

- **Local only.** No uploads, share targets, cloud destinations, accounts or telemetry, ever.
  The update check in `crates/app/src/update.rs` is the only network code. Pull requests that
  add network access anywhere else are closed.
- **Windows first.** The minimum is Windows 10 version 2004 (build 19041), which hiding
  Stinkshot's own windows from captures needs. Windows 11 is the main target and Windows 10 is
  less tested. Keep OS calls inside `crates/platform` and `crates/record` so other platforms stay
  possible later.
- **UI copy.** Title Case for labels (buttons, menus, headings, setting names), sentence case
  for descriptions. One short line per description. Errors say what to do. Use the tokens and
  widgets in `crates/app/src/theme.rs`.
- **No AI attribution** in commits, code, comments or docs.

## Building

Requirements: Rust 1.95+ (MSVC toolchain), Visual Studio Build Tools with the C++ workload and
a Windows SDK.

```
cargo build
cargo test --workspace
```

Before sending a change, run every check:

```
powershell -ExecutionPolicy Bypass -File scripts/check.ps1
```

It runs `cargo fmt`, clippy with warnings as errors, the tests, the UI string rules and
`cargo audit`. CI runs the same script on every push. Add `-Desktop` for the tests that need a
real desktop session (screen, clipboard, input hooks). They open windows, so leave the PC alone
while they run.

The code also has to keep building for Linux and macOS (docs/notes/cross-platform.md). Check
that from Windows with `scripts/xcheck.ps1`. New Windows API calls go in `crates/platform` or
`crates/record` behind their per-OS modules, or get a `#[cfg(windows)]` gate in the app.

The minimum Rust version is the `rust-version` in `Cargo.toml`. A bump is noted in the changelog.

To run a dev build beside an installed Stinkshot without touching its config, history,
hotkeys or registry values, set a profile: `$env:STINKSHOT_PROFILE = "dev"`.

## Where Things Live

`docs/ARCHITECTURE.md` maps the crates and their contracts. Each area keeps notes in
`docs/notes/<area>.md`; update the one you touch, including manual verification steps.

## Commits and Pull Requests

- One topic per pull request, with a short description of what changed and how you checked it.
- Add a line to the `[Unreleased]` section of `CHANGELOG.md` for anything a user would notice.
- By contributing you agree your work is licensed under GPL-3.0-or-later (see `LICENSE`).

## Releases

Maintainers only: `docs/RELEASING.md`.

## Maintenance

Stinkshot has one maintainer, who reviews and decides every change. Expect a reply within a
week. The license is GPL-3.0-or-later, so you are free to fork.
