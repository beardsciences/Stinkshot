//! HDR (advanced color) displays.
//!
//! When Windows HDR is on for a display, the desktop is composed in linear scRGB FP16 where
//! 1.0 = 80 nits, and SDR content (every normal window) is placed at the user's "SDR content
//! brightness" white level: an sRGB value `v` ends up at `srgb_decode(v) * white_nits / 80`.
//! Capturing such a display as 8-bit (GDI, or Desktop Duplication asking for BGRA8) does not
//! undo that scale, so captures look brighter / washed out compared to the screen.
//!
//! This module finds the per-display white level (`DisplayConfigGetDeviceInfo`) and converts
//! FP16 scRGB back to 8-bit sRGB so SDR content maps 1:1 to its original values. Content above
//! SDR white (HDR video, games) cannot be shown in 8-bit SDR without changing SDR whites too,
//! so it is rolled off toward white along its own hue instead of being clipped per channel
//! (which would shift hues, for example bright orange turning yellow).

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SDR_WHITE_LEVEL,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::ERROR_SUCCESS;

pub use crate::portable::hdr::*;

/// Implemented on this OS (the stand-ins say false).
pub const SUPPORTED: bool = true;

/// Color state of every active display (DisplayConfig API). Empty when the query fails.
pub fn displays() -> Vec<DisplayColor> {
    // SAFETY: buffers are sized by GetDisplayConfigBufferSizes and every info packet carries
    // its own correct size in the header.
    unsafe {
        let mut np = 0u32;
        let mut nm = 0u32;
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = Vec::new();
        // The topology can change between the two calls; retry a few times.
        let mut ok = false;
        for _ in 0..3 {
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != ERROR_SUCCESS
            {
                return Vec::new();
            }
            paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
            let r = QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut np,
                paths.as_mut_ptr(),
                &mut nm,
                modes.as_mut_ptr(),
                None,
            );
            if r == ERROR_SUCCESS {
                ok = true;
                break;
            }
        }
        if !ok {
            return Vec::new();
        }
        paths.truncate(np as usize);
        let mut out = Vec::new();
        for p in &paths {
            let mut src = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                    adapterId: p.sourceInfo.adapterId,
                    id: p.sourceInfo.id,
                },
                ..Default::default()
            };
            if DisplayConfigGetDeviceInfo(&mut src.header) != 0 {
                continue;
            }
            let mut aci = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
                    size: std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
                    adapterId: p.targetInfo.adapterId,
                    id: p.targetInfo.id,
                },
                ..Default::default()
            };
            // Bit 1 of the flags is advancedColorEnabled.
            let advanced =
                DisplayConfigGetDeviceInfo(&mut aci.header) == 0 && aci.Anonymous.value & 0x2 != 0;
            let mut white = DISPLAYCONFIG_SDR_WHITE_LEVEL {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL,
                    size: std::mem::size_of::<DISPLAYCONFIG_SDR_WHITE_LEVEL>() as u32,
                    adapterId: p.targetInfo.adapterId,
                    id: p.targetInfo.id,
                },
                ..Default::default()
            };
            let white_nits =
                if DisplayConfigGetDeviceInfo(&mut white.header) == 0 && white.SDRWhiteLevel > 0 {
                    // SDRWhiteLevel is a multiplier of 80 nits, in thousandths.
                    white.SDRWhiteLevel as f32 / 1000.0 * SCRGB_UNIT_NITS
                } else if advanced {
                    DEFAULT_SDR_WHITE_NITS
                } else {
                    SCRGB_UNIT_NITS
                };
            out.push(DisplayColor {
                device: wide_to_string(&src.viewGdiDeviceName),
                advanced_color: advanced,
                sdr_white_nits: if advanced {
                    white_nits
                } else {
                    SCRGB_UNIT_NITS
                },
            });
        }
        out
    }
}

/// Color state of the display with GDI device name `device` (`\\.\DISPLAY1`). Unknown
/// devices are reported as plain SDR.
pub fn display_for_device(device: &str) -> DisplayColor {
    displays()
        .into_iter()
        .find(|d| d.device.eq_ignore_ascii_case(device))
        .unwrap_or_else(|| DisplayColor {
            device: device.to_string(),
            advanced_color: false,
            sdr_white_nits: SCRGB_UNIT_NITS,
        })
}

/// Color state of the display showing monitor `hmonitor` (raw `HMONITOR`).
pub fn display_for_monitor(hmonitor: isize) -> DisplayColor {
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO, MONITORINFOEXW};
    let mut mi = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: cbSize announces the EX struct, which is what the pointer refers to.
    let ok = unsafe {
        GetMonitorInfoW(
            HMONITOR(hmonitor as *mut _),
            &mut mi as *mut MONITORINFOEXW as *mut MONITORINFO,
        )
    }
    .as_bool();
    if !ok {
        return DisplayColor {
            device: String::new(),
            advanced_color: false,
            sdr_white_nits: SCRGB_UNIT_NITS,
        };
    }
    display_for_device(&wide_to_string(&mi.szDevice))
}
