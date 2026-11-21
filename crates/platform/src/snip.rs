//! Replace Snipping Tool: a `WH_KEYBOARD_LL` hook that swallows exactly the two Snipping Tool
//! chords, Win+Shift+S (region snip) and Win+Shift+R (screen recording), and reports them so
//! the app can run its own region capture and recording instead.
//!
//! Rules the hook keeps (see `ChordFilter`):
//! - Only the letter key of a chord is swallowed: its first down, its auto-repeats and its
//!   matching up, whatever order the keys are released in. Win, Shift, Ctrl and Alt always
//!   pass through, so no key can be left logically stuck down.
//! - Win+Shift with Ctrl or Alt also held is not a Snipping Tool chord and passes through.
//! - Before deciding on a letter down, the tracked modifier state is checked against
//!   `GetAsyncKeyState`, so a key-up the hook never saw (lock screen, UAC prompt, a hook that
//!   was removed for a while) cannot turn a plain Shift+S into a capture.
//! - A swallowed letter whose up the hook never saw (the secure desktop took input while it
//!   was held, or the hook was removed for a while) is not stuck: a down counts as an
//!   auto-repeat only within `REPEAT_WINDOW_MS` of the previous swallowed event for that
//!   letter, otherwise it is judged as a fresh press. `GetAsyncKeyState` cannot help here,
//!   since a swallowed down never reaches the async key state.
//! - While pass-through is on (`SnipHook::set_pass_through`, used while Settings records a
//!   shortcut), no new chord is swallowed or reported; state is still tracked.
//! - Events injected by this hook (the Start-menu mask key) carry `SNIP_EXTRA_INFO` and are
//!   ignored. Other injected input (AutoHotkey, Stream Deck, tests) is handled like typing.
//! - After swallowing a chord, an unassigned key (VK 0xE8) is injected while Win is still
//!   down, so releasing Win does not open the Start menu (the AutoHotkey "menu mask" trick).
//! - The callback does no more than the state machine, the mask `SendInput` and a
//!   `PostThreadMessageW` to its own thread; the channel send happens in the message loop.
//!   Windows silently removes a low-level hook whose callback times out, so the hook thread
//!   re-installs the hook every `REFRESH_MS`.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, HHOOK, KBDLLHOOKSTRUCT, KillTimer, MSG,
    PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SetTimer, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN, WM_TIMER,
    WM_USER,
};

pub use crate::portable::snip::*;

/// Re-install interval: undoes a silent removal after a callback timeout.
const REFRESH_MS: u32 = 30_000;
/// Thread message carrying a chord (`wParam` = `SnipChord::index`).
const WM_SNIP_CHORD: u32 = WM_APP + 0x51;
const LLKHF_INJECTED: u32 = 0x10;

// ------------------------------------------------------------------------- the real hook

struct HookCtx {
    filter: ChordFilter,
    thread_id: u32,
    pass_through: Arc<AtomicBool>,
}

thread_local! {
    // The hook procedure runs on the installing thread, so thread-local state is enough.
    static CTX: RefCell<Option<HookCtx>> = const { RefCell::new(None) };
}

fn key_down(vk: i32) -> bool {
    // SAFETY: trivial query.
    (unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000) != 0
}

fn async_mods() -> Mods {
    Mods {
        win: key_down(VK_LWIN as i32) || key_down(VK_RWIN as i32),
        shift: key_down(VK_SHIFT as i32),
        ctrl: key_down(VK_CONTROL as i32),
        alt: key_down(VK_MENU as i32),
    }
}

fn mask_input(up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(MASK_VK as u16),
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: SNIP_EXTRA_INFO,
            },
        },
    }
}

unsafe extern "system" fn snip_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        // SAFETY: for WH_KEYBOARD_LL, lparam points to a valid KBDLLHOOKSTRUCT during the call.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let ours = info.flags.0 & LLKHF_INJECTED != 0 && info.dwExtraInfo == SNIP_EXTRA_INFO;
        if !ours {
            let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let ev = KeyEvent {
                vk: info.vkCode,
                down,
                time: info.time,
            };
            let verdict = CTX.with(|c| {
                let mut c = c.borrow_mut();
                let c = c.as_mut()?;
                if down && SnipChord::from_vk(ev.vk).is_some() {
                    c.filter.sync(async_mods());
                    let through = c.pass_through.load(Ordering::Relaxed);
                    c.filter.set_pass_through(through);
                }
                Some((c.filter.on_key(ev), c.thread_id))
            });
            if let Some((v, tid)) = verdict {
                if v.mask_win {
                    // SAFETY: the INPUT array outlives the call; injection is asynchronous.
                    unsafe {
                        SendInput(
                            &[mask_input(false), mask_input(true)],
                            std::mem::size_of::<INPUT>() as i32,
                        );
                    }
                }
                if let Some(chord) = v.fire {
                    // SAFETY: posting to this hook thread's own queue.
                    let _ = unsafe {
                        PostThreadMessageW(tid, WM_SNIP_CHORD, WPARAM(chord.index()), LPARAM(0))
                    };
                }
                if v.swallow {
                    return LRESULT(1);
                }
            }
        }
    }
    // SAFETY: standard hook chaining.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// The installed Snipping Tool hook. Dropping it removes the hook and joins its thread.
