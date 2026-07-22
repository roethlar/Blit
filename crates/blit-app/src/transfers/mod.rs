//! Transfer dispatch core. Routes a source/destination pair to
//! the right transport (local↔local, local→remote push,
//! remote→local pull, remote↔remote delegated/relayed).
//!
//! The verb-entry functions (`run_transfer`, `run_move`)
//! intentionally stay in `blit-cli` — their bodies are
//! dominated by CLI-specific error messages (referencing
//! `--force`, `--checksum`, `blit rm`, etc.) and interactive
//! prompts. The TUI will write its own analogous entry-points
//! that consume the route-selection primitive in `dispatch.rs`
//! plus the per-transport execution functions in this module.

pub mod compare;
pub mod dispatch;
pub mod filter;
pub mod local;
pub mod remote;
pub mod resolution;
pub mod retry;
