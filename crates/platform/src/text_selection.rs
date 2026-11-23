//! Where the current text selection is on screen, via UI Automation (Capture Selected Text).
//!
//! Reads only: the focused element's TextPattern (`GetSelection`, then each range's
//! `GetBoundingRectangles`). Paragraph mode works on clones of the selection ranges, so the
//! user's selection, focus and clipboard never change, and no input is sent. When the focused
//! element has no TextPattern (a tray click moved focus away, or the app puts the pattern on an
//! ancestor), its ancestors are tried, then the text elements of a fallback window.
//!
//! UIA calls can block on a busy app, so call this from a worker thread; it sets a short
//! transaction timeout so a hung target fails instead of stalling forever.

use stinkshot_core::PxRect;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, SAFEARRAY};
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
    SafeArrayUnaccessData,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationElement, IUIAutomationTextPattern,
    IUIAutomationTextRange, TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start,
    TextUnit_Paragraph, TreeScope_Descendants, UIA_IsTextPatternAvailablePropertyId,
    UIA_TextPatternId,
};
use windows::core::Interface;

use crate::Hwnd;
pub use crate::portable::text_selection::*;

/// Implemented on this OS (the stand-ins say false).
pub const SUPPORTED: bool = true;

/// Ancestors of the focused element checked for a TextPattern.
const MAX_ANCESTORS: usize = 6;
/// Text elements of the fallback window checked for a selection.
const MAX_FALLBACK_ELEMENTS: i32 = 40;
/// Lines whose rectangles are read per range (huge selections stay cheap).
const MAX_RECTS: usize = 4000;
/// UIA transaction timeout, ms.
const TIMEOUT_MS: u32 = 2000;

