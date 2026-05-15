//! Transfer dispatch core. Routes a source/destination pair to
//! the right transport (local↔local, local→remote push,
//! remote→local pull, remote↔remote delegated/relayed).
//!
//! Per-shape modules land in subsequent A.0 commits; this file
//! just declares the surface.

pub mod local;
pub mod remote;
pub mod remote_remote_direct;
