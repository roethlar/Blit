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
    ComparisonMode, FilterSpec, MirrorMode, ResumeSettings, TransferOperationSpec,
};

// Aliases for proto-side types (raw wire shape) so the from_spec()
// translation reads clearly: "fold proto-Unspecified into concrete enum."
use crate::generated::{
    ComparisonMode as ProtoCompareMode, FilterSpec as ProtoFilterSpec,
    MirrorMode as ProtoMirrorMode,
};

/// Highest `spec_version` we know how to interpret. Bumped whenever the
/// wire shape changes in a way that requires the receiver to know.
///
/// History:
///   - 1: original (pre-0.1.0).
///   - 2: added `require_complete_scan` (R49-F2). Safety-critical:
///     a v1 daemon would accept a v2 spec, silently ignore the new
///     field, and reopen the remote-source-move data-loss case
///     where partial source scans silently delete unread files.
///     Bumping forces v1 daemons to fail closed (R51-F3).
///   - 3: added `drop_windows_metadata`. Exact versioning prevents a
///     delegated peer from silently ignoring the explicit lossy policy.
pub const SUPPORTED_SPEC_VERSION: u32 = 3;

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
    /// Whether the initiator requested gRPC bulk transport.
    pub force_grpc: bool,
    /// Skip any file the target already has, regardless of
    /// `compare_mode`. Orthogonal to `compare_mode`: this controls
    /// whether we look at the file at all; `compare_mode` controls
    /// what counts as "matching" once we do.
    pub ignore_existing: bool,
    /// R49-F2: when true, the daemon must refuse the operation if
    /// its source-side scan was incomplete. Set by `blit move`,
    /// which deletes the source after the transfer succeeds —
    /// without this gate an EACCES on a source subtree would
    /// silently lose files that never got copied. Independent of
    /// `mirror_mode`: move always uses mirror_mode=Off (it doesn't
    /// purge dest extras) but carries the same scan-completeness
    /// requirement that a mirror operation does.
    pub require_complete_scan: bool,
    /// Explicitly discard Windows attributes and named data streams at the
    /// SOURCE. False preserves strictly.
    pub drop_windows_metadata: bool,
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

        let compare_mode = compare_mode_from_proto(spec.compare_mode)
            .with_context(|| format!("invalid compare_mode {}", spec.compare_mode))?;
        let mirror_mode = mirror_mode_from_proto(spec.mirror_mode)
            .with_context(|| format!("invalid mirror_mode {}", spec.mirror_mode))?;
        if spec.ignore_existing && matches!(compare_mode, ComparisonMode::Force) {
            bail!("ignore_existing=true with compare_mode=Force is contradictory");
        }
        let resume = spec.resume.unwrap_or_default();
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
            force_grpc: spec.force_grpc,
            ignore_existing: spec.ignore_existing,
            require_complete_scan: spec.require_complete_scan,
            drop_windows_metadata: spec.drop_windows_metadata,
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

/// Fold proto-side `i32` enum into concrete `ComparisonMode`. Folds
/// `Unspecified` to `SizeMtime` (historical default). Rejects any
/// out-of-range value as a wire error — we'd rather a hostile or
/// future peer get a hard `Err` than silently pick a default that
/// doesn't match what they asked for.
fn compare_mode_from_proto(raw: i32) -> Result<ComparisonMode> {
    let parsed = ProtoCompareMode::try_from(raw)
        .map_err(|_| eyre::eyre!("unknown ComparisonMode value {raw}"))?;
    Ok(match parsed {
        ProtoCompareMode::Unspecified => ComparisonMode::SizeMtime,
        other => other,
    })
}

/// Fold proto-side `i32` enum into concrete `MirrorMode`. Folds
/// `Unspecified` to `Off` (the safe default). Rejects out-of-range
/// values for the same reason as `compare_mode_from_proto`.
fn mirror_mode_from_proto(raw: i32) -> Result<MirrorMode> {
    let parsed = ProtoMirrorMode::try_from(raw)
        .map_err(|_| eyre::eyre!("unknown MirrorMode value {raw}"))?;
    Ok(match parsed {
        ProtoMirrorMode::Unspecified => MirrorMode::Off,
        other => other,
    })
}

