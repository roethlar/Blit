//! otp-7b-2 (D-2026-07-09-1 Q2 rider): a typed error-chain marker
//! naming the file a transfer failure concerns. Per-file read/write
//! errors attach it with `eyre`'s `wrap_err`, and the session driver's
//! `fault_from_report` lifts it into `SessionFault.relative_path` —
//! structured file identity end to end, never scraped from a message.

use std::fmt;

/// The relative path of the file a wrapped transfer error concerns.
#[derive(Debug, Clone)]
pub struct FaultedPath(pub String);

impl fmt::Display for FaultedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "while transferring '{}'", self.0)
    }
}

impl std::error::Error for FaultedPath {}