/// The current selection's line rectangles, or `Ok(None)` when no TextPattern with a
/// selection is found. `paragraph` expands each range to its enclosing paragraphs first (a
/// bare caret then counts as the paragraph it is in). `fallback` is searched when the focused
/// element has no usable TextPattern.
pub fn selected_text(
    fallback: Option<Hwnd>,
    paragraph: bool,
) -> anyhow::Result<Option<TextSelection>> {
    crate::util::ensure_com();
    // SAFETY: plain UIA calls on this thread, which has a COM apartment.
    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        if let Ok(a2) = automation.cast::<IUIAutomation2>() {
            let _ = a2.SetTransactionTimeout(TIMEOUT_MS);
            let _ = a2.SetConnectionTimeout(TIMEOUT_MS);
        }
        if let Ok(focused) = automation.GetFocusedElement() {
            let walker = automation.ControlViewWalker()?;
            let mut cur = Some(focused);
            for _ in 0..=MAX_ANCESTORS {
                let Some(e) = cur else { break };
                if let Some(sel) = selection_of(&e, paragraph) {
                    return Ok(Some(sel));
                }
                cur = walker.GetParentElement(&e).ok();
            }
        }
        if let Some(hwnd) = fallback
            && let Ok(root) = automation.ElementFromHandle(HWND(hwnd as *mut _))
        {
            let cond = automation.CreatePropertyCondition(
                UIA_IsTextPatternAvailablePropertyId,
                &VARIANT::from(true),
            )?;
            if let Ok(all) = root.FindAll(TreeScope_Descendants, &cond) {
                let n = all.Length().unwrap_or(0).min(MAX_FALLBACK_ELEMENTS);
                for i in 0..n {
                    if let Ok(e) = all.GetElement(i)
                        && let Some(sel) = selection_of(&e, paragraph)
                    {
                        return Ok(Some(sel));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// The selection of `e`'s TextPattern, if it has one with visible, non-empty text selected.
unsafe fn selection_of(e: &IUIAutomationElement, paragraph: bool) -> Option<TextSelection> {
    // SAFETY: plain UIA calls; failures mean "no usable selection here".
    unsafe {
        let pattern: IUIAutomationTextPattern = e.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
        let ranges = pattern.GetSelection().ok()?;
        let n = ranges.Length().ok()?;
        let mut rects = Vec::new();
        for i in 0..n {
            let Ok(r) = ranges.GetElement(i) else {
                continue;
            };
            let empty = r
                .CompareEndpoints(
                    TextPatternRangeEndpoint_Start,
                    &r,
                    TextPatternRangeEndpoint_End,
                )
                .is_ok_and(|c| c == 0);
            let range = if paragraph {
                match paragraph_range(&r) {
                    Some(p) => p,
                    None => continue,
                }
            } else if empty {
                continue;
            } else {
                r
            };
            rects.extend(line_rects(&range));
            if rects.len() >= MAX_RECTS {
                break;
            }
        }
        if rects.is_empty() {
            return None;
        }
        let element = e
            .CurrentBoundingRectangle()
            .map(|r| crate::monitors::rect_from_win(&r))
            .unwrap_or_default();
        Some(TextSelection { rects, element })
    }
}

/// A copy of `r` grown to whole paragraphs: from the start of the paragraph holding its start
/// to the end of the paragraph holding its end. A selection that ends exactly where the next
/// paragraph starts (a triple-click often does) does not pull that paragraph in.
unsafe fn paragraph_range(r: &IUIAutomationTextRange) -> Option<IUIAutomationTextRange> {
    // SAFETY: plain UIA calls on clones; the user's selection is never touched.
    unsafe {
        let start = r.Clone().ok()?;
        start
            .MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                r,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
        start.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
        let empty = r
            .CompareEndpoints(
                TextPatternRangeEndpoint_Start,
                r,
                TextPatternRangeEndpoint_End,
            )
            .is_ok_and(|c| c == 0);
        if empty {
            return Some(start);
        }
        let end = r.Clone().ok()?;
        end.MoveEndpointByRange(
            TextPatternRangeEndpoint_Start,
            r,
            TextPatternRangeEndpoint_End,
        )
        .ok()?;
        end.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
        // Selection end at the very start of `end`'s paragraph: stop at the previous one.
        let ends_at_next = r
            .CompareEndpoints(
                TextPatternRangeEndpoint_End,
                &end,
                TextPatternRangeEndpoint_Start,
            )
            .is_ok_and(|c| c == 0);
        if !ends_at_next {
            start
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    &end,
                    TextPatternRangeEndpoint_End,
                )
                .ok()?;
        } else {
            start
                .MoveEndpointByRange(
                    TextPatternRangeEndpoint_End,
                    r,
                    TextPatternRangeEndpoint_End,
                )
                .ok()?;
            // Never shrink below the start paragraph.
            let shorter = start
                .CompareEndpoints(
                    TextPatternRangeEndpoint_End,
                    r,
                    TextPatternRangeEndpoint_End,
                )
                .is_ok_and(|c| c < 0);
            if shorter {
                return None;
            }
        }
        Some(start)
    }
}

/// Visible line rectangles of `r` (UIA leaves out lines that are scrolled away).
unsafe fn line_rects(r: &IUIAutomationTextRange) -> Vec<PxRect> {
    // SAFETY: the SAFEARRAY comes from UIA and is ours to destroy; its data is read between
    // a successful Access/Unaccess pair, within the bounds it reports.
    unsafe {
        let Ok(sa) = r.GetBoundingRectangles() else {
            return Vec::new();
        };
        if sa.is_null() {
            return Vec::new();
        }
        let out = read_doubles(sa)
            .map(|v| rects_from_doubles(&v))
            .unwrap_or_default();
        let _ = SafeArrayDestroy(sa);
        out
    }
}

unsafe fn read_doubles(sa: *mut SAFEARRAY) -> Option<Vec<f64>> {
    // SAFETY: see `line_rects`.
    unsafe {
        let lo = SafeArrayGetLBound(sa, 1).ok()?;
        let hi = SafeArrayGetUBound(sa, 1).ok()?;
        let n = usize::try_from(hi - lo + 1).ok()?.min(MAX_RECTS * 4);
        let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
        SafeArrayAccessData(sa, &mut data).ok()?;
        let v = if data.is_null() || n == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(data as *const f64, n).to_vec()
        };
        let _ = SafeArrayUnaccessData(sa);
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Selection of a window's own element (tests only: no focus involved).
    fn selection_in(hwnd: Hwnd, paragraph: bool) -> Option<TextSelection> {
        crate::util::ensure_com();
        // SAFETY: plain UIA calls on this thread.
        unsafe {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let e = automation.ElementFromHandle(HWND(hwnd as *mut _)).ok()?;
            selection_of(&e, paragraph)
        }
    }

    /// Needs a desktop session. Uses only a RichEdit control this test creates (shown without
    /// activation, never focused), on its own thread with a message loop.
    #[test]
    #[ignore]
    fn reads_selection_of_own_richedit() {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, ES_MULTILINE, GetMessageW, MSG,
            PostThreadMessageW, SW_SHOWNOACTIVATE, SendMessageW, ShowWindow, TranslateMessage,
            WINDOW_STYLE, WM_QUIT, WM_SETTEXT, WS_BORDER, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_POPUP,
        };
        use windows::core::w;

        const EM_SETSEL: u32 = 0x00B1;
        let (tx, rx) = std::sync::mpsc::channel::<(isize, u32)>();
        let ui = std::thread::spawn(move || unsafe {
            // RichEdit implements UIA itself; a classic Edit in this process has no TextPattern.
            let _ = windows::Win32::System::LibraryLoader::LoadLibraryW(w!("Msftedit.dll"));
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                w!("RICHEDIT50W"),
                w!(""),
                WS_POPUP | WS_BORDER | WINDOW_STYLE(ES_MULTILINE as u32),
                120,
                120,
                420,
                160,
                None,
                None,
                None,
                None,
            )
            .expect("create edit");
            let text = crate::util::wide("First paragraph text.\r\nSecond paragraph text.");
            SendMessageW(
                hwnd,
                WM_SETTEXT,
                Some(WPARAM(0)),
                Some(LPARAM(text.as_ptr() as isize)),
            );
            SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(5)));
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            tx.send((hwnd.0 as isize, GetCurrentThreadId())).unwrap();
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = DestroyWindow(hwnd);
        });
        let (hwnd, tid) = rx.recv().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));

        let word = selection_in(hwnd, false).expect("selection");
        let para = selection_in(hwnd, true).expect("paragraph");
        // SAFETY: posting WM_QUIT to the thread created above.
        unsafe {
            let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        ui.join().unwrap();

        assert_eq!(word.rects.len(), 1, "{word:?}");
        let w = word.rects[0];
        assert!(w.w > 0 && w.h > 0 && w.x >= 120 && w.y >= 120, "{w:?}");
        // The whole first line is wider than its first word.
        let p = para.rects[0];
        assert!(p.w > w.w, "{p:?} vs {w:?}");
    }
}
