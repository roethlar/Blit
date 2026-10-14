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

use crate::fs_enum::{normalize_separators, parse_duration, parse_size, FileFilter};
use eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};
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
    /// audit-19: the source root when the source is local. An absolute
    /// `--exclude` under it is made source-relative before use (see
    /// [`resolve_excludes`]); `None` for a remote source, where no
    /// absolute pattern can match.
    pub source_root: Option<&'a Path>,
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
    filter.exclude_files = resolve_excludes(inputs.exclude, inputs.source_root).patterns;
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
        // audit-19: relativized HERE, before the spec leaves this host —
        // the peer that applies it never knows this side's source root.
        exclude: resolve_excludes(inputs.exclude, inputs.source_root).patterns,
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

/// audit-19: the `--exclude` patterns once the source root is known.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ResolvedExcludes {
    /// The patterns the filter applies: each absolute pattern under the
    /// source root rewritten source-relative (`/src/a/.java` under `/src`
    /// is `a/.java`; a single remaining name is anchored as `./name`, so
    /// it still names only that top-level path), every other pattern as
    /// typed — on Windows with `\` written `/` (cr-g3fix-3), so the wire
    /// form reads the same on any peer.
    pub patterns: Vec<String>,
    /// The absolute patterns that cannot match anything: outside the
    /// source root, naming the root itself, or any absolute pattern when
    /// the source is not local. They stay in `patterns`, where they match
    /// nothing; the caller warns about each one.
    pub unmatchable: Vec<String>,
}

/// audit-19: make each absolute `--exclude` under `source_root` relative
/// to it, and name the absolute ones that cannot match. Paths compare the
/// way the platform compares them (case-insensitively on Windows); a root
/// or pattern reached through a symlink is also compared in its resolved
/// form, so `/tmp/x/.java` and `/private/tmp/x/.java` name one path on
/// macOS (`through_resolved_root` says how a pattern is resolved).
/// Relative patterns pass through untouched but for their separators:
/// on Windows `\` is written `/` (cr-g3fix-3).
pub fn resolve_excludes(exclude: &[String], source_root: Option<&Path>) -> ResolvedExcludes {
    resolve_excludes_on(exclude, source_root, cfg!(windows))
}

/// [`resolve_excludes`] under one platform's path rule (`windows`), so
/// either rule's string handling is testable on every OS.
fn resolve_excludes_on(
    exclude: &[String],
    source_root: Option<&Path>,
    windows: bool,
) -> ResolvedExcludes {
    let mut resolved = ResolvedExcludes::default();
    if !exclude
        .iter()
        .any(|pattern| is_absolute_pattern(pattern, windows))
    {
        resolved.patterns = exclude
            .iter()
            .map(|pattern| normalize_separators(pattern, windows).into_owned())
            .collect();
        return resolved;
    }
    let resolved_root = source_root.and_then(|root| std::fs::canonicalize(root).ok());
    let mut roots: Vec<String> = Vec::new();
    if let Some(absolute) = source_root.and_then(|root| std::path::absolute(root).ok()) {
        roots.push(absolute.to_string_lossy().into_owned());
    }
    if let Some(canonical) = &resolved_root {
        roots.push(canonical.to_string_lossy().into_owned());
    }
    for pattern in exclude {
        if !is_absolute_pattern(pattern, windows) {
            resolved
                .patterns
                .push(normalize_separators(pattern, windows).into_owned());
            continue;
        }
        let mut pattern_forms = vec![pattern.clone()];
        if let Some(form) = resolved_root
            .as_deref()
            .and_then(|root| through_resolved_root(pattern, root, windows))
        {
            pattern_forms.push(form);
        }
        let relative = pattern_forms.iter().find_map(|form| {
            roots
                .iter()
                .find_map(|root| relative_to_root(form, root, windows))
        });
        match (relative, source_root) {
            (Some(rest), Some(root)) => {
                let rest = stored_spelling(root, rest, windows, stored_name);
                resolved.patterns.push(written(&rest));
            }
            _ => {
                resolved.patterns.push(pattern.clone());
                resolved.unmatchable.push(pattern.clone());
            }
        }
    }
    resolved
}

