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
///   - 4: `FilterSpec.ignore_case` (EXCLUDE_CASE_MATCHING slice 3,
///     cr-ecm3-1). Safety-critical in the same way as 2: the session's
///     contract version gates the two daemons of a delegated transfer
///     against EACH OTHER, but nothing gated the machine that asks
///     against the daemon it asks. Two daemons of the earlier contract,
///     handed a version-3 spec holding the new field, ignored it, read
///     every pattern exactly, and a `mirror --ignore-case` copied what it
///     was told to leave and deleted what it was told to keep, exit 0.
///     A spec that carries a field an older daemon does not know must
///     carry a version that daemon refuses.
///   - 5: `FilterSpec.files_from_set` (cr-ecm4-2, cr-ecm5-1), by that
///     same rule — which cr-ecm4-2 did not follow: it added the field
///     under version 4. A version-4 daemon discards it, reads a list
///     that holds no name as no list, and its mirror deletes what the
///     list kept out of scope.
///   - 6: no new field — the destination's deletion rule under a name
///     list changed (cr-ecm5-5, cr-ecm6-3): a directory the list does not
///     name is no longer a candidate. A version-5 daemon, handed the same
///     request, still removes such a directory when it is empty; the
///     request names the version of the rule it expects.
///   - 7: no new field — what a `FilterSpec.files_from` entry IS changed
///     (cr-ecm7-1): a name's components joined by `/`, read component by
///     component, where it was the text the sending host writes a path
///     in. A version-6 daemon on Linux, handed a Windows list's
///     `sub\name.txt`, reads one name; a version-6 daemon on Windows,
///     handed a Linux name that holds a `\`, reads two.
///   - 8: no new field of its own — what `source_path` says changed
///     (cr-uv2-1): it keeps the `/` that marks a source typed as a
///     directory, and the daemon holding the source refuses one that is
///     none. And the filter it carries may hold `files_from_root`
///     (cr-ecm11-1). cr-uv3-2: both went out under version 7, against the
///     rule above. Two daemons of the earlier build, asked by a newer
///     CLI, dropped the mark and copied `one.txt/` as the file it is; a
///     newer pair, asked by an older CLI, were sent no mark and did the
///     same. The contract version gates the two daemons against each
///     other, never the machine that asks.
///   - 9: `source_removed_after` (cr-uv3-6), by the rule of version 4: a
///     daemon that does not know the field would not learn that a move's
///     source is to be removed, and would serve a move through a link
///     typed as a directory — everything copied, nothing removed.
///   - 10: `FilterSpec.files_from_exact` (cr-ecm12-1), by the same rule:
///     a daemon that does not know the field reads a retry's list as a
///     list of names, and lets a source that has become one file answer
///     to a child's name.
///   - 11: `FilterSpec.files_from_left` (cr-ecm12-2), by the same rule: a
///     daemon that does not know the field reads the retry of an
///     `ignore_existing` run's own incomplete copies as any other list,
///     skips each as a file to keep, and reports the path copied.
///   - 12: `FilterSpec.files_from_left` means something else (cr-ecm13-1):
///     nothing that stands at a listed path is written over. A daemon of
///     version 11 would replace a file there that carries the mark that
///     version stamped on an incomplete copy.
pub const SUPPORTED_SPEC_VERSION: u32 = 12;

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
    /// cr-uv3-6: the asker removes the source when the transfer ends well.
    pub source_removed_after: bool,
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
            source_removed_after: spec.source_removed_after,
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
///
/// EXCLUDE_CASE_MATCHING slice 1b (AC-10) and cr-ecm1-2: the filter is
/// then held to the SAME validation a command line's is
/// ([`FileFilter::validate_globs`]) — a class holding a character beyond
/// ASCII, and a pattern that parses but cannot be compiled, are refused
/// here in the same words, which is at session open on either role.
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
    // EXCLUDE_CASE_MATCHING slice 3 (contract 9): the peer's case mode.
    filter.ignore_case = spec.ignore_case;
    filter.min_size = spec.min_size;
    filter.max_size = spec.max_size;
    filter.min_age = spec.min_age_secs.map(std::time::Duration::from_secs);
    filter.max_age = spec.max_age_secs.map(std::time::Duration::from_secs);
    if filter.min_age.is_some() || filter.max_age.is_some() {
        filter.reference_time = Some(SystemTime::now());
    }
    // cr-ecm4-2: a list that was given is a list, however many names it
    // holds — an empty one lets nothing pass, exactly as it does where it
    // was read. (The two lists alone cannot tell "empty" from "none".)
    if spec.files_from_set
        || spec.files_from_root
        || !spec.files_from.is_empty()
        || !spec.files_from_raw.is_empty()
        || !spec.files_from_unnamed.is_empty()
    {
        let mut listed = std::collections::HashSet::new();
        // cr-ecm11-1: the source itself, by the mark a list has for it.
        if spec.files_from_root {
            listed.insert(std::path::PathBuf::new());
        }
        let mut unnamed = spec.files_from_unnamed;
        for name in spec.files_from {
            match listed_name(&name, cfg!(windows))? {
                Some(path) => {
                    listed.insert(path);
                }
                // A name where it was listed, and no entry's name here.
                None => unnamed.push(name),
            }
        }
        unnamed.sort();
        unnamed.dedup();
        filter.files_from_unnamed = unnamed;
        listed.extend(raw_listed(
            &spec.files_from_raw,
            crate::raw_name::destination_can_store_raw_names(),
        ));
        filter.files_from = Some(listed);
        // cr-ecm12-1: what kind of list it is travels with it.
        filter.files_from_exact = spec.files_from_exact;
        // cr-ecm12-2: and of what.
        filter.files_from_left = spec.files_from_left;
    }
    filter
        .validate_globs()
        .map_err(|refusal| eyre::eyre!(refusal))?;
    Ok(filter)
}

