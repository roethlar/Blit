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

use crate::fs_enum::{
    normalize_separators, parse_duration, parse_size, typed_exclude_dialect, FileFilter,
};
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
    /// typed — on Windows with `\` written `/` outside a bracket class
    /// (cr-g3fix-3, cr-g3fix3-3), and off Windows with each escape `\c`
    /// written in the class dialect (`./a\b` as `./ab`, `x\*y` as `x[*]y`;
    /// cr-g3fix4-3), so the wire form holds no bare `\` and reads the same
    /// on any peer ([`typed_exclude_dialect`]). In a rewritten pattern a
    /// name read from the disk is escaped, so it matches only itself
    /// (cr-g3fix2-2). An unmatchable pattern stays exactly as typed.
    pub patterns: Vec<String>,
    /// The absolute patterns that cannot match anything: outside the
    /// source root, naming the root itself, or any absolute pattern when
    /// the source is not local. They stay in `patterns`, where they match
    /// nothing; the caller warns about each one.
    pub unmatchable: Vec<String>,
    /// cr-g3fix4-1: the absolute patterns that name an entry stored here
    /// under several names (one entry hard-linked under each, none spelled
    /// as typed), each with the patterns it became — one per name, all in
    /// `patterns`: the filesystem matched one of them and nothing names
    /// which, so all are excluded. The caller notes each one.
    pub expanded: Vec<(String, Vec<String>)>,
}

/// cr-g3fix4-1: the one-line note for an absolute `--exclude` that names
/// an entry stored under several names ([`ResolvedExcludes::expanded`]):
/// every name is excluded, and the operator learns which.
pub fn shared_entry_note(pattern: &str, patterns: &[String]) -> String {
    format!(
        "blit: --exclude '{pattern}': {} names share this entry here; excluding {}",
        patterns.len(),
        patterns.join(", ")
    )
}

/// audit-19: make each absolute `--exclude` under `source_root` relative
/// to it, and name the absolute ones that cannot match. Paths compare the
/// way the platform compares them (case-insensitively on Windows); a root
/// or pattern reached through a symlink is also compared in its resolved
/// form, so `/tmp/x/.java` and `/private/tmp/x/.java` name one path on
/// macOS (`through_resolved_root` says how a pattern is resolved).
/// Relative patterns pass through untouched but for their separators and
/// escapes: on Windows `\` is written `/` outside a bracket class
/// (cr-g3fix-3, cr-g3fix3-3); off Windows an escape `\c` outside a class
/// is written in the class dialect (cr-g3fix4-3). `--include` patterns
/// are not rewritten.
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
            .map(|pattern| typed_exclude_dialect(pattern, windows).into_owned())
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
                .push(typed_exclude_dialect(pattern, windows).into_owned());
            continue;
        }
        // Each form of the pattern with the part of it read from the disk:
        // as typed, none; through the resolved root, its resolved prefix.
        let mut pattern_forms = vec![(pattern.clone(), None)];
        if let Some((prefix, rest)) = resolved_root
            .as_deref()
            .and_then(|root| through_resolved_root(pattern, root, windows))
        {
            let form = prefix.join(rest).to_string_lossy().into_owned();
            pattern_forms.push((form, Some(prefix.to_string_lossy().into_owned())));
        }
        let relative = pattern_forms.iter().find_map(|(form, prefix)| {
            roots
                .iter()
                .find_map(|root| relative_parts(form, prefix.as_deref(), root, windows))
        });
        match (relative, source_root) {
            (Some(rest), Some(root)) => {
                // cr-g3fix4-3: the typed parts in the dialect every peer
                // reads alike (a literal part is classes already).
                // cr-g3fix4-1: one pattern per name a tie forks into.
                let spelled: Vec<String> = stored_spelling(root, rest, windows, stored_name)
                    .iter()
                    .map(|parts| {
                        typed_exclude_dialect(&written(parts, windows), windows).into_owned()
                    })
                    .collect();
                if spelled.len() > 1 {
                    resolved.expanded.push((pattern.clone(), spelled.clone()));
                }
                resolved.patterns.extend(spelled);
            }
            _ => {
                resolved.patterns.push(pattern.clone());
                resolved.unmatchable.push(pattern.clone());
            }
        }
    }
    resolved
}