/// cr-g3fix-2: `pattern` written through the resolved source root when
/// its leading literal part reaches the source through a symlink
/// (`/alias/*.txt` with `alias` naming the source, or a directory inside
/// it; macOS `/tmp` for `/private/tmp`). The pattern's leading ancestors
/// up to its first component holding a glob metacharacter are resolved,
/// shortest first; the first that exists and lands at or below
/// `resolved_root` is written resolved, and what follows it is re-appended
/// as typed — a glob unchanged, a path that does not exist yet as it was
/// written, and a symlink inside the source unresolved (the walk does not
/// follow one, so resolving it would name its target instead). `None` when
/// no ancestor lands there.
fn through_resolved_root(pattern: &str, resolved_root: &Path, windows: bool) -> Option<String> {
    let path = Path::new(pattern);
    let mut ancestors: Vec<&Path> = path.ancestors().collect();
    ancestors.reverse();
    for ancestor in ancestors {
        if ancestor
            .file_name()
            .is_some_and(|name| has_glob_meta(&name.to_string_lossy(), windows))
        {
            break;
        }
        let Ok(resolved) = std::fs::canonicalize(ancestor) else {
            continue;
        };
        if !resolved.starts_with(resolved_root) {
            continue;
        }
        let Ok(rest) = path.strip_prefix(ancestor) else {
            continue;
        };
        // A `..` after the landing point could climb back out; a longer
        // ancestor folds it in its resolved form instead.
        if rest
            .components()
            .any(|component| component == std::path::Component::ParentDir)
        {
            continue;
        }
        return Some(resolved.join(rest).to_string_lossy().into_owned());
    }
    None
}

/// A component `globset` reads as glob syntax: it opens a wildcard, a
/// class or an alternation — or, off Windows, holds `\`, the escape (on
/// Windows `\` is a separator and never reaches a component).
fn has_glob_meta(component: &str, windows: bool) -> bool {
    component.contains(['*', '?', '[', '{']) || (!windows && component.contains('\\'))
}

/// An absolute path on the platform's terms: rooted on Unix; a drive
/// (`C:\`), UNC (`\\server`) or rooted path on Windows, either slash.
fn is_absolute_pattern(pattern: &str, windows: bool) -> bool {
    if !windows {
        return pattern.starts_with('/');
    }
    let bytes = pattern.as_bytes();
    let is_sep = |b: u8| b == b'/' || b == b'\\';
    bytes.first().is_some_and(|b| is_sep(*b))
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && is_sep(bytes[2]))
}

/// The components of an absolute path, `/`-split. On Windows `\` is a
/// separator too, a verbatim prefix (`\\?\`) is dropped, and a UNC path
/// keeps a leading marker so it never equals a rooted one.
fn absolute_components(path: &str, windows: bool) -> Vec<String> {
    let mut unified = if windows {
        path.replace('\\', "/")
    } else {
        path.to_string()
    };
    if windows {
        if let Some(rest) = unified.strip_prefix("//?/UNC/") {
            unified = format!("//{rest}");
        } else if let Some(rest) = unified.strip_prefix("//?/") {
            unified = rest.to_string();
        }
    }
    let mut components = Vec::new();
    if windows && unified.starts_with("//") {
        components.push("//".to_string());
    }
    components.extend(
        unified
            .split('/')
            .filter(|component| !component.is_empty() && *component != ".")
            .map(str::to_string),
    );
    components
}

