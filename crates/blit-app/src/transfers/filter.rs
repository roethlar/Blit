//! Filter assembly for transfer + check verbs.
//!
//! Moved from `crates/blit-cli/src/transfers/mod.rs` in A.0.
//! Pre-A.0 the struct had a `from_transfer(&TransferArgs)`
//! constructor — that's now in the CLI as inline field-by-field
//! construction (orphan rule prevents the impl living here:
//! blit-app can't `impl FilterInputs` for `&TransferArgs` because
//! `TransferArgs` lives in blit-cli). Callers explicitly fill the
//! struct, which is also the shape the TUI's transfer-options
//! modal will use.

use blit_core::fs_enum::{parse_duration, parse_size, FileFilter};
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
    pub min_size: Option<&'a str>,
    pub max_size: Option<&'a str>,
    pub min_age: Option<&'a str>,
    pub max_age: Option<&'a str>,
}

/// Build a `FileFilter` from filter inputs. Used by every command
/// (copy/mirror/move/check) so filter behavior is identical
/// regardless of which CLI verb invoked it. The orchestrator-side
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
        // Captured once per command invocation — calculated by orchestrator-side
        // helper, not by leaf code each time `allows_entry` is called.
        filter.reference_time = Some(SystemTime::now());
    }
    if let Some(path) = inputs.files_from {
        filter.files_from = Some(FileFilter::load_files_from(path)?);
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
pub fn build_spec(inputs: &FilterInputs<'_>) -> Result<blit_core::generated::FilterSpec> {
    use blit_core::generated::FilterSpec;
    let mut spec = FilterSpec {
        include: inputs.include.to_vec(),
        exclude: inputs.exclude.to_vec(),
        min_size: None,
        max_size: None,
        min_age_secs: None,
        max_age_secs: None,
        files_from: Vec::new(),
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
    Ok(spec)
}
