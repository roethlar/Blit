use eyre::{Context, Result};
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::enumeration::{EnumeratedEntry, FileEnumerator, PrunedPaths};
use crate::transfers::filter::DestinationNames;
// Filesystem enumeration and categorization (Unix focus)

/// Entry with size information for categorization
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub is_directory: bool,
}

/// Copy job with optional resume offset
#[derive(Debug, Clone)]
pub struct CopyJob {
    pub entry: FileEntry,
}

/// File filter options. Single source-of-truth for what passes through
/// the transfer pipeline regardless of source/destination type.
///
/// Semantics:
/// - `include_files` (whitelist): if any pattern is set, the file must
///   match at least one to pass. Empty list disables the whitelist.
/// - `exclude_files` / `exclude_dirs`: matching files/dirs are blocked.
///   An `exclude_files` pattern (`--exclude`) with no path separator
///   matches any component of an entry's relative path, so a directory of
///   that name is excluded with everything beneath it; a pattern with a
///   separator is source-relative and excludes the path it names and its
///   whole subtree. An excluded directory is pruned — never walked
///   (audit-19).
/// - `min_size` / `max_size`: size constraints applied after pattern checks.
/// - `min_age` / `max_age`: age constraints relative to `reference_time`
///   (set by the orchestrator at filter-build time, not by the leaf code).
/// - `files_from`: when present, only listed relative paths pass; all
///   other rules above are bypassed for the inclusion test.
#[derive(Debug)]
pub struct FileFilter {
    pub include_files: Vec<String>,
    pub exclude_files: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub min_age: Option<Duration>,
    pub max_age: Option<Duration>,
    /// Set by orchestrator when building the filter (calculated, not hardcoded).
    pub reference_time: Option<SystemTime>,
    pub files_from: Option<HashSet<PathBuf>>,
    /// cr-g3fix5-1: the destination's own names for the excludes — set
    /// only for a mirror's deletion scope at a destination
    /// (`transfers::filter`'s `destination_scope`), where an entry the
    /// destination stores in another spelling of an excluded name (another
    /// case, another Unicode normalization, final sigma) is out of the
    /// scope too ([`DestinationNames`]). Every other filter compares as
    /// written. It replaces cr-g3fix4-2's case-insensitive name rules,
    /// which folded ASCII only.
    pub(crate) destination: Option<std::sync::Arc<DestinationNames>>,
    /// cr-g3fix6-2: where a walk under this filter records each entry it
    /// prunes ([`PrunedPaths`]) — set only on the exclude rules a mirror's
    /// source walks with (`FilteredSource::recording_pruned`), so the
    /// destination can keep its own spelling of every entry an exclude
    /// left out. `None` everywhere else: nothing is recorded.
    pub(crate) pruned: Option<std::sync::Arc<PrunedPaths>>,
    compiled_includes: OnceCell<globset::GlobSet>,
    compiled_files: OnceCell<CompiledPatterns>,
    compiled_dirs: OnceCell<globset::GlobSet>,
    compiled_subtrees: OnceCell<ExcludeRules>,
}

/// audit-19: the `--exclude` patterns as the any-depth and subtree rules
/// read them. A pattern's trailing `/` is dropped (`a/.java/` names the
/// same tree as `a/.java`) and a leading `./` anchors it to the source
/// root (`./.java` is the top-level `.java` only). On Windows `\` is read
/// as `/` first (cr-g3fix-3: `.\.java`, `a\.java\`), as globset reads it
/// there, but never inside a bracket class (cr-g3fix3-3: `[\]` is a Unix
/// peer's escape for a name holding `\`, and stays one) — see
/// [`normalize_separators`]. Each rule is matched as a glob only
/// ([`CompiledPatterns`], cr-g3fix3-2).
#[derive(Debug, Default)]
struct ExcludeRules {
    /// Patterns with no separator: each matches any one component of a
    /// relative path, at any depth.
    names: CompiledPatterns,
    /// Source-relative patterns: each matches a leading run of a relative
    /// path's components — the entry itself or a directory above it.
    paths: CompiledPatterns,
}

/// How [`ExcludeRules`] reads one `--exclude` pattern (cr-g3fix4-2: a
/// mirror's deletion scope reads it the same way to respell it at the
/// destination).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExcludeRule {
    /// The pattern with its separators normalized ([`normalize_separators`]),
    /// its trailing `/` dropped and its leading `./` stripped.
    pub(crate) body: String,
    /// A leading `./` anchored it to the root.
    pub(crate) anchored: bool,
    /// A path rule — anchored, or holding `/` — matched against a leading
    /// run of a relative path's components; otherwise a name, matched
    /// against any one component.
    pub(crate) path: bool,
}

/// [`ExcludeRule`] for `pattern` under one platform's path rule; `None`
/// for a pattern that names nothing (empty, `.`).
pub(crate) fn exclude_rule(pattern: &str, windows: bool) -> Option<ExcludeRule> {
    let pattern = normalize_separators(pattern, windows);
    let mut body = pattern.trim_end_matches('/');
    let mut anchored = false;
    while let Some(rest) = body.strip_prefix("./") {
        body = rest;
        anchored = true;
    }
    if body.is_empty() || body == "." {
        return None;
    }
    Some(ExcludeRule {
        body: body.to_string(),
        anchored,
        path: anchored || body.contains('/'),
    })
}

impl ExcludeRules {
    /// The rules `patterns` make under one platform's path rule
    /// (`windows`).
    fn compile(patterns: &[String], windows: bool) -> Self {
        let mut names = Vec::new();
        let mut paths = Vec::new();
        for rule in patterns
            .iter()
            .filter_map(|pattern| exclude_rule(pattern, windows))
        {
            if rule.path {
                paths.push(rule.body);
            } else {
                names.push(rule.body);
            }
        }
        Self {
            names: CompiledPatterns::compile(&names),
            paths: CompiledPatterns::compile(&paths),
        }
    }

