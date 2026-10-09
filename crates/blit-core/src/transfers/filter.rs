//! Filter assembly for transfer + check verbs.
//!
//! Moved from `crates/blit-cli/src/transfers/mod.rs` in A.0.
//! Pre-A.0 the struct had a `from_transfer(&TransferArgs)`
//! constructor — that's now in the CLI as inline field-by-field
//! construction (orphan rule prevents the impl living here:
//! the library can't `impl FilterInputs` for `&TransferArgs` because
//! `TransferArgs` lives in blit-cli). Callers explicitly fill the
//! struct, which is also the shape any front-end's transfer-options
//! modal will use.

use crate::fs_enum::{parse_duration, parse_size, FileFilter};
use eyre::{eyre, Context, Result};
use std::path::PathBuf;
use std::time::SystemTime;

/// Common shape of the filter inputs across commands. Both
/// `TransferArgs` (copy/mirror/move) and `CheckArgs` (check)
/// populate this with their respective field aliases. The single
/// [`build`] helper consumes it so all commands route through
/// identical filter semantics.
pub struct FilterInputs<'a> {
    pub include: &'a [String],
    pub exclude: &'a [String],
    pub files_from: Option<&'a PathBuf>,
    /// ssc-6: an in-memory exact-path set (the previous pass's failed
    /// files) that overrides `files_from` for a retry pass. Never set by
    /// a CLI flag — the orchestrator threads it between passes.
    pub retry_only: Option<&'a std::collections::HashSet<PathBuf>>,
    /// JOB_LOGS review cr-jl4fix1-2: retry entries named by their exact
    /// bytes (`/`-separated) — names that are not UTF-8 — kept as bytes so
    /// any host can send them; only a host that holds such names turns them
    /// into its own paths.
    pub retry_only_raw: Option<&'a std::collections::HashSet<Vec<u8>>>,
    pub min_size: Option<&'a str>,
    pub max_size: Option<&'a str>,
    pub min_age: Option<&'a str>,
    pub max_age: Option<&'a str>,
}

/// Build a `FileFilter` from filter inputs. Used by every command
/// (copy/mirror/move/check) so filter behavior is identical
/// regardless of which CLI verb invoked it. The transfer-side
/// helper — not the leaf code — is what calculates the filter.
///
/// Validates glob patterns at construction time and surfaces
/// malformed globs with a `--include`/`--exclude` pointer (R58-F12).
pub fn build(inputs: &FilterInputs<'_>) -> Result<FileFilter> {
    let mut filter = FileFilter::default();
    filter.include_files = inputs.include.to_vec();
    filter.exclude_files = inputs.exclude.to_vec();
    if let Some(s) = inputs.min_size {
        filter.min_size = Some(parse_size(s).with_context(|| format!("--min-size {s}"))?);
    }
    if let Some(s) = inputs.max_size {
        filter.max_size = Some(parse_size(s).with_context(|| format!("--max-size {s}"))?);
    }
    if let Some(s) = inputs.min_age {
        filter.min_age = Some(parse_duration(s).with_context(|| format!("--min-age {s}"))?);
    }
    if let Some(s) = inputs.max_age {
        filter.max_age = Some(parse_duration(s).with_context(|| format!("--max-age {s}"))?);
    }
    if filter.min_age.is_some() || filter.max_age.is_some() {
        // Captured once per command invocation — calculated by transfer-side
        // helper, not by leaf code each time `allows_entry` is called.
        filter.reference_time = Some(SystemTime::now());
    }
    if let Some(path) = inputs.files_from {
        filter.files_from = Some(FileFilter::load_files_from(path)?);
    }
    if let Some(set) = inputs.retry_only {
        filter.files_from = Some(set.clone());
    }
    if let Some(raw) = inputs.retry_only_raw {
        let listed = filter.files_from.get_or_insert_with(Default::default);
        listed.extend(raw.iter().filter_map(|raw| {
            crate::raw_name::path_from_received_raw(
                raw,
                crate::raw_name::destination_can_store_raw_names(),
            )
        }));
    }
    // R58-F12: validate glob patterns at filter-construction
    // time. The runtime build_globset silently drops invalid
    // patterns (which is OK as a defense-in-depth fallback for
    // corrupted profiles), but at this layer we want to reject
    // malformed globs up front with a pointer to the bad
    // pattern. Operation-spec normalization already validates on
    // the remote-pull path; this closes the symmetry gap for
    // local / push paths.
    filter
        .validate_globs()
        .map_err(|msg| eyre!("invalid filter pattern: {msg}"))?;
    Ok(filter)
}

