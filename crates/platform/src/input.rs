use std::cell::RefCell;
use std::sync::mpsc::Sender;
use std::time::Instant;

use stinkshot_core::PxPoint;
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput,
    VK_LBUTTON, VK_RBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics, HHOOK,
    KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
    SM_SWAPBUTTON, SetCursorPos, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_USER,
};

use crate::portable::input::fixed_key_label;
pub use crate::portable::input::{InputEvent, MouseButton};

/// Implemented on this OS (the stand-ins say false).
pub const SUPPORTED: bool = true;

pub fn cursor_pos() -> PxPoint {
    let mut p = POINT::default();
    // SAFETY: out pointer is a live local.
    if unsafe { GetCursorPos(&mut p) }.is_err() {
        return PxPoint::default();
    }
    PxPoint { x: p.x, y: p.y }
}

/// Cursor position, or None when Windows refuses (secure desktop, lock screen).
pub fn try_cursor_pos() -> Option<PxPoint> {
    let mut p = POINT::default();
    // SAFETY: out pointer is a live local.
    unsafe { GetCursorPos(&mut p) }.ok()?;
    Some(PxPoint { x: p.x, y: p.y })
}

/// Whether the primary mouse button is physically held right now (honours swapped
/// buttons). Independent of which window has the mouse, so a drag loop can notice a
/// release that never reached its window.
pub fn primary_button_down() -> bool {
    // SAFETY: trivial queries.
    unsafe {
        let vk = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        };
        (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0
    }
}

/// Scroll the window under `at` by `notches` (negative = down, like WHEEL_DELTA units of
/// 120 per notch) WITHOUT moving the user's cursor permanently: move, SendInput wheel,
/// restore position. Used by scrolling capture.
pub fn scroll_at(at: PxPoint, notches: i32) {
    if notches == 0 {
        return;
    }
    // None when Windows refused the position: there is then nothing to restore to (the
    // default (0, 0) would throw the pointer into the corner of the primary monitor).
    let orig = try_cursor_pos();
    // SAFETY: SetCursorPos/SendInput take plain values; the INPUT array outlives the call.
    unsafe {
        if let Err(e) = SetCursorPos(at.x, at.y) {
            log::warn!("scroll_at: SetCursorPos failed: {e}");
        }
        // Let the pointer settle so the wheel event is routed to the window under `at`.
        std::thread::sleep(std::time::Duration::from_millis(12));
        let delta: i32 = if notches > 0 { 120 } else { -120 };
        let one = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: delta as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let inputs = vec![one; notches.unsigned_abs() as usize];
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if (sent as usize) != inputs.len() {
            log::warn!(
                "scroll_at: SendInput delivered {sent}/{} events",
                inputs.len()
            );
        }
        // Give the target time to dequeue the wheel messages before the pointer moves back.
        std::thread::sleep(std::time::Duration::from_millis(25));
        if let Some(orig) = orig {
            let _ = SetCursorPos(orig.x, orig.y);
        }
    }
}

/// Global low-level mouse/keyboard event feed for the recording click/key visualizer.
/// Installs WH_MOUSE_LL / WH_KEYBOARD_LL on a dedicated thread with its own message loop;
/// dropping the guard uninstalls the hooks. Hook callbacks must return quickly (push to
/// the channel and call next hook).
pub struct InputHook {
    thread_id: u32,
    join: Option<std::thread::JoinHandle<()>>,
}

struct HookState {
    tx: Sender<(Instant, InputEvent)>,
    down: [bool; 256],
}