    /// True when a rule names `rel_path`, one of its components, or a
    /// directory above it. Components are joined with `/` on every OS.
    fn excludes(&self, rel_path: &Path) -> bool {
        if self.names.is_empty() && self.paths.is_empty() {
            return false;
        }
        let mut prefix = String::new();
        for component in rel_path.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            let name = name.to_string_lossy();
            if !self.names.is_empty() && self.names.is_match(&name) {
                return true;
            }
            if !self.paths.is_empty() {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(&name);
                if self.paths.is_match(&prefix) {
                    return true;
                }
            }
        }
        false
    }
}

/// Whether `patterns`, read as the any-depth and subtree rules under one
/// platform's path rule (`windows`), exclude `rel_path` — so either
/// rule's reading of a pattern is testable on every OS.
#[cfg(test)]
pub(crate) fn excluded_under_rule(patterns: &[String], windows: bool, rel_path: &Path) -> bool {
    ExcludeRules::compile(patterns, windows).excludes(rel_path)
}

/// cr-g3fix3-2: `--exclude` patterns compiled for matching. A pattern the
/// glob compiler accepts is matched as a glob and nothing else; only one it
/// refuses — reachable only past validation ([`FileFilter::validate_globs`],
/// a daemon's spec check), e.g. a filter an embedder builds directly —
/// falls back to [`glob_match`]'s text comparison. For a pattern that
/// compiles, that comparison adds nothing but a misreading: it compares
/// the pattern's text, so an escaped literal (`./data[[]1[]]`, which names
/// `data[1]`) also matched an unrelated entry spelled like the escape.
#[derive(Debug, Default)]
struct CompiledPatterns {
    globs: globset::GlobSet,
    /// The patterns the glob compiler refused (all of them when the set as
    /// a whole could not be built), compared as text.
    refused: Vec<String>,
}

impl CompiledPatterns {
    fn compile(patterns: &[String]) -> Self {
        let mut builder = globset::GlobSetBuilder::new();
        let mut refused = Vec::new();
        for pattern in patterns {
            match globset::Glob::new(pattern) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(_) => refused.push(pattern.clone()),
            }
        }
        match builder.build() {
            Ok(globs) => Self { globs, refused },
            Err(_) => Self {
                globs: globset::GlobSet::empty(),
                refused: patterns.to_vec(),
            },
        }
    }

    fn is_empty(&self) -> bool {
        self.globs.is_empty() && self.refused.is_empty()
    }

    fn is_match(&self, text: &str) -> bool {
        self.globs.is_match(text) || self.refused.iter().any(|p| glob_match(p, text))
    }
}

/// cr-g3fix-3: an `--exclude` pattern with every separator written `/`.
/// On Windows `\` separates path components too (`.\cache`, `cache\`,
/// `a\.java`) — except inside a bracket class, which is copied as it is
/// (cr-g3fix3-3): there globset reads `\` as a literal character on every
/// platform, and `[\]` is how a Unix peer writes a name holding one
/// (`transfers::filter`'s `literal_glob`); rewritten, it became `[/]`,
/// which matches a real separator. This is the reading globset itself
/// gives a pattern on Windows (a bare `\` is `/`, a class's `\` is
/// literal), so the rules built from the text agree with the glob.
/// Elsewhere `\` is an ordinary character (globset reads it as an
/// escape), so the pattern is kept as typed.
pub(crate) fn normalize_separators(pattern: &str, windows: bool) -> std::borrow::Cow<'_, str> {
    if !windows || !pattern.contains('\\') {
        return std::borrow::Cow::Borrowed(pattern);
    }
    let mut written = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        let len = match c {
            '[' => bracket_class_len(rest).unwrap_or(1),
            _ => c.len_utf8(),
        };
        if c == '\\' {
            written.push('/');
        } else {
            written.push_str(&rest[..len]);
        }
        rest = &rest[len..];
    }
    std::borrow::Cow::Owned(written)
}