/// Build the wire-side `FilterSpec` proto message from the same
/// filter inputs. Used by the remote push path so the daemon
/// enforces the same filter the CLI would have applied locally.
/// `--files-from` is read here and shipped expanded so the daemon
/// doesn't have to reach back into the client's filesystem.
pub fn build_spec(inputs: &FilterInputs<'_>) -> Result<crate::generated::FilterSpec> {
    use crate::generated::FilterSpec;
    let mut spec = FilterSpec {
        include: inputs.include.to_vec(),
        exclude: inputs.exclude.to_vec(),
        min_size: None,
        max_size: None,
        min_age_secs: None,
        max_age_secs: None,
        files_from: Vec::new(),
        files_from_raw: Vec::new(),
    };
    if let Some(s) = inputs.min_size {
        spec.min_size = Some(parse_size(s).with_context(|| format!("--min-size {s}"))?);
    }
    if let Some(s) = inputs.max_size {
        spec.max_size = Some(parse_size(s).with_context(|| format!("--max-size {s}"))?);
    }
    if let Some(s) = inputs.min_age {
        spec.min_age_secs = Some(
            parse_duration(s)
                .with_context(|| format!("--min-age {s}"))?
                .as_secs(),
        );
    }
    if let Some(s) = inputs.max_age {
        spec.max_age_secs = Some(
            parse_duration(s)
                .with_context(|| format!("--max-age {s}"))?
                .as_secs(),
        );
    }
    if let Some(path) = inputs.files_from {
        let entries = FileFilter::load_files_from(path)?;
        spec.files_from = entries
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
    }
    if let Some(set) = inputs.retry_only {
        // JOB_LOGS review cr-jl4-2: a name that is not UTF-8 travels as its
        // exact bytes; its lossy text would name no file at the origin.
        let mut entries: Vec<String> = Vec::new();
        let mut raw_entries: Vec<Vec<u8>> = Vec::new();
        for path in set {
            match crate::raw_name::raw_relative_bytes(path) {
                Some(raw) => raw_entries.push(raw),
                None => entries.push(crate::path_posix::relative_path_to_posix(path)),
            }
        }
        entries.sort();
        spec.files_from = entries;
        spec.files_from_raw = raw_entries;
    }
    if let Some(raw) = inputs.retry_only_raw {
        spec.files_from_raw.extend(raw.iter().cloned());
    }
    spec.files_from_raw.sort();
    spec.files_from_raw.dedup();
    // review otp-10a F8: validate the globs at construction time, like
    // `build` does (R58-F12) — a malformed `--include`/`--exclude`
    // must fail before any connection is opened, not when the session
    // end validates the spec at OPEN.
    let mut probe = FileFilter::default();
    probe.include_files = spec.include.clone();
    probe.exclude_files = spec.exclude.clone();
    probe
        .validate_globs()
        .map_err(|msg| eyre!("invalid filter pattern: {msg}"))?;
    Ok(spec)
}

#[cfg(test)]
mod tests {
    //! audit-6 item 1: transfer orchestration glue. `build` / `build_spec`
    //! are pure filter-assembly helpers every transfer/check verb routes
    //! through, so their semantics (glob propagation, size/age parsing,
    //! reference-time capture, malformed-input rejection) are worth
    //! pinning directly.
    use super::*;