/// The components of `pattern` below `root` when it lies strictly beneath
/// it; `None` otherwise (the root itself names nothing a walk visits).
fn relative_to_root(pattern: &str, root: &str, windows: bool) -> Option<Vec<String>> {
    let pattern = absolute_components(pattern, windows);
    let root = absolute_components(root, windows);
    if pattern.len() <= root.len() {
        return None;
    }
    let same = |a: &String, b: &String| {
        if windows {
            a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    };
    if !root.iter().zip(&pattern).all(|(r, p)| same(r, p)) {
        return None;
    }
    let rest = &pattern[root.len()..];
    // `..` would climb back out: such a form names nothing reliably.
    if rest.iter().any(|component| component == "..") {
        return None;
    }
    Some(rest.to_vec())
}

/// A relative pattern from its components, `/`-joined. A single name is
/// written `./name` so it stays anchored to the root instead of matching
/// that name at every depth.
fn written(rest: &[String]) -> String {
    if rest.len() == 1 {
        format!("./{}", rest[0])
    } else {
        rest.join("/")
    }
}

/// cr-g3fix-1: `rest` — a pattern's components below `root`, as typed —
/// with each component the filesystem holds spelled the way it stores it.
/// Every rule compares case-sensitively, so on a case-insensitive
/// filesystem `/src/CACHE` for an on-disk `src/cache` must become
/// `./cache` — locally and in the wire form a push sends — or it names
/// nothing the walk sees. The spelling stops at the first component that
/// holds a glob metacharacter or that `stored` cannot name (missing, or a
/// symlink); that one and the rest stay as typed.
fn stored_spelling(
    root: &Path,
    rest: Vec<String>,
    windows: bool,
    stored: impl Fn(&Path) -> Option<String>,
) -> Vec<String> {
    let mut at = root.to_path_buf();
    let mut spelled = Vec::with_capacity(rest.len());
    let mut components = rest.into_iter();
    for component in components.by_ref() {
        if has_glob_meta(&component, windows) {
            spelled.push(component);
            break;
        }
        at.push(&component);
        match stored(&at) {
            Some(name) => {
                at.set_file_name(&name);
                spelled.push(name);
            }
            None => {
                spelled.push(component);
                break;
            }
        }
    }
    spelled.extend(components);
    spelled
}

/// The name the filesystem stores for `path`'s last component (its own
/// case, and on Windows its long name); `None` when it does not exist or
/// is a symlink — resolving a symlink yields its target's name, and the
/// walk sees the link.
fn stored_name(path: &Path) -> Option<String> {
    if std::fs::symlink_metadata(path)
        .ok()?
        .file_type()
        .is_symlink()
    {
        return None;
    }
    std::fs::canonicalize(path)
        .ok()?
        .file_name()?
        .to_str()
        .map(str::to_string)
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
            source_root: None,
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
            source_root: None,
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

    /// audit-19: an absolute exclude under the source root is made
    /// source-relative by BOTH builders — `build_spec` before the spec
    /// leaves this host, since the peer never knows this side's root.
    #[test]
    fn an_absolute_exclude_under_the_source_root_becomes_relative_in_build_and_build_spec() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let exc = vec![
            format!("{}", root.join(".java").display()),
            format!("{}/", root.join("a").join(".java").display()),
            "*.tmp".to_string(),
        ];
        let expected = vec![
            "./.java".to_string(),
            "a/.java".to_string(),
            "*.tmp".to_string(),
        ];
        let mut i = inputs(&[], &exc);
        i.source_root = Some(root);
        assert_eq!(build(&i).unwrap().exclude_files, expected, "build");
        assert_eq!(build_spec(&i).unwrap().exclude, expected, "build_spec");
        assert!(resolve_excludes(&exc, Some(root)).unmatchable.is_empty());
    }

    /// audit-19: an absolute exclude that names nothing under the source
    /// root — outside it, the root itself, or any absolute pattern when the
    /// source is remote — is reported unmatchable (the CLI warns), and is
    /// kept as typed, where it matches nothing.
    #[test]
    fn an_absolute_exclude_outside_the_source_root_is_unmatchable() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let outside = "/nowhere/.java".to_string();
        let the_root = format!("{}", root.display());
        let exc = vec![outside.clone(), the_root.clone(), ".java".to_string()];

        let resolved = resolve_excludes(&exc, Some(root));
        assert_eq!(resolved.unmatchable, vec![outside.clone(), the_root]);
        assert_eq!(resolved.patterns, exc, "unmatchable patterns stay as typed");

        let remote = resolve_excludes(&exc, None);
        assert_eq!(
            remote.unmatchable,
            vec![outside, format!("{}", root.display())],
            "no local root: every absolute pattern is unmatchable"
        );
        let mut i = inputs(&[], &exc);
        i.source_root = Some(root);
        assert_eq!(build_spec(&i).unwrap().exclude, exc);
    }

    /// A root reached through a symlink still relativizes a pattern
    /// written through the resolved path (macOS `/tmp` is
    /// `/private/tmp`).
    #[cfg(unix)]
    #[test]
    fn an_absolute_exclude_through_a_symlinked_root_becomes_relative() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let canonical = std::fs::canonicalize(&real).unwrap();
        let exc = vec![format!("{}", canonical.join(".java").display())];
        let resolved = resolve_excludes(&exc, Some(&link));
        assert_eq!(resolved.patterns, vec!["./.java".to_string()]);
        assert!(resolved.unmatchable.is_empty());
    }

    /// cr-g3fix-2: an absolute glob written through a symlink to the
    /// source, or to a directory inside it, is made relative — the glob
    /// path itself never exists, so only its literal leading part is
    /// resolved and the rest is kept as typed. A path not there yet stays
    /// as written, and a symlink inside the source is not resolved: the walk
    /// does not follow it, so the pattern still names the link, not its
    /// target.
    #[cfg(unix)]
    #[test]
    fn an_absolute_glob_through_a_symlink_becomes_relative() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("keep.txt"), "k").unwrap();
        let alias = dir.path().join("alias");
        std::os::unix::fs::symlink(&src, &alias).unwrap();
        let into_sub = dir.path().join("into_sub");
        std::os::unix::fs::symlink(src.join("sub"), &into_sub).unwrap();
        std::os::unix::fs::symlink(src.join("sub"), src.join("link")).unwrap();
        let exc = vec![
            format!("{}", alias.join("*.txt").display()),
            format!("{}", into_sub.join("*.log").display()),
            format!("{}", alias.join("new").join("*.tmp").display()),
            format!("{}", alias.join("link").display()),
        ];
        let resolved = resolve_excludes(&exc, Some(&src));
        assert_eq!(
            resolved.patterns,
            vec!["./*.txt", "sub/*.log", "new/*.tmp", "./link"]
        );
        assert!(
            resolved.unmatchable.is_empty(),
            "{:?}",
            resolved.unmatchable
        );
    }

    /// True when `dir`'s filesystem ignores case: a file made as
    /// `case-probe` opens as `CASE-PROBE`.
    fn case_insensitive(dir: &Path) -> bool {
        let probe = dir.join("case-probe");
        std::fs::write(&probe, b"").unwrap();
        let insensitive = std::fs::File::open(dir.join("CASE-PROBE")).is_ok();
        std::fs::remove_file(&probe).unwrap();
        insensitive
    }

    /// cr-g3fix-1: on a case-insensitive filesystem an absolute exclude in
    /// another case names the entry on disk, and both builders — the local
    /// rule and the wire form — spell it the way the disk stores it (every
    /// rule compares case-sensitively). A path that does not exist keeps
    /// its typed case. Skipped where the filesystem is case-sensitive.
    #[test]
    fn an_absolute_exclude_in_another_case_takes_the_stored_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("cache").join("Sub")).unwrap();
        if !case_insensitive(root) {
            eprintln!("skipping: {} is case-sensitive", root.display());
            return;
        }
        let exc = vec![
            format!("{}", root.join("CACHE").display()),
            format!("{}", root.join("CACHE").join("SUB").join("*.tmp").display()),
            format!("{}", root.join("Missing").join("X").display()),
        ];
        let expected = vec!["./cache", "cache/Sub/*.tmp", "Missing/X"];
        let mut i = inputs(&[], &exc);
        i.source_root = Some(root);
        assert_eq!(build(&i).unwrap().exclude_files, expected, "build");
        assert_eq!(build_spec(&i).unwrap().exclude, expected, "build_spec");
        assert!(resolve_excludes(&exc, Some(root)).unmatchable.is_empty());
    }

    /// cr-g3fix-1: the spelling walk on any OS, against a stand-in
    /// case-insensitive disk holding `root/cache/Sub/x.txt`: each component
    /// takes the stored name until one holds a glob metacharacter or cannot
    /// be named, and that one and the rest stay as typed.
    #[test]
    fn the_stored_spelling_runs_until_a_component_cannot_be_named() {
        let disk = |path: &Path| {
            let key = path.to_string_lossy().replace('\\', "/").to_lowercase();
            [
                ("root/cache", "cache"),
                ("root/cache/sub", "Sub"),
                ("root/cache/sub/x.txt", "x.txt"),
            ]
            .iter()
            .find(|(stored, _)| *stored == key)
            .map(|(_, name)| name.to_string())
        };
        let spell = |rest: &[&str]| {
            let rest = rest.iter().map(|c| c.to_string()).collect();
            stored_spelling(Path::new("root"), rest, false, disk)
        };
        assert_eq!(spell(&["CACHE", "SUB", "X.TXT"]), ["cache", "Sub", "x.txt"]);
        assert_eq!(
            spell(&["CACHE", "*.TMP", "SUB"]),
            ["cache", "*.TMP", "SUB"],
            "a glob component and everything after it stay as typed"
        );
        assert_eq!(
            spell(&["CACHE", "Missing", "SUB"]),
            ["cache", "Missing", "SUB"],
            "a component the disk cannot name ends the spelling"
        );
    }

    /// cr-g3fix-1: the stored name is the entry's own — never a symlink's
    /// target's, which would name another entry than the link the walk
    /// sees — and there is none for a missing path.
    #[cfg(unix)]
    #[test]
    fn the_stored_name_is_the_entrys_own_never_a_symlink_targets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("real")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link")).unwrap();
        assert_eq!(
            stored_name(&dir.path().join("real")).as_deref(),
            Some("real")
        );
        assert_eq!(stored_name(&dir.path().join("link")), None);
        assert_eq!(stored_name(&dir.path().join("missing")), None);
    }

    /// Windows forms: either slash, any case, verbatim and UNC prefixes.
    /// Pure string logic, so it runs (and can fail) on every OS.
    #[test]
    fn windows_absolute_excludes_compare_case_insensitively_with_either_slash() {
        let relative = |pattern: &str, root: &str, windows: bool| {
            relative_to_root(pattern, root, windows).map(|rest| written(&rest))
        };
        assert!(is_absolute_pattern(r"C:\Users\me\.java", true));
        assert!(is_absolute_pattern("c:/Users/me", true));
        assert!(is_absolute_pattern(r"\\server\share\x", true));
        assert!(!is_absolute_pattern(r"C:\Users\me", false));
        assert!(!is_absolute_pattern(".java", true));
        assert_eq!(
            relative(r"C:\Users\Me\.java", r"c:\users\me", true).as_deref(),
            Some("./.java")
        );
        assert_eq!(
            relative(r"C:\Users\me\a\.java", r"\\?\C:\Users\me", true).as_deref(),
            Some("a/.java")
        );
        assert_eq!(
            relative(r"\\srv\share\d\x", r"\\?\UNC\srv\share\d", true).as_deref(),
            Some("./x")
        );
        assert_eq!(relative(r"D:\Users\me\.java", r"C:\Users\me", true), None);
        assert_eq!(relative(r"\srv\share\d\x", r"\\srv\share\d", true), None);
        assert_eq!(
            relative("/Users/Me/.java", "/Users/me", false),
            None,
            "case matters off Windows"
        );
    }

    /// cr-g3fix-3: relative patterns reach both builders — and so the wire
    /// form — with `\` written `/` under the Windows rule, and untouched
    /// under the Unix rule, where `\` is no separator. An absolute pattern
    /// that cannot match stays as typed either way. Pure string logic, so
    /// both rules run on every OS.
    #[test]
    fn relative_excludes_take_forward_slashes_under_the_windows_rule() {
        let exc: Vec<String> = [r".\cache", r"cache\", r"a\.java", "*.tmp"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            resolve_excludes_on(&exc, None, true).patterns,
            ["./cache", "cache/", "a/.java", "*.tmp"]
        );
        assert_eq!(resolve_excludes_on(&exc, None, false).patterns, exc);

        let mut mixed = exc.clone();
        mixed.push(r"C:\elsewhere\x".to_string());
        let windows = resolve_excludes_on(&mixed, None, true);
        assert_eq!(
            windows.patterns,
            ["./cache", "cache/", "a/.java", "*.tmp", r"C:\elsewhere\x"]
        );
        assert_eq!(windows.unmatchable, [r"C:\elsewhere\x"]);
        let unix = resolve_excludes_on(&mixed, None, false);
        assert_eq!(unix.patterns, mixed, "no absolute pattern off Windows");
        assert!(unix.unmatchable.is_empty());
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