/// cr-g3fix4-3: a typed `--exclude` pattern in the dialect every peer
/// reads alike: `/`-separated, holding no bare `\`. Under the Windows
/// rule that is [`normalize_separators`]. Under the Unix rule a bare `\`
/// is globset's escape, while a Windows peer reads it as a separator
/// (globset's own reading there, and `normalize_separators`), so a typed
/// Unix escape named another path on a Windows peer — `./a\b` is `ab`
/// here and `a/b` there, and under `mirror` the peer's deletion scope
/// kept `ab` while shielding `a/b`. Each escape `\c` outside a bracket
/// class is therefore written in the class dialect ([`escape_as_class`]),
/// which reads the same on every platform, and a class is copied as it is
/// (its `\` is a member everywhere). A pattern globset refuses under the
/// Unix rule — a trailing lone `\`, an unclosed class — is returned as
/// typed, so validation still refuses it.
pub(crate) fn typed_exclude_dialect(pattern: &str, windows: bool) -> std::borrow::Cow<'_, str> {
    if windows {
        return normalize_separators(pattern, true);
    }
    if !pattern.contains('\\') {
        return std::borrow::Cow::Borrowed(pattern);
    }
    let mut written = String::with_capacity(pattern.len() + 8);
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        match c {
            '[' => {
                let Some(len) = bracket_class_len(rest) else {
                    return std::borrow::Cow::Borrowed(pattern);
                };
                written.push_str(&rest[..len]);
                rest = &rest[len..];
            }
            '\\' => {
                let Some(escaped) = rest[1..].chars().next() else {
                    return std::borrow::Cow::Borrowed(pattern);
                };
                let after = &rest[1 + escaped.len_utf8()..];
                if escaped == '/' {
                    if !escaped_separator(&mut written, after) {
                        return std::borrow::Cow::Borrowed(pattern);
                    }
                } else {
                    written.push_str(&escape_as_class(escaped));
                }
                rest = after;
            }
            c => {
                written.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    std::borrow::Cow::Owned(written)
}

/// cr-g3fix5-3: an escaped `/` written onto `written` (the pattern so far,
/// in the class dialect; `after` is the typed text that follows it) with
/// the meaning globset and the rule reading give `\/` under the Unix rule:
/// a literal separator. A bare `/` reads the same everywhere but in three
/// places, each written otherwise:
/// - after a run of `*`: globset reads `**` as the recursive `**/` only
///   when a bare `/` follows it, so `**\/x` is two `*` (any text, `/`
///   included) then `/x` — it needs a component before `x`. The run is
///   written as one `*`, the same any text, so `**\/x` becomes `*/x`.
/// - after a leading `.` (or `./.`, …): `./` would anchor the pattern to
///   the root, which `.\/x` never was; that `.` is written `[.]`.
/// - at the end, with nothing but `/` after it: the rule reading drops a
///   pattern's trailing bare `/`s and leaves a dangling escape, a reading
///   no spelling without `\` gives — false, and the pattern stays as typed.
///
/// An escaped `/` before `**` needs nothing: globset reads the separator
/// before `**` from the character itself, escaped or not.
fn escaped_separator(written: &mut String, after: &str) -> bool {
    if after.chars().all(|c| c == '/') {
        return false;
    }
    if written.ends_with('*') {
        while written.ends_with('*') {
            written.pop();
        }
        written.push('*');
    } else if written.strip_suffix('.').is_some_and(|before| {
        let mut anchors = before;
        while let Some(rest) = anchors.strip_prefix("./") {
            anchors = rest;
        }
        anchors.is_empty()
    }) {
        written.pop();
        written.push_str("[.]");
    }
    written.push('/');
    true
}

/// cr-g3fix4-3: the class-dialect spelling of the escaped character `c`:
/// `[c]` for a character globset reads as syntax somewhere outside a
/// class (`?*[]{}` as `globset::escape` brackets them, `,` — an
/// alternation's separator — and `\`), `c` itself otherwise.
fn escape_as_class(c: char) -> String {
    match c {
        '?' | '*' | '[' | ']' | '{' | '}' | ',' | '\\' => format!("[{c}]"),
        c => c.to_string(),
    }
}

/// cr-g3fix3-3: the byte length of the bracket class `text` opens, `[`
/// through its closing `]`, read as globset 0.4 reads one: an optional `!`
/// or `^`, then a `]` in first place is a member, and the class ends at
/// the next `]`. `None` when it never closes (globset refuses the pattern,
/// and the `[` is an ordinary character here).
pub(crate) fn bracket_class_len(text: &str) -> Option<usize> {
    let mut body = text.strip_prefix('[')?;
    let mut len = 1;
    if let Some(after) = body.strip_prefix(['!', '^']) {
        body = after;
        len += 1;
    }
    if let Some(after) = body.strip_prefix(']') {
        body = after;
        len += 1;
    }
    Some(len + body.find(']')? + 1)
}

impl FileFilter {
    /// Create a shallow clone that preserves user-specified patterns while
    /// dropping any compiled glob caches. This lets helpers reuse filters
    /// without sharing mutable compilation state.
    pub fn clone_without_cache(&self) -> Self {
        Self {
            include_files: self.include_files.clone(),
            exclude_files: self.exclude_files.clone(),
            exclude_dirs: self.exclude_dirs.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            min_age: self.min_age,
            max_age: self.max_age,
            reference_time: self.reference_time,
            files_from: self.files_from.clone(),
            destination: self.destination.clone(),
            pruned: self.pruned.clone(),
            compiled_includes: OnceCell::new(),
            compiled_files: OnceCell::new(),
            compiled_dirs: OnceCell::new(),
            compiled_subtrees: OnceCell::new(),
        }
    }

    /// audit-19: the part of this filter a source's own directory walk may
    /// apply — the exclude rules alone — so an excluded tree is pruned
    /// where it is walked, never read. `None` when nothing could be pruned,
    /// or when `files_from` is set (a listed path passes whatever the
    /// excludes say, so the walk must not drop it). Every other rule still
    /// applies wherever the full filter runs.
    pub fn pruning_rules(&self) -> Option<FileFilter> {
        if self.files_from.is_some()
            || (self.exclude_files.is_empty() && self.exclude_dirs.is_empty())
        {
            return None;
        }
        Some(FileFilter {
            exclude_files: self.exclude_files.clone(),
            exclude_dirs: self.exclude_dirs.clone(),
            destination: self.destination.clone(),
            ..FileFilter::default()
        })
    }

    /// True when no rules are configured — caller can skip filter checks entirely.
    pub fn is_empty(&self) -> bool {
        self.include_files.is_empty()
            && self.exclude_files.is_empty()
            && self.exclude_dirs.is_empty()
            && self.min_size.is_none()
            && self.max_size.is_none()
            && self.min_age.is_none()
            && self.max_age.is_none()
            && self.files_from.is_none()
    }

    /// Load a `--files-from` list (one relative path per line, blank lines
    /// and `#` comments skipped). Used by the orchestrator/CLI helper —
    /// the leaf TransferSource code should not parse files itself.
    pub fn load_files_from(path: &Path) -> Result<HashSet<PathBuf>> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening files-from list {}", path.display()))?;
        let mut set = HashSet::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            set.insert(PathBuf::from(trimmed));
        }
        Ok(set)
    }

    fn build_globset(patterns: &[String]) -> globset::GlobSet {
        let mut builder = globset::GlobSetBuilder::new();
        for pat in patterns {
            if let Ok(glob) = globset::Glob::new(pat) {
                builder.add(glob);
            }
        }
        builder.build().unwrap_or_else(|_| {
            globset::GlobSetBuilder::new()
                .build()
                .expect("empty globset")
        })
    }

    /// R58-F12: pre-validate patterns at filter-construction time.
    /// The default build_globset() above silently drops invalid
    /// patterns; that's acceptable as a runtime fallback (a
    /// corrupted profile shouldn't crash the binary), but at the
    /// CLI we want to reject malformed globs up front with a
    /// pointer to the bad pattern.
    ///
    /// Returns `Ok(())` if every pattern in `include_files`,
    /// `exclude_files`, and `exclude_dirs` compiles. The error
    /// message includes the bad pattern verbatim so the user
    /// knows what to fix.
    pub fn validate_globs(&self) -> std::result::Result<(), String> {
        for (label, patterns) in [
            ("--include", &self.include_files),
            ("--exclude", &self.exclude_files),
        ] {
            for pat in patterns {
                if let Err(err) = globset::Glob::new(pat) {
                    return Err(format!(
                        "{label} pattern {pat:?} is not a valid glob: {err}"
                    ));
                }
            }
        }
        for pat in &self.exclude_dirs {
            if let Err(err) = globset::Glob::new(pat) {
                return Err(format!(
                    "--exclude-dir pattern {pat:?} is not a valid glob: {err}"
                ));
            }
        }
        Ok(())
    }

    fn include_globs(&self) -> &globset::GlobSet {
        self.compiled_includes
            .get_or_init(|| Self::build_globset(&self.include_files))
    }

    fn file_globs(&self) -> &CompiledPatterns {
        self.compiled_files
            .get_or_init(|| CompiledPatterns::compile(&self.exclude_files))
    }

    fn dir_globs(&self) -> &globset::GlobSet {
        self.compiled_dirs
            .get_or_init(|| Self::build_globset(&self.exclude_dirs))
    }

    fn subtree_rules(&self) -> &ExcludeRules {
        self.compiled_subtrees
            .get_or_init(|| ExcludeRules::compile(&self.exclude_files, cfg!(windows)))
    }

    /// cr-g3fix6-2: a walk under this filter left `rel` out — recorded
    /// when the filter carries a recorder ([`Self::pruned`]).
    pub(crate) fn note_pruned(&self, rel: &Path) {
        if let Some(pruned) = &self.pruned {
            pruned.record(rel);
        }
    }

    /// `rel_path` is the directory's path relative to the walk root;
    /// `abs_path` is where it lives.
    pub(crate) fn allows_dir(&self, rel_path: &Path, abs_path: &Path) -> bool {
        self.should_include_dir(rel_path, abs_path)
    }

    /// The `--exclude` test for one entry, file or directory. Today's
    /// matches are kept exactly (a pattern against the bare name or the
    /// whole relative path, as a glob — [`CompiledPatterns`]); audit-19
    /// adds the any-depth name rule and the subtree rule (see
    /// [`ExcludeRules`]).
    fn excluded(&self, rel_path: Option<&Path>, filename: &str, path_str: &str) -> bool {
        if !self.exclude_files.is_empty() {
            let patterns = self.file_globs();
            if patterns.is_match(filename) || patterns.is_match(path_str) {
                return true;
            }
            if rel_path.is_some_and(|rel| self.subtree_rules().excludes(rel)) {
                return true;
            }
        }
        // cr-g3fix6-2: a destination scope also leaves out the entries the
        // source walk pruned, in the destination's own names — asked even
        // with no `--exclude` pattern, as an `exclude_dirs` rule prunes too.
        rel_path.is_some_and(|rel| {
            self.destination
                .as_deref()
                .is_some_and(|names| names.protects(rel))
        })
    }

    /// Full filter check. `rel_path` enables `files_from` matching;
    /// `mtime` enables age filtering. Both default to permissive when
    /// `None` (so back-compat callers via `allows_file` still work).
    pub fn allows_entry(
        &self,
        rel_path: Option<&Path>,
        abs_path: &Path,
        size: u64,
        mtime: Option<SystemTime>,
    ) -> bool {
        // files_from is exclusive: only listed paths pass. Other rules
        // are bypassed because the user explicitly enumerated targets.
        if let Some(ref allowed) = self.files_from {
            return rel_path.map(|p| allowed.contains(p)).unwrap_or(false);
        }

        let filename = abs_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path_str = rel_path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| abs_path.to_string_lossy().into_owned());

        // Whitelist: when include_files is non-empty, file must match one.
        // Match against both the relative path and the bare filename so
        // `--include '*.log'` works without users writing `**/*.log`.
        if !self.include_files.is_empty() {
            let gs = self.include_globs();
            let matched = gs.is_match(&filename)
                || gs.is_match(&path_str)
                || self
                    .include_files
                    .iter()
                    .any(|p| glob_match(p, &filename) || glob_match(p, &path_str));
            if !matched {
                return false;
            }
        }

        // Blacklist
        if self.excluded(rel_path, &filename, &path_str) {
            return false;
        }

        // Size
        if let Some(min) = self.min_size {
            if size < min {
                return false;
            }
        }
        if let Some(max) = self.max_size {
            if size > max {
                return false;
            }
        }

        // Age (only applied when both mtime and reference_time are present)
        if let (Some(mtime), Some(now)) = (mtime, self.reference_time) {
            if let Ok(age) = now.duration_since(mtime) {
                if let Some(min_age) = self.min_age {
                    if age < min_age {
                        return false;
                    }
                }
                if let Some(max_age) = self.max_age {
                    if age > max_age {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Check if a directory should be included (walked).
    fn should_include_dir(&self, rel_path: &Path, abs_path: &Path) -> bool {
        // files_from doesn't restrict dir traversal — we still need to
        // descend into directories to find listed files.
        if self.files_from.is_some() {
            return true;
        }

        let gs = self.dir_globs();
        if gs.is_match(abs_path.to_string_lossy().as_ref()) {
            return false;
        }
        for pattern in &self.exclude_dirs {
            for component in abs_path.components() {
                if let Some(component_str) = component.as_os_str().to_str() {
                    if glob_match(pattern, component_str) {
                        return false;
                    }
                }
            }
        }

        // audit-19: a directory `--exclude` names is pruned, not just
        // filtered — the walk never opens it, so an excluded tree costs
        // nothing to skip and nothing inside it (an unreadable file
        // included) is ever read or reported.
        let filename = rel_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        !self.excluded(Some(rel_path), &filename, &rel_path.to_string_lossy())
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            include_files: Vec::new(),
            exclude_files: Vec::new(),
            exclude_dirs: Vec::new(),
            min_size: None,
            max_size: None,
            min_age: None,
            max_age: None,
            reference_time: None,
            files_from: None,
            destination: None,
            pruned: None,
            compiled_includes: OnceCell::new(),
            compiled_files: OnceCell::new(),
            compiled_dirs: OnceCell::new(),
            compiled_subtrees: OnceCell::new(),
        }
    }
}

impl Clone for FileFilter {
    fn clone(&self) -> Self {
        self.clone_without_cache()
    }
}

/// Parse a human-readable size like "100K", "10M", "1G", "1.5Mi" into bytes.
/// SI suffixes (K=1000) and binary suffixes (Ki=1024) both supported.
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        eyre::bail!("empty size string");
    }
    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("Ti") {
        (n, 1u64 << 40)
    } else if let Some(n) = s.strip_suffix("Gi") {
        (n, 1u64 << 30)
    } else if let Some(n) = s.strip_suffix("Mi") {
        (n, 1u64 << 20)
    } else if let Some(n) = s.strip_suffix("Ki") {
        (n, 1u64 << 10)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1_000_000_000_000u64)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1_000_000_000)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1_000_000)
    } else if let Some(n) = s.strip_suffix('K') {
        (n, 1_000)
    } else {
        (s, 1)
    };
    let num: f64 = num_str
        .parse()
        .with_context(|| format!("invalid size number: {num_str}"))?;
    Ok((num * multiplier as f64) as u64)
}

