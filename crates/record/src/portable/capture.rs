//! Capture result types and the pure decisions of the duplication and GDI fallback logic,
//! shared by every backend.

// Only the Windows capturer uses these so far.
#![cfg_attr(not(windows), allow(dead_code))]

use std::time::Duration;

use stinkshot_core::PxRect;

/// The capture source went away (display unplugged, resolution or arrangement changed). The
/// recorder builds a new [`Capturer`](crate::capture::Capturer) for the same region when it
/// sees this.
#[derive(Debug)]
pub struct DisplayLost;

impl std::fmt::Display for DisplayLost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "the captured display was disconnected or changed, so the recording cannot continue",
        )
    }
}

impl std::error::Error for DisplayLost {}

#[derive(Clone, Copy, Debug)]
pub struct MonitorRect {
    pub handle: isize,
    /// Virtual-desktop physical pixels.
    pub rect: PxRect,
}

/// A duplication that cannot be re-created for this long is reported as `DisplayLost`.
pub const DUP_LOST_AFTER: Duration = Duration::from_secs(1);

/// The monitor is still attached with the same handle, position and size.
pub fn monitor_unchanged(planned: &MonitorRect, now: &[MonitorRect]) -> bool {
    now.iter()
        .any(|m| m.handle == planned.handle && m.rect == planned.rect)
}

/// Whether a duplication that failed to come back should end this capturer. The secure
/// desktop (UAC prompt, lock screen) refuses duplication for as long as it shows while the
/// display itself is unchanged: a new capturer would fail the same way and end the
/// recording, so that case keeps waiting.
pub fn dup_loss_is_final(lost_for: Duration, secure_desktop: bool) -> bool {
    !secure_desktop && lost_for > DUP_LOST_AFTER
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PollStats {
    /// Frames that updated the staging picture.
    pub new_frames: u32,
    /// Frames delivered by the OS but superseded before we could use them.
    pub dropped: u32,
}

/// What to do with a GDI grab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GdiStep {
    /// Nothing new on screen: keep the stage.
    Skip,
    /// Write the grab to the stage.
    Write,
    /// The screen changes while the duplication is silent: write and turn the fallback on.
    Activate,
}

/// Decide what a GDI grab `cur` means. `prev` is the previous grab of this silent period
/// (what the stage shows), None when the stage holds a duplication picture or nothing.
pub fn gdi_step(got_frame: bool, active: bool, prev: Option<&[u8]>, cur: &[u8]) -> GdiStep {
    if !got_frame || active {
        // Seeding a static screen, or capturing with GDI: every grab is written (the
        // expensive BitBlt already ran; a tiny change such as a caret must not be lost).
        return GdiStep::Write;
    }
    match prev {
        // First probe of a silent period: the screen may have changed after the
        // duplication's last frame, and the stage still shows that older picture.
        None => GdiStep::Write,
        Some(p) if p != cur => GdiStep::Activate,
        Some(_) => GdiStep::Skip,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moved_resized_or_missing_monitor_no_longer_matches_the_plan() {
        let planned = MonitorRect {
            handle: 7,
            rect: PxRect::new(0, 0, 1920, 1080),
        };
        let other = MonitorRect {
            handle: 9,
            rect: PxRect::new(1920, 0, 1920, 1080),
        };
        assert!(monitor_unchanged(&planned, &[other, planned]));
        assert!(!monitor_unchanged(&planned, &[other]));
        assert!(!monitor_unchanged(&planned, &[]));
        let resized = MonitorRect {
            rect: PxRect::new(0, 0, 1280, 720),
            ..planned
        };
        assert!(!monitor_unchanged(&planned, &[resized]));
        let moved = MonitorRect {
            rect: PxRect::new(-1920, 0, 1920, 1080),
            ..planned
        };
        assert!(!monitor_unchanged(&planned, &[moved]));
    }

    #[test]
    fn a_lost_duplication_is_final_after_a_second_except_on_the_secure_desktop() {
        assert!(!dup_loss_is_final(Duration::from_millis(400), false));
        assert!(dup_loss_is_final(Duration::from_millis(1500), false));
        // UAC prompt or lock screen: wait it out however long it shows.
        assert!(!dup_loss_is_final(Duration::from_secs(600), true));
    }

    #[test]
    fn gdi_seed_and_active_write_every_grab() {
        let a = [1u8; 64];
        assert_eq!(gdi_step(false, false, None, &a), GdiStep::Write);
        assert_eq!(gdi_step(false, false, Some(&a), &a), GdiStep::Write);
        // Active: even an unchanged grab is written.
        assert_eq!(gdi_step(true, true, Some(&a), &a), GdiStep::Write);
    }

    #[test]
    fn first_probe_of_a_silent_period_is_written() {
        // The stage holds the duplication's last picture; the screen may have changed
        // since, so the first grab must reach the stage.
        assert_eq!(gdi_step(true, false, None, &[7u8; 64]), GdiStep::Write);
    }

    #[test]
    fn a_one_pixel_change_turns_the_fallback_on() {
        // 1920 px wide rows: a change in a column the old sampled hash skipped (x = 5).
        let w = 1920 * 4;
        let prev = vec![0u8; w * 4];
        let mut cur = prev.clone();
        cur[w * 2 + 5 * 4] = 255;
        assert_eq!(gdi_step(true, false, Some(&prev), &cur), GdiStep::Activate);
        assert_eq!(gdi_step(true, false, Some(&prev), &prev), GdiStep::Skip);
    }
}