/// Convert a wire `FilterSpec` to a concrete `FileFilter`. Validates
/// every glob pattern individually so a malformed pattern from a
/// hostile or buggy peer is a hard error here rather than silently
/// dropped by `FileFilter::build_globset` later (R5-F4 of
/// `docs/reviews/followup_review_2026-05-02.md`).
pub(crate) fn filter_from_spec(spec: ProtoFilterSpec) -> Result<FileFilter> {
    for pat in &spec.include {
        globset::Glob::new(pat).with_context(|| format!("invalid include glob '{pat}'"))?;
    }
    for pat in &spec.exclude {
        globset::Glob::new(pat).with_context(|| format!("invalid exclude glob '{pat}'"))?;
    }

    let mut filter = FileFilter::default();
    filter.include_files = spec.include;
    filter.exclude_files = spec.exclude;
    filter.min_size = spec.min_size;
    filter.max_size = spec.max_size;
    filter.min_age = spec.min_age_secs.map(std::time::Duration::from_secs);
    filter.max_age = spec.max_age_secs.map(std::time::Duration::from_secs);
    if filter.min_age.is_some() || filter.max_age.is_some() {
        filter.reference_time = Some(SystemTime::now());
    }
    if !spec.files_from.is_empty() {
        filter.files_from = Some(spec.files_from.into_iter().map(Into::into).collect());
    }
    Ok(filter)
}

/// Options for building a delegated remote→remote trigger spec —
/// the CLI/TUI-side inputs `delegated_spec_from_options` maps onto
/// the wire [`TransferOperationSpec`] that rides
/// `DelegatedPullRequest.spec`.
///
/// otp-10c-2: relocated verbatim from the deleted old-pull driver
/// (`remote/pull.rs`, where it was `PullSyncOptions` +
/// `RemotePullClient::build_spec_from_options`). The delegated
/// trigger is its only consumer — the destination daemon validates
/// this spec and maps it onto its own `PullSessionOptions`
/// (otp-9b); the CLI verbs build session options directly.
#[derive(Debug, Default, Clone)]
pub struct DelegatedSpecOptions {
    /// Force the in-stream byte carrier (`--force-grpc`).
    pub force_grpc: bool,
    /// Mirror mode: the destination deletes extraneous entries.
    pub mirror_mode: bool,
    /// Mirror scope policy: when true, deletions extend across the
    /// full destination tree (`MirrorMode::All`). Default false →
    /// `MirrorMode::FilteredSubset` so files outside the source
    /// filter scope are never purged.
    pub delete_all_scope: bool,
    /// Filter rules applied at the source daemon's enumeration.
    /// `None` means no filtering.
    pub filter: Option<crate::generated::FilterSpec>,
    /// Compare only by size, ignore modification time.
    pub size_only: bool,
    /// Transfer all files unconditionally.
    pub ignore_times: bool,
    /// Skip files that already exist on target.
    pub ignore_existing: bool,
    /// Overwrite even if target is newer (dangerous).
    pub force: bool,
    /// Force checksum comparison (slower but more accurate).
    pub checksum: bool,
    /// Enable block-level resume for partial/changed files.
    pub resume: bool,
    /// Block size for resume (0 = default 1 MiB).
    pub block_size: u32,
    /// R49-F2: when true, the operation is refused if the source-side
    /// scan was incomplete. Set for a move-shaped delegation (the
    /// caller deletes the source after the transfer succeeds).
    pub require_complete_scan: bool,
    /// Explicitly discard Windows attributes and named data streams at the
    /// source daemon before it emits the manifest.
    pub drop_windows_metadata: bool,
}