/// cr-g3fix-2: `pattern` through the resolved source root when its
/// leading literal part reaches the source through a symlink
/// (`/alias/*.txt` with `alias` naming the source, or a directory inside
/// it; macOS `/tmp` for `/private/tmp`). The pattern's leading ancestors
/// up to its first component holding a glob metacharacter are resolved,
/// shortest first; the first that exists and lands at or below
/// `resolved_root` is returned resolved, with what follows it as typed —
/// a glob unchanged, a path that does not exist yet as it was written,
/// and a symlink inside the source unresolved (the walk does not follow
/// one, so resolving it would name its target instead). The resolved
/// part is the disk's own names, never glob text (cr-g3fix2-2), so it is
/// kept apart from the typed rest. `None` when no ancestor lands there.
fn through_resolved_root<'a>(
    pattern: &'a str,
    resolved_root: &Path,
    windows: bool,
) -> Option<(PathBuf, &'a Path)> {
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
        return Some((resolved, rest));
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
/// separator too, outside a bracket class ([`normalize_separators`]:
/// cr-g3fix4-4 — `C:\src\back[s\]lash` names `back[s\]lash`, a class
/// holding `s` and `\`, not `back[s` and `]lash`), a verbatim prefix
/// (`\\?\`) is dropped, and a UNC path keeps a leading marker so it never
/// equals a rooted one (a prefix holds no class, so it reads as before).
fn absolute_components(path: &str, windows: bool) -> Vec<String> {
    let mut unified = normalize_separators(path, windows).into_owned();
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

/// cr-g3fix2-2: one component of a pattern made source-relative.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    /// A name read from the disk — a component of a resolved prefix, or
    /// the stored spelling of a typed one: literal text, escaped when
    /// written so it matches only itself.
    Literal(String),
    /// As the user typed it: glob syntax, written unchanged.
    Typed(String),
}

/// `form` made relative to `root` as [`Part`]s: the components that lie
/// within `prefix` — the part of the form read from the disk — are
/// literal, the rest as typed. `None` as for [`relative_to_root`].
fn relative_parts(
    form: &str,
    prefix: Option<&str>,
    root: &str,
    windows: bool,
) -> Option<Vec<Part>> {
    let rest = relative_to_root(form, root, windows)?;
    let literal = prefix
        .and_then(|prefix| relative_to_root(prefix, root, windows))
        .map_or(0, |below| below.len());
    Some(
        rest.into_iter()
            .enumerate()
            .map(|(at, component)| {
                if at < literal {
                    Part::Literal(component)
                } else {
                    Part::Typed(component)
                }
            })
            .collect(),
    )
}

/// A relative pattern from its parts, `/`-joined: a literal part escaped
/// ([`literal_glob`]), a typed one unchanged. A single name is written
/// `./name` so it stays anchored to the root instead of matching that
/// name at every depth.
fn written(rest: &[Part], windows: bool) -> String {
    let text: Vec<std::borrow::Cow<'_, str>> = rest
        .iter()
        .map(|part| match part {
            Part::Literal(name) => literal_glob(name, windows).into(),
            Part::Typed(component) => component.as_str().into(),
        })
        .collect();
    if text.len() == 1 {
        format!("./{}", text[0])
    } else {
        text.join("/")
    }
}

/// cr-g3fix2-2: `name` as glob text that matches exactly that name: every
/// character `globset` reads as syntax (`?*[]{}`) bracketed, and, off
/// Windows, `\` too — globset reads it as an escape there (on Windows it
/// is a separator and never reaches a name).
fn literal_glob(name: &str, windows: bool) -> String {
    let escaped = globset::escape(name);
    if windows {
        escaped
    } else {
        escaped.replace('\\', r"[\]")
    }
}

/// cr-g3fix-1: `rest` — a pattern's parts below `root` — with each typed
/// component the filesystem holds spelled the way it stores it. Every
/// rule compares case-sensitively, so on a case-insensitive filesystem
/// `/src/CACHE` for an on-disk `src/cache` must become `./cache` — locally
/// and in the wire form a push sends — or it names nothing the walk sees.
/// A stored name is read from the disk, so it becomes a literal part
/// (cr-g3fix2-2). The spelling stops at the first typed component that
/// holds a glob metacharacter or that `stored` cannot name (see
/// [`stored_name`]); that one and the rest stay as typed. A literal part
/// is the disk's name already and never stops it, whatever characters it
/// holds. cr-g3fix4-1: a component `stored` names under several names (one
/// entry hard-linked under each, none spelled as typed) forks the
/// spelling, one result per name, each continued below it; otherwise
/// there is exactly one result.
fn stored_spelling(
    root: &Path,
    rest: Vec<Part>,
    windows: bool,
    stored: impl Fn(&Path) -> Vec<String>,
) -> Vec<Vec<Part>> {
    spelled_below(root, &rest, windows, &stored)
}

/// [`stored_spelling`] of `rest` below `at`.
fn spelled_below(
    at: &Path,
    rest: &[Part],
    windows: bool,
    stored: &impl Fn(&Path) -> Vec<String>,
) -> Vec<Vec<Part>> {
    let Some((part, below)) = rest.split_first() else {
        return vec![Vec::new()];
    };
    let names = match part {
        Part::Literal(name) => vec![name.clone()],
        Part::Typed(component) if has_glob_meta(component, windows) => {
            return vec![rest.to_vec()];
        }
        Part::Typed(component) => {
            let names = stored(&at.join(component));
            if names.is_empty() {
                return vec![rest.to_vec()];
            }
            names
        }
    };
    let mut spelled = Vec::new();
    for name in names {
        for tail in spelled_below(&at.join(&name), below, windows, stored) {
            let mut parts = Vec::with_capacity(rest.len());
            parts.push(Part::Literal(name.clone()));
            parts.extend(tail);
            spelled.push(parts);
        }
    }
    spelled
}

/// The names the filesystem stores for `path`'s last component (its own
/// case and Unicode normalization, and on Windows its long name), read
/// without following it: a symlink is named as the link the walk sees,
/// never as its target (cr-g3fix2-1 — resolving the link would name
/// another entry, and leaving it unnamed kept a differently spelled
/// exclude as typed, matching nothing, so a mirror could delete the
/// destination's entry of that name). The names come from the parent
/// directory's listing ([`listed_name`]): one, or every name of the entry
/// when it is hard-linked under several and none is spelled as typed
/// (cr-g3fix4-1). A non-symlink the listing cannot pair (an unreadable
/// listing or identity) falls back to the last component of
/// `canonicalize`. Empty when the path does not exist, or is a symlink the
/// listing cannot pair. `pub(crate)`: the mirror's deletion scope spells
/// its patterns against the destination with it too.
pub(crate) fn stored_name(path: &Path) -> Vec<String> {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Vec::new();
    };
    let listed = listed_name(path, &metadata);
    if !listed.is_empty() || metadata.file_type().is_symlink() {
        return listed;
    }
    std::fs::canonicalize(path)
        .ok()
        .and_then(|resolved| resolved.file_name()?.to_str().map(str::to_string))
        .into_iter()
        .collect()
}

