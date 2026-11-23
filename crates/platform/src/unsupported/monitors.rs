//! Monitor enumeration stand-in: no monitors are reported on this OS yet.

use stinkshot_core::{PxPoint, PxRect};

pub use crate::portable::monitors::*;

/// Not implemented on this OS yet.
pub const SUPPORTED: bool = false;

/// Always empty on this OS yet.
pub fn list() -> Vec<Monitor> {
    Vec::new()
}

/// An empty rect at (0, 0) on this OS yet.
pub fn virtual_bounds() -> PxRect {
    PxRect::new(0, 0, 0, 0)
}

/// Always None on this OS yet (there are no monitors to hit).
pub fn at_point(p: PxPoint) -> Option<Monitor> {
    list().into_iter().find(|m| m.rect.contains(p))
}