/// Build the delegated trigger's wire [`TransferOperationSpec`] from a
/// source endpoint + [`DelegatedSpecOptions`]. Body ported verbatim
/// from the deleted `RemotePullClient::build_spec_from_options`
/// (otp-10c-2) — same precedence, same wire bytes.
pub fn delegated_spec_from_options(
    endpoint: &crate::remote::endpoint::RemoteEndpoint,
    options: &DelegatedSpecOptions,
) -> Result<TransferOperationSpec> {
    use crate::remote::endpoint::RemotePath;

    let (module, rel_path) = match &endpoint.path {
        RemotePath::Module { module, rel_path } => (module.clone(), rel_path.clone()),
        RemotePath::Root { rel_path } => (String::new(), rel_path.clone()),
        RemotePath::Discovery => {
            bail!("remote source must specify a module (server:/module/...)");
        }
    };

    let path_str = if rel_path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel_path
            .iter()
            .map(|component| component.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    };

    // ComparisonMode covers only the "given the file is being
    // considered, what counts as a match?" axis; the orthogonal
    // "skip if dst exists" axis travels in the top-level
    // `ignore_existing` spec field.
    let compare_mode = if options.ignore_times {
        ComparisonMode::IgnoreTimes
    } else if options.force {
        ComparisonMode::Force
    } else if options.size_only {
        ComparisonMode::SizeOnly
    } else if options.checksum {
        ComparisonMode::Checksum
    } else {
        ComparisonMode::SizeMtime
    };
    let mirror = if options.mirror_mode {
        if options.delete_all_scope {
            MirrorMode::All
        } else {
            // Default — files outside the filter scope are not
            // purged from the destination, since the source
            // filter excluded them on purpose.
            MirrorMode::FilteredSubset
        }
    } else {
        MirrorMode::Off
    };
    let filter_spec = options.filter.clone().unwrap_or_default();
    Ok(TransferOperationSpec {
        spec_version: SUPPORTED_SPEC_VERSION,
        module,
        source_path: path_str,
        filter: Some(filter_spec),
        compare_mode: compare_mode as i32,
        mirror_mode: mirror as i32,
        resume: Some(ResumeSettings {
            enabled: options.resume,
            block_size: options.block_size,
        }),
        force_grpc: options.force_grpc,
        ignore_existing: options.ignore_existing,
        require_complete_scan: options.require_complete_scan,
        drop_windows_metadata: options.drop_windows_metadata,
    })
}

#[cfg(test)]
mod delegated_spec_tests {
    //! otp-10c-2 codex F3: the delegated trigger's spec builder is
    //! live code and lost its direct pins when the old pull driver's
    //! `spec_extraction_tests` died with it — re-pinned here against
    //! the relocated `delegated_spec_from_options`.

    use super::*;
    use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
    use std::path::PathBuf;

    fn ep(path: RemotePath) -> RemoteEndpoint {
        RemoteEndpoint {
            host: "h".into(),
            port: 9031,
            path,
        }
    }

    fn module_ep(rel: &str) -> RemoteEndpoint {
        ep(RemotePath::Module {
            module: "mod".into(),
            rel_path: PathBuf::from(rel),
        })
    }

    #[test]
    fn endpoint_module_subpath_joins_with_forward_slashes() {
        let spec = delegated_spec_from_options(&module_ep("a/b"), &DelegatedSpecOptions::default())
            .unwrap();
        assert_eq!(spec.module, "mod");
        assert_eq!(spec.source_path, "a/b");
        assert_eq!(spec.spec_version, SUPPORTED_SPEC_VERSION);
    }

    #[test]
    fn endpoint_module_empty_rel_path_yields_dot_source() {
        let spec =
            delegated_spec_from_options(&module_ep(""), &DelegatedSpecOptions::default()).unwrap();
        assert_eq!(spec.source_path, ".");
    }

    #[test]
    fn endpoint_root_variant_yields_empty_module() {
        let spec = delegated_spec_from_options(
            &ep(RemotePath::Root {
                rel_path: PathBuf::from("sub"),
            }),
            &DelegatedSpecOptions::default(),
        )
        .unwrap();
        assert_eq!(spec.module, "");
        assert_eq!(spec.source_path, "sub");
    }

    #[test]
    fn endpoint_discovery_variant_bails() {
        let err = delegated_spec_from_options(
            &ep(RemotePath::Discovery),
            &DelegatedSpecOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("must specify a module"));
    }

    /// The old pull driver's full compare precedence, verbatim:
    /// ignore_times > force > size_only > checksum > SizeMtime.
    #[test]
    fn compare_precedence_matches_the_old_drivers_table() {
        let cell = |ignore_times: bool, force: bool, size_only: bool, checksum: bool| {
            let opts = DelegatedSpecOptions {
                ignore_times,
                force,
                size_only,
                checksum,
                ..DelegatedSpecOptions::default()
            };
            delegated_spec_from_options(&module_ep(""), &opts)
                .unwrap()
                .compare_mode
        };
        assert_eq!(
            cell(true, true, true, true),
            ComparisonMode::IgnoreTimes as i32
        );
        assert_eq!(cell(false, true, true, true), ComparisonMode::Force as i32);
        assert_eq!(
            cell(false, false, true, true),
            ComparisonMode::SizeOnly as i32
        );
        assert_eq!(
            cell(false, false, false, true),
            ComparisonMode::Checksum as i32
        );
        assert_eq!(
            cell(false, false, false, false),
            ComparisonMode::SizeMtime as i32
        );
    }