/// cr-g3fix2-1, cr-g3fix3-1, cr-g3fix4-1: the names `path`'s parent
/// directory lists for the entry `path` denotes — `metadata` is its own
/// `symlink_metadata` — or none when the listing or the identity cannot be
/// read, or nothing pairs. The filesystem's rule made the typed path
/// resolve, so the filesystem decides the pairing, through the entry's
/// identity ([`entry_identity`]) on every platform; no text rule
/// reproduces every filesystem's table (APFS pairs final sigma `ς` with
/// `σ` and NFC with NFD, NTFS upcases both sigmas alike). Runs once per
/// typed component of an absolute exclude, when the filter is built.
fn listed_name(path: &Path, metadata: &std::fs::Metadata) -> Vec<String> {
    let Some(typed) = path.file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let Ok(listing) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let names: Vec<String> = listing
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    if names.iter().any(|name| name == typed) {
        return vec![typed.to_string()];
    }
    let Some((identity, links)) = entry_identity(path) else {
        return Vec::new();
    };
    // A directory has one name; anything else may be hard-linked under
    // several in this listing.
    let shared = !metadata.is_dir() && links > 1;
    paired_name(typed, &names, shared, |name| {
        entry_identity(&parent.join(name)).is_some_and(|(entry, _)| entry == identity)
    })
}

/// cr-g3fix4-1: the identity of the entry `path` names — the entry itself,
/// never a symlink's target — and how many names it has. On Unix
/// `(dev, ino)` and `nlink` from `symlink_metadata`.
#[cfg(unix)]
fn entry_identity(path: &Path) -> Option<((u64, u64), u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path).ok()?;
    Some(((metadata.dev(), metadata.ino()), metadata.nlink()))
}

/// cr-g3fix4-1: the identity of the entry `path` names — the entry itself,
/// never a reparse point's target — and how many names it has. On Windows
/// the volume serial number and the file index, and `nNumberOfLinks`, from
/// `GetFileInformationByHandle`, on a handle `CreateFileW` opens (through
/// std's `OpenOptions`) with `FILE_FLAG_OPEN_REPARSE_POINT` (the entry, not
/// its target) and `FILE_FLAG_BACKUP_SEMANTICS` (a directory opens too),
/// asking no access rights — std's own `symlink_metadata` opens it the same
/// way. std offers no stable identity there (`MetadataExt::file_index` is
/// unstable).
#[cfg(windows)]
fn entry_identity(path: &Path) -> Option<((u64, u64), u64)> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT,
    };
    let file = std::fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0 | FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(path)
        .ok()?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `file` keeps the handle open for the call, and `info` is a
    // valid, writable BY_HANDLE_FILE_INFORMATION.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info) }.ok()?;
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Some((
        (u64::from(info.dwVolumeSerialNumber), index),
        u64::from(info.nNumberOfLinks),
    ))
}

/// No file identity on this platform: nothing pairs, and a non-symlink
/// takes its stored name from `canonicalize`.
#[cfg(not(any(unix, windows)))]
fn entry_identity(_path: &Path) -> Option<((u64, u64), u64)> {
    None
}

