//! Text selection result type and the pure rectangle math, shared by every backend.

use stinkshot_core::PxRect;

/// The on-screen rectangles of a text selection.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSelection {
    /// One rectangle per visible line (physical px, virtual-desktop coordinates).
    pub rects: Vec<PxRect>,
    /// Bounding rectangle of the element holding the text. Lines scrolled partly out of a
    /// view can report rectangles past it.
    pub element: PxRect,
}

/// UIA's flat `[left, top, width, height, ...]` doubles as whole-pixel rects (outward
/// rounded). Empty or non-finite entries are dropped.
pub fn rects_from_doubles(v: &[f64]) -> Vec<PxRect> {
    v.chunks_exact(4)
        .filter_map(|c| {
            let (l, t, w, h) = (c[0], c[1], c[2], c[3]);
            if !(l.is_finite() && t.is_finite() && w.is_finite() && h.is_finite())
                || w <= 0.0
                || h <= 0.0
            {
                return None;
            }
            let x0 = l.floor() as i32;
            let y0 = t.floor() as i32;
            let x1 = (l + w).ceil() as i32;
            let y1 = (t + h).ceil() as i32;
            Some(PxRect::new(x0, y0, x1 - x0, y1 - y0))
        })
        .collect()
}

/// The capture rectangle for a selection: the union of its line `rects`, grown by `pad` px,
/// kept inside the text element (plus the padding, when the element is known), then clipped
/// to the monitors (bounding box of the parts on each monitor). None when nothing is left.
pub fn capture_rect(
    rects: &[PxRect],
    element: PxRect,
    pad: i32,
    monitors: &[PxRect],
) -> Option<PxRect> {
    let mut it = rects.iter().filter(|r| r.w > 0 && r.h > 0);
    let first = *it.next()?;
    let union = it.fold(first, |acc, r| acc.union(r));
    let pad = pad.max(0);
    let mut grown = PxRect::new(
        union.x - pad,
        union.y - pad,
        union.w + 2 * pad,
        union.h + 2 * pad,
    );
    if element.w > 0 && element.h > 0 {
        let limit = PxRect::new(
            element.x - pad,
            element.y - pad,
            element.w + 2 * pad,
            element.h + 2 * pad,
        );
        grown = grown.intersect(&limit)?;
    }
    let mut parts = monitors.iter().filter_map(|m| grown.intersect(m));
    let first = parts.next()?;
    let out = parts.fold(first, |acc, r| acc.union(&r));
    (out.w > 0 && out.h > 0).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_become_outward_rounded_rects() {
        let r = rects_from_doubles(&[
            10.4,
            20.6,
            100.2,
            15.0, // a line
            0.0,
            0.0,
            0.0,
            10.0, // empty: dropped
            f64::NAN,
            0.0,
            5.0,
            5.0, // garbage: dropped
            -5.5,
            -3.0,
            2.0,
            2.0, // negative coordinates (left monitor)
            1.0, // trailing partial chunk: ignored
        ]);
        assert_eq!(
            r,
            vec![PxRect::new(10, 20, 101, 16), PxRect::new(-6, -3, 3, 2)]
        );
    }

    #[test]
    fn union_is_padded() {
        let lines = [
            PxRect::new(100, 100, 200, 20),
            PxRect::new(100, 120, 120, 20),
        ];
        let mons = [PxRect::new(0, 0, 1920, 1080)];
        assert_eq!(
            capture_rect(&lines, PxRect::default(), 6, &mons),
            Some(PxRect::new(94, 94, 212, 52))
        );
        // No padding: the exact union.
        assert_eq!(
            capture_rect(&lines, PxRect::default(), 0, &mons),
            Some(PxRect::new(100, 100, 200, 40))
        );
    }

    #[test]
    fn clamped_to_element_and_monitors() {
        let mons = [PxRect::new(0, 0, 1920, 1080)];
        // A line scrolled half out of its text box keeps only the box (plus padding).
        let element = PxRect::new(50, 50, 400, 100);
        let lines = [PxRect::new(60, 130, 300, 40)];
        assert_eq!(
            capture_rect(&lines, element, 4, &mons),
            Some(PxRect::new(56, 126, 308, 28))
        );
        // Padding never runs past the screen edge.
        let lines = [PxRect::new(0, 0, 50, 10)];
        assert_eq!(
            capture_rect(&lines, PxRect::default(), 8, &mons),
            Some(PxRect::new(0, 0, 58, 18))
        );
    }

    #[test]
    fn spans_monitors_and_drops_offscreen() {
        let mons = [
            PxRect::new(-1920, 0, 1920, 1080),
            PxRect::new(0, 0, 2560, 1440),
        ];
        let lines = [PxRect::new(-100, 500, 300, 20)];
        assert_eq!(
            capture_rect(&lines, PxRect::default(), 2, &mons),
            Some(PxRect::new(-102, 498, 304, 24))
        );
        // Entirely off every monitor.
        let lines = [PxRect::new(5000, 5000, 10, 10)];
        assert_eq!(capture_rect(&lines, PxRect::default(), 2, &mons), None);
        // Nothing to union.
        assert_eq!(capture_rect(&[], PxRect::default(), 2, &mons), None);
        assert_eq!(
            capture_rect(&[PxRect::new(0, 0, 0, 5)], PxRect::default(), 2, &mons),
            None
        );
    }
}
