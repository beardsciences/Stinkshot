//! Quick overlay input state and its pure click/drag rules, shared by every backend.

// `button_up` and `capture_lost` are only called by the Windows window procedure so far.
#![cfg_attr(not(windows), allow(dead_code))]

use stinkshot_core::PxPoint;

/// Pointer travel (px) below which a press and release is a click, not a drag.
const CLICK_SLOP: i32 = 4;

/// What the user did in the quick overlay so far (virtual-desktop px).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuickInput {
    /// Where the left button went down, if it did.
    pub press: Option<PxPoint>,
    /// Last pointer position seen.
    pub cursor: Option<PxPoint>,
    /// The button is still down.
    pub down: bool,
    /// Where the button went up again (the end of a finished drag).
    pub release: Option<PxPoint>,
    /// Where a plain click (press and release without a drag) landed: the overlay takes the
    /// window highlighted under it, like a click in the region overlay.
    pub clicked: Option<PxPoint>,
    /// Esc was pressed.
    pub cancelled: bool,
}

/// The left button went up at `up`. Ignored unless this window saw it go down: a button
/// pressed before the window was shown (in the app underneath) is not a click here.
pub(crate) fn button_up(i: &mut QuickInput, up: PxPoint) {
    i.cursor = Some(up);
    if !i.down {
        return;
    }
    i.down = false;
    // A click (no drag) takes the highlighted window once the overlay is up.
    let moved = i
        .press
        .is_some_and(|p| (p.x - up.x).abs().max((p.y - up.y).abs()) >= CLICK_SLOP);
    if moved {
        i.release = Some(up);
    } else {
        i.press = None;
        i.clicked = Some(up);
    }
}

/// The mouse capture went elsewhere, so the button-up may never arrive here. While the
/// button is still held the press stays down: the real overlay takes the foreground (and
/// with it the capture) during the handover and continues that drag. Once it is no longer
/// held, the press ends at the last pointer position seen.
pub(crate) fn capture_lost(i: &mut QuickInput, button_held: bool) {
    if i.down
        && !button_held
        && let Some(at) = i.cursor.or(i.press)
    {
        button_up(i, at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i32, y: i32) -> PxPoint {
        PxPoint { x, y }
    }

    fn pressed(at: PxPoint) -> QuickInput {
        QuickInput {
            press: Some(at),
            cursor: Some(at),
            down: true,
            ..Default::default()
        }
    }

    #[test]
    fn button_up_without_a_press_is_not_a_click() {
        let mut i = QuickInput::default();
        button_up(&mut i, pt(50, 60));
        assert_eq!(i.clicked, None);
        assert_eq!(i.release, None);
        assert_eq!(i.press, None);
        assert_eq!(i.cursor, Some(pt(50, 60)));
        // A second up after a finished click changes nothing either.
        let mut i = pressed(pt(1, 1));
        button_up(&mut i, pt(2, 2));
        button_up(&mut i, pt(90, 90));
        assert_eq!(i.clicked, Some(pt(2, 2)));
        assert_eq!(i.release, None);
    }

    #[test]
    fn button_up_after_a_press_is_a_click_or_a_drag() {
        let mut i = pressed(pt(10, 10));
        button_up(&mut i, pt(11, 12));
        assert!(!i.down);
        assert_eq!(
            (i.clicked, i.press, i.release),
            (Some(pt(11, 12)), None, None)
        );

        let mut i = pressed(pt(10, 10));
        button_up(&mut i, pt(40, 10));
        assert!(!i.down);
        assert_eq!(
            (i.clicked, i.press, i.release),
            (None, Some(pt(10, 10)), Some(pt(40, 10)))
        );
    }

    #[test]
    fn capture_lost_keeps_a_held_press_and_ends_a_released_one() {
        // Still held (the handover to the real overlay): the drag stays in progress.
        let mut i = pressed(pt(10, 10));
        i.cursor = Some(pt(80, 90));
        capture_lost(&mut i, true);
        assert!(i.down);
        assert_eq!((i.press, i.release), (Some(pt(10, 10)), None));
        // Released somewhere this window did not see: ends at the last position.
        capture_lost(&mut i, false);
        assert!(!i.down);
        assert_eq!(i.release, Some(pt(80, 90)));
        // Nothing pressed: nothing to end.
        let mut i = QuickInput::default();
        capture_lost(&mut i, false);
        assert_eq!(i, QuickInput::default());
    }
}