/// cr-ecm7-1: one listed name off the wire, as this host's path. The wire
/// form is a name's components joined by `/` and nothing else, so each
/// piece between two `/` is ONE component whatever characters it holds —
/// it is never handed to this host's path parser to split again
/// (`windows`: the path rule it is read under, this host's own outside
/// tests):
/// - a piece that is no name at all — empty, `.`, `..`, or holding a NUL —
///   refuses the list: no sender of this contract writes one;
/// - a piece that is a name where it was written and cannot be one here —
///   it holds `\` where that separates components, or this host reads it
///   as a drive — names no entry of this host (`None`). It is never read
///   as the path its text would be here; the caller keeps it with the
///   lines that name nothing (`FileFilter::files_from_unnamed`), and a
///   source on this host reports it as listed and not found.
fn listed_name(wire: &str, windows: bool) -> Result<Option<std::path::PathBuf>> {
    let mut path = std::path::PathBuf::new();
    let mut named_here = true;
    for component in wire.split('/') {
        if component.is_empty() || component == "." || component == ".." || component.contains('\0')
        {
            bail!(
                "the listed name {wire:?} is not a relative path of names joined by `/`, which \
                 is how a `--files-from` name travels"
            );
        }
        named_here = named_here && one_name_here(component, windows);
        if named_here {
            path.push(component);
        }
    }
    Ok(named_here.then_some(path))
}

/// Whether `component` — one piece of a wire name — is one entry's name
/// under a platform's path rule. Under this host's own rule its path
/// parser is asked too: it must read the piece as exactly one name.
fn one_name_here(component: &str, windows: bool) -> bool {
    if windows && component.contains('\\') {
        return false;
    }
    if windows != cfg!(windows) {
        return true;
    }
    let mut parts = std::path::Path::new(component).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(name)), None) if name == std::ffi::OsStr::new(component)
    )
}

/// JOB_LOGS review cr-jl4-2: the listed paths a spec names by their exact
/// bytes, as this host's paths — none where this host cannot hold such
/// names (`storable`), since no file of its own can carry one.
fn raw_listed(raw: &[Vec<u8>], storable: bool) -> Vec<std::path::PathBuf> {
    raw.iter()
        .filter_map(|raw| crate::raw_name::path_from_received_raw(raw, storable))
        .collect()
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
    /// cr-uv3-6: a move — the source is removed when the transfer ends well.
    pub source_removed_after: bool,
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

    // As typed (cr-uv2-1): the daemon that pulls opens its session to the
    // source with this path, and the source must be told what was asked.
    let path_str = match crate::path_posix::source_path_to_posix(&rel_path) {
        root if root.is_empty() => ".".to_string(),
        path => path,
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
        source_removed_after: options.source_removed_after,
        drop_windows_metadata: options.drop_windows_metadata,
    })
}

#[cfg(test)]
mod delegated_spec_tests {
    //! otp-10c-2 review F3: the delegated trigger's spec builder is
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