/// Parse a duration like "30s", "5m", "1h", "7d", or compounds "1h30m".
/// A bare number is interpreted as seconds.
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        eyre::bail!("empty duration string");
    }
    let mut total_secs: u64 = 0;
    let mut num_buf = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_buf.push(ch);
        } else {
            if num_buf.is_empty() {
                eyre::bail!("invalid duration: {s}");
            }
            let num: f64 = num_buf
                .parse()
                .with_context(|| format!("invalid number in duration: {s}"))?;
            num_buf.clear();
            let multiplier: u64 = match ch {
                's' => 1,
                'm' => 60,
                'h' => 3600,
                'd' => 86400,
                'w' => 604800,
                _ => eyre::bail!("unknown duration unit '{ch}' in: {s}"),
            };
            total_secs += (num * multiplier as f64) as u64;
        }
    }
    if !num_buf.is_empty() {
        let num: f64 = num_buf
            .parse()
            .with_context(|| format!("invalid number in duration: {s}"))?;
        total_secs += num as u64;
    }
    if total_secs == 0 {
        eyre::bail!("duration must be non-zero: {s}");
    }
    Ok(Duration::from_secs(total_secs))
}

/// Simple glob matching (supports * wildcards)
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Simple wildcard matching
    if pattern.contains('*') {
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
            return text.contains(middle);
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            return text.ends_with(suffix);
        } else if let Some(prefix) = pattern.strip_suffix('*') {
            return text.starts_with(prefix);
        }
    }

    // Exact match
    pattern == text
}

