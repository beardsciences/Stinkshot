//! Platform layer, implemented for Windows (other OSes get stand-ins, see "Per-OS boundary"
//! below). Every function here works in PHYSICAL pixels in virtual-desktop
//! coordinates (the process is Per-Monitor-V2 DPI aware; call `dpi::init` first thing in
//! main). Window handles cross the API as `Hwnd` (raw isize) so callers don't need the
//! `windows` crate.
//!
//! Threading: functions are callable from any thread unless noted. COM is initialized
//! per-thread internally where needed (MTA for workers; UIA and OCR are used off the UI
//! thread by the app).
//!
//! # Per-OS boundary
//!
//! The public API (module paths, names and signatures) is the same on every OS:
//!
//! - Windows implementations stay in place (`src/<module>.rs`) and only build on Windows.
//! - On other OSes each such module is swapped for a stand-in in `src/unsupported/` (see the
//!   `path` attributes below). Stand-ins never panic: calls that return a `Result` return
//!   an "is not supported on this OS yet" error, other calls return a harmless default
//!   (empty list, None, false, (0, 0)) or do nothing. Types that only come from a failed
//!   constructor (hooks, OCR engine, overlay) cannot be created there. The error is an
//!   [`Unsupported`], so callers can tell it apart ([`is_unsupported`]).
//! - Each such module has `pub const SUPPORTED: bool` (`system` has one per feature): true for a
//!   real implementation, false for a stand-in. The app hides settings by it, and
//!   `tests/contract.rs` checks stand-in behavior by it, so a new implementation sets it to
//!   true.
//! - Data types and pure logic that do not depend on the OS live once in `src/portable/`
//!   and are re-exported from the module that always held them, with their unit tests, so
//!   those tests run on every OS.
//! - `hang`, `occlusion` and `wgc_memo` have no OS calls and build everywhere as they are.
//!
//! To give a module a real macOS or Linux implementation, add a file for it with the same
//! public items (for example `src/macos/capture.rs`, starting with
//! `pub use crate::portable::capture::*;`) and narrow the stand-in to the OSes still
//! missing:
//!
//! ```ignore
//! #[cfg_attr(target_os = "macos", path = "macos/capture.rs")]
//! #[cfg_attr(not(any(windows, target_os = "macos")), path = "unsupported/capture.rs")]
//! pub mod capture;
//! ```
//!
//! Move any logic the new file shares with the Windows one into `portable/` first. An
//! OS-specific dependency goes under `[target.'cfg(..)'.dependencies]` in Cargo.toml.

/// OS-independent types and logic, re-exported by the per-OS modules.
mod portable {
    pub(crate) mod capture;
    pub(crate) mod display_power;
    pub(crate) mod files;
    pub(crate) mod hdr;
    pub(crate) mod input;
    pub(crate) mod monitors;
    pub(crate) mod ocr;
    pub(crate) mod quick_overlay;
    pub(crate) mod snip;
    pub(crate) mod system;
    pub(crate) mod text_selection;
    pub(crate) mod uia;
    pub(crate) mod window_util;
    pub(crate) mod windows_enum;
}

#[cfg_attr(not(windows), path = "unsupported/capture.rs")]
pub mod capture;
#[cfg_attr(not(windows), path = "unsupported/clipboard.rs")]
pub mod clipboard;
#[cfg_attr(not(windows), path = "unsupported/display_power.rs")]
pub mod display_power;
#[cfg_attr(not(windows), path = "unsupported/dpi.rs")]
pub mod dpi;
#[cfg_attr(not(windows), path = "unsupported/files.rs")]
pub mod files;
pub mod hang;
#[cfg_attr(not(windows), path = "unsupported/hdr.rs")]
pub mod hdr;
#[cfg_attr(not(windows), path = "unsupported/input.rs")]
pub mod input;
#[cfg_attr(not(windows), path = "unsupported/monitors.rs")]
pub mod monitors;
pub mod occlusion;
#[cfg_attr(not(windows), path = "unsupported/ocr.rs")]
pub mod ocr;
#[cfg_attr(not(windows), path = "unsupported/quick_overlay.rs")]
pub mod quick_overlay;
#[cfg_attr(not(windows), path = "unsupported/snip.rs")]
pub mod snip;
#[cfg_attr(not(windows), path = "unsupported/system.rs")]
pub mod system;
#[cfg_attr(not(windows), path = "unsupported/text_selection.rs")]
pub mod text_selection;
#[cfg_attr(not(windows), path = "unsupported/uia.rs")]
pub mod uia;
#[cfg_attr(not(windows), path = "unsupported/util.rs")]
mod util;
pub mod wgc_memo;
#[cfg_attr(not(windows), path = "unsupported/window_util.rs")]
pub mod window_util;
#[cfg_attr(not(windows), path = "unsupported/windows_enum.rs")]
pub mod windows_enum;

pub use util::keep_mta_alive;

/// The error a stand-in returns for a feature this OS has no implementation of yet. Its text
/// always contains [`UNSUPPORTED_MARK`], so an error that crossed a thread as a string is still
/// recognized ([`is_unsupported_text`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unsupported(pub &'static str);

/// The fixed part of every [`Unsupported`] message.
pub const UNSUPPORTED_MARK: &str = "is not supported on this OS yet";

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {UNSUPPORTED_MARK}", self.0)
    }
}

impl std::error::Error for Unsupported {}

/// An [`Unsupported`] error for `what` (for example "screen capture").
pub fn unsupported(what: &'static str) -> anyhow::Error {
    anyhow::Error::new(Unsupported(what))
}

/// Whether `e` (or anything in its chain) says the feature is not supported on this OS yet.
pub fn is_unsupported(e: &anyhow::Error) -> bool {
    e.chain().any(|c| c.downcast_ref::<Unsupported>().is_some())
        || is_unsupported_text(&format!("{e:#}"))
}

/// [`is_unsupported`] for an error passed on as text.
pub fn is_unsupported_text(e: &str) -> bool {
    e.contains(UNSUPPORTED_MARK)
}

/// Raw HWND value.
pub type Hwnd = isize;

pub use stinkshot_core::{PxPoint, PxRect, Rgba};
