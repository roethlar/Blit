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
    /// EXCLUDE_CASE_MATCHING (D-2026-11-20-1): read every `include_files`
    /// and `exclude_files` pattern ignoring case. Each pattern is widened
    /// once, where it is compiled ([`crate::case_fold::widen_case`]); a
    /// name is always matched as it is stored, so a wildcard counts the
    /// same bytes in both modes. `files_from` is a list of exact names and
    /// is never read this way.
    pub ignore_case: bool,
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
    /// The compiled patterns, taken at this filter's FIRST match and kept:
    /// the pattern fields are read once. To match other patterns, build
    /// another filter ([`Self::clone_without_cache`], then set them).
    /// Validating a filter does not fill these (cr-ecm3-4).
    compiled_includes: OnceCell<std::sync::Arc<CompiledPatterns>>,
    compiled_files: OnceCell<std::sync::Arc<CompiledPatterns>>,
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
    names: std::sync::Arc<CompiledPatterns>,
    /// Source-relative patterns: each matches a leading run of a relative
    /// path's components — the entry itself or a directory above it.
    paths: std::sync::Arc<CompiledPatterns>,
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
    /// (`windows`), each read ignoring case when `ignore_case`.
    fn compile(patterns: &[String], windows: bool, ignore_case: bool) -> Self {
        let (names, paths) = Self::bodies(patterns, windows);
        let widen = ignore_case.then_some(windows);
        Self {
            names: CompiledPatterns::compile(&names, widen),
            paths: CompiledPatterns::compile(&paths, widen),
        }
    }

    /// [`Self::compile`], or the error when a set cannot be built
    /// (cr-ecm1-2). What it builds is kept ([`compiled`]), so the filter
    /// that is validated this way compiles nothing again.
    fn build(
        patterns: &[String],
        windows: bool,
        ignore_case: bool,
    ) -> std::result::Result<Self, globset::Error> {
        let (names, paths) = Self::bodies(patterns, windows);
        let widen = ignore_case.then_some(windows);
        Ok(Self {
            names: compiled(&names, widen)?,
            paths: compiled(&paths, widen)?,
        })
    }

    /// The name rules and the path rules `patterns` make, as text.
    fn bodies(patterns: &[String], windows: bool) -> (Vec<String>, Vec<String>) {
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
        (names, paths)
    }

    /// True when a rule names `rel_path`, one of its components, or a
    /// directory above it. Components are joined with `/` on every OS.
    fn excludes(&self, rel_path: &Path) -> bool {
        if self.names.is_empty() && self.paths.is_empty() {
            return false;
        }
        let mut prefix: Vec<u8> = Vec::new();
        for component in rel_path.components() {
            let Component::Normal(name) = component else {
                continue;
            };
            let name = match_bytes(name);
            if !self.names.is_empty() && self.names.is_match(&name) {
                return true;
            }
            if !self.paths.is_empty() {
                if !prefix.is_empty() {
                    prefix.push(b'/');
                }
                prefix.extend_from_slice(&name);
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
    ExcludeRules::compile(patterns, windows, false).excludes(rel_path)
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
    /// `widen`: `Some(windows)` to read each pattern ignoring case — it is
    /// widened under that platform's rule before it is compiled
    /// ([`crate::case_fold::widen_case`]); `None` to read it as written. A
    /// pattern the compiler refuses is kept, and compared, as written
    /// either way.
    fn compile(patterns: &[String], widen: Option<bool>) -> std::sync::Arc<Self> {
        compiled(patterns, widen).unwrap_or_else(|_| {
            std::sync::Arc::new(Self {
                globs: globset::GlobSet::empty(),
                refused: patterns.to_vec(),
            })
        })
    }

    /// [`Self::compile`], or the error when the set as a whole cannot be
    /// built. cr-ecm1-2: `compile` answers that failure by comparing EVERY
    /// pattern as text, which names far less than the globs did — so
    /// [`FileFilter::validate_globs`] asks here first and refuses the
    /// filter, and no validated filter reaches that fallback.
    fn build(
        patterns: &[String],
        widen: Option<bool>,
    ) -> std::result::Result<Self, globset::Error> {
        #[cfg(test)]
        COMPILATIONS.lock().push((patterns.to_vec(), widen));
        let mut builder = globset::GlobSetBuilder::new();
        let mut refused = Vec::new();
        for pattern in patterns {
            match globset::Glob::new(&read_pattern(pattern, widen)) {
                Ok(glob) => {
                    builder.add(glob);
                }
                Err(_) => refused.push(pattern.clone()),
            }
        }
        Ok(Self {
            globs: builder.build()?,
            refused,
        })
    }

    fn is_empty(&self) -> bool {
        self.globs.is_empty() && self.refused.is_empty()
    }

    /// Whether a pattern names `name` — its bytes as [`match_bytes`] gives
    /// them. Only a pattern the compiler refused is compared as text.
    fn is_match(&self, name: &[u8]) -> bool {
        self.globs
            .is_match_candidate(&globset::Candidate::from_bytes(name))
            || (!self.refused.is_empty() && {
                let text = String::from_utf8_lossy(name);
                self.refused.iter().any(|p| glob_match(p, &text))
            })
    }
}

/// EXCLUDE_CASE_MATCHING slice 3b (Q5): one line of a list file — the
/// `--files-from` list or a pattern file — as both are read, so the two
/// cannot drift apart: whitespace stripped at both ends (and with it the
/// CR of a CR LF line ending), and `None` for a blank line or one that
/// starts with `#`. Stripping is `str::trim`: every Unicode whitespace
/// character, at the ends only.
pub(crate) fn list_line(line: &str) -> Option<&str> {
    let entry = line.trim();
    (!entry.is_empty() && !entry.starts_with('#')).then_some(entry)
}

/// AC-11 (Q8): for a pattern-file line that is an rsync filter rule — it
/// begins `- ` or `+ `, or is `!` alone — the spelling that names an entry
/// really called that (`[-] name`, `{!}`); `None` for any other line. The
/// check sits above [`list_line`], never in it: the `--files-from` list
/// holds names, and a name that begins `- ` stays a name there.
fn rsync_rule(pattern: &str) -> Option<String> {
    if pattern == "!" {
        return Some("{!}".to_string());
    }
    let sign = pattern.chars().next().filter(|c| matches!(c, '-' | '+'))?;
    pattern[1..]
        .starts_with(' ')
        .then(|| format!("[{sign}]{}", &pattern[1..]))
}

/// cr-ecm2-3: the compiled pattern sets this process holds, oldest first.
/// One set is compiled per distinct list of patterns and case mode, and
/// everything that reads those patterns shares it: the validation a
/// command runs before it asks anything, the filter built at dispatch, and
/// every filter derived from that one (the walk's pruning rules, a
/// mirror's deletion scope). Compiling is the cost of a filter — a set of
/// 250 patterns takes tens of milliseconds — and each of those readers
/// used to pay it again; validating before the question, which cr-ecm1-2
/// and slice 1b added, had roughly tripled a pattern-heavy command.
struct CompiledCache {
    kept: usize,
    sets: Vec<(Vec<String>, Option<bool>, std::sync::Arc<CompiledPatterns>)>,
}

impl CompiledCache {
    const fn new(kept: usize) -> Self {
        Self {
            kept,
            sets: Vec::new(),
        }
    }

    fn get(
        &self,
        patterns: &[String],
        widen: Option<bool>,
    ) -> Option<std::sync::Arc<CompiledPatterns>> {
        self.sets
            .iter()
            .find(|(held, mode, _)| *mode == widen && held.as_slice() == patterns)
            .map(|(_, _, set)| set.clone())
    }

    /// Keep `built` for `patterns`, dropping the oldest set when full —
    /// unless another thread kept one first, which is then the one shared.
    fn keep(
        &mut self,
        patterns: &[String],
        widen: Option<bool>,
        built: std::sync::Arc<CompiledPatterns>,
    ) -> std::sync::Arc<CompiledPatterns> {
        if let Some(held) = self.get(patterns, widen) {
            return held;
        }
        if self.sets.len() >= self.kept {
            self.sets.remove(0);
        }
        self.sets.push((patterns.to_vec(), widen, built.clone()));
        built
    }
}

/// How many sets a process keeps: a command holds a handful (its includes,
/// its excludes as whole-path globs, as names and as subtrees), a daemon
/// that many per distinct filter in use. A test process keeps every set,
/// so that a guard counting compilations cannot be disturbed by the tests
/// running beside it.
const COMPILED_SETS_KEPT: usize = if cfg!(test) { usize::MAX } else { 16 };

static COMPILED: parking_lot::Mutex<CompiledCache> =
    parking_lot::Mutex::new(CompiledCache::new(COMPILED_SETS_KEPT));

/// Every set this test process really compiled — its patterns and case
/// mode — in order ([`CompiledPatterns::build`]).
#[cfg(test)]
static COMPILATIONS: parking_lot::Mutex<Vec<(Vec<String>, Option<bool>)>> =
    parking_lot::Mutex::new(Vec::new());

/// cr-ecm2-3: `patterns` compiled under one case mode — built at most once
/// per process while kept ([`CompiledCache`]). The error when the set
/// cannot be built; a failure is not kept.
fn compiled(
    patterns: &[String],
    widen: Option<bool>,
) -> std::result::Result<std::sync::Arc<CompiledPatterns>, globset::Error> {
    if patterns.is_empty() {
        return Ok(std::sync::Arc::new(CompiledPatterns::default()));
    }
    if let Some(held) = COMPILED.lock().get(patterns, widen) {
        return Ok(held);
    }
    // Compiled outside the lock: it is the slow part.
    let built = std::sync::Arc::new(CompiledPatterns::build(patterns, widen)?);
    Ok(COMPILED.lock().keep(patterns, widen, built))
}

/// cr-ecm2-2: the bytes a pattern is matched against for `name` — a file
/// name, or a relative path. On Unix they are the name's OWN bytes, valid
/// UTF-8 or not: `?` and a class match one byte (Q10), so `?.txt` names a
/// file whose name is the single byte `0xFF` and then `.txt`. Matching
/// used to go through `to_string_lossy`, which writes each bad byte as the
/// three bytes of U+FFFD: that file was named by `???.txt` and not by
/// `?.txt`, and a mirror deleted it from under an exclude that named it.
/// On Windows a name is UTF-16 and has no bytes of its own; it is read as
/// text, ill-formed UTF-16 replaced, exactly as the glob engine itself
/// reads a Windows path.
pub(crate) fn match_bytes(name: &std::ffi::OsStr) -> std::borrow::Cow<'_, [u8]> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        std::borrow::Cow::Borrowed(name.as_bytes())
    }
    #[cfg(not(unix))]
    {
        match name.to_string_lossy() {
            std::borrow::Cow::Borrowed(text) => std::borrow::Cow::Borrowed(text.as_bytes()),
            std::borrow::Cow::Owned(text) => std::borrow::Cow::Owned(text.into_bytes()),
        }
    }
}

