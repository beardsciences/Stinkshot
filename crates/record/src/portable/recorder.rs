//! Small recorder helpers with no OS calls.

// Only the Windows recorder calls these so far.
#![cfg_attr(not(windows), allow(dead_code))]

/// Webcam bubble diameter: at least 32 px, but never larger than the picture. Not `clamp`:
/// that panics when the picture is under 32 px (min above max).
pub fn bubble_diameter(wanted: u32, w: usize, h: usize) -> u32 {
    wanted.max(32).min(w.min(h) as u32)
}

/// Text of a caught panic, for the log and the shared error.
pub fn panic_text(p: &(dyn std::any::Any + Send)) -> &str {
    p.downcast_ref::<&str>()
        .copied()
        .or_else(|| p.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_diameter_never_panics_on_a_tiny_picture() {
        assert_eq!(bubble_diameter(200, 1920, 1080), 200);
        assert_eq!(bubble_diameter(10, 1920, 1080), 32);
        assert_eq!(bubble_diameter(5000, 1920, 1080), 1080);
        // Under 32 px the picture wins (this used to panic in `clamp`).
        assert_eq!(bubble_diameter(200, 640, 20), 20);
        assert_eq!(bubble_diameter(0, 2, 2), 2);
    }

    #[test]
    fn panic_text_reads_both_payload_kinds() {
        let a = std::panic::catch_unwind(|| panic!("plain")).unwrap_err();
        assert_eq!(panic_text(&*a), "plain");
        let n = 3;
        let b = std::panic::catch_unwind(|| panic!("formatted {n}")).unwrap_err();
        assert_eq!(panic_text(&*b), "formatted 3");
    }
}