// All Windows-specific code removed.

/// Fast directory enumeration with filtering for non-Windows platforms
#[cfg(not(windows))]
/// Enumerate files under `root`, applying `filter` rules. Does not follow symlinks.
pub fn enumerate_directory_filtered(
    root: &Path,
    filter: &mut FileFilter,
) -> Result<Vec<FileEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache());
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(EnumeratedEntry::into_file_entry)
        .collect())
}

#[cfg(windows)]
pub fn enumerate_directory_filtered(
    root: &Path,
    filter: &mut FileFilter,
) -> Result<Vec<FileEntry>> {
    let enumerator = FileEnumerator::new(filter.clone_without_cache());
    let entries = enumerator.enumerate_local(root)?;
    Ok(entries
        .into_iter()
        .filter_map(EnumeratedEntry::into_file_entry)
        .collect())
}

#[cfg(test)]
mod tests {
    //! audit-19: the `--exclude` contract — a separator-less pattern names
    //! a component at any depth, a source-relative path names its whole
    //! subtree, and an excluded directory is pruned, never walked.
    use super::*;
    use crate::enumeration::FileEnumerator;

    fn excluding(patterns: &[&str]) -> FileFilter {
        FileFilter {
            exclude_files: patterns.iter().map(|p| p.to_string()).collect(),
            ..FileFilter::default()
        }
    }

    /// The `allows_entry` verdict on one file reached by its relative path
    /// alone — how every session source (`FilteredSource`) asks.
    fn allows(filter: &FileFilter, rel: &str) -> bool {
        filter.allows_entry(Some(Path::new(rel)), Path::new(rel), 1, None)
    }