    /// cr-uv2-1: a source typed as a directory is delegated as one.
    #[test]
    fn a_source_typed_as_a_directory_is_delegated_as_one() {
        // cr-uv3-5: the root is `.`, as it always was, and `./` when a
        // `.` was typed after the module.
        for (typed, wire) in [
            ("a/b/", "a/b/"),
            ("a/b/.", "a/b/"),
            ("", "."),
            ("./", "./"),
            (".", "./"),
        ] {
            let spec =
                delegated_spec_from_options(&module_ep(typed), &DelegatedSpecOptions::default())
                    .unwrap();
            assert_eq!(spec.source_path, wire, "{typed:?}");
        }
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
            source_removed_after: true,
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
        assert!(spec.source_removed_after, "and that the source is removed");
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

    /// JOB_LOGS review cr-jl4-2: a retry's file named by its exact bytes
    /// is listed by them on an origin that can hold such names, and on
    /// one that cannot, names nothing.
    #[cfg(unix)]
    #[test]
    fn raw_listed_paths_name_files_by_their_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let raw = vec![b"dir/bad\xff.txt".to_vec()];
        let listed = raw_listed(&raw, true);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].as_os_str().as_bytes(), b"dir/bad\xff.txt");
        assert!(raw_listed(&raw, false).is_empty());
    }

    /// ...and a spec's raw list reaches the filter, beside its text list,
    /// on an origin that can hold such names (Linux: runs in CI).
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_specs_raw_list_reaches_the_filter() {
        use std::os::unix::ffi::OsStrExt;
        let filter = filter_from_spec(ProtoFilterSpec {
            files_from: vec!["plain.txt".into()],
            files_from_raw: vec![b"bad\xff.txt".to_vec()],
            files_from_unnamed: Vec::new(),
            ..ProtoFilterSpec::default()
        })
        .unwrap();
        let listed = filter.files_from.expect("a list");
        assert!(listed.contains(std::path::Path::new("plain.txt")));
        assert!(listed
            .iter()
            .any(|path| path.as_os_str().as_bytes() == b"bad\xff.txt"));
    }

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
            source_removed_after: false,
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
            files_from_raw: Vec::new(),
            files_from_unnamed: Vec::new(),
            files_from_root: false,
            files_from_exact: false,
            files_from_left: false,
            ignore_case: false,
            files_from_set: false,
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
            files_from_raw: Vec::new(),
            files_from_unnamed: Vec::new(),
            files_from_root: false,
            files_from_exact: false,
            files_from_left: false,
            ignore_case: false,
            files_from_set: false,
        });
        let err = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("invalid exclude glob"),
            "expected exclude-glob rejection, got: {msg}"
        );
    }

    /// EXCLUDE_CASE_MATCHING AC-10 (slice 1b): a peer's filter is held to
    /// the validation a command line's is. A class holding a character
    /// beyond ASCII is refused in the same words, for an include and an
    /// exclude; the spelling it advises is accepted.
    #[test]
    fn a_peers_class_beyond_ascii_is_refused_like_a_typed_one() {
        for include in [false, true] {
            let patterns = vec!["[aé].txt".to_string()];
            let error = filter_from_spec(ProtoFilterSpec {
                include: if include {
                    patterns.clone()
                } else {
                    Vec::new()
                },
                exclude: if include { Vec::new() } else { patterns },
                ..ProtoFilterSpec::default()
            })
            .expect_err("a class beyond ASCII");
            let msg = format!("{error:#}");
            assert!(
                msg.contains("cannot work") && msg.contains(r#"Write "{a,é}.txt" instead"#),
                "{msg}"
            );
        }
        let filter = filter_from_spec(ProtoFilterSpec {
            exclude: vec!["{a,é}.txt".to_string()],
            ..ProtoFilterSpec::default()
        })
        .expect("the advised spelling");
        assert_eq!(filter.exclude_files, ["{a,é}.txt"]);
    }

    /// cr-ecm1-2, its known gap closed: a peer's pattern that PARSES and
    /// cannot be compiled is refused here too. Before, the receiving end
    /// only parsed, and a set that cannot be built is compared as text.
    #[test]
    fn a_peers_pattern_that_cannot_be_compiled_is_refused() {
        let nested = |depth: usize| format!("{}s{}", "{".repeat(depth), "}".repeat(depth));
        let accepted = |depth: usize| {
            filter_from_spec(ProtoFilterSpec {
                exclude: vec!["*.cfg".to_string(), nested(depth)],
                ..ProtoFilterSpec::default()
            })
        };
        let deepest = (1..2000)
            .take_while(|&depth| accepted(depth).is_ok())
            .last()
            .expect("one level compiles");
        assert!(deepest < 1999, "the compiler has a nesting limit");
        let too_deep = nested(deepest + 1);
        globset::Glob::new(&too_deep).expect("one level deeper still parses");
        let msg = format!(
            "{:#}",
            accepted(deepest + 1).expect_err("too deep to compile")
        );
        assert!(msg.contains("cannot be compiled"), "{msg}");
    }

    /// cr-ecm3-1: the case mode is a field of the spec a daemon is HANDED
    /// (the delegated request), and no session stands between the machine
    /// that asks and that daemon to compare contracts. The spec's own
    /// version is the gate: version 3 — the last one that did not know the
    /// field — is refused, so a daemon that knows the field refuses a
    /// client that does not, and a daemon of version 3 refuses this one.
    #[test]
    fn a_spec_of_the_version_before_the_case_mode_is_refused() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            exclude: vec!["*cache".to_string()],
            ignore_case: true,
            ..ProtoFilterSpec::default()
        });
        NormalizedTransferOperation::from_spec(spec.clone()).expect("this build's own version");
        spec.spec_version = 3;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 3"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 4,
            "a spec that carries the case mode is version 4 or later, not {version}"
        );
    }

    /// cr-ecm5-1: a request that says a list of exact names was given
    /// carries a version a daemon of spec version 4 refuses — that daemon
    /// would discard the marker and delete what the list kept out of
    /// scope — and this build refuses a version-4 request in turn.
    #[test]
    fn a_request_with_the_empty_list_marker_is_version_5_or_later() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            include: vec!["*.jpg".to_string()],
            files_from_set: true,
            ..ProtoFilterSpec::default()
        });
        let accepted =
            NormalizedTransferOperation::from_spec(spec.clone()).expect("this build's own version");
        let filter = accepted.filter.as_ref().expect("a filter");
        let listed = filter.files_from.as_ref().expect("a list is in force");
        assert!(listed.is_empty(), "and it holds no name");
        spec.spec_version = 4;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 4"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 5,
            "a spec that carries the empty-list marker is version 5 or later, not {version}"
        );
    }

    /// cr-uv3-2: a request whose source keeps the mark that it was typed
    /// as a directory, or whose list names the source itself, carries a
    /// version a daemon of spec version 7 refuses — that daemon would drop
    /// the mark and copy `one.txt/` as the file — and this build refuses a
    /// version-7 request in turn, whose sender sent no mark.
    #[test]
    fn a_request_that_says_how_its_source_was_typed_is_version_8_or_later() {
        let mut spec = empty_spec();
        spec.source_path = "one.txt/".to_string();
        spec.filter = Some(ProtoFilterSpec {
            files_from_root: true,
            ..ProtoFilterSpec::default()
        });
        let accepted =
            NormalizedTransferOperation::from_spec(spec.clone()).expect("this build's own version");
        assert_eq!(
            accepted.source_path, "one.txt/",
            "the mark is kept for the source"
        );
        assert!(accepted
            .filter
            .as_ref()
            .is_some_and(|filter| filter.lists_the_source_itself()));
        spec.spec_version = 7;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 7"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 8,
            "a request that says how its source was typed is version 8 or later, not {version}"
        );
    }

    /// cr-uv3-6: a request that says its source will be removed carries a
    /// version a daemon of spec version 8 refuses — that daemon read the
    /// intent off the complete-scan flag and does not know the field —
    /// and the field reaches the normalized operation as it was sent,
    /// whatever the complete-scan flag says.
    #[test]
    fn a_request_that_says_its_source_is_removed_is_version_9_or_later() {
        for (removed, complete) in [(true, true), (false, true), (true, false), (false, false)] {
            let mut spec = empty_spec();
            spec.source_removed_after = removed;
            spec.require_complete_scan = complete;
            let accepted = NormalizedTransferOperation::from_spec(spec).expect("this build's own");
            assert_eq!(
                (
                    accepted.source_removed_after,
                    accepted.require_complete_scan
                ),
                (removed, complete)
            );
        }
        let mut spec = empty_spec();
        spec.source_removed_after = true;
        spec.spec_version = 8;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 8"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 9,
            "a request that says its source is removed is version 9 or later, not {version}"
        );
    }

    /// cr-ecm12-1: a request whose list is of exact paths carries a
    /// version a daemon of spec version 9 refuses — that daemon would read
    /// it as a list of names — and the kind of list reaches the filter.
    #[test]
    fn a_request_with_a_list_of_exact_paths_is_version_10_or_later() {
        for exact in [true, false] {
            let mut spec = empty_spec();
            spec.filter = Some(ProtoFilterSpec {
                files_from: vec!["Report.txt".to_string()],
                files_from_set: true,
                files_from_exact: exact,
                ..ProtoFilterSpec::default()
            });
            let accepted = NormalizedTransferOperation::from_spec(spec).expect("this build's own");
            assert_eq!(accepted.filter.expect("a filter").files_from_exact, exact);
        }
        let mut spec = empty_spec();
        spec.spec_version = 9;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 9"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 10,
            "a request with a list of exact paths is version 10 or later, not {version}"
        );
    }

    /// cr-ecm12-2: a request whose list is of this run's own incomplete
    /// copies carries a version a daemon of spec version 10 refuses — that
    /// daemon would skip each as a file to keep — and what the list is
    /// reaches the filter. cr-ecm13-1: and one a daemon of version 11
    /// refuses, which would replace a file at such a path by a mark.
    #[test]
    fn a_request_for_the_runs_own_incomplete_copies_is_version_12_or_later() {
        for left in [true, false] {
            let mut spec = empty_spec();
            spec.filter = Some(ProtoFilterSpec {
                files_from: vec!["Report.txt".to_string()],
                files_from_set: true,
                files_from_exact: true,
                files_from_left: left,
                ..ProtoFilterSpec::default()
            });
            let accepted = NormalizedTransferOperation::from_spec(spec).expect("this build's own");
            assert_eq!(accepted.filter.expect("a filter").files_from_left, left);
        }
        for earlier in [10, 11] {
            let mut spec = empty_spec();
            spec.spec_version = earlier;
            let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!("spec_version {earlier}")),
                "{error:#}"
            );
        }
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 12,
            "a request for the run's own incomplete copies is version 12 or later, not {version}"
        );
    }

    /// cr-ecm6-3: a request whose mirror is scoped by a name list is
    /// answered by the rule of spec version 6 — a directory the list does
    /// not name is never deleted — and a daemon of version 5, which plans
    /// otherwise, is not asked: it refuses the version, and this build
    /// refuses a version-5 request in turn.
    #[test]
    fn a_request_under_the_name_list_deletion_rule_is_version_6_or_later() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            files_from_set: true,
            ..ProtoFilterSpec::default()
        });
        NormalizedTransferOperation::from_spec(spec.clone()).expect("this build's own version");
        spec.spec_version = 5;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 5"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 6,
            "a request under the name-list deletion rule is version 6 or later, not {version}"
        );
    }

    /// EXCLUDE_CASE_MATCHING slice 3 (AC-6): a peer's case mode reaches the
    /// filter its patterns are matched with. A spec that carries the mode
    /// and no pattern is still no filter: there is nothing for it to read.
    #[test]
    fn a_peers_case_mode_reaches_the_filter() {
        let named = |ignore_case: bool| {
            let mut spec = empty_spec();
            spec.filter = Some(ProtoFilterSpec {
                exclude: vec!["*cache".to_string()],
                ignore_case,
                ..ProtoFilterSpec::default()
            });
            let filter = NormalizedTransferOperation::from_spec(spec)
                .unwrap()
                .filter
                .expect("a filter with a pattern");
            assert_eq!(filter.ignore_case, ignore_case);
            let rel = std::path::Path::new("aCache");
            !filter.allows_entry(Some(rel), rel, 1, None)
        };
        assert!(!named(false), "as written, *cache does not name aCache");
        assert!(named(true), "the peer's mode is the filter's mode");

        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            ignore_case: true,
            ..ProtoFilterSpec::default()
        });
        assert!(NormalizedTransferOperation::from_spec(spec)
            .unwrap()
            .filter
            .is_none());
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
            files_from_raw: Vec::new(),
            files_from_unnamed: Vec::new(),
            files_from_root: false,
            files_from_exact: false,
            files_from_left: false,
            ignore_case: false,
            files_from_set: false,
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

    /// cr-ecm7-1: a listed name off the wire is read component by
    /// component — each piece between two `/` one name, whatever it holds
    /// — and never handed to this host's path parser to split again.
    #[test]
    fn a_listed_name_off_the_wire_is_read_by_its_components() {
        use std::path::{Path, PathBuf};
        // A file in a folder, under either platform's rule.
        for windows in [false, true] {
            let read = listed_name("sub/name.txt", windows).unwrap();
            if windows == cfg!(windows) {
                assert_eq!(read, Some(Path::new("sub").join("name.txt")));
            } else {
                assert!(read.is_some());
            }
        }
        // One piece holding a `\`: one name where that is a character of
        // names, and no entry at all where it separates components —
        // never the two components its text would be there.
        assert!(listed_name(r"sub\name.txt", false).unwrap().is_some());
        assert_eq!(listed_name(r"sub\name.txt", true).unwrap(), None);
        assert_eq!(listed_name(r"dir/sub\name.txt", true).unwrap(), None);
        if !cfg!(windows) {
            let one = listed_name(r"sub\name.txt", false).unwrap().unwrap();
            assert_eq!(one.components().count(), 1, "{one:?}");
        }
        // No name at all: the list is refused, naming it.
        for malformed in [
            "", "a//b", "/abs", "a/", "./a", "a/./b", "a/../b", "..", "a/\0b",
        ] {
            for windows in [false, true] {
                let error = listed_name(malformed, windows).unwrap_err();
                assert!(
                    error.to_string().contains("not a relative path of names"),
                    "{malformed:?}: {error:#}"
                );
            }
        }

        // Through the filter a peer builds from the spec.
        let received = |names: &[&str]| {
            filter_from_spec(ProtoFilterSpec {
                files_from: names.iter().map(|name| name.to_string()).collect(),
                files_from_set: true,
                ..ProtoFilterSpec::default()
            })
        };
        let passes = |filter: &FileFilter, rel: &Path| filter.allows_entry(Some(rel), rel, 1, None);
        let nested: PathBuf = Path::new("sub").join("name.txt");
        let filter = received(&["sub/name.txt"]).unwrap();
        assert!(passes(&filter, &nested));
        assert!(filter.files_from_unnamed.is_empty());
        // The lines that name nothing arrive apart, and stay apart: in
        // force as a list, matched against nothing.
        let apart = filter_from_spec(ProtoFilterSpec {
            files_from_unnamed: vec!["./name.txt".to_string(), "name.txt".to_string()],
            ..ProtoFilterSpec::default()
        })
        .unwrap();
        assert_eq!(apart.files_from.as_ref().map(|names| names.len()), Some(0));
        assert_eq!(apart.files_from_unnamed, ["./name.txt", "name.txt"]);
        assert!(!passes(&apart, Path::new("name.txt")));
        if !cfg!(windows) {
            // The reviewer's case, on the host it was replayed on: the
            // destination's own file called `sub\name.txt` is not the one
            // listed — and is, when the wire holds a Windows list's text,
            // which is what a sender no longer writes.
            let literal = Path::new(r"sub\name.txt");
            assert!(!passes(&filter, literal));
            let as_it_went = received(&[r"sub\name.txt"]).unwrap();
            assert!(passes(&as_it_went, literal) && !passes(&as_it_went, &nested));
        }
        let error = received(&["ok.txt", "a//b"]).unwrap_err();
        assert!(format!("{error:#}").contains(r#""a//b""#), "{error:#}");
    }

    /// cr-ecm7-1: a request whose listed names are components joined by
    /// `/` carries a version a daemon of spec version 6 refuses — that
    /// daemon reads each as the text of a path on its own host — and this
    /// build refuses a version-6 request in turn.
    #[test]
    fn a_request_with_names_listed_by_component_is_version_7_or_later() {
        let mut spec = empty_spec();
        spec.filter = Some(ProtoFilterSpec {
            files_from: vec!["sub/name.txt".to_string()],
            files_from_set: true,
            ..ProtoFilterSpec::default()
        });
        NormalizedTransferOperation::from_spec(spec.clone()).expect("this build's own version");
        spec.spec_version = 6;
        let error = NormalizedTransferOperation::from_spec(spec).unwrap_err();
        assert!(error.to_string().contains("spec_version 6"), "{error:#}");
        // Read at run time: the number is what a daemon is told.
        let version = std::hint::black_box(SUPPORTED_SPEC_VERSION);
        assert!(
            version >= 7,
            "a request whose listed names travel by component is version 7 or later, not {version}"
        );
    }
}
