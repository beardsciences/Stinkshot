//! Display power state shared by every backend: the state type, the reported-state store and
//! its epoch. A backend feeds reports in through `apply`.

// `apply` and `epoch` are only called by the Windows backend so far.
#![cfg_attr(not(windows), allow(dead_code))]

use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayPower {
    /// Not reported (yet), or the notification could not be registered.
    Unknown,
    Off,
    On,
    Dimmed,
}

impl DisplayPower {
    pub(crate) fn from_raw(v: i32) -> Self {
        match v {
            0 => DisplayPower::Off,
            1 => DisplayPower::On,
            2 => DisplayPower::Dimmed,
            _ => DisplayPower::Unknown,
        }
    }
}

static STATE: AtomicI32 = AtomicI32::new(-1);
/// Incremented when the displays go off or come back (not on dimming: a dimmed display still
/// gets every frame, so a picture validated before dimming is still good).
static EPOCH: AtomicU64 = AtomicU64::new(0);

/// Apply one reported raw state. Returns true when the displays came back on from off.
pub(crate) fn apply(raw: i32) -> bool {
    let old = STATE.swap(raw, Ordering::SeqCst);
    if old >= 0 && (old == 0) != (raw == 0) {
        EPOCH.fetch_add(1, Ordering::SeqCst);
    }
    old == 0 && raw != 0
}

/// Current console display state.
pub fn state() -> DisplayPower {
    match OVERRIDE.load(Ordering::SeqCst) {
        -2 => DisplayPower::from_raw(STATE.load(Ordering::SeqCst)),
        v => DisplayPower::from_raw(v),
    }
}

/// -2: no override.
static OVERRIDE: AtomicI32 = AtomicI32::new(-2);

/// Diagnostics only: report `state` instead of the real one (None restores it). Lets a probe
/// time the duplication path while the displays are idle-off.
#[doc(hidden)]
pub fn override_state(state: Option<DisplayPower>) {
    let v = match state {
        None => -2,
        Some(DisplayPower::Unknown) => -1,
        Some(DisplayPower::Off) => 0,
        Some(DisplayPower::On) => 1,
        Some(DisplayPower::Dimmed) => 2,
    };
    OVERRIDE.store(v, Ordering::SeqCst);
}

/// Whether the displays are known to be off (Desktop Duplication is silent).
pub fn is_off() -> bool {
    state() == DisplayPower::Off
}

/// Transitions into or out of Off seen so far; a picture validated under another epoch may
/// predate a display-off period.
pub(crate) fn epoch() -> u64 {
    EPOCH.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_bump_the_epoch_and_report_wake() {
        // One test owns the statics (tests in this module run in one process).
        STATE.store(-1, Ordering::SeqCst);
        let e0 = epoch();
        assert!(!apply(1), "first report is not a wake");
        assert_eq!(epoch(), e0, "first report sets the state without a change");
        assert_eq!(state(), DisplayPower::On);
        assert!(!apply(1));
        assert_eq!(epoch(), e0, "same state again is not a change");
        assert!(!apply(0));
        assert!(is_off());
        assert_eq!(epoch(), e0 + 1);
        assert!(apply(1), "off -> on is a wake");
        assert_eq!(epoch(), e0 + 2);
        assert!(!apply(2), "on -> dimmed is not a wake");
        assert_eq!(state(), DisplayPower::Dimmed);
        assert!(!is_off());
        assert_eq!(epoch(), e0 + 2, "dimming does not invalidate pictures");
        assert!(!apply(1));
        assert_eq!(epoch(), e0 + 2, "dimmed -> on neither");
        assert!(!apply(2));
        assert!(!apply(0));
        assert_eq!(epoch(), e0 + 3, "dimmed -> off does");
        assert!(apply(2), "off -> dimmed is a wake");
        assert_eq!(epoch(), e0 + 4);
        STATE.store(-1, Ordering::SeqCst);
    }
}