/// cr-g3fix3-1, cr-g3fix4-1: the names in `names` (a directory's listing)
/// for the entry a typed name denotes: the one spelled exactly as typed,
/// with no further lookup; else the first `same` says is that entry (the
/// same identity — the link itself, never its target), the walk stopping
/// there. When the identity may have several names (`shared`: a
/// hard-linked entry), the whole listing is walked and every match is
/// returned, sorted: the filesystem matched one of them and no API names
/// which, so a tie pairs them all (excluding each is over-exclusion, the
/// safe direction). No text folding: none pairs nothing.
fn paired_name(
    typed: &str,
    names: &[String],
    shared: bool,
    mut same: impl FnMut(&str) -> bool,
) -> Vec<String> {
    if let Some(exact) = names.iter().find(|name| *name == typed) {
        return vec![exact.clone()];
    }
    let mut matches = Vec::new();
    for name in names {
        if same(name) {
            if !shared {
                return vec![name.clone()];
            }
            matches.push(name.clone());
        }
    }
    matches.sort();
    matches
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

    /// Makes each `/`-separated `rel` under `root` as a file (its
    /// directories too).
    #[cfg(unix)]
    fn files(root: &Path, rels: &[&str]) {
        for rel in rels {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "x").unwrap();
        }
    }

    /// cr-g3fix2-2: the resolved part of a pattern is the disk's own names,
    /// so both builders write it escaped and it matches only itself. Through
    /// an alias into `data[1]`, `*.txt` leaves out `data[1]/private.txt`,
    /// not the unrelated `data1/private.txt` (as glob text `[1]` is a class
    /// naming `data1`), and the typed `*.txt` stays a glob; through an alias
    /// into `set[2]` the anchored rule `./set[[]2[]]` leaves out that
    /// subtree only. cr-g3fix3-2: nor does an escape match an entry spelled
    /// like it — `data[[]1[]]`, `set[[]2[]]` are other names, which the
    /// matcher's text comparison used to leave out too.
    #[cfg(unix)]
    #[test]
    fn a_name_read_from_the_disk_is_matched_literally() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        files(
            &src,
            &[
                "data[1]/private.txt",
                "data[1]/keep.dat",
                "data[1]/notes/n",
                "data1/private.txt",
                "data[[]1[]]/private.txt",
                "data[[]1[]]/notes/n",
                "set[2]/x",
                "set2/x",
                "set[[]2[]]/x",
            ],
        );
        let into_class = dir.path().join("into_class");
        std::os::unix::fs::symlink(src.join("data[1]"), &into_class).unwrap();
        let into_set = dir.path().join("into_set");
        std::os::unix::fs::symlink(src.join("set[2]"), &into_set).unwrap();
        let exc = vec![
            format!("{}", into_class.join("*.txt").display()),
            format!("{}", into_set.display()),
            format!("{}", into_class.join("notes").display()),
        ];
        let mut i = inputs(&[], &exc);
        i.source_root = Some(&src);

        let filter = build(&i).unwrap();
        let allows = |rel: &str| filter.allows_entry(Some(Path::new(rel)), &src.join(rel), 1, None);
        assert!(!allows("data[1]/private.txt"), "the file the exclude names");
        assert!(allows("data[1]/keep.dat"), "the typed glob is still a glob");
        assert!(
            allows("data1/private.txt"),
            "a name the exclude never named"
        );
        assert!(
            !allows("set[2]") && !allows("set[2]/x"),
            "the named subtree"
        );
        assert!(allows("set2/x"), "a name the exclude never named");
        assert!(
            !allows("data[1]/notes") && !allows("data[1]/notes/n"),
            "the named subtree"
        );
        for encoded in [
            "data[[]1[]]/private.txt",
            "set[[]2[]]",
            "set[[]2[]]/x",
            "data[[]1[]]/notes",
            "data[[]1[]]/notes/n",
        ] {
            assert!(
                allows(encoded),
                "{encoded}: spelled like the escape, never named"
            );
        }

        let expected = vec!["data[[]1[]]/*.txt", "./set[[]2[]]", "data[[]1[]]/notes"];
        assert_eq!(filter.exclude_files, expected, "build");
        assert_eq!(build_spec(&i).unwrap().exclude, expected, "build_spec");
        assert!(resolve_excludes(&exc, Some(&src)).unmatchable.is_empty());
    }

    /// cr-g3fix2-2: an alias into a directory named with an unmatched `[`
    /// neither fails nor warns — as glob text `open[x` does not compile, so
    /// a pattern the user wrote correctly failed the run — and it leaves
    /// out that directory.
    #[cfg(unix)]
    #[test]
    fn a_resolved_name_with_an_unmatched_bracket_neither_fails_nor_warns() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        files(&src, &["open[x/private.txt", "open/private.txt"]);
        let into_open = dir.path().join("into_open");
        std::os::unix::fs::symlink(src.join("open[x"), &into_open).unwrap();
        let exc = vec![format!("{}", into_open.display())];
        let mut i = inputs(&[], &exc);
        i.source_root = Some(&src);

        let filter = build(&i).map_err(|e| format!("{e:#}"));
        let spec = build_spec(&i).map_err(|e| format!("{e:#}"));
        assert_eq!(
            filter.as_ref().map(|f| f.exclude_files.clone()),
            Ok(vec!["./open[[]x".to_string()]),
            "build"
        );
        assert_eq!(
            spec.map(|s| s.exclude),
            Ok(vec!["./open[[]x".to_string()]),
            "build_spec"
        );
        assert!(resolve_excludes(&exc, Some(&src)).unmatchable.is_empty());
        let filter = filter.unwrap();
        let allows = |rel: &str| filter.allows_entry(Some(Path::new(rel)), &src.join(rel), 1, None);
        assert!(!allows("open[x") && !allows("open[x/private.txt"));
        assert!(allows("open/private.txt"));
    }

    /// cr-g3fix2-2: off Windows `\` is an ordinary character in a name and
    /// globset's escape in a pattern, so a resolved name holding one must
    /// be escaped to match itself — written as typed, `back\slash` reads
    /// as `backslash`.
    #[cfg(unix)]
    #[test]
    fn a_resolved_name_holding_a_backslash_matches_itself() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        files(&src, &[r"back\slash/private.txt", "backslash/private.txt"]);
        let alias = dir.path().join("alias");
        std::os::unix::fs::symlink(src.join(r"back\slash"), &alias).unwrap();
        let exc = vec![format!("{}", alias.join("*.txt").display())];
        let mut i = inputs(&[], &exc);
        i.source_root = Some(&src);
        let expected = vec![r"back[\]slash/*.txt"];
        assert_eq!(build(&i).unwrap().exclude_files, expected, "build");
        assert_eq!(build_spec(&i).unwrap().exclude, expected, "build_spec");

        let filter = build(&i).unwrap();
        let allows = |rel: &str| filter.allows_entry(Some(Path::new(rel)), &src.join(rel), 1, None);
        assert!(
            !allows(r"back\slash/private.txt"),
            "the file the exclude names"
        );
        assert!(
            allows("backslash/private.txt"),
            "a name the exclude never named"
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

    /// cr-g3fix3-1: spellings only a filesystem's own rule pairs — not
    /// per-character lowercase: final sigma `ς` for a stored `σ`, and NFC
    /// `CAFÉ` for a stored NFD `café` (both on APFS case-insensitive).
    #[cfg(unix)]
    const FILESYSTEM_PAIRS: [(&str, &str); 2] = [("σ", "ς"), ("cafe\u{301}", "CAF\u{c9}")];

    /// True when `dir`'s filesystem resolves `typed` to an entry made as
    /// `stored` (probed with a file of that name, removed again).
    fn pairs(dir: &Path, stored: &str, typed: &str) -> bool {
        let probe = dir.join(stored);
        std::fs::write(&probe, b"").unwrap();
        let paired = std::fs::symlink_metadata(dir.join(typed)).is_ok();
        std::fs::remove_file(&probe).unwrap();
        paired
    }

    /// The one spelling of a walk that forked nowhere.
    fn only(mut spelled: Vec<Vec<Part>>) -> Vec<Part> {
        assert_eq!(spelled.len(), 1, "one spelling expected: {spelled:?}");
        spelled.remove(0)
    }

    /// cr-g3fix4-1: hard-links `dst` to the entry `src` itself — a symlink
    /// stays a symlink (`linkat` without `AT_SYMLINK_FOLLOW`; macOS `link`
    /// and `ln` follow it). False when the filesystem refuses, or followed
    /// the link anyway.
    #[cfg(unix)]
    fn hard_link_entry(src: &Path, dst: &Path) -> bool {
        use std::os::unix::ffi::OsStrExt;
        let c = |path: &Path| std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        let (from, to) = (c(src), c(dst));
        // SAFETY: both arguments are valid NUL-terminated paths for the call.
        let linked = unsafe {
            libc::linkat(
                libc::AT_FDCWD,
                from.as_ptr(),
                libc::AT_FDCWD,
                to.as_ptr(),
                0,
            )
        };
        linked == 0
            && std::fs::symlink_metadata(dst).is_ok_and(|entry| {
                entry.file_type()
                    == std::fs::symlink_metadata(src)
                        .expect("the source entry")
                        .file_type()
            })
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
        let mut exc = vec![
            format!("{}", root.join("CACHE").display()),
            format!("{}", root.join("CACHE").join("SUB").join("*.tmp").display()),
            format!("{}", root.join("Missing").join("X").display()),
        ];
        let mut expected: Vec<String> = ["./cache", "cache/Sub/*.tmp", "Missing/X"]
            .map(String::from)
            .to_vec();
        // cr-g3fix2-1: a symlink to a directory outside the source is
        // spelled as stored too — named as the link, never through its
        // target. Unix only: making a symlink on Windows needs privilege.
        #[cfg(unix)]
        let outside = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), root.join("link")).unwrap();
        if cfg!(unix) {
            exc.push(format!("{}", root.join("LINK").display()));
            expected.push("./link".to_string());
        }
        // cr-g3fix3-1: so is a symlink named in a spelling only the
        // filesystem pairs (`ς` for `σ`, NFC `CAFÉ` for NFD `café`), each
        // where the filesystem pairs it.
        #[cfg(unix)]
        for (stored, typed) in FILESYSTEM_PAIRS {
            if !pairs(root, stored, typed) {
                eprintln!(
                    "skipping {typed:?}: {} does not pair it with {stored:?}",
                    root.display()
                );
                continue;
            }
            std::os::unix::fs::symlink(outside.path(), root.join(stored)).unwrap();
            exc.push(format!("{}", root.join(typed).display()));
            expected.push(format!("./{stored}"));
        }
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
            .filter(|(stored, _)| *stored == key)
            .map(|(_, name)| name.to_string())
            .collect::<Vec<_>>()
        };
        let spell = |rest: &[&str]| {
            let rest = rest.iter().map(|c| Part::Typed(c.to_string())).collect();
            written(
                &only(stored_spelling(Path::new("root"), rest, false, disk)),
                false,
            )
        };
        assert_eq!(spell(&["CACHE", "SUB", "X.TXT"]), "cache/Sub/x.txt");
        assert_eq!(
            spell(&["CACHE", "*.TMP", "SUB"]),
            "cache/*.TMP/SUB",
            "a glob component and everything after it stay as typed"
        );
        assert_eq!(
            spell(&["CACHE", "Missing", "SUB"]),
            "cache/Missing/SUB",
            "a component the disk cannot name ends the spelling"
        );
    }

    /// cr-g3fix2-2: the spelling walk on any OS, against a stand-in disk
    /// whose names hold glob syntax. A stored name is read from the disk,
    /// so it is written escaped — `DATA~1`, a Windows short name whose
    /// long name is `data[1]`, must not become the class `data[1]` — and a
    /// literal part, which holds the disk's name already, never ends the
    /// walk the way a typed glob component does. Off Windows a literal `\`
    /// is escaped as well; under the Windows rule it is a separator and
    /// never reaches a name.
    #[test]
    fn a_name_read_from_the_disk_is_escaped_and_never_ends_the_spelling() {
        let disk = |path: &Path| {
            let key = path.to_string_lossy().replace('\\', "/").to_lowercase();
            [("root/data~1", "data[1]"), ("root/data[1]/sub", "Sub")]
                .iter()
                .filter(|(stored, _)| *stored == key)
                .map(|(_, name)| name.to_string())
                .collect::<Vec<_>>()
        };
        let spell = |rest: Vec<Part>, windows: bool| {
            written(
                &only(stored_spelling(Path::new("root"), rest, windows, disk)),
                windows,
            )
        };
        let typed = |c: &str| Part::Typed(c.to_string());
        let literal = |c: &str| Part::Literal(c.to_string());
        for windows in [false, true] {
            assert_eq!(
                spell(vec![typed("DATA~1"), typed("SUB"), typed("*.txt")], windows),
                "data[[]1[]]/Sub/*.txt",
                "a stored name is escaped (windows rule: {windows})"
            );
            assert_eq!(
                spell(
                    vec![literal("data[1]"), typed("SUB"), typed("*.txt")],
                    windows
                ),
                "data[[]1[]]/Sub/*.txt",
                "a literal part holding `[` does not end the walk (windows rule: {windows})"
            );
        }
        assert_eq!(
            written(&[literal(r"back\slash"), typed("*.txt")], false),
            r"back[\]slash/*.txt"
        );
        assert_eq!(
            written(&[literal("a{b}*?"), typed("[!x]")], false),
            "a[{]b[}][*][?]/[!x]",
            "only the literal part is escaped"
        );
    }

    /// cr-g3fix-1, cr-g3fix2-1: the stored name is the entry's own, read
    /// without following it. A symlink reports its own name — never its
    /// target's, which would name another entry than the link the walk
    /// sees, and never none, which kept a differently cased exclude as
    /// typed — a dangling one included; a missing path has none. On a
    /// case-insensitive filesystem a name typed in another case reports
    /// the stored spelling, for the link as for the directory (skipped
    /// with a message where the filesystem is case-sensitive), and so
    /// does a link typed in a spelling only the filesystem pairs
    /// (cr-g3fix3-1; each skipped with a message where it is not paired).
    #[cfg(unix)]
    #[test]
    fn the_stored_name_is_the_entrys_own_even_for_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("real")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("gone"), dir.path().join("dangling")).unwrap();
        let name = |rel: &str| stored_name(&dir.path().join(rel));
        assert_eq!(name("real"), ["real"]);
        assert_eq!(name("link"), ["link"], "the link's own name");
        assert_eq!(name("dangling"), ["dangling"]);
        assert!(name("missing").is_empty());
        // cr-g3fix3-1: a spelling only the filesystem pairs names the link
        // as stored — the pairing is the entry's identity, not a text rule.
        for (stored, typed) in FILESYSTEM_PAIRS {
            if !pairs(dir.path(), stored, typed) {
                eprintln!(
                    "skipping {typed:?}: {} does not pair it with {stored:?}",
                    dir.path().display()
                );
                continue;
            }
            std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join(stored)).unwrap();
            assert_eq!(
                name(typed),
                [stored],
                "the link's stored spelling for {typed:?}"
            );
        }
        if !case_insensitive(dir.path()) {
            eprintln!(
                "skipping the case checks: {} is case-sensitive",
                dir.path().display()
            );
            return;
        }
        assert_eq!(name("REAL"), ["real"]);
        assert_eq!(name("LINK"), ["link"], "the link's stored spelling");
    }

    /// cr-g3fix4-1: a symlink hard-linked under a second name (`σ` and
    /// `alias`, one entry) and typed in a spelling only the filesystem
    /// pairs (`ς`) names both: the filesystem matched one of them and no
    /// API names which (macOS reports the vnode's cached name for either),
    /// so both are excluded — locally and in the wire form, with a note
    /// naming them. Spelled exactly, each still names itself alone. Before,
    /// the pairing fell back to case folding, paired neither, and the
    /// exclude stayed `./ς`, matching nothing, so a mirror deleted the
    /// destination's `σ/`. The same tie for a hard-linked file typed in
    /// another case, where the filesystem ignores case; a second name in
    /// another directory is no tie. Unix (a symlink); skipped with a
    /// message where the link cannot be made or the spellings are not
    /// paired.
    #[cfg(unix)]
    #[test]
    fn a_hard_linked_entry_typed_in_a_paired_spelling_names_every_name() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("src");
        std::fs::create_dir(&root).unwrap();
        let outside = tempfile::tempdir().unwrap();
        if !pairs(&root, "σ", "ς") {
            eprintln!("skipping: {} does not pair ς with σ", root.display());
            return;
        }
        std::os::unix::fs::symlink(outside.path(), root.join("σ")).unwrap();
        if !hard_link_entry(&root.join("σ"), &root.join("alias")) {
            eprintln!("skipping: {} cannot hard-link a symlink", root.display());
            return;
        }
        let name = |rel: &str| stored_name(&root.join(rel));
        assert_eq!(name("ς"), ["alias", "σ"], "a tie names every name");
        assert_eq!(name("σ"), ["σ"], "the exact spelling names itself");
        assert_eq!(name("alias"), ["alias"], "the exact spelling names itself");

        let exc = vec![format!("{}", root.join("ς").display())];
        let mut i = inputs(&[], &exc);
        i.source_root = Some(&root);
        assert_eq!(
            build(&i).unwrap().exclude_files,
            ["./alias", "./σ"],
            "build"
        );
        assert_eq!(
            build_spec(&i).unwrap().exclude,
            ["./alias", "./σ"],
            "build_spec"
        );
        let resolved = resolve_excludes(&exc, Some(&root));
        assert!(resolved.unmatchable.is_empty());
        assert_eq!(
            resolved.expanded,
            [(
                exc[0].clone(),
                vec!["./alias".to_string(), "./σ".to_string()]
            )]
        );
        let exact = vec![format!("{}", root.join("σ").display())];
        let resolved = resolve_excludes(&exact, Some(&root));
        assert_eq!(resolved.patterns, ["./σ"]);
        assert!(resolved.expanded.is_empty(), "no tie when spelled exactly");

        if !case_insensitive(&root) {
            eprintln!(
                "skipping the case checks: {} is case-sensitive",
                root.display()
            );
            return;
        }
        std::fs::write(root.join("hard"), b"h").unwrap();
        std::fs::hard_link(root.join("hard"), root.join("other")).unwrap();
        assert_eq!(name("HARD"), ["hard", "other"], "a hard-linked file's tie");
        let elsewhere = dir.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(root.join("single"), b"s").unwrap();
        std::fs::hard_link(root.join("single"), elsewhere.join("single")).unwrap();
        assert_eq!(
            name("SINGLE"),
            ["single"],
            "one name of the entry in this listing"
        );
    }

    /// cr-g3fix3-1, cr-g3fix4-1: a listing pairs the typed name by the
    /// entry's identity on every platform, tested here against a stand-in
    /// listing on every OS. The name spelled exactly as typed wins with no
    /// lookup; else the first entry that is the typed path's own (the walk
    /// stops there); an identity several names share (a hard-linked entry)
    /// pairs every one of them, sorted — no text folding picks one; and no
    /// such entry pairs nothing.
    #[test]
    fn the_listing_pairs_the_exact_name_else_the_same_entry() {
        let list = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let mut looked = Vec::new();
        let exact = paired_name("cache", &list(&["Cache", "cache"]), true, |name| {
            looked.push(name.to_string());
            true
        });
        assert_eq!(exact, ["cache"]);
        assert!(
            looked.is_empty(),
            "the exact name needs no lookup: {looked:?}"
        );

        let mut looked = Vec::new();
        let same = paired_name("ς", &list(&["a", "σ", "b"]), false, |name| {
            looked.push(name.to_string());
            name == "σ"
        });
        assert_eq!(same, ["σ"], "the filesystem's pairing");
        assert_eq!(looked, ["a", "σ"], "the walk stops at the first match");
        let nfd = "cafe\u{301}";
        assert_eq!(
            paired_name("CAF\u{c9}", &list(&["x", nfd]), false, |name| name == nfd),
            [nfd]
        );

        let all = |_: &str| true;
        assert_eq!(
            paired_name("ς", &list(&["σ", "alias"]), true, all),
            ["alias", "σ"],
            "several names of one identity: every one, sorted"
        );
        assert_eq!(
            paired_name("HARD", &list(&["other", "hard"]), true, all),
            ["hard", "other"],
            "no case fold picks one"
        );
        let mut looked = Vec::new();
        assert_eq!(
            paired_name("ς", &list(&["x", "σ", "y"]), true, |name| {
                looked.push(name.to_string());
                name == "σ"
            }),
            ["σ"],
            "a shared identity with one name in this listing"
        );
        assert_eq!(looked, ["x", "σ", "y"], "a shared identity walks it all");
        assert!(paired_name("ς", &list(&["σ", "x"]), false, |_| false).is_empty());
    }

    /// cr-g3fix4-1: a walk that meets a tie forks — one spelling per name,
    /// each continued below it — and both builders write one pattern per
    /// spelling. Against a stand-in disk on every OS, under both rules.
    #[test]
    fn a_tie_forks_the_spelling_into_one_pattern_per_name() {
        let disk = |path: &Path| -> Vec<String> {
            match path.to_string_lossy().replace('\\', "/").as_str() {
                "root/ς" => vec!["alias".into(), "σ".into()],
                "root/alias/X" | "root/σ/X" => vec!["x".into()],
                "root/cache" => vec!["cache".into()],
                _ => Vec::new(),
            }
        };
        let spell = |rest: &[&str], windows: bool| -> Vec<String> {
            let rest = rest.iter().map(|c| Part::Typed(c.to_string())).collect();
            stored_spelling(Path::new("root"), rest, windows, disk)
                .iter()
                .map(|parts| written(parts, windows))
                .collect()
        };
        for windows in [false, true] {
            assert_eq!(spell(&["ς"], windows), ["./alias", "./σ"]);
            assert_eq!(
                spell(&["ς", "X", "*.tmp"], windows),
                ["alias/x/*.tmp", "σ/x/*.tmp"],
                "each fork continues below its name"
            );
            assert_eq!(spell(&["cache", "Y"], windows), ["cache/Y"]);
            assert_eq!(spell(&["Missing"], windows), ["./Missing"]);
        }
    }

    /// cr-g3fix4-1: on Windows a listing pairs the typed name by the
    /// entry's identity too — volume serial number and file index, read
    /// without following a reparse point — never by case folding: `CACHE`
    /// names a file `cache`; hard-linked as `alias` beside it, both; with
    /// its second name in another directory, `cache` alone; and `ς` names
    /// `σ` where NTFS pairs them (skipped with a message where it does
    /// not). Runs on CI's Windows leg (std's hard links need no privilege
    /// there).
    #[cfg(windows)]
    #[test]
    fn a_windows_listing_pairs_by_identity() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("a");
        let elsewhere = dir.path().join("b");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&elsewhere).unwrap();
        std::fs::write(root.join("cache"), b"c").unwrap();
        std::fs::create_dir(root.join("Dir")).unwrap();
        let name = |rel: &str| stored_name(&root.join(rel));
        assert_eq!(name("CACHE"), ["cache"]);
        assert_eq!(name("DIR"), ["Dir"]);
        std::fs::hard_link(root.join("cache"), elsewhere.join("cache")).unwrap();
        assert_eq!(name("CACHE"), ["cache"], "the other name is elsewhere");
        std::fs::hard_link(root.join("cache"), root.join("alias")).unwrap();
        assert_eq!(name("CACHE"), ["alias", "cache"], "a tie names both");
        assert_eq!(name("cache"), ["cache"], "the exact spelling names itself");
        if pairs(&root, "σ", "ς") {
            std::fs::write(root.join("σ"), b"s").unwrap();
            assert_eq!(name("ς"), ["σ"], "NTFS's pairing, by identity");
        } else {
            eprintln!("skipping: {} does not pair ς with σ", root.display());
        }
    }

    /// Windows forms: either slash, any case, verbatim and UNC prefixes.
    /// Pure string logic, so it runs (and can fail) on every OS.
    #[test]
    fn windows_absolute_excludes_compare_case_insensitively_with_either_slash() {
        let relative = |pattern: &str, root: &str, windows: bool| {
            relative_to_root(pattern, root, windows).map(|rest| {
                written(
                    &rest.into_iter().map(Part::Typed).collect::<Vec<_>>(),
                    windows,
                )
            })
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

    /// cr-g3fix4-4: under the Windows rule an absolute exclude keeps a
    /// bracket class's `\` — `back[s\]lash` is one component, a class
    /// holding `s` and `\`, as globset reads it there. Before, every `\`
    /// of an absolute pattern was a separator, so it became
    /// `back[s/]lash`, whose class matches a real separator: the filter
    /// left out the unrelated `back/lash` too. The prefixes still read as
    /// before. String handling and a filter over the output, so it runs on
    /// every OS (the `build`/`build_spec` part under this host's rule).
    #[test]
    fn a_windows_absolute_exclude_keeps_a_bracket_class_backslash() {
        let relative = |pattern: &str, root: &str| {
            relative_to_root(pattern, root, true)
                .map(|rest| written(&rest.into_iter().map(Part::Typed).collect::<Vec<_>>(), true))
        };
        assert_eq!(
            relative(r"C:\src\back[s\]lash", r"C:\src").as_deref(),
            Some(r"./back[s\]lash")
        );
        assert_eq!(
            relative(r"C:\src\a\back[s\]lash\x", r"\\?\C:\src").as_deref(),
            Some(r"a/back[s\]lash/x")
        );
        assert_eq!(
            relative(r"\\srv\share\d\[\]x", r"\\?\UNC\srv\share\d").as_deref(),
            Some(r"./[\]x")
        );

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let exc = vec![format!(r"{}/back[s\]lash", root.display())];
        let resolved = resolve_excludes_on(&exc, Some(root), true);
        assert_eq!(resolved.patterns, [r"./back[s\]lash"]);
        assert!(resolved.unmatchable.is_empty());
        let mut filter = FileFilter::default();
        filter.exclude_files = resolved.patterns;
        let allows = |rel: &str| filter.allows_entry(Some(Path::new(rel)), Path::new(rel), 1, None);
        assert!(!allows("backslash/saved.txt"), "the class names `s`");
        assert!(
            allows("back/lash/saved.txt"),
            "a separator is no member of the class"
        );

        let mut i = inputs(&[], &exc);
        i.source_root = Some(root);
        assert_eq!(build(&i).unwrap().exclude_files, [r"./back[s\]lash"]);
        assert_eq!(build_spec(&i).unwrap().exclude, [r"./back[s\]lash"]);
    }

    /// cr-g3fix-3: relative patterns reach both builders — and so the wire
    /// form — with `\` written `/` under the Windows rule. Under the Unix
    /// rule `\` is no separator but globset's escape, and each escape is
    /// written in the class dialect with the same meaning (cr-g3fix4-3:
    /// `.\cache` is the name `.cache` there, so it is written `.cache`); a
    /// trailing lone `\` stays for validation to refuse. An absolute pattern
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
        assert_eq!(
            resolve_excludes_on(&exc, None, false).patterns,
            [".cache", r"cache\", "a.java", "*.tmp"]
        );

        let mut mixed = exc.clone();
        mixed.push(r"C:\elsewhere\x".to_string());
        let windows = resolve_excludes_on(&mixed, None, true);
        assert_eq!(
            windows.patterns,
            ["./cache", "cache/", "a/.java", "*.tmp", r"C:\elsewhere\x"]
        );
        assert_eq!(windows.unmatchable, [r"C:\elsewhere\x"]);
        let unix = resolve_excludes_on(&mixed, None, false);
        assert_eq!(
            unix.patterns,
            [".cache", r"cache\", "a.java", "*.tmp", "C:elsewherex"],
            "no absolute pattern off Windows"
        );
        assert!(unix.unmatchable.is_empty());

        // cr-g3fix3-3: normalized where it is typed, `.\cache` reaches the
        // filter (and the wire) as `./cache` and still anchors; a bracket
        // class keeps its `\` there too, as globset reads it.
        let typed: Vec<String> = [r".\cache", r"data[\]x\y"].map(String::from).to_vec();
        let written = resolve_excludes_on(&typed, None, true).patterns;
        assert_eq!(written, ["./cache", r"data[\]x/y"]);
        let mut filter = FileFilter::default();
        filter.exclude_files = written;
        let allows = |rel: &str| filter.allows_entry(Some(Path::new(rel)), Path::new(rel), 1, None);
        assert!(!allows("cache/x"), "anchored to the root");
        assert!(allows("a/cache/x"), "a nested cache is not named");
    }

    /// cr-g3fix4-3: a Unix user's typed escape never leaves this host as a
    /// bare `\`, which a Windows peer reads as a separator: `./a\b` (the
    /// name `ab` here) is written `./ab`, so a Windows peer's deletion
    /// scope leaves out `ab` — not `a/b` — exactly as this side's walk
    /// does. A class keeps its `\`; an absolute pattern's typed parts are
    /// written the same way. Both rules run on every OS (the absolute part
    /// on Unix, where a tempdir is an absolute path under the Unix rule).
    #[test]
    fn a_typed_unix_escape_names_the_same_path_under_either_rule() {
        let exc: Vec<String> = [r"./a\b", r"x\*y", r"[\]z", r"p\\q", "./cache"]
            .map(String::from)
            .to_vec();
        let written = resolve_excludes_on(&exc, None, false).patterns;
        assert_eq!(written, ["./ab", "x[*]y", r"[\]z", r"p[\]q", "./cache"]);
        for windows in [false, true] {
            let excluded =
                |rel: &str| crate::fs_enum::excluded_under_rule(&written, windows, Path::new(rel));
            assert!(
                excluded("ab/saved.txt"),
                "the name typed as a\\b (windows rule: {windows})"
            );
            assert!(
                !excluded("a/b/saved.txt"),
                "a/b was never named (windows rule: {windows})"
            );
            assert!(excluded("x*y"), "x*y (windows rule: {windows})");
            assert!(!excluded("x/zy"), "x/zy (windows rule: {windows})");
        }
        let host = resolve_excludes_on(&exc, None, cfg!(windows)).patterns;
        assert_eq!(build(&inputs(&[], &exc)).unwrap().exclude_files, host);
        assert_eq!(build_spec(&inputs(&[], &exc)).unwrap().exclude, host);
        let include = [r"a\*b".to_string()];
        assert_eq!(
            build_spec(&inputs(&include, &[])).unwrap().include,
            include,
            "--include is passed as typed"
        );

        if cfg!(unix) {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path();
            let absolute = vec![
                format!(r"{}/a\b", root.display()),
                format!(r"{}/sub/x\*y", root.display()),
            ];
            assert_eq!(
                resolve_excludes_on(&absolute, Some(root), false).patterns,
                ["./ab", "sub/x[*]y"]
            );
        }
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