/// `pattern` as a filter reads it: widened under one platform's rule when
/// it ignores case (`widen`: `Some(windows)`), as written otherwise.
fn read_pattern(pattern: &str, widen: Option<bool>) -> std::borrow::Cow<'_, str> {
    match widen {
        Some(windows) => std::borrow::Cow::Owned(crate::case_fold::widen_case(pattern, windows)),
        None => std::borrow::Cow::Borrowed(pattern),
    }
}

/// cr-ecm1-2: the error, if any, in compiling `pattern` alone exactly as a
/// filter will use it — parsed AND built into a matcher. Parsing is not
/// enough: a pattern can parse and still be more than the compiler takes
/// (alternations nested too deep), and a widened pattern is one level
/// deeper than the one typed.
fn compile_error(pattern: &str, widen: Option<bool>) -> Option<globset::Error> {
    let glob = match globset::Glob::new(&read_pattern(pattern, widen)) {
        Ok(glob) => glob,
        Err(error) => return Some(error),
    };
    let mut builder = globset::GlobSetBuilder::new();
    builder.add(glob);
    builder.build().err()
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

/// EXCLUDE_CASE_MATCHING AC-10 (Q10, D-2026-11-20-1): the first bracket
/// class of `pattern` that holds a character beyond ASCII — where it
/// starts, and its text (`[aé]`) — or `None`. A class matches ONE BYTE in
/// this filter, as in rsync and git, so such a class can never match the
/// letter it shows: `[aé].txt` does not name `é.txt`. The pattern does
/// less than it reads, and that is known the moment it is read, so it is
/// refused ([`class_beyond_ascii_refusal`]) rather than run. Read under
/// one platform's rule: off Windows a `\` makes the next character
/// literal, a `[` included; on Windows it separates components and
/// escapes nothing. Negated classes and ranges are classes like any other.
pub(crate) fn class_beyond_ascii(pattern: &str, windows: bool) -> Option<(usize, &str)> {
    let mut at = 0;
    while let Some(c) = pattern[at..].chars().next() {
        let rest = &pattern[at..];
        at += match c {
            '\\' if !windows => 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
            '[' => match bracket_class_len(rest) {
                Some(len) if !rest[..len].is_ascii() => return Some((at, &rest[..len])),
                Some(len) => len,
                None => 1,
            },
            c => c.len_utf8(),
        };
    }
    None
}

/// The refusal for `pattern` when a bracket class of it holds a character
/// beyond ASCII ([`class_beyond_ascii`]; `None` when none does): what is
/// wrong, in the user's own pattern, and the spelling that works. EVERY
/// such class is rewritten — a class of plain single characters becomes
/// alternatives (`[aé].txt` is `{a,é}.txt`, which matches each letter
/// whole) — so that the pattern advised is one the next run accepts
/// (cr-ecm2-4: only the first was, and `[aé][bß].txt` was told to write
/// `{a,é}[bß].txt`, which is refused in turn). A range or a negated class
/// has no such spelling; the message then names that class and says what
/// to write instead.
pub(crate) fn class_beyond_ascii_refusal(
    label: &str,
    pattern: &str,
    windows: bool,
) -> Option<String> {
    let (_, first) = class_beyond_ascii(pattern, windows)?;
    // The whole pattern with each such class rewritten, or the first class
    // that has no rewrite.
    let mut rewritten = String::with_capacity(pattern.len() + 8);
    let mut stuck: Option<&str> = None;
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        let len = match c {
            '\\' if !windows => 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
            '[' => bracket_class_len(rest).unwrap_or(1),
            c => c.len_utf8(),
        };
        let piece = &rest[..len];
        let rewrite = c == '[' && len > 1 && !piece.is_ascii();
        if rewrite {
            let inner = &piece[1..len - 1];
            let plain = !inner.starts_with(['!', '^'])
                && !inner.contains(['-', '{', '}', ',', '*', '?', '[', ']', '\\', '/']);
            if !plain {
                stuck = Some(piece);
                break;
            }
            let members: Vec<String> = inner.chars().map(String::from).collect();
            match members.as_slice() {
                [one] => rewritten.push_str(one),
                many => {
                    rewritten.push('{');
                    rewritten.push_str(&many.join(","));
                    rewritten.push('}');
                }
            }
        } else {
            rewritten.push_str(piece);
        }
        rest = &rest[len..];
    }
    let class = stuck.unwrap_or(first);
    let letter = class.chars().find(|c| !c.is_ascii()).unwrap_or('é');
    let advice = match stuck {
        // Advised only when it is itself a pattern the filter accepts.
        None if class_beyond_ascii(&rewritten, windows).is_none()
            && globset::Glob::new(&rewritten).is_ok() =>
        {
            format!("Write {rewritten:?} instead")
        }
        None => format!("List the letters wanted as alternatives instead, like {{a,{letter}}}"),
        Some(_) => format!(
            "A range or a negated class cannot hold such a letter; list the letters wanted \
             as alternatives instead, like {{a,{letter}}}"
        ),
    };
    Some(format!(
        "{label} pattern {pattern:?} cannot work: a class like {class} matches one byte, \
         and {letter} is more than one byte, so the class can never match it. {advice}."
    ))
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
            ignore_case: self.ignore_case,
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
            ignore_case: self.ignore_case,
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
            let Some(entry) = list_line(&line) else {
                continue;
            };
            set.insert(PathBuf::from(entry));
        }
        Ok(set)
    }

    /// EXCLUDE_CASE_MATCHING slice 3b (AC-8, AC-11): the patterns a
    /// pattern file holds, in order — what `--exclude-from FILE` and
    /// `--include-from FILE` read. One pattern per line, each line read by
    /// the rule the `--files-from` list follows ([`list_line`]: whitespace
    /// stripped at both ends, blank lines and `#` comments skipped), and
    /// each pattern then exactly what the same text is on a command line.
    /// A space wanted at an edge is written `[ ]`.
    ///
    /// Refused, naming the file and the line:
    /// - an rsync filter rule (`- pattern`, `+ pattern`, `!`): this file
    ///   holds bare patterns, and read as one such a line would name an
    ///   entry called `- pattern` and protect nothing the user meant;
    /// - text that is not UTF-8, or holds a NUL (a UTF-16 file reads so):
    ///   such a pattern would silently match nothing.
    ///
    /// A UTF-8 byte-order mark at the start of the file is not part of the
    /// first pattern. Used by a front end; a daemon never sees a file,
    /// only patterns.
    pub fn load_pattern_file(path: &Path) -> Result<Vec<String>> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening pattern file {}", path.display()))?;
        let mut patterns = Vec::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let at = format!("pattern file {}, line {}", path.display(), index + 1);
            let line = line.with_context(|| format!("{at}: the file must be UTF-8 text"))?;
            let line = if index == 0 {
                line.strip_prefix('\u{feff}').unwrap_or(&line)
            } else {
                line.as_str()
            };
            let Some(pattern) = list_line(line) else {
                continue;
            };
            if pattern.contains('\0') {
                eyre::bail!(
                    "{at}: the line holds a NUL character; the file must be UTF-8 text \
                     (is it saved as UTF-16?)"
                );
            }
            if let Some(real) = rsync_rule(pattern) {
                eyre::bail!(
                    "{at}: {pattern:?} is an rsync filter rule; a blit pattern file holds one \
                     pattern per line and nothing else. To name an entry really called \
                     {pattern:?}, write {real:?}"
                );
            }
            patterns.push(pattern.to_string());
        }
        Ok(patterns)
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
                // AC-10: a class that can never match the letter it shows.
                if let Some(refusal) = class_beyond_ascii_refusal(label, pat, cfg!(windows)) {
                    return Err(refusal);
                }
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
        // cr-ecm1-2: each pattern is validated as the filter will COMPILE
        // it, not only as it parses — alone, to name it, and then the sets
        // as they are built. A set that cannot be built would otherwise be
        // matched as text (`CompiledPatterns::compile`), silently naming
        // less than the patterns say; under a mirror that is a deletion. A
        // pattern widened for `--ignore-case` nests one level deeper than
        // the one typed, so a pattern can compile as typed and not widened.
        let widen = self.ignore_case.then_some(cfg!(windows));
        let how = if self.ignore_case {
            " with --ignore-case"
        } else {
            ""
        };
        // cr-ecm2-3: the sets are compiled ONCE, here, and kept for the
        // process ([`compiled`]): this filter finds them at its first
        // match, and so does any filter with the same patterns. Only when
        // a set cannot be built is each pattern compiled alone, to name
        // the one at fault.
        //
        // cr-ecm3-4: validation changes nothing IN the filter. It once put
        // the sets it had built into the filter's own cells, and a filter
        // validated, given other patterns and validated again then matched
        // with the first patterns' sets — the cells take a value once.
        let refusal = |label: &str, patterns: &[String], error: globset::Error| match patterns
            .iter()
            .find_map(|pat| Some((pat, compile_error(pat, widen)?)))
        {
            Some((pat, err)) => {
                format!("{label} pattern {pat:?} cannot be compiled{how}: {err}")
            }
            None => {
                format!("the {label} patterns cannot be compiled together{how}: {error}")
            }
        };
        if let Err(error) = compiled(&self.include_files, widen) {
            return Err(refusal("--include", &self.include_files, error));
        }
        if let Err(error) = compiled(&self.exclude_files, widen) {
            return Err(refusal("--exclude", &self.exclude_files, error));
        }
        if let Err(error) =
            ExcludeRules::build(&self.exclude_files, cfg!(windows), self.ignore_case)
        {
            return Err(refusal("--exclude", &self.exclude_files, error));
        }
        Ok(())
    }

    fn include_globs(&self) -> &CompiledPatterns {
        self.compiled_includes.get_or_init(|| {
            CompiledPatterns::compile(
                &self.include_files,
                self.ignore_case.then_some(cfg!(windows)),
            )
        })
    }

    fn file_globs(&self) -> &CompiledPatterns {
        self.compiled_files.get_or_init(|| {
            CompiledPatterns::compile(
                &self.exclude_files,
                self.ignore_case.then_some(cfg!(windows)),
            )
        })
    }

    fn dir_globs(&self) -> &globset::GlobSet {
        self.compiled_dirs
            .get_or_init(|| Self::build_globset(&self.exclude_dirs))
    }

    fn subtree_rules(&self) -> &ExcludeRules {
        self.compiled_subtrees.get_or_init(|| {
            ExcludeRules::compile(&self.exclude_files, cfg!(windows), self.ignore_case)
        })
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
    fn excluded(&self, rel_path: Option<&Path>, filename: &[u8], path: &[u8]) -> bool {
        if !self.exclude_files.is_empty() {
            let patterns = self.file_globs();
            if patterns.is_match(filename) || patterns.is_match(path) {
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

        // cr-ecm2-2: the name's own bytes, never a lossy copy of them.
        let filename = abs_path.file_name().map(match_bytes).unwrap_or_default();
        let path_bytes = match_bytes(rel_path.unwrap_or(abs_path).as_os_str());

        // Whitelist: when include_files is non-empty, file must match one.
        // Match against both the relative path and the bare filename so
        // `--include '*.log'` works without users writing `**/*.log`.
        // cr-ecm1-3: an include is matched as a glob and nothing else, as
        // an exclude has been since cr-g3fix3-2 ([`CompiledPatterns`]; only
        // a pattern the glob compiler refuses is compared as text). The
        // text comparison this replaces also named an entry SPELLED like
        // the pattern (`[a].txt` named a file called `[a].txt`), in its
        // typed case only.
        if !self.include_files.is_empty() {
            let patterns = self.include_globs();
            if !(patterns.is_match(&filename) || patterns.is_match(&path_bytes)) {
                return false;
            }
        }

        // Blacklist
        if self.excluded(rel_path, &filename, &path_bytes) {
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
        if gs.is_match(abs_path) {
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
        let filename = rel_path.file_name().map(match_bytes).unwrap_or_default();
        !self.excluded(
            Some(rel_path),
            &filename,
            &match_bytes(rel_path.as_os_str()),
        )
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
            ignore_case: false,
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
            false,
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

        let refused = ExcludeRules::compile(&["*[x".to_string()], cfg!(windows), false);
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

        let windows = |pattern: &str| ExcludeRules::compile(&[pattern.to_string()], true, false);
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
        let unix = ExcludeRules::compile(&[r"a\.java".to_string()], false, false);
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
            let rules = ExcludeRules::compile(&[r"./back[\]slash".to_string()], windows, false);
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

    // ----- EXCLUDE_CASE_MATCHING slice 1 (D-2026-11-20-1, -4) -----

    fn excluding_any_case(patterns: &[&str]) -> FileFilter {
        FileFilter {
            ignore_case: true,
            ..excluding(patterns)
        }
    }

    /// AC-1 and AC-2 at the filter: with `ignore_case` every kind of
    /// exclude rule names an entry in any case, and without it each names
    /// its own spelling only. One row per way a pattern is read — the
    /// whole-path glob, the any-depth name rule, the subtree rule — chosen
    /// so that only that reading can name the entry: a reading left as
    /// written turns its row red.
    #[test]
    fn ignore_case_reads_every_kind_of_exclude_rule_in_any_case() {
        for (pattern, rel, read_by) in [
            // `*` crosses `/` in the whole-path glob; no single component
            // matches, and no leading run of components does.
            ("A*TMP", "a/b/u.tmp", "the whole-path glob"),
            // The file's own name, by the glob.
            ("*.TMP", "a/t.tmp", "the glob on the name"),
            // A directory above the entry, named alone.
            ("CACHE", "x/cache/deep/saved.txt", "the any-depth name rule"),
            (
                "*ACHE",
                "x/aCache/deep/saved.txt",
                "the any-depth name rule, as a glob",
            ),
            // A leading run of the entry's path.
            ("A/B", "a/b/deep/u.tmp", "the subtree rule"),
            ("./LOGS", "logs/app.log", "the anchored subtree rule"),
            ("X/*/SAVED", "x/y/saved/file", "the subtree rule, as a glob"),
            // Beyond ASCII, and the one-to-one oddities of simple folding.
            ("Übung", "übung/x", "a non-ASCII name"),
            ("σ*", "ς.txt", "the final sigma"),
            ("straße", "STRAẞE/x", "the capital sharp s"),
        ] {
            assert!(
                !allows(&excluding_any_case(&[pattern]), rel),
                "--ignore-case: {pattern} names {rel} through {read_by}"
            );
            assert!(
                allows(&excluding(&[pattern]), rel),
                "as typed: {pattern} does not name {rel}"
            );
        }
        // Ignoring case pairs a letter with itself in another case, never
        // with other letters, and leaves everything else in scope.
        for (pattern, rel) in [
            ("straße", "STRASSE/x"),
            ("ﬁle", "file"),
            ("cache", "cachet/x"),
            ("*.tmp", "a/t.tmpx"),
        ] {
            assert!(
                allows(&excluding_any_case(&[pattern]), rel),
                "--ignore-case: {pattern} does not name {rel}"
            );
        }
    }

    /// D-2026-11-20-4: a wildcard counts the same bytes in both modes,
    /// because the name is never folded — through the filter, on the name
    /// and on a directory above it. `ſ` is two bytes and folds to the
    /// one-byte `s`.
    #[test]
    fn a_wildcard_counts_the_same_bytes_with_and_without_ignore_case() {
        for (pattern, rel, named) in [
            ("??.txt", "ſ.txt", true),
            ("?.txt", "ſ.txt", false),
            ("??", "ſ/saved.txt", true),
            ("?", "ſ/saved.txt", false),
            ("?.txt", "S.txt", true),
        ] {
            for filter in [excluding(&[pattern]), excluding_any_case(&[pattern])] {
                assert_eq!(
                    !allows(&filter, rel),
                    named,
                    "{pattern} and {rel}, ignore_case {}",
                    filter.ignore_case
                );
            }
        }
        assert!(!allows(&excluding_any_case(&["s.txt"]), "ſ.txt"));
        assert!(allows(&excluding(&["s.txt"]), "ſ.txt"));
    }

    /// `--include` follows the same bit; the `--files-from` list never does
    /// (Q2): it is a list of exact names in both modes.
    #[test]
    fn ignore_case_reads_includes_and_never_the_files_from_list() {
        let including = |ignore_case: bool| FileFilter {
            include_files: vec!["*.LOG".to_string(), "Docs/*".to_string()],
            ignore_case,
            ..FileFilter::default()
        };
        for rel in ["logs/app.log", "docs/readme"] {
            assert!(
                allows(&including(true), rel),
                "--ignore-case includes {rel}"
            );
            assert!(
                !allows(&including(false), rel),
                "as typed, {rel} is not included"
            );
        }
        assert!(!allows(&including(true), "keep.txt"));

        let listed = |ignore_case: bool| FileFilter {
            files_from: Some([PathBuf::from("Keep.txt")].into_iter().collect()),
            ignore_case,
            ..FileFilter::default()
        };
        for ignore_case in [false, true] {
            assert!(allows(&listed(ignore_case), "Keep.txt"));
            assert!(
                !allows(&listed(ignore_case), "keep.txt"),
                "a listed name is exact, ignore_case {ignore_case}"
            );
        }
    }

    /// The bit travels with the filter: a walk prunes by it (the pruning
    /// rules a source walks with, and a clone, read the patterns the same
    /// way), so an excluded directory in another case is never opened.
    #[test]
    fn ignore_case_travels_with_the_filter_onto_the_walk() {
        let filter = excluding_any_case(&[".JAVA", "*.TMP"]);
        let rules = filter.pruning_rules().expect("excludes prune");
        assert!(rules.ignore_case);
        assert!(!rules.allows_dir(Path::new("a/.java"), Path::new("a/.java")));
        assert!(!allows(&filter.clone(), "a/.java/y"));
        assert!(!allows(&filter.clone_without_cache(), "a/t.tmp"));

        let dir = tree();
        let walked = walk(dir.path(), filter);
        assert_eq!(
            walked,
            ["a", "a/b", "a/keep.txt", "keep.txt", "logs", "logs/app.log"]
        );
        // As typed, the same patterns name nothing in this tree.
        let typed = walk(dir.path(), excluding(&[".JAVA", "*.TMP"]));
        assert!(typed.contains(&"a/.java/y".to_string()) && typed.contains(&"a/t.tmp".to_string()));
    }

    /// A pattern is validated as it will be read: one that is a glob as
    /// typed is a glob once widened, and a malformed one is refused with
    /// the same pointer in both modes.
    #[test]
    fn validation_reads_a_pattern_as_the_filter_will() {
        let mut filter = excluding_any_case(&["*cache", "[a-c]x", "{s,k}/**", r"a\*b", "ſ?"]);
        filter.include_files = vec!["*.LOG".to_string()];
        filter.validate_globs().expect("valid in any case");
        for ignore_case in [false, true] {
            let mut bad = excluding(&["a["]);
            bad.ignore_case = ignore_case;
            let error = bad.validate_globs().expect_err("an unclosed class");
            assert!(error.contains("--exclude pattern \"a[\""), "{error}");
        }
    }

    /// A pattern nested as deep as the glob compiler takes as typed: one
    /// letter inside `depth` alternations. Widened for `--ignore-case`, the
    /// letter becomes one alternation more.
    fn nested(depth: usize) -> String {
        format!("{}s{}", "{".repeat(depth), "}".repeat(depth))
    }

    fn deepest_that_compiles_as_typed() -> usize {
        let deepest = (1..2000)
            .take_while(|&depth| excluding(&[&nested(depth)]).validate_globs().is_ok())
            .last()
            .expect("one level compiles");
        assert!(
            deepest < 1999,
            "the compiler has a nesting limit to test against"
        );
        deepest
    }

    /// cr-ecm1-2: a pattern is refused unless it compiles as the filter
    /// will use it. This one parses in both modes and compiles as typed,
    /// but widened it is one level deeper than the compiler takes. Left to
    /// run, the whole exclude set could not be built and every pattern in
    /// it — `*.cfg` too — was compared as text exactly as written: with
    /// `--ignore-case`, `SAVED.CFG` and `s` both fell out of the excludes,
    /// and a mirror deleted them.
    #[test]
    fn a_pattern_that_cannot_be_compiled_as_it_will_be_read_is_refused() {
        let deep = nested(deepest_that_compiles_as_typed());
        let typed = excluding(&["*.cfg", &deep]);
        typed.validate_globs().expect("as typed, it compiles");
        assert!(!allows(&typed, "s") && !allows(&typed, "saved.cfg"));

        let widened = excluding_any_case(&["*.cfg", &deep]);
        let error = widened
            .validate_globs()
            .expect_err("widened, it is past what the compiler takes");
        assert!(
            error.contains("--exclude pattern") && error.contains("with --ignore-case"),
            "{error}"
        );
        assert!(
            !error.contains("*.cfg"),
            "the pattern named is the one at fault: {error}"
        );

        // The same holds for an include, and for a pattern past the limit
        // as typed (no mode named in that refusal).
        let included = FileFilter {
            include_files: vec![deep.clone()],
            ignore_case: true,
            ..FileFilter::default()
        };
        let error = included
            .validate_globs()
            .expect_err("an include is compiled too");
        assert!(error.contains("--include pattern"), "{error}");
        let error = excluding(&[&nested(deepest_that_compiles_as_typed() + 1)])
            .validate_globs()
            .expect_err("too deep as typed");
        assert!(
            error.contains("cannot be compiled:") && !error.contains("--ignore-case"),
            "{error}"
        );
    }

    /// cr-ecm1-3: an include is a glob and nothing else. `[a].txt` is the
    /// class `a` followed by `.txt`: it names `a.txt`, and never a file
    /// that is merely SPELLED like the pattern. The text comparison it used
    /// to fall through to named `[a].txt` — in the typed case only, so
    /// `--ignore-case` included `[a].txt` and not `[A].TXT`. The literal
    /// name is written with its brackets escaped, and follows the mode.
    #[test]
    fn an_include_is_a_glob_and_never_its_own_spelling() {
        let including = |pattern: &str, ignore_case: bool| FileFilter {
            include_files: vec![pattern.to_string()],
            ignore_case,
            ..FileFilter::default()
        };
        for ignore_case in [false, true] {
            let class = including("[a].txt", ignore_case);
            assert!(allows(&class, "a.txt"));
            assert_eq!(allows(&class, "A.TXT"), ignore_case);
            for spelled in ["[a].txt", "[A].TXT", "sub/[a].txt"] {
                assert!(
                    !allows(&class, spelled),
                    "{spelled} is not named by the class, ignore_case {ignore_case}"
                );
            }
            let literal = including("[[]a[]].txt", ignore_case);
            assert!(allows(&literal, "[a].txt"));
            assert_eq!(allows(&literal, "[A].TXT"), ignore_case);
            assert!(!allows(&literal, "a.txt"));
        }
        // Today's working forms are untouched.
        for (pattern, rel) in [
            ("*", "x"),
            ("*.log", "logs/app.log"),
            ("app*", "app.log"),
            ("*pp*", "logs/app.log"),
            ("keep.txt", "a/keep.txt"),
        ] {
            assert!(
                allows(&including(pattern, false), rel),
                "{pattern} includes {rel}"
            );
        }
    }

    /// AC-10 (slice 1b): a class holding a character beyond ASCII is
    /// refused where a filter is validated — for an exclude and an include,
    /// in either case mode — naming the pattern, the class, and the
    /// spelling that works. A class matches one byte, so `[aé].txt` never
    /// named `é.txt`: the pattern did less than it read.
    #[test]
    fn a_class_holding_a_letter_beyond_ascii_is_refused() {
        for (pattern, class, advice) in [
            ("[aé].txt", "[aé]", r#"Write "{a,é}.txt" instead."#),
            ("x[é]", "[é]", r#"Write "xé" instead."#),
            ("*/[€a]b/**", "[€a]", r#"Write "*/{€,a}b/**" instead."#),
            ("[ab]*[cü]", "[cü]", r#"Write "[ab]*{c,ü}" instead."#),
            ("é[é]é", "[é]", r#"Write "ééé" instead."#),
            // cr-ecm2-4: every such class is rewritten, not the first only.
            ("[aé][bß].txt", "[aé]", r#"Write "{a,é}{b,ß}.txt" instead."#),
            ("[é]x[ü]y[ab][ñ]", "[é]", r#"Write "éxüy[ab]ñ" instead."#),
            // One that has no rewrite is the one named.
            ("[aé][!ß]", "[!ß]", "like {a,ß}."),
            ("[a-é]x[bü]", "[a-é]", "like {a,é}."),
            ("[!é]x", "[!é]", "like {a,é}."),
            ("[^é]x", "[^é]", "like {a,é}."),
            ("[a-é]", "[a-é]", "like {a,é}."),
            ("[]ü]", "[]ü]", "like {a,ü}."),
        ] {
            for ignore_case in [false, true] {
                for (label, include) in [("--exclude", false), ("--include", true)] {
                    let patterns = vec![pattern.to_string()];
                    let filter = FileFilter {
                        include_files: if include {
                            patterns.clone()
                        } else {
                            Vec::new()
                        },
                        exclude_files: if include { Vec::new() } else { patterns },
                        ignore_case,
                        ..FileFilter::default()
                    };
                    let error = filter
                        .validate_globs()
                        .expect_err("a class beyond ASCII is refused");
                    assert!(
                        error.starts_with(&format!("{label} pattern {pattern:?} cannot work:")),
                        "{error}"
                    );
                    assert!(
                        error.contains(&format!("a class like {class} matches one byte")),
                        "{error}"
                    );
                    assert!(error.ends_with(advice), "{pattern}: {error}");
                }
            }
            // cr-ecm2-4: a spelling that is advised is one the filter takes.
            if let Some(advised) = advice
                .strip_prefix("Write \"")
                .and_then(|rest| rest.strip_suffix("\" instead."))
            {
                for filter in [excluding(&[advised]), excluding_any_case(&[advised])] {
                    filter
                        .validate_globs()
                        .unwrap_or_else(|error| panic!("{pattern} advised {advised}: {error}"));
                }
            }
        }
    }

    /// AC-10: what the refusal leaves alone — alternatives, a bare letter,
    /// every ASCII class — is accepted in both modes and matches as it
    /// always has; and the spelling the refusal advises does what the
    /// refused class read as.
    #[test]
    fn letters_beyond_ascii_outside_a_class_are_accepted_as_before() {
        for pattern in [
            "{a,é}.txt",
            "é.txt",
            "é",
            "*é*",
            "[ab].txt",
            "[!a-c]x",
            "[]a]",
            "caf[e]",
            "[[]é[]]",
            "é[ab]ü",
        ] {
            excluding(&[pattern])
                .validate_globs()
                .unwrap_or_else(|error| panic!("{pattern} as typed: {error}"));
            excluding_any_case(&[pattern])
                .validate_globs()
                .unwrap_or_else(|error| panic!("{pattern} in any case: {error}"));
        }
        let advised = excluding(&["{a,é}.txt"]);
        assert!(!allows(&advised, "a.txt") && !allows(&advised, "é.txt"));
        assert!(allows(&advised, "b.txt") && allows(&advised, "É.txt"));
        let any_case = excluding_any_case(&["{a,é}.txt"]);
        assert!(!allows(&any_case, "A.txt") && !allows(&any_case, "É.txt"));
        // The class `[[]é[]]` is three ASCII classes around a bare letter:
        // it names the file spelled `[é]`.
        assert!(!allows(&excluding(&["[[]é[]]"]), "[é]"));
    }

    /// AC-10: the check reads a pattern under ONE platform's rule, like the
    /// rest of the filter. Off Windows `\[` is a literal bracket and opens
    /// no class; on Windows `\` separates and the bracket opens one. An
    /// unclosed bracket is no class (the glob compiler refuses it), a `]`
    /// in first place is a member, and the place reported is in bytes.
    #[test]
    fn the_class_check_reads_a_backslash_by_one_platforms_rule() {
        assert_eq!(class_beyond_ascii(r"\[é]", false), None);
        assert_eq!(class_beyond_ascii(r"\[é]", true), Some((1, "[é]")));
        assert_eq!(class_beyond_ascii(r"caf\é", false), None);
        assert_eq!(class_beyond_ascii(r"caf\é", true), None);
        assert_eq!(class_beyond_ascii(r"\\[é]", false), Some((2, "[é]")));
        assert_eq!(class_beyond_ascii("[é", false), None);
        assert_eq!(class_beyond_ascii("a[b]é[c]", false), None);
        assert_eq!(class_beyond_ascii("[]é]", false), Some((0, "[]é]")));
        assert_eq!(class_beyond_ascii("é[é]", false), Some((2, "[é]")));
        assert_eq!(class_beyond_ascii("[a][bé]", true), Some((3, "[bé]")));
        assert_eq!(class_beyond_ascii(r"a\", false), None);

        // The advice rewrites the class where it is, not the first text
        // spelled like it: under the Unix rule the first `[é]` here is an
        // escaped bracket and two ordinary characters.
        let pattern = r"\[é][é]";
        let (at, class) = class_beyond_ascii(pattern, false).expect("the second is a class");
        assert_eq!((at, class), (5, "[é]"));
        let refusal =
            class_beyond_ascii_refusal("--exclude", pattern, false).expect("a class beyond ASCII");
        assert!(
            refusal.ends_with(&format!("Write {:?} instead.", r"\[é]é")),
            "{refusal}"
        );
    }

    /// AC-10 (plan, Design §Class refusal): the escape dialect and the
    /// class check cannot disagree. The dialect writes an escaped syntax
    /// character as a one-character class; an escaped letter beyond ASCII
    /// it writes BARE, never as a class, so a pattern the check accepts as
    /// typed is accepted as written for a peer, under either rule — and a
    /// refused class is copied as it is, so it stays refused.
    #[test]
    fn the_escape_dialect_never_writes_a_class_beyond_ascii() {
        for typed in [
            r"caf\é",
            r"\é",
            r"a\€b\*",
            r"\[é\]",
            r"x\é[ab]",
            r"\ü\?",
            r"dir\/é\{\}",
        ] {
            assert_eq!(class_beyond_ascii(typed, false), None, "{typed}");
            let written = typed_exclude_dialect(typed, false);
            assert_ne!(written, typed, "{typed} is rewritten for a peer");
            for windows in [false, true] {
                assert_eq!(
                    class_beyond_ascii(&written, windows),
                    None,
                    "{typed} written {written}"
                );
            }
        }
        for typed in ["[aé]", r"x\*[é]", "[!é]", r"\é[é]"] {
            assert!(class_beyond_ascii(typed, false).is_some(), "{typed}");
            let written = typed_exclude_dialect(typed, false);
            for windows in [false, true] {
                assert!(
                    class_beyond_ascii(&written, windows).is_some(),
                    "{typed} written {written}"
                );
            }
        }
    }

    /// AC-10: widening a pattern for `--ignore-case` never writes a class
    /// beyond ASCII either. A bare letter's partners beyond ASCII (`ſ` for
    /// `s`, the Kelvin sign for `k`) are written as alternatives, never as
    /// class members, where they could not match; a class gains ASCII
    /// partners only (cr-ecm2-1).
    #[test]
    fn widening_never_writes_a_class_beyond_ascii() {
        for pattern in [
            "[ks]", "[!ks]", "[a-z]x", "[K]elvin", "k[s]", "*[sS]*", "[!a-z]", "[]k]", "s", "ſ",
        ] {
            for windows in [false, true] {
                let widened = crate::case_fold::widen_case(pattern, windows);
                assert_eq!(
                    class_beyond_ascii(&widened, windows),
                    None,
                    "{pattern} widened {widened}"
                );
            }
        }
    }

    /// cr-ecm2-2: a name that is not valid UTF-8 is matched by its OWN
    /// bytes. `?` and a class match one byte (Q10), so the file whose name
    /// is the byte `0xFF` and then `.txt` is named by `?.txt` — by the
    /// whole-path glob, the any-depth name rule and the path rule alike,
    /// as an exclude and as an include, for a file and for a directory to
    /// prune, in either case mode. Matching once went through a lossy copy
    /// that writes a bad byte as three: `???.txt` named that file and
    /// `?.txt` did not. Unix only: a Windows name has no bytes of its own.
    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_utf8_is_matched_by_its_own_bytes() {
        use std::os::unix::ffi::OsStrExt as _;
        let path = |bytes: &[u8]| PathBuf::from(std::ffi::OsStr::from_bytes(bytes));
        let named = |filter: &FileFilter, rel: &[u8]| {
            let rel = path(rel);
            !filter.allows_entry(Some(&rel), &Path::new("/src").join(&rel), 1, None)
        };
        let accent = "é.txt".as_bytes();
        for (pattern, rel, names) in [
            ("?.txt", &b"\xff.txt"[..], true),
            ("???.txt", b"\xff.txt", false),
            ("[!a].txt", b"\xff.txt", true),
            ("*.txt", b"\xff.txt", true),
            ("??.txt", b"\xff\xfe.txt", true),
            // The name rule at any depth, and the path rule.
            ("?.txt", b"dir/\xff.txt", true),
            ("dir/?.txt", b"dir/\xff.txt", true),
            ("dir/???.txt", b"dir/\xff.txt", false),
            ("?/a.txt", b"\xff/a.txt", true),
            ("???/a.txt", b"\xff/a.txt", false),
            // A directory only the name rule or the subtree rule can name.
            ("?", b"\xff/a.txt", true),
            ("???", b"\xff/a.txt", false),
            ("dir/?", b"dir/\xff/a.txt", true),
            ("dir/???", b"dir/\xff/a.txt", false),
            // A name that IS valid text reads as it always has: `é` is two.
            ("??.txt", accent, true),
            ("?.txt", accent, false),
        ] {
            assert_eq!(
                named(&excluding(&[pattern]), rel),
                names,
                "{pattern} against {}",
                String::from_utf8_lossy(rel)
            );
        }
        // The reviewer's case: ignoring case, `?.txt` names both.
        let any_case = excluding_any_case(&["?.txt"]);
        assert!(named(&any_case, b"\xff.TXT") && named(&any_case, b"X.TXT"));
        // An include reads the same bytes.
        let including = |pattern: &str| FileFilter {
            include_files: vec![pattern.to_string()],
            ..FileFilter::default()
        };
        assert!(!named(&including("?.txt"), b"\xff.txt"));
        assert!(named(&including("???.txt"), b"\xff.txt"));
        // And so does the pruning of a directory.
        let dir = path(b"\xff");
        let abs = Path::new("/src").join(&dir);
        assert!(!excluding(&["?"]).should_include_dir(&dir, &abs));
        assert!(excluding(&["???"]).should_include_dir(&dir, &abs));
    }

    /// cr-ecm2-3: a list of patterns is compiled ONCE per case mode, and
    /// everything that reads it shares the result — the validation, the
    /// filter's own use, a clone, the walk's pruning rules, and a second
    /// filter built from the same patterns (which is what a command does:
    /// it validates before it asks anything and builds at dispatch).
    /// Compiling is the cost of a filter; paying it per reader had roughly
    /// tripled a command with many patterns. Counted in real compilations
    /// of this test's own patterns, which no other test uses.
    #[test]
    fn a_pattern_set_is_compiled_once_whoever_reads_it() {
        let include = vec!["ecm23-marker-*.txt".to_string()];
        let exclude = vec![
            "ecm23-marker-name".to_string(),
            "ecm23-marker/dir/*.o".to_string(),
        ];
        let mine = || -> Vec<(Vec<String>, Option<bool>)> {
            COMPILATIONS
                .lock()
                .iter()
                .filter(|(patterns, _)| patterns.iter().any(|p| p.contains("ecm23-marker")))
                .cloned()
                .collect()
        };
        let read = |filter: &FileFilter| {
            let rel = Path::new("ecm23-marker/dir/a.o");
            assert!(!filter.allows_entry(Some(rel), &Path::new("/src").join(rel), 1, None));
            assert!(!filter.allows_dir(
                Path::new("ecm23-marker-name"),
                Path::new("/src/ecm23-marker-name")
            ));
        };
        for ignore_case in [false, true] {
            let widen = ignore_case.then_some(cfg!(windows));
            let build = || FileFilter {
                include_files: include.clone(),
                exclude_files: exclude.clone(),
                ignore_case,
                ..FileFilter::default()
            };
            // The command's early validation, on a filter it throws away...
            build().validate_globs().unwrap();
            // ...the filter built at dispatch, validated and used...
            let filter = build();
            filter.validate_globs().unwrap();
            read(&filter);
            // ...and the filters derived from it.
            read(&filter.clone_without_cache());
            read(&filter.pruning_rules().expect("excludes to prune with"));
            // A filter that was never validated reads the same sets.
            read(&build());

            let compiled: Vec<Vec<String>> = mine()
                .into_iter()
                .filter(|(_, mode)| *mode == widen)
                .map(|(patterns, _)| patterns)
                .collect();
            let mut expected = vec![
                include.clone(),
                exclude.clone(),
                vec!["ecm23-marker-name".to_string()],
                vec!["ecm23-marker/dir/*.o".to_string()],
            ];
            let mut seen = compiled.clone();
            seen.sort();
            expected.sort();
            assert_eq!(
                seen, expected,
                "ignore_case {ignore_case}: the includes, the excludes as whole paths, as \
                 names and as subtrees — each compiled once"
            );
        }
    }

    /// cr-ecm2-3: the cache itself — a hit is the very set that was kept,
    /// the case mode is part of the key, two builders racing share the
    /// first one kept, and when full the OLDEST set makes room.
    #[test]
    fn the_compiled_cache_shares_keys_and_drops_the_oldest() {
        let list = |name: &str| vec![name.to_string()];
        let set = || std::sync::Arc::new(CompiledPatterns::default());
        let mut cache = CompiledCache::new(2);
        assert!(cache.get(&list("a"), None).is_none());
        let a = cache.keep(&list("a"), None, set());
        assert!(std::sync::Arc::ptr_eq(
            &cache.get(&list("a"), None).unwrap(),
            &a
        ));
        // Another case mode is another set, whichever was kept first.
        assert!(cache.get(&list("a"), Some(false)).is_none());
        assert!(cache.get(&list("a"), Some(true)).is_none());
        let mut by_mode = CompiledCache::new(4);
        let widened = by_mode.keep(&list("a"), Some(false), set());
        assert!(by_mode.get(&list("a"), None).is_none());
        assert!(by_mode.get(&list("a"), Some(true)).is_none());
        let typed = by_mode.keep(&list("a"), None, set());
        assert!(!std::sync::Arc::ptr_eq(&typed, &widened));
        assert_eq!(by_mode.sets.len(), 2);
        // A second builder of the same key is handed the first set.
        let raced = cache.keep(&list("a"), None, set());
        assert!(std::sync::Arc::ptr_eq(&raced, &a));
        assert_eq!(cache.sets.len(), 1);
        // Full: the oldest goes, the newest two stay.
        cache.keep(&list("b"), None, set());
        cache.keep(&list("c"), None, set());
        assert!(cache.get(&list("a"), None).is_none());
        assert!(cache.get(&list("b"), None).is_some() && cache.get(&list("c"), None).is_some());
        assert_eq!(cache.sets.len(), 2);
    }

    /// Write `bytes` as a list file in a fresh directory.
    fn list_file(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("list.txt");
        std::fs::write(&path, bytes).unwrap();
        (dir, path)
    }

    /// AC-8 (slice 3b): a pattern file holds one pattern per line. Both
    /// ends of a line are stripped — spaces, tabs, a CR before the LF —
    /// blank lines and `#` comments are skipped, and what is left is the
    /// pattern exactly as it would be typed. A space wanted at an edge is
    /// written `[ ]`. A byte-order mark is not part of the first pattern.
    #[test]
    fn a_pattern_file_holds_one_pattern_per_line() {
        let text = "\u{feff}*.tmp\r\n# a comment\r\n\r\n   *.bak   \n\t*.jpg \n*.png[ ]\n  # indented comment\n[ ]lead*\nno newline at the end";
        let (_dir, path) = list_file(text.as_bytes());
        assert_eq!(
            FileFilter::load_pattern_file(&path).unwrap(),
            [
                "*.tmp",
                "*.bak",
                "*.jpg",
                "*.png[ ]",
                "[ ]lead*",
                "no newline at the end"
            ]
        );
        // What a stripped line and a class-written space each name.
        let trailing = excluding(&["*.jpg"]);
        assert!(!allows(&trailing, "a.jpg") && allows(&trailing, "a.jpg "));
        let kept = excluding(&["*.png[ ]"]);
        assert!(!allows(&kept, "a.png ") && allows(&kept, "a.png"));
        // An empty file, and one of comments alone, hold no pattern.
        for empty in ["", "\n\n", "# only\n   # comments\n"] {
            let (_dir, path) = list_file(empty.as_bytes());
            assert!(FileFilter::load_pattern_file(&path).unwrap().is_empty());
        }
    }

    /// AC-11 (Q8): an rsync filter rule in a pattern file refuses the run,
    /// naming the file, the line and the spelling for an entry really
    /// called that. Read as a pattern such a line would name an entry
    /// called `- *.jpg` and protect nothing the user meant. A line that
    /// only looks like one is a pattern like any other.
    #[test]
    fn an_rsync_rule_in_a_pattern_file_is_refused_with_its_line() {
        for (text, line, rule, real) in [
            ("*.tmp\n- *.jpg\n", 2, "- *.jpg", "[-] *.jpg"),
            ("+ *.jpg\n", 1, "+ *.jpg", "[+] *.jpg"),
            (
                "# c\n\n   - name with spaces  \n",
                3,
                "- name with spaces",
                "[-] name with spaces",
            ),
            ("a\nb\n!\n", 3, "!", "{!}"),
            ("  !  \r\n", 1, "!", "{!}"),
        ] {
            let (_dir, path) = list_file(text.as_bytes());
            let error = format!("{:#}", FileFilter::load_pattern_file(&path).unwrap_err());
            assert!(
                error.contains(&format!("pattern file {}, line {line}: ", path.display())),
                "{error}"
            );
            assert!(
                error.contains(&format!("{rule:?} is an rsync filter rule"))
                    && error.contains(&format!("write {real:?}")),
                "{error}"
            );
            // The spelling advised is a pattern, and names that entry.
            let advised = excluding(&[real]);
            advised.validate_globs().expect("the advised spelling");
            assert!(!allows(&advised, rule), "{real} names {rule}");
        }
        let (_dir, path) = list_file(b"[-] name\n-x\n+x\n-\n+\n!x\n!!\na - b\n{!}\n");
        assert_eq!(
            FileFilter::load_pattern_file(&path).unwrap(),
            ["[-] name", "-x", "+x", "-", "+", "!x", "!!", "a - b", "{!}"]
        );
    }

    /// Q5: the `--files-from` list and a pattern file read a LINE by one
    /// rule, so the two cannot drift apart — and the list is not a pattern
    /// file: a name that begins `- ` stays a name there.
    #[test]
    fn the_files_from_list_and_a_pattern_file_read_a_line_alike() {
        let text = "  a.txt  \r\n\n# comment\n\tdir/b.txt\t\n   # indented\nc d.txt \n";
        let (_dir, path) = list_file(text.as_bytes());
        let patterns = FileFilter::load_pattern_file(&path).unwrap();
        let listed = FileFilter::load_files_from(&path).unwrap();
        assert_eq!(patterns, ["a.txt", "dir/b.txt", "c d.txt"]);
        assert_eq!(
            listed,
            patterns.iter().map(PathBuf::from).collect::<HashSet<_>>()
        );
        for line in [
            "x",
            "  x  ",
            "",
            "   ",
            "#x",
            "  # x",
            "x # y",
            "\u{a0}x\u{a0}",
        ] {
            let expected = match line.trim() {
                "" => None,
                entry if entry.starts_with('#') => None,
                entry => Some(entry),
            };
            assert_eq!(list_line(line), expected, "{line:?}");
        }
        assert_eq!(list_line("\u{a0}x\u{a0}"), Some("x"), "every Unicode space");

        let (_dir, path) = list_file(b"- name\n+ other\n!\n");
        assert!(FileFilter::load_pattern_file(&path).is_err());
        assert_eq!(
            FileFilter::load_files_from(&path).unwrap(),
            ["- name", "+ other", "!"]
                .into_iter()
                .map(PathBuf::from)
                .collect::<HashSet<_>>()
        );
    }

    /// A pattern file that cannot be read as patterns refuses the run and
    /// says which file: one that is missing, one that is not UTF-8, one
    /// holding a NUL (what a UTF-16 file of plain letters reads as). Each
    /// would otherwise be a pattern that silently matches nothing.
    #[test]
    fn a_pattern_file_that_is_not_utf8_text_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-file.txt");
        let error = format!("{:#}", FileFilter::load_pattern_file(&missing).unwrap_err());
        assert!(
            error.contains(&format!("opening pattern file {}", missing.display())),
            "{error}"
        );

        let (_dir, path) = list_file(b"*.tmp\n\xff\xfe*.jpg\n");
        let error = format!("{:#}", FileFilter::load_pattern_file(&path).unwrap_err());
        assert!(
            error.contains(&format!("pattern file {}, line 2", path.display()))
                && error.contains("must be UTF-8 text"),
            "{error}"
        );

        // `*.jpg` as UTF-16LE without a byte-order mark is valid UTF-8.
        let utf16: Vec<u8> = "*.jpg\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let (_dir, path) = list_file(&utf16);
        let error = format!("{:#}", FileFilter::load_pattern_file(&path).unwrap_err());
        assert!(
            error.contains("line 1") && error.contains("NUL") && error.contains("UTF-16"),
            "{error}"
        );
    }

    /// cr-ecm3-4: validating a filter changes nothing in it. A filter that
    /// is validated, given other patterns (or another case mode) and
    /// validated again matches by what it holds NOW. Validation once kept
    /// the sets it had compiled in the filter, which takes them only once:
    /// the second validation passed and the filter went on matching `old`,
    /// letting `a.cfg` through.
    #[test]
    fn a_filter_validated_again_matches_the_patterns_it_now_holds() {
        let mut filter = FileFilter {
            include_files: vec!["*.old".to_string()],
            exclude_files: vec!["old".to_string(), "dir/old".to_string()],
            ..FileFilter::default()
        };
        filter.validate_globs().unwrap();
        filter.include_files = vec!["*.cfg".to_string(), "*.txt".to_string()];
        filter.exclude_files = vec!["a.cfg".to_string(), "dir/b.cfg".to_string()];
        filter.validate_globs().unwrap();
        assert!(
            !allows(&filter, "a.cfg"),
            "the whole-path glob and the name rule"
        );
        assert!(!allows(&filter, "dir/b.cfg"), "the subtree rule");
        assert!(allows(&filter, "c.cfg") && allows(&filter, "dir/c.txt"));
        assert!(
            !allows(&filter, "x.old"),
            "the includes are the new ones too"
        );

        // The case mode is part of what a filter holds.
        let mut filter = excluding(&["*cache"]);
        filter.validate_globs().unwrap();
        filter.ignore_case = true;
        filter.validate_globs().unwrap();
        assert!(!allows(&filter, "aCache"));
    }
}
