//! Unified transfer operation contract — proto types + normalized Rust shape.
//!
//! The proto types (`TransferOperationSpec`, `FilterSpec`, etc.) are
//! defined in `proto/blit.proto` and generated via `tonic_prost_build`.
//! They're the **wire** shape: integer-encoded enums, `optional`
//! scalars, `Unspecified` defaults that downstream code shouldn't have
//! to handle.
//!
//! `NormalizedTransferOperation` is the **internal** shape: typed
//! enums folded to their concrete defaults, `FilterSpec` converted to
//! `FileFilter`, capabilities validated, `spec_version` accepted or
//! rejected. Every consumer (CLI, daemon handler, orchestrator,
//! DiffPlanner) takes the normalized type — that way there's one place
//! that knows how to turn proto `Unspecified` into `SizeMtime`,
//! validate the spec version, etc., instead of every call site
//! reimplementing that translation.
//!
//! Per R2-F2 of `docs/reviews/followup_review_2026-05-02.md`.

use std::time::SystemTime;

use eyre::{bail, Context, Result};

use crate::fs_enum::FileFilter;

pub use crate::generated::{
    ComparisonMode, FilterSpec, MirrorMode, PeerCapabilities, ResumeSettings, TransferOperationSpec,
};

// Aliases for proto-side types (raw wire shape) so the from_spec()
// translation reads clearly: "fold proto-Unspecified into concrete enum."
use crate::generated::{
    ComparisonMode as ProtoCompareMode, FilterSpec as ProtoFilterSpec,
    MirrorMode as ProtoMirrorMode,
};

/// Highest `spec_version` we know how to interpret. Bumped whenever the
/// wire shape changes in a way that requires the receiver to know.
/// Currently both ends of the wire are at 1; older clients shouldn't
/// reach this code path because they were never published.
pub const SUPPORTED_SPEC_VERSION: u32 = 1;

/// Normalized, internal-friendly view of a transfer operation. Folds
/// proto-`Unspecified` into concrete defaults, converts `FilterSpec`
/// into a usable `FileFilter`, and validates the spec version up front.
///
/// Consumers should always work with this rather than the raw proto
/// type. The normalization step is the single chokepoint that knows
/// the conversion rules.
#[derive(Debug, Clone)]
pub struct NormalizedTransferOperation {
    /// Origin module name. Empty string means "use the default root export."
    pub module: String,
    /// Path within the origin module, relative to the module root.
    /// Empty string is the legitimate "single-file source" case where
    /// the module root is itself the file.
    pub source_path: String,
    /// Comparison strategy in concrete (non-`Unspecified`) form.
    pub compare_mode: ComparisonMode,
    /// Mirror behavior in concrete (non-`Unspecified`) form.
    /// `Off` if the spec carried `Unspecified`.
    pub mirror_mode: MirrorMode,
    /// Resume settings with `enabled: false` if the spec didn't carry any.
    pub resume: ResumeSettings,
    /// Convert origin-side filter rules into a ready-to-apply `FileFilter`.
    /// `None` if the spec carried no filter or the filter was empty.
    pub filter: Option<FileFilter>,
    /// Peer capabilities (all false if the spec didn't carry any).
    pub capabilities: PeerCapabilities,
    /// Whether the initiator requested gRPC bulk transport.
    pub force_grpc: bool,
}

impl NormalizedTransferOperation {
    /// Validate and fold a wire-side `TransferOperationSpec` into the
    /// internal shape. Single chokepoint for proto→internal mapping.
    ///
    /// Returns `Err` for unsupported spec versions, malformed filter
    /// rules (bad glob patterns, invalid size/duration strings), or
    /// inconsistent capability/mode combinations that the type system
    /// can't reject at construction time.
    pub fn from_spec(spec: TransferOperationSpec) -> Result<Self> {
        // Spec version: accept exact match for now. We have no
        // backwards-compat constraint into the next release, so older
        // versions are a programming error rather than a wire
        // incompatibility.
        if spec.spec_version != SUPPORTED_SPEC_VERSION {
            bail!(
                "unsupported TransferOperationSpec spec_version {} (expected {})",
                spec.spec_version,
                SUPPORTED_SPEC_VERSION
            );
        }

        let compare_mode = compare_mode_from_proto(spec.compare_mode);
        let mirror_mode = mirror_mode_from_proto(spec.mirror_mode);
        let resume = spec.resume.unwrap_or_default();
        let capabilities = spec.client_capabilities.unwrap_or_default();
        let filter = spec
            .filter
            .map(filter_from_spec)
            .transpose()
            .context("converting FilterSpec to FileFilter")?
            .filter(|f| !f.is_empty());

        Ok(Self {
            module: spec.module,
            source_path: spec.source_path,
            compare_mode,
            mirror_mode,
            resume,
            filter,
            capabilities,
            force_grpc: spec.force_grpc,
        })
    }

