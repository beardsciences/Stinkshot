//! HDR display query stand-in: every display reads as plain SDR on this OS yet. The
//! conversion math is shared and works everywhere.

pub use crate::portable::hdr::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Always empty on this OS yet.
pub fn displays() -> Vec<DisplayColor> {
    Vec::new()
}

/// Plain SDR for every device on this OS yet.
pub fn display_for_device(device: &str) -> DisplayColor {
    DisplayColor {
        device: device.to_string(),
        advanced_color: false,
        sdr_white_nits: SCRGB_UNIT_NITS,
    }
}

/// Plain SDR with no device name on this OS yet.
pub fn display_for_monitor(_hmonitor: isize) -> DisplayColor {
    display_for_device("")
}