    /// `.java/fonts/x`, `a/.java/y`, `a/b/.java/deep/z`, plus files that
    /// must survive every exclude below.
    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for (rel, body) in [
            (".java/fonts/x", "x"),
            ("a/.java/y", "y"),
            ("a/b/.java/deep/z", "z"),
            ("keep.txt", "k"),
            ("a/keep.txt", "k"),
            ("a/t.tmp", "t"),
            ("a/b/u.tmp", "u"),
            ("logs/app.log", "l"),
        ] {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, body).expect("write");
        }
        dir
    }

    /// Every file and directory the walk yields, `/`-joined and sorted;
    /// fails if the walk had to skip anything it could not read.
    fn walk(root: &Path, filter: FileFilter) -> Vec<String> {
        let (entries, outcome) = FileEnumerator::new(filter)
            .enumerate_local_capturing(root)
            .expect("enumerate");
        assert!(
            outcome.suppressed_errors.is_empty(),
            "the walk opened something it could not read: {:?}",
            outcome.suppressed_errors
        );
        let mut rels: Vec<String> = entries
            .iter()
            .map(|e| crate::path_posix::relative_path_to_posix(&e.relative_path))
            .collect();
        rels.sort();
        rels
    }

    #[test]
    fn a_bare_name_excludes_that_directory_and_its_contents_at_any_depth() {
        let filter = excluding(&[".java"]);
        for rel in [".java/fonts/x", "a/.java/y", "a/b/.java/deep/z", ".java"] {
            assert!(!allows(&filter, rel), "{rel} must be excluded");
        }
        assert!(allows(&filter, "a/keep.txt"));
        assert!(allows(&filter, "a/not.java"), "only the whole name matches");

        let dir = tree();
        let walked = walk(dir.path(), excluding(&[".java"]));
        assert!(
            walked
                .iter()
                .all(|rel| !rel.split('/').any(|c| c == ".java")),
            "no .java directory, nor anything beneath one, is walked: {walked:?}"
        );
        assert!(walked.contains(&"keep.txt".to_string()));
        assert!(walked.contains(&"a/keep.txt".to_string()));
    }

    #[test]
    fn a_filename_glob_still_matches_at_any_depth() {
        let filter = excluding(&["*.tmp"]);
        assert!(!allows(&filter, "a/t.tmp"));
        assert!(!allows(&filter, "a/b/u.tmp"));
        assert!(allows(&filter, "a/keep.txt"));

        let dir = tree();
        let walked = walk(dir.path(), excluding(&["*.tmp"]));
        assert!(
            !walked.iter().any(|rel| rel.ends_with(".tmp")),
            "{walked:?}"
        );
        assert!(walked.contains(&"a/keep.txt".to_string()));
    }

    #[test]
    fn a_relative_directory_path_excludes_its_subtree_only() {
        for pattern in ["a/.java", "a/.java/"] {
            let filter = excluding(&[pattern]);
            assert!(!allows(&filter, "a/.java/y"), "{pattern}: a/.java/y");
            assert!(!allows(&filter, "a/.java/more/deep"), "{pattern}: deep");
            assert!(allows(&filter, ".java/fonts/x"), "{pattern}: other tree");
            assert!(allows(&filter, "a/b/.java/deep/z"), "{pattern}: other tree");

            let dir = tree();
            let walked = walk(dir.path(), excluding(&[pattern]));
            assert!(
                !walked.iter().any(|rel| rel.starts_with("a/.java")),
                "{pattern}: {walked:?}"
            );
            assert!(walked.contains(&".java/fonts/x".to_string()));
            assert!(walked.contains(&"a/b/.java/deep/z".to_string()));
        }
    }

    /// `./name` (the form an absolute pattern naming a top-level entry is
    /// relativized to) names that top-level path only.
    #[test]
    fn a_dot_slash_name_is_anchored_to_the_root() {
        let filter = excluding(&["./.java"]);
        assert!(!allows(&filter, ".java/fonts/x"));
        assert!(allows(&filter, "a/.java/y"), "a nested .java is not named");
    }

    /// Forms that worked before audit-19 keep working.
    #[test]
    fn todays_working_forms_still_match() {
        assert!(!allows(&excluding(&["**/*.log"]), "logs/app.log"));
        assert!(!allows(&excluding(&["logs/**"]), "logs/app.log"));
        assert!(!allows(&excluding(&["logs/app.log"]), "logs/app.log"));
        assert!(!allows(&excluding(&[".java/**"]), ".java/fonts/x"));
        assert!(allows(&excluding(&["logs/app.log"]), "logs/other.log"));
    }

    /// cr-g3fix3-2: an escaped literal — the form a name read from the disk
    /// takes (`data[1]` written `data[[]1[]]`) — matches that name only,
    /// never an entry spelled like its escape: a compiled rule is matched as
    /// a glob and nothing else, in the any-depth and subtree rules as in
    /// the whole-pattern test. A pattern the glob compiler refuses still
    /// falls back to the text comparison.
    #[test]
    fn an_escaped_literal_matches_its_name_and_not_its_encoded_spelling() {
        let rules = ExcludeRules::compile(
            &["./data[[]1[]]".to_string(), "set[[]2[]]".to_string()],
            cfg!(windows),
        );
        assert!(rules.excludes(Path::new("data[1]")));
        assert!(rules.excludes(Path::new("data[1]/x")));
        assert!(
            !rules.excludes(Path::new("data[[]1[]]"))
                && !rules.excludes(Path::new("data[[]1[]]/x")),
            "the escape's own spelling is another name"
        );
        assert!(!rules.excludes(Path::new("data1/x")));
        assert!(rules.excludes(Path::new("q/set[2]/x")));
        assert!(
            !rules.excludes(Path::new("q/set[[]2[]]/x")),
            "the escape's own spelling is another name"
        );

        let filter = excluding(&["a[[]b[]]/c"]);
        assert!(!allows(&filter, "a[b]/c"));
        assert!(
            allows(&filter, "a[[]b[]]/c"),
            "the whole-pattern test matches the glob only"
        );

        let refused = ExcludeRules::compile(&["*[x".to_string()], cfg!(windows));
        assert!(
            refused.excludes(Path::new("a/foo[x")),
            "a pattern the glob compiler refuses is compared as text"
        );
        let refused = excluding(&["*[x"]);
        assert!(!allows(&refused, "a/foo[x"), "the whole-pattern test too");
        assert!(allows(&refused, "a/keep.txt"));
    }

    /// cr-g3fix-3: under the Windows rule `\` is a separator, so `.\name`
    /// anchors, a trailing `\` is dropped and `a\name` is a source-relative
    /// path; under the Unix rule `\` is part of the pattern. Both rules run
    /// on every OS.
    #[test]
    fn backslash_forms_follow_the_platform_rule() {
        assert_eq!(normalize_separators(r".\cache", true), "./cache");
        assert_eq!(normalize_separators(r"cache\", true), "cache/");
        assert_eq!(normalize_separators(r"a\.java", true), "a/.java");
        assert_eq!(normalize_separators(r"a\.java", false), r"a\.java");

        let windows = |pattern: &str| ExcludeRules::compile(&[pattern.to_string()], true);
        let anchored = windows(r".\.java");
        assert!(anchored.excludes(Path::new(".java/fonts/x")));
        assert!(
            !anchored.excludes(Path::new("a/.java/y")),
            "anchored to the root"
        );
        assert!(
            windows(r".java\").excludes(Path::new("a/.java/y")),
            "a trailing \\ is dropped: a name at any depth"
        );
        let subtree = windows(r"a\.java\");
        assert!(subtree.excludes(Path::new("a/.java/y")));
        assert!(
            !subtree.excludes(Path::new(".java/fonts/x")),
            "a source-relative path names one subtree"
        );
        let unix = ExcludeRules::compile(&[r"a\.java".to_string()], false);
        assert!(
            !unix.excludes(Path::new("a/.java/y")),
            "off Windows a backslash is no separator"
        );
    }

    /// cr-g3fix3-3: under the Windows rule a bracket class is copied as it
    /// is, `\` included — `[\]` is how a Unix peer writes a name holding
    /// `\`, and globset reads a class's `\` literally on every platform — so
    /// a Unix-generated wire pattern `./back[\]slash` compiled by a Windows
    /// peer never becomes `./back[/]slash`, which matched the unrelated
    /// subtree `back/slash` and kept it out of a mirror's deletion scope.
    /// Outside a class `\` is still a separator there. Both rules run on
    /// every OS; the wire path a daemon takes runs under this host's rule.
    #[test]
    fn a_bracket_class_keeps_its_backslash_under_either_rule() {
        for (typed, windows_form) in [
            (r"./back[\]slash", r"./back[\]slash"),
            (r"a\[\]\b", r"a/[\]/b"),
            (r"[]\]\x", r"[]\]/x"),
            (r"[!]\]\x", r"[!]\]/x"),
            (r"[^\]x\y", r"[^\]x/y"),
            (r"data[[]1[]]\sub", "data[[]1[]]/sub"),
            (r"open[x\y", "open[x/y"),
        ] {
            assert_eq!(normalize_separators(typed, true), windows_form, "{typed}");
            assert_eq!(normalize_separators(typed, false), typed, "{typed}");
        }

        for windows in [true, false] {
            let rules = ExcludeRules::compile(&[r"./back[\]slash".to_string()], windows);
            for rel in ["back/slash/saved.txt", "back/slash", "backslash/x"] {
                assert!(
                    !rules.excludes(Path::new(rel)),
                    "{rel} stays in scope (windows rule: {windows})"
                );
            }
            if !cfg!(windows) {
                assert!(
                    rules.excludes(Path::new(r"back\slash/saved.txt")),
                    "the name it was written for (windows rule: {windows})"
                );
            }
        }

        let received = crate::remote::transfer::operation_spec::filter_from_spec(
            crate::generated::FilterSpec {
                exclude: vec![r"./back[\]slash".to_string()],
                ..Default::default()
            },
        )
        .expect("a valid wire pattern");
        for rel in ["back/slash/saved.txt", "back/slash"] {
            assert!(allows(&received, rel), "{rel} stays in scope");
            assert!(
                received.allows_dir(Path::new(rel), Path::new(rel)),
                "{rel} is walked"
            );
        }
    }

    /// cr-g3fix4-3: under the Unix rule a typed escape `\c` outside a
    /// bracket class is written in the class dialect — `[c]` for a
    /// character globset reads as syntax outside a class, `c` itself
    /// otherwise — and a class is copied as it is, so the written pattern
    /// holds no bare `\` (a Windows peer reads a bare one as a separator)
    /// and matches exactly what the typed one matched here. A pattern
    /// globset refuses (a trailing `\`, an unclosed class) comes back as
    /// typed, so validation still refuses it; the Windows rule is the
    /// separator rewrite. Pure, so it runs on every OS.
    #[test]
    fn a_typed_unix_escape_is_written_in_the_class_dialect() {
        for (typed, written) in [
            (r"./a\b", "./ab"),
            (r"x\*y", "x[*]y"),
            (r"[\]z", r"[\]z"),
            (r"p\\q", r"p[\]q"),
            (r"\[ab]", "[[]ab]"),
            (r"{a\,b,c}", "{a[,]b,c}"),
            (r"{a\}b,c}", "{a[}]b,c}"),
            (r"q\{x\}\?", "q[{]x[}][?]"),
            (r"\!x\-", "!x-"),
            (r"dir\/f", "dir/f"),
            (r"[!\]]x\*", r"[!\]]x[*]"),
            (r"caf\é", "café"),
            ("plain/*.tmp", "plain/*.tmp"),
        ] {
            assert_eq!(typed_exclude_dialect(typed, false), written, "{typed}");
            let mut rest = written;
            while let Some(c) = rest.chars().next() {
                let len = match c {
                    '[' => bracket_class_len(rest).unwrap(),
                    _ => c.len_utf8(),
                };
                assert_ne!(c, '\\', "{written}: no bare backslash");
                rest = &rest[len..];
            }
            let unix = |pattern: &str| {
                globset::GlobBuilder::new(pattern)
                    .backslash_escape(true)
                    .build()
                    .unwrap()
                    .compile_matcher()
            };
            let (before, after) = (unix(typed), unix(written));
            for name in [
                "ab",
                r"a\b",
                "a/b",
                "x*y",
                "xzy",
                r"\z",
                "z",
                r"p\q",
                "[ab]",
                "a",
                "a,b",
                "b",
                "c",
                "a}b",
                "q{x}?",
                "q{x}x",
                "!x-",
                "dir/f",
                r"\x*",
                "]x*",
                "]x*x",
                "café",
                "plain/a.tmp",
            ] {
                assert_eq!(
                    before.is_match(name),
                    after.is_match(name),
                    "{typed} and {written} on {name:?}"
                );
            }
        }
        for refused in [r"a\", r"a[\*", r"x\*[y"] {
            assert_eq!(typed_exclude_dialect(refused, false), refused, "{refused}");
        }
        assert_eq!(typed_exclude_dialect(r".\a\[\]x", true), r"./a/[\]x");
    }

    /// cr-g3fix5-3: an escaped `/` keeps its Unix meaning in the class
    /// dialect — a literal separator, which takes no part in `**`'s
    /// recursive reading — next to `**`, at the start, at the end, and in
    /// and out of alternations. Before, every `\/` was written `/`, so
    /// `**\/cache` (a component before `cache` required) became
    /// `**/cache`, which also names a top-level `cache`: the filter left out
    /// `cache/needed.txt` too. Each typed and written form is compared as
    /// globset reads it (the Unix reading, forced on every OS) and as the
    /// whole filter reads it — the any-depth and subtree rules plus the
    /// whole-pattern test — under the Unix rule for the typed form and
    /// under both rules for the written one (the wire form a Windows peer
    /// reads). Pure, so it runs (and can fail) on every OS.
    #[test]
    fn an_escaped_separator_keeps_its_meaning_in_the_class_dialect() {
        let unix = |pattern: &str| {
            globset::GlobBuilder::new(pattern)
                .backslash_escape(true)
                .build()
                .unwrap()
                .compile_matcher()
        };
        // The whole filter's reading under one platform rule: the rules,
        // and the pattern against the bare name and the whole path.
        let excludes = |pattern: &str, windows: bool, rel: &str| {
            let glob = globset::GlobBuilder::new(&normalize_separators(pattern, windows))
                .backslash_escape(!windows)
                .build()
                .unwrap()
                .compile_matcher();
            let name = rel.rsplit('/').next().unwrap();
            excluded_under_rule(&[pattern.to_string()], windows, Path::new(rel))
                || glob.is_match(name)
                || glob.is_match(rel)
        };
        let names = [
            "cache",
            "cache/needed.txt",
            "a/cache",
            "a/cache/x",
            "a/b/cache",
            "acache",
            "x",
            "a/x",
            "a/b/x",
            "a/b",
            "a/b/c",
            "b",
            "c",
            "z",
            "a",
            "./x",
            "/x",
            "/cache",
            "[/]x",
            "x/",
            r"x\",
            r"x\/y",
            "*/x",
        ];
        for (typed, written) in [
            (r"**\/cache", "*/cache"),
            (r"a/**\/cache", "a/*/cache"),
            (r"***\/cache", "*/cache"),
            (r"\**\/x", "[*]*/x"),
            (r"{**\/cache,z}", "{*/cache,z}"),
            (r"{z,**\/cache}", "{z,*/cache}"),
            (r"**\/**", "*/**"),
            (r"a\/**", "a/**"),
            (r"a\/b", "a/b"),
            (r"{a\/b,c}", "{a/b,c}"),
            (r"\/x", "/x"),
            (r".\/x", "[.]/x"),
            (r"./.\/x", "./[.]/x"),
            (r"\.\/x", "[.]/x"),
            (r"{.\/x,c}", "{./x,c}"),
            (r"[\/]x", r"[\/]x"),
            (r"x\/", r"x\/"),
            (r"x\//", r"x\//"),
            (r"a\/b\/c", "a/b/c"),
        ] {
            assert_eq!(typed_exclude_dialect(typed, false), written, "{typed}");
            let (before, after) = (unix(typed), unix(written));
            // globset's oracle for the typed form reads the Unix rule only
            // off Windows: its `**` tokenizer (0.4.18 `parse_star`) asks
            // `is_separator` of the raw character after `**`, and on
            // Windows `\` is a separator whatever `backslash_escape` says,
            // so `**\/cache` tokenizes there as a recursive `**` and a
            // separator — not the Unix reading this test is about (CI's
            // Windows leg, run 39076188400). A Windows client never holds a
            // typed Unix escape (its rule writes `\` as `/`), so the string
            // rewrite above is the whole claim there.
            if !cfg!(unix) {
                continue;
            }
            for name in names {
                assert_eq!(
                    before.is_match(name),
                    after.is_match(name),
                    "globset: {typed} and {written} on {name:?}"
                );
                let reading = excludes(typed, false, name);
                assert_eq!(
                    reading,
                    excludes(written, false, name),
                    "the filter, Unix rule: {typed} and {written} on {name:?}"
                );
                // A pattern left as typed keeps its bare `\`, which a
                // Windows peer reads as a separator (a recorded gap).
                if written != typed {
                    assert_eq!(
                        reading,
                        excludes(written, true, name),
                        "the filter, Windows rule: {typed} and {written} on {name:?}"
                    );
                }
            }
        }
        assert!(
            !excludes("*/cache", false, "cache/needed.txt"),
            "a top-level cache is not named"
        );
        assert!(excludes("*/cache", false, "a/cache/x"));
    }

    /// cr-g3fix-3: on Windows a filter built directly — by an embedder, or
    /// from a spec a daemon received — reads `\` as a separator as well.
    #[cfg(windows)]
    #[test]
    fn backslash_forms_match_on_windows() {
        assert!(!allows(&excluding(&[r".\.java"]), ".java/fonts/x"));
        assert!(allows(&excluding(&[r".\.java"]), "a/.java/y"));
        assert!(!allows(&excluding(&[r"a\.java\"]), "a/.java/y"));
        assert!(allows(&excluding(&[r"a\.java\"]), ".java/fonts/x"));
        assert!(!allows(&excluding(&[r".java\"]), "a/.java/y"));
    }

    /// The excluded directory is pruned: the walk never opens it. Made
    /// unreadable, it would otherwise be a suppressed scan error (which
    /// refuses a mirror's deletions) — and an excluded 377k-file tree would
    /// be enumerated for nothing.
    #[cfg(unix)]
    #[test]
    fn an_excluded_directory_is_pruned_never_opened() {
        use std::os::unix::fs::PermissionsExt;
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, PermissionsExt::from_mode(0o755));
            }
        }

        let dir = tree();
        let blocked = dir.path().join(".java");
        std::fs::set_permissions(&blocked, PermissionsExt::from_mode(0o000)).expect("chmod");
        let _restore = Restore(blocked.clone());
        // Privileged runs read through mode 000; the "never opened" proof
        // needs an unprivileged one.
        if std::fs::read_dir(&blocked).is_ok() {
            eprintln!("skipping the never-opened assertion: running privileged");
            return;
        }

        // `walk` fails on any suppressed error, i.e. on any open attempt.
        let walked = walk(dir.path(), excluding(&[".java"]));
        assert!(
            !walked.iter().any(|rel| rel.starts_with(".java")),
            "{walked:?}"
        );
        assert!(walked.contains(&"keep.txt".to_string()));
    }
}