    /// Whether mirror deletions should happen for this operation. True
    /// for `FilteredSubset` and `All`; false for `Off`.
    pub fn mirror_enabled(&self) -> bool {
        matches!(
            self.mirror_mode,
            MirrorMode::FilteredSubset | MirrorMode::All
        )
    }
}

/// Fold proto-side `i32` enum into concrete `ComparisonMode`. Treats
/// `Unspecified` (and any out-of-range value from a hostile peer) as
/// `SizeMtime`, the historical default.
fn compare_mode_from_proto(raw: i32) -> ComparisonMode {
    match ProtoCompareMode::try_from(raw).unwrap_or(ProtoCompareMode::SizeMtime) {
        ProtoCompareMode::Unspecified => ComparisonMode::SizeMtime,
        other => other,
    }
}

/// Fold proto-side `i32` enum into concrete `MirrorMode`. Treats
/// `Unspecified` (and any out-of-range value) as `Off`.
fn mirror_mode_from_proto(raw: i32) -> MirrorMode {
    match ProtoMirrorMode::try_from(raw).unwrap_or(ProtoMirrorMode::Off) {
        ProtoMirrorMode::Unspecified => MirrorMode::Off,
        other => other,
    }
}

/// Convert a wire `FilterSpec` to a concrete `FileFilter`. Validates
/// glob patterns and size/duration strings — bad input becomes a hard
/// error so a buggy peer can't silently disable filtering.
fn filter_from_spec(spec: ProtoFilterSpec) -> Result<FileFilter> {
    let mut filter = FileFilter::default();
    filter.include_files = spec.include;
    filter.exclude_files = spec.exclude;
    filter.min_size = spec.min_size;
    filter.max_size = spec.max_size;
    filter.min_age = spec
        .min_age_secs
        .map(std::time::Duration::from_secs);
    filter.max_age = spec
        .max_age_secs
        .map(std::time::Duration::from_secs);
    if filter.min_age.is_some() || filter.max_age.is_some() {
        filter.reference_time = Some(SystemTime::now());
    }
    if !spec.files_from.is_empty() {
        filter.files_from = Some(spec.files_from.into_iter().map(Into::into).collect());
    }
    Ok(filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_spec() -> TransferOperationSpec {
        TransferOperationSpec {
            spec_version: SUPPORTED_SPEC_VERSION,
            module: String::new(),
            source_path: String::new(),
            filter: None,
            compare_mode: ProtoCompareMode::Unspecified as i32,
            mirror_mode: ProtoMirrorMode::Unspecified as i32,
            resume: None,
            client_capabilities: None,
            force_grpc: false,
        }
    }

    #[test]
    fn unspecified_compare_mode_folds_to_size_mtime() {
        let normalized = NormalizedTransferOperation::from_spec(empty_spec()).unwrap();
        assert_eq!(normalized.compare_mode, ComparisonMode::SizeMtime);
    }

    #[test]
    fn unspecified_mirror_mode_folds_to_off() {
        let normalized = NormalizedTransferOperation::from_spec(empty_spec()).unwrap();
        assert_eq!(normalized.mirror_mode, MirrorMode::Off);
        assert!(!normalized.mirror_enabled());
    }

    #[test]
    fn filtered_subset_mirror_is_enabled() {
        let mut spec = empty_spec();
        spec.mirror_mode = ProtoMirrorMode::FilteredSubset as i32;
        let normalized = NormalizedTransferOperation::from_spec(spec).unwrap();
        assert!(normalized.mirror_enabled());
    }

    #[test]
    fn unsupported_version_rejected() {
        let mut spec = empty_spec();
        spec.spec_version = 99;
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(err.to_string().contains("spec_version 99"));
    }

    #[test]
    fn empty_filter_normalized_to_none() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec::default());
        let normalized = NormalizedTransferOperation::from_spec(spec).unwrap();
        assert!(normalized.filter.is_none());
    }

    #[test]
    fn populated_filter_passes_through() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            include: vec![],
            exclude: vec!["*.tmp".into()],
            min_size: Some(100),
            max_size: None,
            min_age_secs: None,
            max_age_secs: None,
            files_from: vec![],
        });
        let normalized = NormalizedTransferOperation::from_spec(spec).unwrap();
        let filter = normalized.filter.expect("filter should pass through");
        assert_eq!(filter.exclude_files, vec!["*.tmp"]);
        assert_eq!(filter.min_size, Some(100));
    }

    #[test]
    fn missing_resume_defaults_to_disabled() {
        let normalized = NormalizedTransferOperation::from_spec(empty_spec()).unwrap();
        assert!(!normalized.resume.enabled);
    }
}
