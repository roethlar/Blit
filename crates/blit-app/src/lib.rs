//! Orchestration library shared by `blit-cli` and (post-Phase-5-A.1)
//! `blit-tui`.
//!
//! Houses the verb-implementation glue: endpoint parsing, transfer
//! dispatch (local↔remote↔remote-remote), the browser / admin RPC
//! clients, the local-tree compare core, the diagnostics-dump
//! emitter. No clap, no indicatif, no stdout — presentation lives
//! in the consuming crates.
//!
//! See `docs/plan/TUI_DESIGN.md` §7 for the design contract and
//! §7.3 for the per-module move map this crate was extracted from.
//!
//! Phase 5 A.0 status: scaffold landed; module-by-module moves are
//! tracked in subsequent commits on `phase5/blit-app-extract`.

pub mod admin;
pub mod check;
pub mod client;
pub mod diagnostics;
pub mod display;
pub mod endpoints;
pub mod profile;
pub mod scan;
pub mod transfers;