    #[test]
    fn mirror_scope_maps_off_subset_and_all() {
        let cell = |mirror_mode: bool, delete_all_scope: bool| {
            let opts = DelegatedSpecOptions {
                mirror_mode,
                delete_all_scope,
                ..DelegatedSpecOptions::default()
            };
            delegated_spec_from_options(&module_ep(""), &opts)
                .unwrap()
                .mirror_mode
        };
        assert_eq!(cell(false, false), MirrorMode::Off as i32);
        assert_eq!(cell(false, true), MirrorMode::Off as i32);
        assert_eq!(cell(true, false), MirrorMode::FilteredSubset as i32);
        assert_eq!(cell(true, true), MirrorMode::All as i32);
    }

    /// Field carriage: every remaining option lands on its wire field,
    /// and the built spec passes the receiver's own normalization gate
    /// (what `pull_sync_wrapper_emits_same_spec_as_build_spec_from_options`
    /// used to prove via the wire).
    #[test]
    fn options_carry_onto_the_wire_spec_and_normalize() {
        let filter = FilterSpec {
            exclude: vec!["*.tmp".into()],
            ..Default::default()
        };
        let opts = DelegatedSpecOptions {
            force_grpc: true,
            ignore_existing: true,
            require_complete_scan: true,
            drop_windows_metadata: true,
            resume: true,
            block_size: 4096,
            filter: Some(filter.clone()),
            ..DelegatedSpecOptions::default()
        };
        let spec = delegated_spec_from_options(&module_ep("x"), &opts).unwrap();
        assert!(spec.force_grpc);
        assert!(spec.ignore_existing);
        assert!(spec.require_complete_scan);
        assert!(spec.drop_windows_metadata);
        let resume = spec.resume.expect("resume settings present");
        assert!(resume.enabled);
        assert_eq!(resume.block_size, 4096);
        assert_eq!(spec.filter.as_ref(), Some(&filter));

        let normalized = NormalizedTransferOperation::from_spec(spec)
            .expect("built spec passes the receiver gate");
        assert!(normalized.force_grpc);
        assert!(normalized.ignore_existing);
        assert!(normalized.require_complete_scan);
        assert!(normalized.drop_windows_metadata);
        assert!(normalized.resume.enabled);
        assert!(normalized.filter.is_some());
    }
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
            force_grpc: false,
            ignore_existing: false,
            require_complete_scan: false,
            drop_windows_metadata: false,
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
    fn unknown_compare_mode_rejected() {
        let mut spec = empty_spec();
        spec.compare_mode = 999;
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(err.to_string().contains("compare_mode 999"));
    }

    #[test]
    fn unknown_mirror_mode_rejected() {
        let mut spec = empty_spec();
        spec.mirror_mode = 999;
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(err.to_string().contains("mirror_mode 999"));
    }

    #[test]
    fn force_with_ignore_existing_rejected() {
        let mut spec = empty_spec();
        spec.compare_mode = ProtoCompareMode::Force as i32;
        spec.ignore_existing = true;
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(err.to_string().contains("contradictory"));
    }

    #[test]
    fn ignore_existing_passes_through() {
        let mut spec = empty_spec();
        spec.ignore_existing = true;
        let normalized = NormalizedTransferOperation::from_spec(spec).unwrap();
        assert!(normalized.ignore_existing);
    }

    #[test]
    fn malformed_include_glob_rejected() {
        // Unbalanced bracket — globset rejects this.
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            include: vec!["[abc".into()],
            exclude: vec![],
            min_size: None,
            max_size: None,
            min_age_secs: None,
            max_age_secs: None,
            files_from: vec![],
        });
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("invalid include glob"),
            "expected include-glob rejection, got: {msg}"
        );
    }

    #[test]
    fn malformed_exclude_glob_rejected() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            include: vec![],
            exclude: vec!["[bad".into()],
            min_size: None,
            max_size: None,
            min_age_secs: None,
            max_age_secs: None,
            files_from: vec![],
        });
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("invalid exclude glob"),
            "expected exclude-glob rejection, got: {msg}"
        );
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

    #[test]
    fn drop_windows_metadata_passes_through_normalization() {
        let mut spec = empty_spec();
        spec.drop_windows_metadata = true;
        let normalized = NormalizedTransferOperation::from_spec(spec).unwrap();
        assert!(normalized.drop_windows_metadata);
    }
}