pub struct SnipHook {
    thread_id: u32,
    join: Option<std::thread::JoinHandle<()>>,
    pass_through: Arc<AtomicBool>,
}

impl SnipHook {
    /// Install the hook on a dedicated thread. Every chord it swallows is sent to `tx`
    /// from that thread's message loop (never from inside the hook callback).
    pub fn install(tx: Sender<SnipChord>) -> anyhow::Result<SnipHook> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<anyhow::Result<u32>>();
        let pass_through = Arc::new(AtomicBool::new(false));
        let flag = pass_through.clone();
        let join = std::thread::Builder::new()
            .name("stinkshot-snip-hook".into())
            .spawn(move || hook_thread(tx, ready_tx, flag))
            .map_err(|e| anyhow::anyhow!("spawn Snipping Tool hook thread: {e}"))?;
        match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(thread_id)) => Ok(SnipHook {
                thread_id,
                join: Some(join),
                pass_through,
            }),
            Ok(Err(e)) => {
                let _ = join.join();
                Err(e)
            }
            // The thread unhooks and exits on its own when its ready send fails.
            Err(e) => Err(anyhow::anyhow!(
                "Snipping Tool hook thread did not start: {e}"
            )),
        }
    }
}

impl SnipHook {
    /// Let the chords through untouched (Win+Shift+S opens the Snipping Tool again) and
    /// report none, e.g. while Settings records a shortcut. The hook stays installed.
    pub fn set_pass_through(&self, on: bool) {
        self.pass_through.store(on, Ordering::Relaxed);
    }
}

fn install_hook() -> windows::core::Result<HHOOK> {
    // SAFETY: installs a hook whose procedure lives in this module for the process lifetime.
    unsafe {
        let hmod = GetModuleHandleW(None).ok().map(|m| m.into());
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(snip_proc), hmod, 0)
    }
}

fn hook_thread(
    tx: Sender<SnipChord>,
    ready: Sender<anyhow::Result<u32>>,
    pass_through: Arc<AtomicBool>,
) {
    // SAFETY: trivial query.
    let thread_id = unsafe { GetCurrentThreadId() };
    CTX.with(|c| {
        *c.borrow_mut() = Some(HookCtx {
            filter: ChordFilter::new(),
            thread_id,
            pass_through,
        })
    });
    let mut hook = match install_hook() {
        Ok(h) => h,
        Err(e) => {
            CTX.with(|c| *c.borrow_mut() = None);
            let _ = ready.send(Err(anyhow::anyhow!("SetWindowsHookExW: {e}")));
            return;
        }
    };
    // SAFETY: the hook, timer and message queue all belong to this thread.
    unsafe {
        // Create the message queue before announcing the thread id, so a WM_QUIT posted
        // right after install cannot fail with ERROR_INVALID_THREAD_ID.
        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE);
        if ready.send(Ok(thread_id)).is_err() {
            let _ = UnhookWindowsHookEx(hook);
            CTX.with(|c| *c.borrow_mut() = None);
            return;
        }
        let timer = SetTimer(None, 0, REFRESH_MS, None);
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.hwnd.is_invalid() && msg.message == WM_SNIP_CHORD {
                if let Some(chord) = SnipChord::ALL.get(msg.wParam.0) {
                    let _ = tx.send(*chord);
                }
                continue;
            }
            if msg.hwnd.is_invalid() && msg.message == WM_TIMER && msg.wParam.0 == timer {
                // Hook callbacks only run while this thread waits for messages, so no
                // event can be seen twice between installing the new hook and removing the
                // old one.
                match install_hook() {
                    Ok(fresh) => {
                        let _ = UnhookWindowsHookEx(hook);
                        hook = fresh;
                    }
                    Err(e) => log::warn!("Snipping Tool hook refresh failed: {e}"),
                }
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if timer != 0 {
            let _ = KillTimer(None, timer);
        }
        let _ = UnhookWindowsHookEx(hook);
    }
    CTX.with(|c| *c.borrow_mut() = None);
}

impl Drop for SnipHook {
    fn drop(&mut self) {
        // SAFETY: posting WM_QUIT to our own hook thread.
        let posted =
            unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) }.is_ok();
        if let Some(j) = self.join.take() {
            if posted {
                let _ = j.join();
            } else {
                log::warn!("Snipping Tool hook thread could not be told to quit");
            }
        }
    }
}
