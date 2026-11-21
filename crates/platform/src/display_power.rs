//! Console display power state (on, off, dimmed), tracked through a power-setting callback.
//!
//! Desktop Duplication delivers no frames while the displays are off (DWM does not feed
//! duplications then, measured Nov 7 2026), which looks exactly like a static screen: a
//! cached duplication session would keep returning the picture from before the displays went
//! off. `capture` asks this module whether the displays are off and whether the state changed
//! since an output's picture was last validated.
//!
//! The state comes from `PowerSettingRegisterNotification(GUID_CONSOLE_DISPLAY_STATE)` with a
//! callback (no window or message loop needed); Windows reports the current state right after
//! registering and then every change. Until the first report, or if registering fails, the
//! state is `Unknown` and treated as on (the behavior before this module existed).

use std::sync::Once;

use windows::Win32::Foundation::{HANDLE, WIN32_ERROR};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, POWERBROADCAST_SETTING, PowerSettingRegisterNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{
    PBT_POWERSETTINGCHANGE, REGISTER_NOTIFICATION_FLAGS,
};
use windows::core::GUID;

pub use crate::portable::display_power::*;

/// GUID_CONSOLE_DISPLAY_STATE: 0 off, 1 on, 2 dimmed.
const CONSOLE_DISPLAY_STATE: GUID = GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47);
/// `DEVICE_NOTIFY_CALLBACK`: the recipient is a `DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS`.
const DEVICE_NOTIFY_CALLBACK: u32 = 2;

/// Called (on a new thread) when the displays come back on after being off.
static ON_WAKE: std::sync::OnceLock<fn()> = std::sync::OnceLock::new();
static START: Once = Once::new();

unsafe extern "system" fn callback(
    _context: *const core::ffi::c_void,
    kind: u32,
    setting: *const core::ffi::c_void,
) -> u32 {
    if kind != PBT_POWERSETTINGCHANGE || setting.is_null() {
        return 0;
    }
    // SAFETY: for PBT_POWERSETTINGCHANGE `setting` points to a POWERBROADCAST_SETTING whose
    // data holds at least `DataLength` bytes (a DWORD for this setting).
    let s = unsafe { &*(setting as *const POWERBROADCAST_SETTING) };
    if s.PowerSetting != CONSOLE_DISPLAY_STATE || s.DataLength < 1 {
        return 0;
    }
    let raw = s.Data[0] as i32;
    log::debug!("console display state: {:?}", DisplayPower::from_raw(raw));
    if apply(raw)
        && let Some(f) = ON_WAKE.get().copied()
    {
        let _ = std::thread::Builder::new()
            .name("display-wake".into())
            .spawn(f);
    }
    0
}

/// Start tracking (idempotent, cheap after the first call). `on_wake` runs on a new thread
/// each time the displays come back on after being off; only the first caller's is kept.
pub(crate) fn watch(on_wake: fn()) {
    let _ = ON_WAKE.set(on_wake);
    START.call_once(|| {
        // Leaked on purpose: the registration lives as long as the process.
        let params = Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(callback),
            Context: std::ptr::null_mut(),
        }));
        let mut handle: *mut core::ffi::c_void = std::ptr::null_mut();
        // SAFETY: `params` lives forever; `handle` is a live out pointer.
        let err = unsafe {
            PowerSettingRegisterNotification(
                &CONSOLE_DISPLAY_STATE,
                REGISTER_NOTIFICATION_FLAGS(DEVICE_NOTIFY_CALLBACK),
                HANDLE(params as *mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS as *mut _),
                &mut handle,
            )
        };
        if err != WIN32_ERROR(0) {
            log::debug!("display power notifications unavailable: {err:?}");
        }
    });
}
