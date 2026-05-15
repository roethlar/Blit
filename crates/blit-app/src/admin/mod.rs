//! Admin / browser verbs: `ls`, `find`, `du`, `df`, `rm`,
//! `list_modules`. Each is a thin async wrapper over the matching
//! gRPC client call. No presentation — callers format the
//! returned structs themselves.
//!
//! Per-module moves land in subsequent A.0 commits; this file
//! just declares the surface.

pub mod df;
pub mod du;
pub mod find;
pub mod list_modules;
pub mod ls;
pub mod rm;