thread_local! {
    // Hook procedures run on the installing thread, so thread-local state is enough.
    static STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

fn emit(ev: InputEvent) {
    STATE.with(|s| {
        if let Some(st) = s.borrow().as_ref() {
            let _ = st.tx.send((Instant::now(), ev));
        }
    });
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        // SAFETY: for WH_MOUSE_LL, lparam points to a valid MSLLHOOKSTRUCT during the callback.
        let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let pos = PxPoint {
            x: info.pt.x,
            y: info.pt.y,
        };
        let ev = match wparam.0 as u32 {
            WM_LBUTTONDOWN => Some(InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
            }),
            WM_LBUTTONUP => Some(InputEvent::MouseUp {
                pos,
                button: MouseButton::Left,
            }),
            WM_RBUTTONDOWN => Some(InputEvent::MouseDown {
                pos,
                button: MouseButton::Right,
            }),
            WM_RBUTTONUP => Some(InputEvent::MouseUp {
                pos,
                button: MouseButton::Right,
            }),
            WM_MBUTTONDOWN => Some(InputEvent::MouseDown {
                pos,
                button: MouseButton::Middle,
            }),
            WM_MBUTTONUP => Some(InputEvent::MouseUp {
                pos,
                button: MouseButton::Middle,
            }),
            _ => None,
        };
        if let Some(ev) = ev {
            emit(ev);
        }
    }
    // SAFETY: standard hook chaining.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        // SAFETY: for WH_KEYBOARD_LL, lparam points to a valid KBDLLHOOKSTRUCT during the callback.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let vk = info.vkCode;
        let extended = info.flags.0 & 1 != 0;
        match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // Suppress auto-repeat: only the first down since the last up is reported.
                let fresh = STATE.with(|s| {
                    s.borrow_mut().as_mut().is_some_and(|st| {
                        let slot = &mut st.down[(vk & 0xff) as usize];
                        !std::mem::replace(slot, true)
                    })
                });
                if fresh {
                    emit(InputEvent::KeyDown {
                        label: key_label(vk, info.scanCode, extended),
                        vk,
                    });
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                STATE.with(|s| {
                    if let Some(st) = s.borrow_mut().as_mut() {
                        st.down[(vk & 0xff) as usize] = false;
                    }
                });
                emit(InputEvent::KeyUp { vk });
            }
            _ => {}
        }
    }
    // SAFETY: standard hook chaining.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Human-readable label for a virtual key: "Ctrl", "Shift", "Alt", "Win", "Enter", "F5",
/// "P", "1", "Num 7", ... Falls back to the layout's key name.
pub fn key_label(vk: u32, scan: u32, extended: bool) -> String {
    if let Some(label) = fixed_key_label(vk) {
        return label;
    }
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyNameTextW;
    let lparam = ((scan & 0xff) << 16) | if extended { 1 << 24 } else { 0 };
    let mut buf = [0u16; 64];
    // SAFETY: buffer is a live local.
    let n = unsafe { GetKeyNameTextW(lparam as i32, &mut buf) };
    if n > 0 {
        String::from_utf16_lossy(&buf[..n as usize])
    } else {
        format!("VK{vk:02X}")
    }
}

impl InputHook {
    pub fn install(tx: Sender<(Instant, InputEvent)>) -> anyhow::Result<InputHook> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<anyhow::Result<u32>>();
        let join = std::thread::Builder::new()
            .name("stinkshot-input-hook".into())
            .spawn(move || hook_thread(tx, ready_tx))
            .map_err(|e| anyhow::anyhow!("spawn input hook thread: {e}"))?;
        match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(thread_id)) => Ok(InputHook {
                thread_id,
                join: Some(join),
            }),
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            // `ready_rx` is dropped on return, so if the thread installs its hooks later its
            // `ready.send` fails and it unhooks and exits on its own.
            Err(e) => Err(anyhow::anyhow!("input hook thread did not start: {e}")),
        }
    }
}

fn hook_thread(tx: Sender<(Instant, InputEvent)>, ready: Sender<anyhow::Result<u32>>) {
    STATE.with(|s| {
        *s.borrow_mut() = Some(HookState {
            tx,
            down: [false; 256],
        })
    });
    // SAFETY: hooks are installed and removed on this same thread, which pumps messages below.
    unsafe {
        let hmod = GetModuleHandleW(None).ok().map(|m| m.into());
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0);
        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0);
        let (mouse, kb): (HHOOK, HHOOK) = match (mouse, kb) {
            (Ok(m), Ok(k)) => (m, k),
            (m, k) => {
                if let Ok(m) = m {
                    let _ = UnhookWindowsHookEx(m);
                }
                if let Ok(k) = k {
                    let _ = UnhookWindowsHookEx(k);
                }
                let _ = ready.send(Err(anyhow::anyhow!("SetWindowsHookExW failed")));
                return;
            }
        };
        // Create this thread's message queue before announcing the thread id, so a WM_QUIT
        // posted right after install cannot fail with ERROR_INVALID_THREAD_ID.
        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE);
        if ready.send(Ok(GetCurrentThreadId())).is_err() {
            // The installer gave up waiting (timeout): nobody owns these hooks.
            let _ = UnhookWindowsHookEx(mouse);
            let _ = UnhookWindowsHookEx(kb);
            STATE.with(|s| *s.borrow_mut() = None);
            return;
        }
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(mouse);
        let _ = UnhookWindowsHookEx(kb);
    }
    STATE.with(|s| *s.borrow_mut() = None);
}

impl Drop for InputHook {
    fn drop(&mut self) {
        // SAFETY: posting WM_QUIT to our own hook thread.
        let posted =
            unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) }.is_ok();
        if let Some(j) = self.join.take() {
            if posted {
                let _ = j.join();
            } else {
                // Without a quit message the thread would never leave GetMessageW and a
                // join would hang the caller; leave it detached.
                log::warn!("input hook thread could not be told to quit");
            }
        }
    }
}
