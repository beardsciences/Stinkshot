//! Crate-private helpers stand-in. The Windows file holds COM and registry helpers; only the
//! public one exists here.

/// Does nothing on this OS (there is no COM apartment to keep alive).
pub fn keep_mta_alive() {}