    /// JOB_LOGS review cr-jl4fix1-2: a retry's raw names reach a local
    /// source's filter as its own paths where it can hold such names
    /// (Linux: runs in CI).
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_retry_raw_name_reaches_a_local_filter() {
        use std::os::unix::ffi::OsStrExt;
        let names: std::collections::HashSet<PathBuf> = [PathBuf::from("plain.txt")].into();
        let raw: std::collections::HashSet<Vec<u8>> = [b"bad\xff.txt".to_vec()].into();
        let filter = build(&FilterInputs {
            include: &[],
            exclude: &[],
            files_from: None,
            retry_only: Some(&names),
            retry_only_raw: Some(&raw),
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
        })
        .unwrap();
        let listed = filter.files_from.expect("a list");
        assert!(listed.contains(&PathBuf::from("plain.txt")));
        assert!(listed
            .iter()
            .any(|path| path.as_os_str().as_bytes() == b"bad\xff.txt"));
    }

    fn inputs<'a>(include: &'a [String], exclude: &'a [String]) -> FilterInputs<'a> {
        FilterInputs {
            include,
            exclude,
            files_from: None,
            retry_only: None,
            retry_only_raw: None,
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
        }
    }

    #[test]
    fn build_empty_inputs_yields_unconstrained_filter() {
        let f = build(&inputs(&[], &[])).unwrap();
        assert!(f.include_files.is_empty());
        assert!(f.exclude_files.is_empty());
        assert_eq!(f.min_size, None);
        assert_eq!(f.max_size, None);
        assert_eq!(f.min_age, None);
        assert_eq!(f.max_age, None);
        assert!(
            f.reference_time.is_none(),
            "no age constraint ⇒ no reference_time captured"
        );
    }

    #[test]
    fn build_propagates_globs_and_sizes() {
        let inc = vec!["*.rs".to_string()];
        let exc = vec!["*.tmp".to_string()];
        let mut i = inputs(&inc, &exc);
        i.min_size = Some("10M");
        i.max_size = Some("1G");
        let f = build(&i).unwrap();
        assert_eq!(f.include_files, inc);
        assert_eq!(f.exclude_files, exc);
        // Routes through blit-core's parse_size — cross-check the wiring.
        assert_eq!(f.min_size, Some(parse_size("10M").unwrap()));
        assert_eq!(f.max_size, Some(parse_size("1G").unwrap()));
    }

    #[test]
    fn build_age_constraint_captures_reference_time() {
        let mut i = inputs(&[], &[]);
        i.max_age = Some("7d");
        let f = build(&i).unwrap();
        assert_eq!(f.max_age, Some(parse_duration("7d").unwrap()));
        assert!(
            f.reference_time.is_some(),
            "an age constraint must capture reference_time once at build time"
        );
    }

    #[test]
    fn build_rejects_malformed_glob_with_pointer() {
        let inc = vec!["a[".to_string()]; // unclosed character class
        let err = build(&inputs(&inc, &[])).unwrap_err();
        assert!(
            format!("{err:#}").contains("invalid filter pattern"),
            "expected a glob-pattern pointer, got: {err:#}"
        );
    }

    #[test]
    fn build_rejects_bad_size_with_flag_context() {
        let mut i = inputs(&[], &[]);
        i.min_size = Some("not-a-size");
        let err = build(&i).unwrap_err();
        assert!(
            format!("{err:#}").contains("--min-size"),
            "expected the --min-size flag in the error, got: {err:#}"
        );
    }

    #[test]
    fn build_spec_maps_age_to_seconds_and_propagates_globs() {
        let inc = vec!["*.log".to_string()];
        let mut i = inputs(&inc, &[]);
        i.min_age = Some("1h");
        let spec = build_spec(&i).unwrap();
        assert_eq!(spec.include, inc);
        assert_eq!(
            spec.min_age_secs,
            Some(parse_duration("1h").unwrap().as_secs())
        );
        assert_eq!(spec.max_age_secs, None);
    }

    /// review otp-10a F8: the wire-spec builder rejects malformed globs
    /// up front, exactly like `build` — a bad `--exclude` on a push
    /// verb must fail before any connection is opened.
    #[test]
    fn build_spec_rejects_malformed_glob_before_any_connection() {
        let exc = vec!["a[".to_string()]; // unclosed character class
        let err = build_spec(&inputs(&[], &exc)).unwrap_err();
        assert!(
            format!("{err:#}").contains("invalid filter pattern"),
            "expected a glob-pattern pointer, got: {err:#}"
        );
    }
}
