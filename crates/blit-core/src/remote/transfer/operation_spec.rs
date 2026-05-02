//! Re-exports for the unified transfer operation contract.
//!
//! These messages are defined in `proto/blit.proto` and generated via
//! `tonic_prost_build`; this module is the canonical Rust import path
//! so callers don't have to reach into the generated `blit.v2.*`
//! namespace directly.
//!
//! No code in the workspace consumes these types yet — they're the
//! contract that step 3 of the pipeline-unification plan
//! (`docs/plan/PIPELINE_UNIFICATION.md`) will use to extract the
//! DiffPlanner stage out of `pull_sync.rs`. PushHeader and
//! PullSyncHeader remain the today-active control messages until the
//! refactor lands in step 4.
//!
//! Conventions:
//! - `ComparisonMode::Unspecified` and `MirrorMode::Unspecified` mean
//!   "use the historical default" (`SizeMtime` and `Off` respectively).
//!   Receivers should fold the unspecified value into the default at
//!   the boundary so downstream code never has to handle both shapes.
//! - All `optional` scalar fields on `FilterSpec` follow proto3
//!   semantics: `None` means "no constraint."
//! - `PeerCapabilities` defaults to all-false. Receivers treat missing
//!   capabilities as "not supported" so adding a capability is a
//!   backward-compatible wire change.

pub use crate::generated::{
    ComparisonMode, FilterSpec, MirrorMode, PeerCapabilities, ResumeSettings,
    TransferOperationSpec,
};
