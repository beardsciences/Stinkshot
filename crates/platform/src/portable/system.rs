//! System event types and the instance-to-instance argument encoding, shared by every
//! backend.

// The argument encoding is only used by the Windows IPC window so far.
#![cfg_attr(not(windows), allow(dead_code))]

pub(crate) fn encode_args(args: &[String]) -> Vec<u8> {
    args.join("\0").into_bytes()
}

pub(crate) fn decode_args(bytes: &[u8]) -> Vec<String> {
    if bytes.is_empty() {
        return Vec::new();
    }
    String::from_utf8_lossy(bytes)
        .split('\0')
        .map(str::to_owned)
        .collect()
}

/// A system event after which the cached capture session is likely invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    /// The session was unlocked or reconnected to the console (left the secure desktop).
    Unlock,
    /// A display was added, removed, or changed mode.
    DisplayChange,
    /// Resumed from sleep or hibernation.
    Resume,
    /// Windows is shutting down, restarting or signing the user out (`WM_ENDSESSION` with
    /// the session really ending). The end-session handler has already run.
    EndSession,
}

/// Step of a Windows shutdown, restart or sign-out, passed to the handler set with
/// `set_end_session_handler`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndSessionPhase {
    /// `WM_QUERYENDSESSION`: the session is about to end (it can still be cancelled). Start
    /// finishing work here and return true when that needs time: Windows is then given the
    /// handler's reason text and waits instead of ending the process after a few seconds.
    Query,
    /// `WM_ENDSESSION`, session ending: the last chance to do exit work, synchronously. The
    /// process may be ended as soon as the handler returns.
    End,
    /// `WM_ENDSESSION`, the shutdown was cancelled: keep running.
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_roundtrip() {
        let a = vec!["region".to_string(), "--delay".to_string(), "3".to_string()];
        assert_eq!(decode_args(&encode_args(&a)), a);
        assert!(decode_args(&encode_args(&[])).is_empty());
    }
}
