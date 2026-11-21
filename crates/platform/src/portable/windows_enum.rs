//! Top-level window description and pure lookups, shared by every backend.

use stinkshot_core::{PxPoint, PxRect};

use crate::Hwnd;

#[derive(Clone, Debug, PartialEq)]
pub struct WinInfo {
    pub hwnd: Hwnd,
    /// Visible frame (DWMWA_EXTENDED_FRAME_BOUNDS), physical virtual coords.
    pub rect: PxRect,
    pub title: String,
    /// Process image name without ".exe", e.g. "firefox".
    pub process: String,
    pub pid: u32,
    /// True when the window is maximized (no rounded corners on Win11).
    pub maximized: bool,
}

/// Topmost window from `windows` containing `p` (pure helper over a snapshot).
pub fn window_at(windows: &[WinInfo], p: PxPoint) -> Option<&WinInfo> {
    windows.iter().find(|w| w.rect.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(hwnd: isize, r: PxRect) -> WinInfo {
        WinInfo {
            hwnd,
            rect: r,
            title: String::new(),
            process: String::new(),
            pid: 1,
            maximized: false,
        }
    }

    #[test]
    fn window_at_picks_topmost() {
        let ws = vec![
            w(1, PxRect::new(0, 0, 100, 100)),
            w(2, PxRect::new(0, 0, 500, 500)),
        ];
        assert_eq!(
            window_at(&ws, PxPoint { x: 50, y: 50 }).map(|x| x.hwnd),
            Some(1)
        );
        assert_eq!(
            window_at(&ws, PxPoint { x: 300, y: 300 }).map(|x| x.hwnd),
            Some(2)
        );
        assert!(window_at(&ws, PxPoint { x: 600, y: 600 }).is_none());
    }
}
