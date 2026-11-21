//! File helper stand-ins: the guarded swap needs OS file-sharing rules not written for this
//! OS yet. Parked-file recovery is shared and works everywhere.

use std::path::Path;
use std::time::SystemTime;

pub use crate::portable::files::*;

/// Not supported on this OS yet; always an error, so the caller keeps `replacement`.
pub fn replace_if_unchanged(
    _original: &Path,
    _replacement: &Path,
    _expect_len: u64,
    _expect_modified: SystemTime,
) -> anyhow::Result<Replace> {
    Err(anyhow::anyhow!(
        "replacing a file in place is not supported on this OS yet"
    ))
}

/// Does nothing on this OS yet (the thread keeps its normal priority).
pub fn background_thread() {}
