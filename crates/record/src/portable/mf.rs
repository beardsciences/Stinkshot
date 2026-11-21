//! Time and packing helpers shared by every media backend.

// Some helpers are only called by the Windows backend so far.
#![cfg_attr(not(windows), allow(dead_code))]

pub fn pack(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

pub fn unpack(v: u64) -> (u32, u32) {
    ((v >> 32) as u32, v as u32)
}

pub fn duration_to_hns(d: std::time::Duration) -> i64 {
    (d.as_nanos() / 100) as i64
}

pub fn hns_to_duration(h: i64) -> std::time::Duration {
    std::time::Duration::from_nanos((h.max(0) as u64).saturating_mul(100))
}

/// Precise sleep: coarse sleep then a short spin. Windows' default timer granularity is
/// ~15 ms, so callers also hold a `TimerResolution`.
pub fn sleep_until(deadline: std::time::Instant) {
    loop {
        let now = std::time::Instant::now();
        if now >= deadline {
            return;
        }
        let left = deadline - now;
        if left > std::time::Duration::from_millis(2) {
            std::thread::sleep(left - std::time::Duration::from_millis(1));
        } else {
            std::thread::yield_now();
        }
    }
}
