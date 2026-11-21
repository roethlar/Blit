//! `--ignore-case` for the transfer filters (EXCLUDE_CASE_MATCHING,
//! D-2026-11-20-1 and D-2026-11-20-4).
//!
//! Ignoring case is done to the PATTERN, never to a name: each literal
//! letter of a glob is widened to the letters that share its Unicode simple
//! case fold (`s` to `s`, `S`, `ſ`; `σ` to `σ`, `Σ`, `ς`), an ASCII class
//! gains its letters' other case, and the widened glob is then compiled and
//! matched in byte mode against the name exactly as it is stored. So `?`,
//! `*`, `**` and classes see the same bytes with the option and without it.
//! Folding the name instead would change what `?` counts: `ſ` is two bytes
//! and its fold `s` is one, so `??.txt` would name `ſ.txt` in one mode and
//! `?.txt` in the other.
//!
//! "Simple" folding pairs one letter with one letter: never `ß` with `SS`
//! or `ﬁ` with `fi`. The table is the regex engine's own (`regex-syntax`).

use regex_syntax::hir::{ClassUnicode, ClassUnicodeRange};

use crate::fs_enum::bracket_class_len;

/// Every character that shares `c`'s Unicode simple case fold, `c`
/// included, in code-point order: `['K', 'k', 'K']` (the last the Kelvin
/// sign) for `k`, `[c]` alone for a character with no other case.
pub(crate) fn case_variants(c: char) -> Vec<char> {
    // No ASCII character but a letter has another case; skip the table.
    if c.is_ascii() && !c.is_ascii_alphabetic() {
        return vec![c];
    }
    let mut class = ClassUnicode::new([ClassUnicodeRange::new(c, c)]);
    if class.try_case_fold_simple().is_err() {
        // The fold table is compiled out. `Cargo.toml` asks for it
        // (`unicode-case`) and a guard below pins a non-ASCII pair, so
        // this is unreachable in a build that passes its tests.
        return vec![c];
    }
    let mut variants = Vec::new();
    for range in class.iter() {
        variants.extend(range.start()..=range.end());
    }
    variants
}

/// EXCLUDE_CASE_MATCHING slice 2 (AC-3, AC-5): the one line that tells an
/// operator their patterns are read exactly as typed, built from the first
/// pattern in which a letter's case matters:
///
/// `"*Cache" matches Cache only; capital letters matter (--ignore-case would also match cache, CACHE).`
///
/// The pattern is echoed as typed. What follows is its text without the
/// wildcards at its ends, then that text all lower and all upper —
/// whichever differs from the typed one — each letter changed for the
/// letter `--ignore-case` pairs it with and a class for the class of its
/// letters' other case, so every spelling shown is one the option really
/// matches (`straße` is shown `STRAẞE`, never `STRASSE`).
///
/// cr-ecm3-5: which patterns are noted is the MATCHER's answer — a
/// pattern is noted exactly when ignoring case would change what it
/// names ([`Spellings::of`]). `None` when that is true of no pattern
/// (`??`, `*.1`, `[Aa]x`). Further patterns of which it is true are
/// counted, never listed, and the line is one line whatever the patterns
/// hold ([`one_line`]).
///
/// Slice 3b (AC-8): each pattern comes with the pattern file it was read
/// from, if any, and the line says so — `"*Cache" (from skip.txt) matches
/// …` for the pattern shown, `and 2 more patterns (from skip.txt)` for the
/// ones counted: the operator did not type those on the command line.
///
/// cr-ecm4-3: this is [`case_note_of`] for patterns that are read as they
/// are typed (a relative pattern, an include) — the form this module's
/// guards drive it in.
#[cfg(test)]
pub(crate) fn case_note(patterns: &[(&str, Option<&str>)], windows: bool) -> Option<String> {
    let noted: Vec<Noted> = patterns
        .iter()
        .map(|&(pattern, file)| Noted {
            typed: pattern,
            read_as: pattern,
            file,
            role: Role::Exclude,
        })
        .collect();
    case_note_of(&noted, windows)
}

/// What a pattern is in the filter: it leaves entries out, or lets them in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Exclude,
    Include,
}

/// One pattern for [`case_note_of`]: as the operator typed it, as the
/// filter reads it, and the pattern file it came from, if any.
pub(crate) struct Noted<'a> {
    /// Echoed in quotes, so the operator knows which pattern is meant.
    pub typed: &'a str,
    /// What the matcher is given, and so what the line describes: the
    /// same text for a relative pattern; for an absolute `--exclude`, the
    /// pattern below the source it became (`Data` for `/srv/Data` under
    /// `/srv`).
    pub read_as: &'a str,
    pub file: Option<&'a str>,
    /// How the filter uses it — what each claim about it is checked as.
    pub role: Role,
}

/// cr-ecm4-3: [`case_note`] built from the EFFECTIVE patterns. What the
/// line says a pattern matches, and what `--ignore-case` would do to it,
/// is said of the pattern the matcher is given ([`Noted::read_as`]) —
/// the source's own path is no part of that, and its letters are never
/// shown in another case (`/SRV/DATA` is no entry under `/srv`). A caller
/// leaves out a pattern that can match nothing.
///
/// A pattern in which the option would make a negated class take LESS
/// (`photo[!x].jpg`: `[!x]` matches `X` as typed, and would not) is shown
/// with no other spelling at all — `PHOTO[!x].JPG` reads as a glob that
/// admits `PHOTOX.JPG`, which the option does not match — and the line
/// says what the class would stop matching:
///
/// `"photo[!x].jpg" matches exactly as typed; capital letters matter (--ignore-case changes what it matches: [!x] would stop matching X).`
///
/// cr-ecm7-2, cr-ecm7-3, cr-ecm7-4: every claim the line makes is CHECKED
/// against the filter before it is made ([`Spellings::of_role`]), and the
/// pattern it is made about is chosen for what the operator most needs
/// told:
/// - a pattern `--ignore-case` would NARROW comes first, wherever it
///   stands among the patterns — with `Keep` and `[!a].dat` the line used
///   to give `Keep`'s other spellings and merely count the second, the one
///   that stops protecting `A.dat`;
/// - else the first pattern with a spelling that really is matched only
///   with the option — for `{a,A}` the line offered `{a,a}` and `{A,A}`,
///   both matched already;
/// - a pattern whose reading the option does not change at all is neither
///   shown nor counted.
pub(crate) fn case_note_of(patterns: &[Noted], windows: bool) -> Option<String> {
    // By the text alone, and cheaply, for every pattern: the ones the
    // option could read otherwise. Only the ones looked at for showing are
    // checked against the filter.
    let candidates: Vec<&Noted> = patterns
        .iter()
        .filter(|noted| widen_case(noted.read_as, windows) != noted.read_as)
        .collect();
    let mut unchanged: Vec<usize> = Vec::new();
    let mut shown: Option<(usize, Spellings)> = None;
    for (at, noted) in candidates.iter().enumerate() {
        if has_narrowed_class(noted.read_as, windows) {
            match Spellings::of_role(noted.read_as, noted.role, windows) {
                Some(spellings) if spellings.narrowed.is_some() => {
                    shown = Some((at, spellings));
                    break;
                }
                _ => {}
            }
        }
    }
    if shown.is_none() {
        let mut in_words: Option<(usize, Spellings)> = None;
        for (at, noted) in candidates.iter().enumerate() {
            match Spellings::of_role(noted.read_as, noted.role, windows) {
                None => unchanged.push(at),
                Some(spellings) if !spellings.others.is_empty() => {
                    shown = Some((at, spellings));
                    break;
                }
                Some(spellings) => {
                    in_words.get_or_insert((at, spellings));
                }
            }
        }
        shown = shown.or(in_words);
    }
    let (shown_at, spellings) = shown?;
    let (pattern, file) = (candidates[shown_at].typed, candidates[shown_at].file);
    // The patterns that are only counted. One the option reads no
    // differently is not counted — and only a pattern that spells out its
    // own alternatives, in braces or a class, can be such a one (`{a,A}`,
    // `[Jj]`): those are asked about, the first few of them; plain text
    // and wildcards always read otherwise, and are counted unasked. A long
    // list is not compiled pattern by pattern for one line.
    let mut asked = 0;
    for (at, noted) in candidates.iter().enumerate() {
        if at <= shown_at || unchanged.contains(&at) || !noted.read_as.contains(['{', '[']) {
            continue;
        }
        if asked == MOST_PATTERNS_ASKED {
            break;
        }
        asked += 1;
        if Spellings::of_role(noted.read_as, noted.role, windows).is_none() {
            unchanged.push(at);
        }
    }
    let noted = candidates
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != shown_at && !unchanged.contains(at))
        .map(|(_, noted)| (noted.typed, noted.file, ()));
    let shown_from = file.map_or(String::new(), |file| format!(" (from {file})"));
    let others: Vec<Option<&str>> = noted.map(|(_, file, _)| file).collect();
    let more = if others.is_empty() {
        String::new()
    } else {
        let files: Vec<&str> = others.iter().flatten().copied().collect();
        let from = match files.first() {
            None => String::new(),
            Some(first) if files.len() == others.len() && files.iter().all(|f| f == first) => {
                format!(" (from {first})")
            }
            Some(_) if files.len() == others.len() => " (from pattern files)".to_string(),
            Some(_) => " (some from pattern files)".to_string(),
        };
        match others.len() {
            1 => format!(", and 1 more pattern{from} matches only as typed"),
            n => format!(", and {n} more patterns{from} match only as typed"),
        }
    };
    let line = if let Some((class, lost)) = &spellings.narrowed {
        // cr-ecm4-3: ignoring case makes this class take less, so the
        // option does not simply match more — and no spelling shows that.
        let they = if others.is_empty() {
            "it matches"
        } else {
            "they match"
        };
        format!(
            "\"{pattern}\"{shown_from} matches exactly as typed{more}; capital letters matter \
             (--ignore-case changes what {they}: {class} would stop matching {lost})."
        )
    } else if spellings.others.is_empty() {
        // Ignoring case would change what the pattern names, and no other
        // spelling says how (a range across both cases).
        let them = if others.is_empty() { "it" } else { "them" };
        format!(
            "\"{pattern}\"{shown_from} matches exactly as typed{more}; capital letters matter \
             (--ignore-case reads {them} otherwise)."
        )
    } else {
        format!(
            "\"{pattern}\"{shown_from} matches {} only{more}; capital letters matter \
             (--ignore-case would also match {}).",
            spellings.typed,
            spellings.others.join(", ")
        )
    };
    Some(one_line(&line))
}

/// `text` on one line: a line break or any other control character in a
/// pattern (or a file's name) is written as its escape, so the line stays
/// the one line it is promised to be.
fn one_line(text: &str) -> String {
    let mut line = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() {
            line.extend(c.escape_default());
        } else {
            line.push(c);
        }
    }
    line
}

/// A pattern's text as [`case_note`] shows it, and the other spellings of
/// it that `--ignore-case` would match.
struct Spellings<'a> {
    typed: &'a str,
    others: Vec<String>,
    /// cr-ecm4-3: the first negated class `--ignore-case` would make take
    /// less, with the letters it would stop matching (`[!x]`, `X`). When
    /// there is one, `others` is empty: no spelling of the pattern says
    /// what the option does to it.
    narrowed: Option<(&'a str, String)>,
}

impl<'a> Spellings<'a> {
    /// `None` when ignoring case would change nothing about `pattern`:
    /// widened for `--ignore-case` ([`widen_case`]) it is the same text.
    /// That is the matcher's own answer (cr-ecm3-5), so a pattern whose
    /// only letters sit in a class (`*[a-z]`), or whose letter has no
    /// single upper form in the standard library's tables (`*ß`, which
    /// the option pairs with `ẞ`), is noted like any other.
    ///
    /// The other spellings are the pattern all lower and all upper: each
    /// letter changed for the one `--ignore-case` pairs it with, and a
    /// class for the class of its letters' other case (`[a-z]` for
    /// `[A-Z]`). A class whose ranges mix cases is copied — ignoring case
    /// changes what it takes, in no way another spelling shows — and when
    /// nothing else differs there is no other spelling to show at all.
    ///
    /// cr-ecm4-3: nor is there one when the option would make a negated
    /// class take less ([`narrowed_class`]). Such a class was copied too,
    /// beside letters in their other case, and `photo[!x].jpg` was shown
    /// as `PHOTO[!x].JPG` — which reads as a glob admitting `PHOTOX.JPG`,
    /// a name the option does not match (a mirror deleted it).
    #[cfg(test)]
    fn of(pattern: &'a str, windows: bool) -> Option<Self> {
        Self::of_role(pattern, Role::Exclude, windows)
    }

    /// [`Self::of`] for a pattern in `role`, with every claim CHECKED
    /// against the filter (cr-ecm7-2, cr-ecm7-4). The spellings are made
    /// by rule from the pattern's text, and five rounds of review each
    /// found a text the rules got wrong — a level trimmed that the pattern
    /// requires, a spelling the option does not match, one it matches
    /// already. So nothing made by rule is shown on trust:
    /// - the text shown for the pattern (its end wildcards trimmed) must
    ///   name nothing the pattern does not; else the pattern is shown whole;
    /// - each other spelling must name only what the option would match,
    ///   and something the pattern as typed does NOT — else it is dropped;
    /// - a class said to stop matching a letter must really stop;
    /// - and a pattern left with nothing to say is spoken of only when the
    ///   option does change what it names, `None` when it does not
    ///   (`{a,A}` already takes both).
    ///
    /// Checked with the filter itself ([`FileFilter::probing`]), over
    /// names read off each text's own syntax ([`samples`]). Under the
    /// other platform's path rule nothing can be asked of this machine's
    /// filter, and the rules stand unchecked.
    fn of_role(pattern: &'a str, role: Role, windows: bool) -> Option<Self> {
        let asked = (windows == cfg!(windows))
            .then(|| Asked::about(pattern, role))
            .flatten();
        Self::with(pattern, windows, asked.as_ref())
    }

    /// What the rules alone make of an exclude, nothing checked: so that a
    /// rule that goes wrong is seen to, and not only covered for.
    #[cfg(test)]
    fn by_rule(pattern: &'a str, windows: bool) -> Option<Self> {
        Self::with(pattern, windows, None)
    }

    fn with(pattern: &'a str, windows: bool, asked: Option<&Asked>) -> Option<Self> {
        if widen_case(pattern, windows) == pattern {
            return None;
        }
        let trimmed = without_end_wildcards(pattern, windows);
        let typed = match asked {
            Some(asked) if trimmed != pattern && !asked.shown_is_faithful(trimmed, windows) => {
                pattern
            }
            _ => trimmed,
        };
        let mut narrowed: Option<(&'a str, String)> = None;
        let (mut lower, mut upper) = (String::new(), String::new());
        let mut rest = typed;
        while let Some(c) = rest.chars().next() {
            let class = if c == '[' {
                bracket_class_len(rest)
            } else {
                None
            };
            let len = match class {
                Some(len) => {
                    if narrowed.is_none() {
                        narrowed = narrowed_class(&rest[..len]).map(|lost| (&rest[..len], lost));
                    }
                    lower.push_str(&class_in_case(&rest[..len], false));
                    upper.push_str(&class_in_case(&rest[..len], true));
                    len
                }
                // globset's escape under the Unix rule: the next character
                // is itself — a `[` after it opens no class.
                None if c == '\\' && !windows => {
                    let escaped = rest[1..].chars().next();
                    lower.push(c);
                    upper.push(c);
                    if let Some(escaped) = escaped {
                        lower.push(in_case(escaped, false));
                        upper.push(in_case(escaped, true));
                    }
                    1 + escaped.map_or(0, char::len_utf8)
                }
                None => {
                    lower.push(in_case(c, false));
                    upper.push(in_case(c, true));
                    c.len_utf8()
                }
            };
            rest = &rest[len..];
        }
        if let (Some(asked), Some((class, lost))) = (asked, &narrowed) {
            if !asked.stops_matching(pattern, class, lost, windows) {
                narrowed = None;
            }
        }
        let mut others = Vec::new();
        if narrowed.is_none() {
            for other in [lower, upper] {
                let shows = match asked {
                    Some(asked) => asked.matched_only_ignoring_case(&other, windows),
                    None => true,
                };
                if other != typed && !others.contains(&other) && shows {
                    others.push(other);
                }
            }
        }
        if let Some(asked) = asked {
            if others.is_empty() && narrowed.is_none() && !asked.reads_otherwise(pattern, windows) {
                return None;
            }
        }
        Some(Self {
            typed,
            others,
            narrowed,
        })
    }
}

/// Whether `pattern` holds a negated class `--ignore-case` would make
/// take less — by its text alone ([`narrowed_class`]); cheap enough to ask
/// of every pattern.
fn has_narrowed_class(pattern: &str, windows: bool) -> bool {
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        let len = match c {
            '[' => match bracket_class_len(rest) {
                Some(len) if narrowed_class(&rest[..len]).is_some() => return true,
                Some(len) => len,
                None => 1,
            },
            '\\' if !windows => 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
            c => c.len_utf8(),
        };
        rest = &rest[len..];
    }
    false
}

/// How many of a run's counted patterns are asked about (the ones holding
/// braces or a class), beyond the one shown.
const MOST_PATTERNS_ASKED: usize = 16;

/// One pattern as the filter holds it, to ask what it names: as typed,
/// and as `--ignore-case` would read it.
struct Asked {
    role: Role,
    as_typed: crate::fs_enum::FileFilter,
    ignoring_case: crate::fs_enum::FileFilter,
}

/// What a filter of one pattern says of one name: taken as a file, and
/// taken as a folder.
type Named = (bool, bool);

impl Asked {
    fn about(pattern: &str, role: Role) -> Option<Self> {
        let probing = |ignore_case| {
            crate::fs_enum::FileFilter::probing(pattern, role == Role::Exclude, ignore_case)
        };
        Some(Self {
            role,
            as_typed: probing(false)?,
            ignoring_case: probing(true)?,
        })
    }

    fn named(role: Role, filter: &crate::fs_enum::FileFilter, name: &str) -> Named {
        let path = std::path::Path::new(name);
        match role {
            Role::Exclude => (
                !filter.allows_entry(Some(path), path, 1, None),
                !filter.allows_dir(path, path),
            ),
            // A folder is walked whatever an include says.
            Role::Include => (filter.allows_entry(Some(path), path, 1, None), false),
        }
    }

    /// `text`, alone in a filter as this pattern is, and the names read
    /// off its syntax; `None` when either cannot be had.
    fn text(&self, text: &str, windows: bool) -> Option<(crate::fs_enum::FileFilter, Vec<String>)> {
        let filter = crate::fs_enum::FileFilter::probing(text, self.role == Role::Exclude, false)?;
        Some((filter, samples(text, windows, None)?))
    }

    /// The text shown for the pattern names something, and nothing the
    /// pattern as typed does not.
    fn shown_is_faithful(&self, shown: &str, windows: bool) -> bool {
        let Some((filter, names)) = self.text(shown, windows) else {
            return false;
        };
        let mut names_something = false;
        for name in &names {
            let (by_shown, by_pattern) = (
                Self::named(self.role, &filter, name),
                Self::named(self.role, &self.as_typed, name),
            );
            if (by_shown.0 && !by_pattern.0) || (by_shown.1 && !by_pattern.1) {
                return false;
            }
            names_something |= by_shown.0 || by_shown.1;
        }
        names_something
    }

    /// `spelling` names only what the option would match, and something
    /// the pattern as typed does not.
    fn matched_only_ignoring_case(&self, spelling: &str, windows: bool) -> bool {
        let Some((filter, names)) = self.text(spelling, windows) else {
            return false;
        };
        let mut shows_a_difference = false;
        for name in &names {
            let by_spelling = Self::named(self.role, &filter, name);
            let (typed, ignoring) = (
                Self::named(self.role, &self.as_typed, name),
                Self::named(self.role, &self.ignoring_case, name),
            );
            if (by_spelling.0 && !ignoring.0) || (by_spelling.1 && !ignoring.1) {
                return false;
            }
            shows_a_difference |= (by_spelling.0 && !typed.0) || (by_spelling.1 && !typed.1);
        }
        shows_a_difference
    }

    /// The pattern as typed names a name with one of `lost`'s letters
    /// where `class` stands, and with the option it does not.
    fn stops_matching(&self, pattern: &str, class: &str, lost: &str, windows: bool) -> bool {
        let Some(letter) = lost.chars().next() else {
            return false;
        };
        let Some(names) = samples(pattern, windows, Some((class, letter))) else {
            return false;
        };
        names.iter().any(|name| {
            let (typed, ignoring) = (
                Self::named(self.role, &self.as_typed, name),
                Self::named(self.role, &self.ignoring_case, name),
            );
            (typed.0 && !ignoring.0) || (typed.1 && !ignoring.1)
        })
    }

    /// The option changes what the pattern names at all: some name read
    /// off the pattern, in another case — all lower, all upper, or one
    /// letter changed — is named with the option and not without, or the
    /// other way round. Every name read off it, and every letter of each:
    /// the names a pattern takes in one sequence of letters are joined by
    /// single changes of case, so if it does not take them all, one change
    /// away from a name it takes is a name it does not.
    ///
    /// What cannot be asked — more alternatives than are tried, or a
    /// pattern no name can be read off — counts as changed: a line too
    /// many is the safe mistake.
    fn reads_otherwise(&self, pattern: &str, windows: bool) -> bool {
        let Some(names) = samples(pattern, windows, None).filter(|names| !names.is_empty()) else {
            return true;
        };
        let differs = |name: &String| {
            Self::named(self.role, &self.as_typed, name)
                != Self::named(self.role, &self.ignoring_case, name)
        };
        // A class that gains a letter with the option (`[A-c]` gains
        // `d`): a name with that letter where the class stands.
        let mut rest = pattern;
        while let Some(c) = rest.chars().next() {
            let len = match c {
                '[' => match bracket_class_len(rest) {
                    Some(len) => {
                        let class = &rest[..len];
                        if let Some((_, ranges)) = parse_class(class) {
                            let holds = |c: char| ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
                            for (lo, hi) in other_cases(&ranges) {
                                let Some(gained) = (lo..=hi).find(|&c| !holds(c)) else {
                                    continue;
                                };
                                match samples(pattern, windows, Some((class, gained))) {
                                    Some(with_it) if !with_it.iter().any(differs) => {}
                                    _ => return true,
                                }
                            }
                        }
                        len
                    }
                    None => 1,
                },
                '\\' if !windows => 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
                c => c.len_utf8(),
            };
            rest = &rest[len..];
        }
        names.iter().any(|name| {
            let letters: Vec<usize> = name
                .char_indices()
                .filter(|(_, c)| case_variants(*c).len() > 1)
                .map(|(at, _)| at)
                .collect();
            let in_case = |upper: bool, only: Option<usize>| -> String {
                name.char_indices()
                    .map(|(at, c)| {
                        if only.is_none_or(|only| only == at) {
                            in_case(c, upper)
                        } else {
                            c
                        }
                    })
                    .collect()
            };
            let mut variants = vec![in_case(false, None), in_case(true, None)];
            for at in letters {
                variants.push(in_case(false, Some(at)));
                variants.push(in_case(true, Some(at)));
            }
            variants.iter().any(|variant| {
                Self::named(self.role, &self.as_typed, variant)
                    != Self::named(self.role, &self.ignoring_case, variant)
            })
        })
    }
}

/// The most names [`samples`] reads off one text.
const MOST_SAMPLES: usize = 16;

/// Names `text` matches, read off its own syntax — the names a claim
/// about `text` is checked with: a wildcard run stands for `x`, a class
/// for one of its members, an alternation for each of its alternatives in
/// turn. `pick` puts a chosen character where the first class written
/// exactly so stands. `None` when the text holds more alternatives than
/// [`MOST_SAMPLES`] names can cover, or a class no member can be read off.
///
/// Only names a walk can give ([`walked`]): what a pattern says of any
/// other text is nothing a run will ever see, and a claim resting on it
/// would be about no file. `**//Cache` read off as `/Cache` — the text
/// matches, and no entry is ever called that — leaves no name at all.
fn samples(text: &str, windows: bool, pick: Option<(&str, char)>) -> Option<Vec<String>> {
    let mut pick = pick;
    let mut names: Vec<String> = Vec::new();
    for name in expand(text, windows, &mut pick)? {
        if let Some(name) = walked(&name) {
            if !names.iter().any(|kept| kept == name) {
                names.push(name.to_string());
            }
        }
    }
    Some(names)
}

/// `name` as a walk gives an entry's path under the root: no `./` before
/// it and no separator after it (a pattern may be written with either),
/// every component a name. `None` for a text no entry is called — an empty
/// component (`/x`, `a//b`), `.` or `..`.
fn walked(name: &str) -> Option<&str> {
    let mut name = name.trim_end_matches('/');
    while let Some(rest) = name.strip_prefix("./") {
        name = rest;
    }
    let every_component_names = !name.is_empty()
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    every_component_names.then_some(name)
}

fn expand(text: &str, windows: bool, pick: &mut Option<(&str, char)>) -> Option<Vec<String>> {
    let mut names = vec![String::new()];
    let mut rest = text;
    let push = |names: &mut Vec<String>, c: char| names.iter_mut().for_each(|name| name.push(c));
    while let Some(c) = rest.chars().next() {
        let len = match c {
            '[' => match bracket_class_len(rest) {
                Some(len) => {
                    let class = &rest[..len];
                    let members = match *pick {
                        Some((picked, letter)) if picked == class => {
                            *pick = None;
                            vec![letter]
                        }
                        _ => class_members(class)?,
                    };
                    names = names
                        .iter()
                        .flat_map(|name| {
                            members.iter().map(move |member| format!("{name}{member}"))
                        })
                        .collect();
                    if names.len() > MOST_SAMPLES {
                        return None;
                    }
                    len
                }
                None => {
                    push(&mut names, '[');
                    1
                }
            },
            '\\' if !windows => match rest[1..].chars().next() {
                Some(escaped) => {
                    push(&mut names, escaped);
                    1 + escaped.len_utf8()
                }
                None => {
                    push(&mut names, '\\');
                    1
                }
            },
            '\\' => {
                push(&mut names, '/');
                1
            }
            '*' => {
                push(&mut names, 'x');
                rest.len() - rest.trim_start_matches('*').len()
            }
            '?' => {
                push(&mut names, 'x');
                1
            }
            '{' => match alternatives(rest, windows) {
                Some((alternatives, len)) => {
                    let mut next = Vec::new();
                    for alternative in alternatives {
                        for ending in expand(alternative, windows, pick)? {
                            for name in &names {
                                next.push(format!("{name}{ending}"));
                            }
                        }
                        if next.len() > MOST_SAMPLES {
                            return None;
                        }
                    }
                    names = next;
                    len
                }
                None => {
                    push(&mut names, '{');
                    1
                }
            },
            c => {
                push(&mut names, c);
                c.len_utf8()
            }
        };
        rest = &rest[len..];
    }
    Some(names)
}

/// The members of a bracket class a claim is checked with: its first —
/// and for a negated class a character it does not hold, together with
/// the OTHER case of a letter it does hold, when the class as written
/// lets that through. That last one is the name a spelling goes wrong on:
/// `[!x]` written in a spelling admits `X`, which the option refuses.
fn class_members(class: &str) -> Option<Vec<char>> {
    let (negated, ranges) = parse_class(class)?;
    if !negated {
        return ranges.first().map(|&(lo, _)| vec![lo]);
    }
    let holds = |c: char| ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
    let mut members: Vec<char> = ['x', 'q', '7', '_', 'X', 'Q', '~']
        .into_iter()
        .find(|c| !holds(*c))
        .into_iter()
        .collect();
    let other_case = other_cases(&ranges)
        .into_iter()
        .flat_map(|(lo, hi)| lo..=hi)
        .find(|c| !holds(*c));
    members.extend(other_case.filter(|c| !members.contains(c)));
    (!members.is_empty()).then_some(members)
}

/// The alternatives of the `{…}` that `text` begins with, and its length;
/// `None` when the brace never closes. Split at the commas of its own
/// level: one inside a class, inside a nested brace, or after an escape
/// is part of an alternative.
fn alternatives(text: &str, windows: bool) -> Option<(Vec<&str>, usize)> {
    let mut found = Vec::new();
    let (mut depth, mut from, mut at) = (0usize, 1usize, 0usize);
    while let Some(c) = text[at..].chars().next() {
        let len = match c {
            '[' => bracket_class_len(&text[at..]).unwrap_or(1),
            '\\' if !windows => 1 + text[at + 1..].chars().next().map_or(0, char::len_utf8),
            '{' => {
                depth += 1;
                1
            }
            ',' if depth == 1 => {
                found.push(&text[from..at]);
                from = at + 1;
                1
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    found.push(&text[from..at]);
                    return Some((found, at + 1));
                }
                1
            }
            c => c.len_utf8(),
        };
        at += len;
    }
    None
}

/// cr-ecm4-3: for a negated class that `--ignore-case` would make take
/// LESS, the letters it would stop matching, as the class itself would
/// write them (`X` for `[!x]`; `A-C, X` for `[!a-cx]`; `B-C` for
/// `[!a-cA]`, which refuses `A` already). `None` for any other class —
/// one that is not negated, or holds both cases already (`[!xX]`), or no
/// letter (`[!0-9]`), or that the option leaves as typed because it
/// cannot be rewritten with certainty ([`negated_class`]).
fn narrowed_class(class: &str) -> Option<String> {
    let (negated, ranges) = parse_class(class)?;
    if !negated || widen_class(class) == class {
        return None;
    }
    let holds = |c: char| ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
    // The letters the class does not name yet, each run of neighbours as
    // one range.
    let mut lost: Vec<(char, char)> = Vec::new();
    for (lo, hi) in other_cases(&ranges) {
        for c in (lo..=hi).filter(|&c| !holds(c)) {
            if lost.iter().any(|&(lo, hi)| lo <= c && c <= hi) {
                continue;
            }
            match lost.last_mut() {
                Some(run) if run.1 as u32 + 1 == c as u32 => run.1 = c,
                _ => lost.push((c, c)),
            }
        }
    }
    let written: Vec<String> = lost
        .into_iter()
        .map(|(lo, hi)| {
            let mut range = String::new();
            push_range(&mut range, lo, hi);
            range
        })
        .collect();
    (!written.is_empty()).then(|| written.join(", "))
}

/// `c` in lower or upper case, among the letters `--ignore-case` pairs it
/// with ([`case_variants`]): the standard library's form when that is one
/// of them (`S` for `s`), else the first of them in that case (`ẞ` for
/// `ß`, whose standard upper form is the two letters `SS`, which the
/// option never matches), else `c` itself.
fn in_case(c: char, upper: bool) -> char {
    let variants = case_variants(c);
    if variants.len() < 2 {
        return c;
    }
    let one = |mut mapped: Box<dyn Iterator<Item = char>>| match (mapped.next(), mapped.next()) {
        (Some(form), None) => Some(form),
        _ => None,
    };
    let standard = if upper {
        one(Box::new(c.to_uppercase()))
    } else {
        one(Box::new(c.to_lowercase()))
    };
    if let Some(form) = standard.filter(|form| variants.contains(form)) {
        return form;
    }
    variants
        .iter()
        .copied()
        .find(|form| {
            if upper {
                form.is_uppercase()
            } else {
                form.is_lowercase()
            }
        })
        .unwrap_or(c)
}

/// A bracket class with its ASCII letters in lower or upper case — the
/// class `--ignore-case` adds beside it ([`widen_class`]). Copied as it is
/// when that is not a simple change of case: a negated class (ignoring
/// case makes it take LESS), or a range whose ends are not letters of one
/// case (`[A-z]`).
fn class_in_case(class: &str, upper: bool) -> String {
    let case_of = |c: char| {
        if c.is_ascii_lowercase() {
            Some(false)
        } else if c.is_ascii_uppercase() {
            Some(true)
        } else {
            None
        }
    };
    let one_case = match parse_class(class) {
        Some((false, ranges)) => ranges.iter().all(|&(lo, hi)| case_of(lo) == case_of(hi)),
        _ => false,
    };
    // A class that gains nothing when case is ignored (`[Cc]`) already
    // takes both cases: it is the same in every spelling.
    if !one_case || widen_class(class) == class {
        return class.to_string();
    }
    class
        .chars()
        .map(|c| {
            if upper {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

/// `pattern` without the wildcards at its ends, for showing: `*Cache`,
/// `Cache*` and `**/Cache` are all shown `Cache`. A wildcard inside the
/// pattern stays (`IMG_*.jpg`), and so does an escaped `*`, which is a
/// star and no wildcard.
///
/// What is trimmed is only what leaves the shown text naming NO MORE than
/// the pattern does: text shown as `Cache` reads as "an entry called
/// Cache", so it may stand for `*Cache`, `Cache*` and `**/Cache`, each of
/// which names such an entry.
///
/// cr-ecm5-4: a star that is a whole directory level the pattern requires
/// stays. `*/Cache` names `Cache` only one level down, and `Cache/*` what
/// is directly inside it; trimmed, the line said `--ignore-case` would
/// also match `cache` for `*/Cache`, and the mirror run that way deleted
/// the top-level `cache` and kept `sub/cache`.
///
/// cr-ecm6-2: and so does a trailing `/**`. `Cache/**` names what is
/// UNDER `Cache` — it requires `Cache` to be a folder, and does not name a
/// file called `Cache`. Trimmed, the line said the option would also match
/// `cache`, and the mirror run that way deleted the destination's own
/// FILE `cache`.
fn without_end_wildcards(pattern: &str, windows: bool) -> &str {
    let separator = |c: char| c == '/' || (windows && c == '\\');
    let mut text = pattern;
    loop {
        let rest = text.trim_start_matches('*');
        let stars = text.len() - rest.len();
        match rest.chars().next() {
            _ if stars == 0 => break,
            // `**/x`: `x` at any depth. One separator, and no more: in
            // `**//x` the second is the pattern's own (cr-ecm7-2).
            Some(c) if separator(c) && stars == 2 => text = &rest[c.len_utf8()..],
            // `*/x`: a directory above `x` is required, and stays shown.
            Some(c) if separator(c) => break,
            // `*x`: any text before `x`.
            _ => {
                text = rest;
                break;
            }
        }
    }
    let rest = text.trim_end_matches('*');
    let stays = match rest.chars().next_back() {
        // Under the Unix rule the star after a `\` is itself.
        Some('\\') if !windows => true,
        // `x/*` and `x/**`: what is inside `x`, never `x` itself, so the
        // level stays shown.
        Some(c) => separator(c),
        // Nothing but stars.
        None => false,
    };
    if stays {
        text
    } else {
        // `x*`: any text after `x`.
        rest
    }
}

/// `pattern` — a glob as globset reads it under one platform's rule
/// (`windows`: `\` separates components and escapes nothing) — widened so
/// that it ignores case: see the module documentation. Wildcards,
/// separators, alternation syntax and every non-letter are copied as they
/// are; the result is a glob in the same dialect.
pub(crate) fn widen_case(pattern: &str, windows: bool) -> String {
    let mut out = String::with_capacity(pattern.len() * 4);
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        match c {
            '[' => match bracket_class_len(rest) {
                Some(len) => {
                    out.push_str(&widen_class(&rest[..len]));
                    rest = &rest[len..];
                }
                // Never closed: an ordinary character (and a pattern
                // globset refuses); copied.
                None => {
                    out.push('[');
                    rest = &rest[1..];
                }
            },
            // globset's escape under the Unix rule: the next character is
            // itself, whatever it is.
            '\\' if !windows => match rest[1..].chars().next() {
                Some(escaped) => {
                    let variants = case_variants(escaped);
                    if variants.len() > 1 {
                        push_letter(&mut out, &variants);
                    } else {
                        out.push('\\');
                        out.push(escaped);
                    }
                    rest = &rest[1 + escaped.len_utf8()..];
                }
                None => {
                    out.push('\\');
                    rest = &rest[1..];
                }
            },
            c => {
                let variants = case_variants(c);
                if variants.len() > 1 {
                    push_letter(&mut out, &variants);
                } else {
                    out.push(c);
                }
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

/// One letter as the glob text matching each of its `variants`: a class
/// when every variant is a single byte (`[aA]`), else an alternation
/// (`{s,S,ſ}`) — a class holds one byte, and `ſ` is two.
fn push_letter(out: &mut String, variants: &[char]) {
    if variants.iter().all(char::is_ascii) {
        out.push('[');
        out.extend(variants);
        out.push(']');
        return;
    }
    out.push('{');
    for (at, variant) in variants.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        out.push(*variant);
    }
    out.push('}');
}

/// A bracket class (`[` through its `]`) widened to ignore case: each ASCII
/// letter it holds, alone or inside a range, gains its other case — and
/// nothing else. A class matches ONE BYTE, with the option and without it
/// (the plan's rule, Q10), so a letter's fold-mates beyond ASCII (`ſ` for
/// `s`, two bytes; the Kelvin sign for `k`, three) are NOT added: the bare
/// letter `s` names `ſ` when case is ignored, the class `[s]` does not.
/// cr-ecm2-1: they once were, as alternatives beside the class, and
/// `--include '[sk].txt'` then put `ſ.txt` in a mirror's deletion scope,
/// which the same pattern as typed never did. A non-ASCII member is left
/// as it is (validation refuses such a class before it is ever read).
///
/// The class is returned unchanged when nothing is to be added, or when it
/// cannot be rewritten with certainty (see [`negated_class`]).
fn widen_class(class: &str) -> String {
    let Some((negated, ranges)) = parse_class(class) else {
        return class.to_string();
    };
    let added = other_cases(&ranges);
    if negated {
        if added.is_empty() {
            return class.to_string();
        }
        return negated_class(&ranges, &added).unwrap_or_else(|| class.to_string());
    }
    if added.is_empty() {
        return class.to_string();
    }
    // The class as typed, or a second class of the other cases: one byte
    // either way.
    let mut out = String::with_capacity(class.len() * 2 + 8);
    out.push('{');
    out.push_str(class);
    out.push_str(",[");
    for &(lo, hi) in &added {
        push_range(&mut out, lo, hi);
    }
    out.push_str("]}");
    out
}

/// The other case of each run of ASCII letters a class of `ranges` holds,
/// where the class does not hold that run already: what [`widen_class`]
/// adds to it.
fn other_cases(ranges: &[(char, char)]) -> Vec<(char, char)> {
    let holds = |c: char| ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
    let mut added: Vec<(char, char)> = Vec::new();
    for &(lo, hi) in ranges {
        for (from, to, other) in [('a', 'z', 'A'), ('A', 'Z', 'a')] {
            let (start, end) = (lo.max(from), hi.min(to));
            if start > end {
                continue;
            }
            let shift = |c: char| char::from(other as u8 + (c as u8 - from as u8));
            let run = (shift(start), shift(end));
            if !(run.0..=run.1).all(holds) && !added.contains(&run) {
                added.push(run);
            }
        }
    }
    added
}

fn push_range(out: &mut String, lo: char, hi: char) {
    out.push(lo);
    if lo != hi {
        out.push('-');
        out.push(hi);
    }
}

/// A negated class holding `ranges` and `added`, written so that globset
/// reads exactly those members: a member `]` first, a member `-` last, and
/// nothing else that could read as a range it is not. `None` when the text
/// written does not read back as intended (a range that starts at `-`, say),
/// and the caller keeps the class as typed — which matches MORE names than
/// the widened one would, so an exclude names more, never less.
fn negated_class(ranges: &[(char, char)], added: &[(char, char)]) -> Option<String> {
    let single = |c: char| ranges.contains(&(c, c));
    let mut out = String::from("[!");
    if single(']') {
        out.push(']');
    }
    for &(lo, hi) in ranges.iter().chain(added) {
        if (lo, hi) == (']', ']') || (lo, hi) == ('-', '-') {
            continue;
        }
        push_range(&mut out, lo, hi);
    }
    if single('-') {
        out.push('-');
    }
    out.push(']');
    let (negated, read) = parse_class(&out)?;
    let same = |a: &[(char, char)], b: &[(char, char)]| a.iter().all(|range| b.contains(range));
    let wanted: Vec<(char, char)> = ranges.iter().chain(added).copied().collect();
    (negated
        && bracket_class_len(&out) == Some(out.len())
        && same(&wanted, &read)
        && same(&read, &wanted))
    .then_some(out)
}

/// A bracket class's members as globset 0.4 reads them: whether it is
/// negated, and each member as a range (a single character is a range of
/// itself). A `]` in first place is a member, `-` is a member in first or
/// last place and otherwise joins the characters around it. `None` for text
/// that is not one whole class, or holds a range that runs backwards
/// (globset refuses it).
fn parse_class(class: &str) -> Option<(bool, Vec<(char, char)>)> {
    if bracket_class_len(class) != Some(class.len()) {
        return None;
    }
    let inner = class.strip_prefix('[')?.strip_suffix(']')?;
    let (negated, body) = match inner.strip_prefix(['!', '^']) {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let mut ranges: Vec<(char, char)> = Vec::new();
    let mut first = true;
    let mut in_range = false;
    for c in body.chars() {
        match c {
            '-' if first => ranges.push(('-', '-')),
            '-' if !in_range => in_range = true,
            c => {
                if in_range {
                    let range = ranges.last_mut()?;
                    range.1 = c;
                    if range.1 < range.0 {
                        return None;
                    }
                } else {
                    ranges.push((c, c));
                }
                in_range = false;
            }
        }
        first = false;
    }
    if in_range {
        ranges.push(('-', '-'));
    }
    Some((negated, ranges))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The glob `pattern` widened under the Unix rule, matched as the
    /// filters match one: globset's defaults, forced to the Unix reading of
    /// `\` so the test reads the same on every OS.
    fn ignoring_case(pattern: &str) -> globset::GlobMatcher {
        globset::GlobBuilder::new(&widen_case(pattern, false))
            .backslash_escape(true)
            .build()
            .unwrap_or_else(|error| panic!("{pattern} widened is not a glob: {error}"))
            .compile_matcher()
    }

    fn as_typed(pattern: &str) -> globset::GlobMatcher {
        globset::GlobBuilder::new(pattern)
            .backslash_escape(true)
            .build()
            .unwrap()
            .compile_matcher()
    }

    /// Q1 (owner, 2026-10-21): "ignore case" is Unicode SIMPLE case
    /// folding — one letter to the same letter in another case, in every
    /// alphabet, the one-to-one oddities included, and never one letter to
    /// several. A build without the fold table, or a table of another
    /// extent, fails here.
    #[test]
    fn a_letter_pairs_with_its_simple_case_fold_and_nothing_else() {
        let pairs =
            |a: char, b: char| case_variants(a).contains(&b) && case_variants(b).contains(&a);
        for (a, b) in [
            ('a', 'A'),
            ('ü', 'Ü'),
            ('я', 'Я'),
            ('σ', 'Σ'),
            ('σ', 'ς'),
            ('s', 'ſ'),
            ('k', '\u{212a}'),
            ('ß', 'ẞ'),
        ] {
            assert!(pairs(a, b), "{a} and {b} share a simple fold");
        }
        for (a, b) in [
            ('ß', 'S'),
            ('ß', 's'),
            ('ﬁ', 'f'),
            ('i', 'ı'),
            ('I', 'İ'),
            ('a', 'b'),
        ] {
            assert!(!pairs(a, b), "{a} and {b} do not share a simple fold");
        }
        for alone in [
            '1', '_', '-', '.', '/', '*', '?', '[', '{', ',', '\\', '中', 'ß',
        ] {
            let variants = case_variants(alone);
            let expected: &[char] = if alone == 'ß' {
                &['ß', 'ẞ']
            } else {
                &[alone]
            };
            assert_eq!(variants, expected, "{alone}");
        }
    }

    /// AC-2: a widened pattern matches every casing of its letters and
    /// nothing a letter does not spell; the typed pattern is exact (AC-1).
    #[test]
    fn a_widened_pattern_ignores_case_and_the_typed_one_does_not() {
        for (pattern, names, others) in [
            (
                "*cache",
                &["aCache", "ACACHE", "cache", "x.Cache"][..],
                &["cach", "cachet", "aCäche"][..],
            ),
            ("Übung", &["übung", "ÜBUNG", "Übung"], &["ubung", "Ubung"]),
            ("σ*", &["ς.txt", "Σ.txt", "σ"], &["s.txt"]),
            ("s.txt", &["S.txt", "s.TXT", "ſ.txt"], &["ss.txt", "ß.txt"]),
            ("straße", &["STRAßE", "Straẞe"], &["STRASSE", "strasse"]),
            ("ﬁle", &["ﬁLE"], &["file", "FILE"]),
            ("k1", &["K1", "\u{212a}1"], &["k2"]),
            (r"a\*b", &["A*B", "a*b"], &["axb", "AxB"]),
            ("{foo,bar}.txt", &["FOO.txt", "Bar.TXT"], &["baz.txt"]),
            ("dir/*.Tmp", &["DIR/x.tmp", "dir/y.TMP"], &["dirx.tmp"]),
        ] {
            let widened = ignoring_case(pattern);
            let typed = as_typed(pattern);
            for name in names {
                assert!(widened.is_match(name), "{pattern} widened names {name}");
            }
            for name in others {
                assert!(
                    !widened.is_match(name),
                    "{pattern} widened does not name {name}"
                );
                assert!(
                    !typed.is_match(name),
                    "{pattern} as typed does not name {name}"
                );
            }
            assert!(
                names.iter().any(|name| !typed.is_match(name)),
                "{pattern} as typed must miss some casing, or this row proves nothing"
            );
        }
    }

    /// D-2026-11-20-4: `?` and `*` count the same bytes with the option as
    /// without it, because the name is never folded. `ſ` is two bytes and
    /// folds to the one-byte `s`; the composed `é` is two bytes with no
    /// one-byte fold. A widening that folded names would name `ſ.txt` by
    /// `?.txt` here.
    #[test]
    fn a_wildcard_counts_the_same_bytes_in_both_modes() {
        for (pattern, name, matches) in [
            ("??.txt", "ſ.txt", true),
            ("?.txt", "ſ.txt", false),
            ("??.txt", "é.txt", true),
            ("?.txt", "é.txt", false),
            ("?.txt", "S.txt", true),
            ("??.txt", "S.txt", false),
            ("???.txt", "\u{212a}.txt", true),
            ("?.txt", "\u{212a}.txt", false),
        ] {
            assert_eq!(
                as_typed(pattern).is_match(name),
                matches,
                "{pattern} as typed, {name}"
            );
            assert_eq!(
                ignoring_case(pattern).is_match(name),
                matches,
                "{pattern} widened, {name}"
            );
        }
        // The literal `s` does name `ſ` once case is ignored.
        assert!(ignoring_case("s.txt").is_match("ſ.txt"));
        assert!(!as_typed("s.txt").is_match("ſ.txt"));
    }

    /// A class gains the other case of its ASCII letters — alone, in a
    /// range, and negated — and nothing beyond ASCII (cr-ecm2-1: a class
    /// is one byte). Every other member stays.
    #[test]
    fn a_class_gains_the_other_case_of_its_letters() {
        for (pattern, names, others) in [
            ("[abc]x", &["Ax", "bx", "CX"][..], &["dx", "_x"][..]),
            ("[a-c]x", &["Bx", "ax"], &["Dx", "dx"]),
            ("[A-C0-9_]x", &["bx", "7x", "_x"], &["dx", "-x"]),
            ("[st]x", &["Sx", "Tx"], &["ux", "ſx"]),
            ("[j-l]x", &["Kx"], &["mx", "\u{212a}x"]),
            ("[]a]x", &["]x", "Ax"], &["bx"]),
            ("[a-]x", &["-x", "Ax"], &["bx"]),
            ("[-a]x", &["-x", "Ax"], &["bx"]),
            ("[!a]x", &["bx", "Bx", "_x"], &["ax", "Ax"]),
            ("[!a-c_]x", &["dx", "Dx", "-x"], &["bx", "Bx", "_x"]),
            ("[!]a]x", &["bx"], &["]x", "ax", "Ax"]),
            ("[!a-]x", &["bx"], &["-x", "ax", "Ax"]),
            ("[!-a]x", &["bx"], &["-x", "ax", "Ax"]),
            ("[^a]x", &["bx"], &["ax", "Ax"]),
        ] {
            let widened = ignoring_case(pattern);
            for name in names {
                assert!(widened.is_match(name), "{pattern} widened names {name}");
            }
            for name in others {
                assert!(
                    !widened.is_match(name),
                    "{pattern} widened does not name {name}"
                );
            }
        }
        // cr-g3fix6-1's case: a range across both cases keeps what lies
        // between them (`[A-z]` holds `_`) and gains nothing: it already
        // holds both cases of every ASCII letter.
        assert_eq!(widen_case("[A-z]", false), "[A-z]");
        for name in ["_x", "ax", "Zx"] {
            assert!(ignoring_case("[A-z]x").is_match(name), "{name}");
        }
        assert!(!ignoring_case("[A-z]x").is_match("ſx"));
        assert_eq!(widen_case("[sk]", false), "{[sk],[SK]}");
        // Nothing to add: the class is copied byte for byte.
        for unchanged in ["[0-9]", "[aA]", "[!0-9]", "[_.]", "[é]"] {
            assert_eq!(widen_case(unchanged, false), unchanged);
        }
    }

    /// Wildcards, separators, alternation syntax, the recursive `**` and
    /// escaped syntax characters read the same after widening; only letters
    /// change. Under the Windows rule `\` is a separator and is copied.
    #[test]
    fn only_letters_are_widened() {
        for unchanged in [
            "*", "**", "?", "**/*", "*/?/**", "{1,2}", "[0-9]*.?", r"\*\?\[", "./-_.1",
        ] {
            assert_eq!(widen_case(unchanged, false), unchanged, "{unchanged}");
        }
        assert_eq!(widen_case("a", false), "[Aa]");
        assert_eq!(widen_case(r"\a", false), "[Aa]");
        assert_eq!(widen_case(r"x\y", true), r"[Xx]\[Yy]");
        assert_eq!(widen_case("s", false), "{S,s,ſ}");
        assert_eq!(widen_case("ü", false), "{Ü,ü}");
        assert_eq!(widen_case("[", false), "[");
        assert_eq!(widen_case("a[", false), "[Aa][");
        assert_eq!(widen_case("a\\", false), "[Aa]\\");
        // `**` keeps its recursive reading beside a widened letter.
        for (pattern, name, matches) in [
            ("**/cache", "a/b/CACHE", true),
            ("**/cache", "CACHE", true),
            ("cache/**", "Cache/x/y", true),
            ("cache/**", "Cache", false),
            ("a/**/z", "A/Z", true),
            ("a/**/z", "A/m/n/Z", true),
            ("{a/**,b}", "A/x/y", true),
        ] {
            assert_eq!(
                ignoring_case(pattern).is_match(name),
                matches,
                "{pattern}, {name}"
            );
        }
    }

    /// Every pattern the glob compiler accepts as typed is still a glob
    /// once widened. One with no negated class names everything it named
    /// before; a negated class names less by design (`[!a]` once named `A`).
    #[test]
    fn a_widened_pattern_is_a_glob_that_names_at_least_what_the_typed_one_did() {
        let names = [
            "a", "A", "ab", "a/b", "A/B", "x.txt", "X.TXT", "[", "]", "-", "a-b", "é", "É", "ſ",
            "s", "{", "}", ",", "a,b", "a*b", "cache", "Cache/x", ".a", "_", "z", "Z", "0",
        ];
        for pattern in [
            "a",
            "*",
            "?",
            "a*",
            "*a*",
            "[a]",
            "[]]",
            "[]a-]",
            "[a-z]",
            "[A-Z]",
            "[a-zA-Z]",
            "[m-p]",
            "[-]",
            "{a,b}",
            "{a,{b,c}}",
            "{a*,?b}",
            "a/b",
            "**/a",
            "a/**",
            "a/**/b",
            r"\[",
            r"\]",
            r"\{a\}",
            r"a\,b",
            "é",
            "[é]",
            "ſ",
            "{s,k}",
            "[sk]",
            "a-b",
            ".a",
            "[.]a",
            "[a-]",
            "[-a]",
            "[a^]",
            "[a!]",
        ] {
            let typed = as_typed(pattern);
            let widened = ignoring_case(pattern);
            for name in names {
                if typed.is_match(name) {
                    assert!(
                        widened.is_match(name),
                        "{pattern} widened still names {name}"
                    );
                }
            }
        }
        for negated in [
            "[!a]", "[!]a-]", "[!m-p]", "[!-]", "[!é]", "[!sk]", "[!a-]", "[!-a]", "[^]]", "[!!]",
            "[!^]", "[!--9a]",
        ] {
            as_typed(negated);
            ignoring_case(negated);
        }
        // A negated class is rewritten whole; what it is rewritten to reads
        // back as the same members plus the added case.
        for (typed, widened) in [
            ("[!a]", "[!aA]"),
            ("[!]a-]", "[!]aA-]"),
            ("[!m-p]", "[!m-pM-P]"),
            ("[!--9a]", "[!--9aA]"),
            ("[!-]", "[!-]"),
            ("[!é]", "[!é]"),
        ] {
            assert_eq!(widen_case(typed, false), widened, "{typed}");
        }
    }

    /// cr-ecm2-1: a class matches one byte with `--ignore-case` as without
    /// it. `ſ` (two bytes) and the Kelvin sign (three) are fold-mates of
    /// `s` and `k`: the BARE letters name them once case is ignored, a
    /// class holding those letters never does — positive, negated, a range
    /// or a single member — and never matches more bytes than one.
    #[test]
    fn a_class_stays_one_byte_wide_when_case_is_ignored() {
        let (long_s, kelvin) = ("ſ", "\u{212a}");
        for class in [
            "[s]", "[k]", "[sk]", "[a-z]", "[A-Z]", "[S]", "[rst]", "[j-l0-9]",
        ] {
            let pattern = format!("{class}.txt");
            for (mode, matcher) in [
                ("as typed", as_typed(&pattern)),
                ("ignoring case", ignoring_case(&pattern)),
            ] {
                for letter in [long_s, kelvin] {
                    assert!(
                        !matcher.is_match(format!("{letter}.txt")),
                        "{pattern} {mode} does not name {letter}.txt"
                    );
                }
            }
            assert!(
                !widen_case(&pattern, false).contains(['ſ', '\u{212a}']),
                "{pattern} widened holds no letter beyond ASCII"
            );
        }
        // The class still gains its ASCII partners...
        assert!(ignoring_case("[sk].txt").is_match("S.txt"));
        assert!(ignoring_case("[sk].txt").is_match("K.txt"));
        // ...a negated class, which matches any ONE byte but its members,
        // still matches no letter of several bytes...
        for mode in [as_typed("[!x].txt"), ignoring_case("[!x].txt")] {
            assert!(mode.is_match("y.txt"));
            assert!(!mode.is_match("ſ.txt"));
        }
        // ...and the bare letters do name their fold-mates.
        assert!(ignoring_case("s.txt").is_match("ſ.txt"));
        assert!(ignoring_case("k.txt").is_match("\u{212a}.txt"));
    }

    /// EXCLUDE_CASE_MATCHING slice 2 (AC-3, AC-5): the line, to the letter.
    #[test]
    fn the_case_note_says_what_the_first_lettered_pattern_matches() {
        let note = |patterns: &[&str]| {
            let typed: Vec<(&str, Option<&str>)> = patterns.iter().map(|p| (*p, None)).collect();
            case_note(&typed, false)
        };
        let line = |shown: &str, more: &str, others: &str| {
            Some(format!(
                "{shown} only{more}; capital letters matter (--ignore-case would also match \
                 {others})."
            ))
        };
        // AC-3, as the plan writes it.
        assert_eq!(
            note(&["*Cache"]).as_deref(),
            Some(
                "\"*Cache\" matches Cache only; capital letters matter (--ignore-case would \
                 also match cache, CACHE)."
            )
        );
        // Whichever spelling equals the typed one is left out.
        assert_eq!(
            note(&["*cache"]),
            line("\"*cache\" matches cache", "", "CACHE")
        );
        assert_eq!(note(&["*.CFG"]), line("\"*.CFG\" matches .CFG", "", ".cfg"));
        // The wildcards at the ends are not part of what is shown; one
        // inside the pattern is.
        for pattern in ["Cache*", "*Cache*", "**/Cache", "**/*Cache*"] {
            assert_eq!(
                note(&[pattern]),
                line(&format!("\"{pattern}\" matches Cache"), "", "cache, CACHE"),
                "{pattern}"
            );
        }
        // cr-ecm6-2: what is UNDER a folder is not the folder: `/**` stays.
        for pattern in ["Cache/**", "**/Cache/**"] {
            assert_eq!(
                note(&[pattern]),
                line(
                    &format!("\"{pattern}\" matches Cache/**"),
                    "",
                    "cache/**, CACHE/**"
                ),
                "{pattern}"
            );
        }
        assert_eq!(
            note(&["IMG_*.jpg"]),
            line(
                "\"IMG_*.jpg\" matches IMG_*.jpg",
                "",
                "img_*.jpg, IMG_*.JPG"
            )
        );
        assert_eq!(
            note(&["srv/Data"]),
            line("\"srv/Data\" matches srv/Data", "", "srv/data, SRV/DATA")
        );
        // AC-5: no letter whose case could matter, no line.
        for patterns in [
            &["??"][..],
            &["*.1"],
            &["??", "*.1", "*", "**", "[Aa]1", "{1,2}"],
            &[],
        ] {
            assert_eq!(note(patterns), None, "{patterns:?}");
        }
        // AC-5: several patterns, one line — the first that holds a letter,
        // and a count of the others that do.
        assert_eq!(
            note(&["??", "*.1", "Thumbs.db", "*"]),
            line(
                "\"Thumbs.db\" matches Thumbs.db",
                "",
                "thumbs.db, THUMBS.DB"
            )
        );
        assert_eq!(
            note(&["??", "Thumbs.db", "*.1", "*.tmp"]),
            line(
                "\"Thumbs.db\" matches Thumbs.db",
                ", and 1 more pattern matches only as typed",
                "thumbs.db, THUMBS.DB"
            )
        );
        assert_eq!(
            note(&["*.o", "*.tmp", "??", "Build"]),
            line(
                "\"*.o\" matches .o",
                ", and 2 more patterns match only as typed",
                ".O"
            )
        );
    }

    /// Every other spelling the note shows is one `--ignore-case` really
    /// matches, and the note never shows one it does not: each letter is
    /// changed only for the single letter the option pairs it with.
    #[test]
    fn the_case_note_shows_only_spellings_ignore_case_matches() {
        let others = |pattern: &str| {
            Spellings::of(pattern, false)
                .map(|spellings| spellings.others)
                .unwrap_or_default()
        };
        // `ß` has no single upper letter in the standard tables; the option
        // pairs it with `ẞ`, and that is what is shown — never `SS`.
        assert_eq!(others("straße"), ["STRAẞE"]);
        // The sigmas pair one to one; the final sigma lowers to itself.
        assert_eq!(others("σ*"), ["Σ"]);
        assert_eq!(others("Σ"), ["σ"]);
        assert_eq!(others("Übung"), ["übung", "ÜBUNG"]);
        // A class is copied as it is; a pattern whose only letters are in
        // one gets no note.
        assert_eq!(others("[Cc]ache"), ["[Cc]ACHE"]);
        // cr-ecm3-5: a class of one case has the class of the other.
        assert_eq!(others("[abc]"), ["[ABC]"]);
        assert_eq!(others("[A-C_]x"), ["[a-c_]x", "[A-C_]X"]);
        // An escaped character under the Unix rule is itself: an escaped
        // star at the end is no wildcard, an escaped bracket opens no class.
        assert_eq!(others(r"Cache\*"), [r"cache\*", r"CACHE\*"]);
        assert_eq!(others(r"\[Ab]"), [r"\[ab]", r"\[AB]"]);
        // On Windows `\` separates; the wildcard ends are trimmed with it.
        assert_eq!(
            Spellings::of(r"**\Cache\**", true).map(|s| s.typed),
            Some(r"Cache\**")
        );
        assert_eq!(
            Spellings::of(r"**\Cache", true).map(|s| s.typed),
            Some("Cache")
        );
        // cr-ecm5-4: a wildcard that is a whole directory level the
        // pattern REQUIRES stays in what is shown, under either rule.
        for (pattern, windows, typed, spellings) in [
            ("*/Cache", false, "*/Cache", &["*/cache", "*/CACHE"][..]),
            ("Cache/*", false, "Cache/*", &["cache/*", "CACHE/*"]),
            ("*/Cache/*", false, "*/Cache/*", &["*/cache/*", "*/CACHE/*"]),
            ("**/*/Cache", false, "*/Cache", &["*/cache", "*/CACHE"]),
            (
                "**/*Cache*/**",
                false,
                "Cache*/**",
                &["cache*/**", "CACHE*/**"],
            ),
            ("Cache/**", false, "Cache/**", &["cache/**", "CACHE/**"]),
            (r"Cache\**", true, r"Cache\**", &[r"cache\**", r"CACHE\**"]),
            (r"*\Cache", true, r"*\Cache", &[r"*\cache", r"*\CACHE"]),
            (r"Cache\*", true, r"Cache\*", &[r"cache\*", r"CACHE\*"]),
        ] {
            let shown = Spellings::of(pattern, windows).expect("noted");
            assert_eq!(shown.typed, typed, "{pattern}");
            assert_eq!(shown.others, spellings, "{pattern}");
        }

        for pattern in ["*Cache", "straße", "σx", "Übung", "IMG_*.jpg", "ſ", "K"] {
            let widened = globset::Glob::new(&widen_case(pattern, false))
                .unwrap()
                .compile_matcher();
            let shown = Spellings::of(pattern, false);
            for other in shown.map(|s| s.others).unwrap_or_default() {
                // A wildcard kept inside the spelling stands for itself here.
                let name = other.replace('*', "x");
                assert!(
                    widened.is_match(&name),
                    "{pattern} shows {other}, which --ignore-case must match"
                );
            }
        }
    }

    /// Slice 3b (AC-8): the line says which patterns the operator did not
    /// type on the command line — the one shown, and the ones counted.
    #[test]
    fn the_case_note_says_which_patterns_came_from_a_file() {
        let note = |patterns: &[(&str, Option<&str>)]| case_note(patterns, false).unwrap();
        // AC-8's case: the first from the command line, the rest from a file.
        assert_eq!(
            note(&[
                ("Thumbs.db", None),
                ("*.tmp", Some("skip.txt")),
                ("??", Some("skip.txt")),
                ("*.bak", Some("skip.txt")),
            ]),
            "\"Thumbs.db\" matches Thumbs.db only, and 2 more patterns (from skip.txt) match only \
             as typed; capital letters matter (--ignore-case would also match thumbs.db, \
             THUMBS.DB)."
        );
        // The pattern shown came from a file itself.
        assert_eq!(
            note(&[("??", None), ("*Cache", Some("skip.txt"))]),
            "\"*Cache\" (from skip.txt) matches Cache only; capital letters matter (--ignore-case \
             would also match cache, CACHE)."
        );
        // Counted from two files; from a file and the command line; typed only.
        let counted = |others: &[Option<&str>]| {
            let mut patterns = vec![("a", None)];
            patterns.extend(others.iter().map(|file| ("b", *file)));
            let line = note(&patterns);
            let (_, rest) = line.split_once(", and ").expect("a count");
            rest.split_once(" match")
                .expect("the count's words")
                .0
                .to_string()
        };
        assert_eq!(counted(&[Some("x.txt")]), "1 more pattern (from x.txt)");
        assert_eq!(
            counted(&[Some("x.txt"), Some("y.txt")]),
            "2 more patterns (from pattern files)"
        );
        assert_eq!(
            counted(&[Some("x.txt"), None]),
            "2 more patterns (some from pattern files)"
        );
        assert_eq!(counted(&[None, None]), "2 more patterns");
    }

    /// cr-ecm3-5: which patterns the line speaks of is the matcher's own
    /// answer — a pattern is noted exactly when ignoring case would change
    /// what it names. `*[a-z]` and `*ß` were passed over (the first has
    /// its letters in a class, the second has no single upper form in the
    /// standard tables) though a mirror deleted `TEST` and `CAPẞ` as typed
    /// and kept them with the option.
    #[test]
    fn the_case_note_follows_what_the_matcher_does() {
        let note = |pattern: &str| case_note(&[(pattern, None)], false);
        assert_eq!(
            note("*[a-z]").as_deref(),
            Some(
                "\"*[a-z]\" matches [a-z] only; capital letters matter (--ignore-case would \
                 also match [A-Z])."
            )
        );
        assert_eq!(
            note("*ß").as_deref(),
            Some(
                "\"*ß\" matches ß only; capital letters matter (--ignore-case would also \
                 match ẞ)."
            )
        );
        // Ignoring case makes a negated class take LESS, and no other
        // spelling says so: the line does, without one (cr-ecm4-3).
        assert_eq!(
            note("*[!a]").as_deref(),
            Some(
                "\"*[!a]\" matches exactly as typed; capital letters matter (--ignore-case \
                 changes what it matches: [!a] would stop matching A)."
            )
        );
        assert_eq!(
            case_note(&[("[!a]", None), ("b", None)], false).as_deref(),
            Some(
                "\"[!a]\" matches exactly as typed, and 1 more pattern matches only as typed; \
                 capital letters matter (--ignore-case changes what they match: [!a] would stop \
                 matching A)."
            )
        );
        // A class across both cases that the option still widens: no
        // spelling shows how, and nothing is lost.
        assert_eq!(
            note("[A-c]1").as_deref(),
            Some(
                "\"[A-c]1\" matches exactly as typed; capital letters matter (--ignore-case \
                 reads it otherwise)."
            )
        );
        // Noted exactly when widening changes the pattern, never otherwise.
        for (pattern, noted) in [
            ("*[a-z]", true),
            ("*ß", true),
            ("[!a]x1", true),
            ("[A-z]1", false),
            ("[A-c]1", true),
            ("[Aa]1", false),
            ("[0-9_]*", false),
            ("??", false),
            ("ſ", true),
            ("日本", false),
        ] {
            assert_eq!(
                note(pattern).is_some(),
                noted,
                "{pattern} widens to {}",
                widen_case(pattern, false)
            );
            assert_eq!(noted, widen_case(pattern, false) != pattern, "{pattern}");
        }
        // Every other spelling shown — a class standing for its first
        // member, a wildcard for one letter — is a name the option matches.
        for pattern in [
            "*[a-z]",
            "*ß",
            "[abc]x",
            "[A-C_]x",
            "st[r]aße",
            "IMG_*.[jJ]pg",
        ] {
            let widened = globset::Glob::new(&widen_case(pattern, false))
                .unwrap()
                .compile_matcher();
            let shown = Spellings::of(pattern, false).expect("noted");
            assert!(!shown.others.is_empty(), "{pattern}");
            for other in &shown.others {
                let mut name = String::new();
                let mut rest = other.as_str();
                while let Some(c) = rest.chars().next() {
                    let len = match c {
                        '[' => {
                            let len = bracket_class_len(rest).expect("a class");
                            name.push(rest[1..].chars().next().unwrap());
                            len
                        }
                        '*' => {
                            name.push('x');
                            1
                        }
                        c => {
                            name.push(c);
                            c.len_utf8()
                        }
                    };
                    rest = &rest[len..];
                }
                assert!(
                    widened.is_match(&name),
                    "{pattern} shows {other}; --ignore-case must match {name}"
                );
            }
        }
    }

    /// cr-ecm3-5: the line is ONE line whatever a pattern or a file's name
    /// holds: a line break typed into a pattern is written as its escape.
    #[test]
    fn the_case_note_is_one_line_whatever_the_pattern_holds() {
        let line = case_note(
            &[
                ("first\nSecond\tthird", Some("odd\nname.txt")),
                ("more\r\n", None),
            ],
            false,
        )
        .unwrap();
        assert_eq!(line.lines().count(), 1, "{line:?}");
        assert!(!line.contains(['\n', '\r', '\t']), "{line:?}");
        let start = concat!(
            r#""first\nSecond\tthird" (from odd\nname.txt) matches "#,
            r#"first\nSecond\tthird only, and 1 more pattern matches only as typed;"#
        );
        assert!(line.starts_with(start), "{line}");
    }

    /// cr-ecm4-3: `--ignore-case` makes a negated class take LESS, and the
    /// line says exactly that instead of showing a spelling. For
    /// `photo[!x].jpg` it showed `PHOTO[!x].JPG` as something the option
    /// would also match: a glob that admits `PHOTOX.JPG`, which the option
    /// does not match — the mirror deleted it — while `photoX.jpg` lost
    /// the protection it had as typed.
    #[test]
    fn the_case_note_says_what_a_negated_class_would_stop_matching() {
        let note = |pattern: &str| case_note(&[(pattern, None)], false).unwrap();
        assert_eq!(
            note("photo[!x].jpg"),
            "\"photo[!x].jpg\" matches exactly as typed; capital letters matter (--ignore-case \
             changes what it matches: [!x] would stop matching X)."
        );
        // The matcher, which the line follows: as typed `[!x]` takes `X`;
        // with the option it takes neither case.
        assert!(as_typed("photo[!x].jpg").is_match("photoX.jpg"));
        assert!(!ignoring_case("photo[!x].jpg").is_match("photoX.jpg"));
        assert!(!ignoring_case("photo[!x].jpg").is_match("PHOTOX.JPG"));
        assert!(ignoring_case("photo[!x].jpg").is_match("PHOTOY.JPG"));

        for (pattern, class, lost) in [
            ("photo[!x].jpg", "[!x]", "X"),
            ("*[^Q]", "[^Q]", "q"),
            ("[!a-cx]1", "[!a-cx]", "A-C, X"),
            ("[!a-cA]1", "[!a-cA]", "B-C"),
            ("a[!0-9][!b]", "[!b]", "B"),
        ] {
            let shown = Spellings::of(pattern, false).expect("noted");
            assert!(shown.others.is_empty(), "{pattern}: {:?}", shown.others);
            assert_eq!(shown.narrowed, Some((class, lost.to_string())), "{pattern}");
            assert!(
                note(pattern).ends_with(&format!("{class} would stop matching {lost})."))
                    && !note(pattern).contains("also match"),
                "{}",
                note(pattern)
            );
        }
        // A negated class the option leaves alone is no reason for it: the
        // spelling shown says what is true.
        for (pattern, others) in [
            ("photo[!0-9].jpg", &["PHOTO[!0-9].JPG"][..]),
            ("Photo[!xX]", &["photo[!xX]", "PHOTO[!xX]"]),
        ] {
            let shown = Spellings::of(pattern, false).expect("noted");
            assert_eq!(shown.narrowed, None, "{pattern}");
            assert_eq!(shown.others, others, "{pattern}");
        }

        // Whatever is shown as another spelling, READ AS A GLOB, admits
        // only names the option matches — a class standing for any of
        // these characters, a wildcard for one letter.
        let pool = ['x', 'X', 'a', 'A', 'b', 'q', 'Q', '1', '_'];
        for pattern in [
            "photo[!x].jpg",
            "photo[!0-9].jpg",
            "Photo[!xX]",
            "*[a-z]",
            "[abc]x",
            "[A-C_]x",
            "IMG_*.[jJ]pg",
            "a[!0-9][!b]",
            "[!a]x1",
            // cr-ecm5-4: a required directory level is part of the name.
            "*/Cache",
            "Cache/*",
            "*/Cache/*",
            "**/*/Cache",
        ] {
            let widened = ignoring_case(pattern);
            let Some(shown) = Spellings::of(pattern, false) else {
                continue;
            };
            for other in &shown.others {
                let glob = as_typed(other);
                // Every name made by putting a pool character where the
                // spelling has a class, the spelling's own text elsewhere.
                let mut names = vec![String::new()];
                let mut rest = other.as_str();
                while let Some(c) = rest.chars().next() {
                    let (choices, len): (Vec<char>, usize) = match c {
                        '[' => (pool.to_vec(), bracket_class_len(rest).expect("a class")),
                        '*' => (vec!['x'], 1),
                        c => (vec![c], c.len_utf8()),
                    };
                    names = names
                        .iter()
                        .flat_map(|name| choices.iter().map(move |c| format!("{name}{c}")))
                        .collect();
                    rest = &rest[len..];
                }
                for name in names.iter().filter(|name| glob.is_match(name.as_str())) {
                    assert!(
                        widened.is_match(name),
                        "{pattern} shows {other}, which admits {name}; --ignore-case does not \
                         match it"
                    );
                }
            }
        }
    }

    /// cr-ecm4-3: the line describes the pattern the matcher is given. An
    /// absolute exclude is echoed as typed, and what it matches is said of
    /// the pattern it became below the source — the source's own letters
    /// are never shown in another case.
    #[test]
    fn the_case_note_describes_the_pattern_as_the_filter_reads_it() {
        let absolute = Noted {
            typed: "/srv/Data",
            read_as: "Data",
            file: None,
            role: Role::Exclude,
        };
        assert_eq!(
            case_note_of(&[absolute], false).as_deref(),
            Some(
                "\"/srv/Data\" matches Data only; capital letters matter (--ignore-case would \
                 also match data, DATA)."
            )
        );
        // Whether a pattern is spoken of at all is asked of that pattern too.
        let digits = Noted {
            typed: "/srv/Data/2024",
            read_as: "2024",
            file: Some("skip.txt"),
            role: Role::Exclude,
        };
        assert_eq!(case_note_of(&[digits], false), None);
    }

    /// cr-ecm6-2: what the line shows is checked against the FILTER, for
    /// every way a pattern's ends can be written. The line says a pattern
    /// "matches T only" and that `--ignore-case` "would also match S": so
    /// T, read as a pattern, must name nothing the pattern as typed does
    /// not, and each S nothing the pattern does not name once case is
    /// ignored — as an exclude, for a file and for a folder alike. `/**`
    /// was trimmed: for `Cache/**` the line showed `cache`, which names a
    /// FILE of that name; the pattern does not, and a mirror deleted it.
    #[test]
    fn every_spelling_the_line_shows_names_only_what_the_filter_names() {
        use crate::fs_enum::FileFilter;
        use std::path::Path;
        let filter = |pattern: &str, ignore_case: bool| {
            let mut filter = FileFilter::default();
            filter.exclude_files = vec![pattern.to_string()];
            filter.ignore_case = ignore_case;
            filter
                .validate_globs()
                .unwrap_or_else(|e| panic!("{pattern}: {e}"));
            filter
        };
        // (as a file, as a folder)
        let named = |filter: &FileFilter, rel: &str| {
            let path = Path::new(rel);
            (
                !filter.allows_entry(Some(path), path, 1, None),
                !filter.allows_dir(path, path),
            )
        };
        let names = [
            "cache",
            "Cache",
            "CACHE",
            "x/cache",
            "x/Cache",
            "x/CACHE",
            "sub/cache",
            "SUB/CACHE",
            "cache/x",
            "Cache/x",
            "CACHE/x",
            "CACHE/X",
            "x/cache/y",
            "X/CACHE/Y",
            "xcache",
            "XCACHE",
            "cachex",
            "CACHEX",
            "x/y/cache",
            "cache/x/y",
            "CACHE/X/Y",
            "sub/cache/x",
            "SUB/CACHE/X",
            "sub/x/cache",
            "x",
            "sub",
        ];
        let within = |narrow: (bool, bool), wide: (bool, bool)| {
            (!narrow.0 || wide.0) && (!narrow.1 || wide.1)
        };
        let mut shown_patterns = 0;
        for prefix in [
            "", "*", "**/", "*/", "**/*/", "sub/", "*/sub/", "**//", "//", "/", "./", "***/",
        ] {
            for core in ["Cache", "Ca?he", "C[a]che", "Ca*he"] {
                for suffix in ["", "*", "/*", "/**", "/**/*", "/x", "*/**", "//**", "/"] {
                    let pattern = format!("{prefix}{core}{suffix}");
                    // What the rules make of it alone, and what the line
                    // shows once each claim is checked: the rules must
                    // hold without the checks to cover for them.
                    for (how, shown) in [
                        ("by rule", Spellings::by_rule(&pattern, false)),
                        ("checked", Spellings::of(&pattern, cfg!(windows))),
                    ] {
                        let Some(shown) = shown else {
                            continue;
                        };
                        shown_patterns += 1;
                        let (as_typed, ignoring) =
                            (filter(&pattern, false), filter(&pattern, true));
                        let typed = filter(shown.typed, false);
                        for name in names {
                            assert!(
                                within(named(&typed, name), named(&as_typed, name)),
                                "{how}, \"{pattern}\" is shown as {}, which names {name}; the \
                                 pattern does not",
                                shown.typed
                            );
                        }
                        for other in &shown.others {
                            let spelling = filter(other, false);
                            for name in names {
                                assert!(
                                    within(named(&spelling, name), named(&ignoring, name)),
                                    "{how}, \"{pattern}\" shows {other}, which names {name}; \
                                     --ignore-case does not match it"
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(
            shown_patterns > 600,
            "{shown_patterns} patterns were checked"
        );
        // cr-ecm7-2: after `**/` the next separator is the pattern's own.
        for (pattern, windows, shown) in [
            ("**//Cache", false, "/Cache"),
            ("**/Cache", false, "Cache"),
            ("**///Cache", false, "//Cache"),
            (r"**\\Cache", true, r"\Cache"),
            (r"**\Cache", true, "Cache"),
        ] {
            assert_eq!(
                Spellings::by_rule(pattern, windows).map(|s| s.typed),
                Some(shown),
                "{pattern}"
            );
        }
    }

    /// cr-ecm7-2, cr-ecm7-3, cr-ecm7-4: the line, to the letter, for the
    /// patterns where it claimed something the filter does not bear out.
    #[test]
    fn the_case_note_claims_only_what_the_filter_bears_out() {
        // Under this machine's own path rule, the one a claim can be
        // checked under; none of these patterns reads otherwise under the
        // other.
        let note = |patterns: &[&str]| {
            let typed: Vec<(&str, Option<&str>)> = patterns.iter().map(|p| (*p, None)).collect();
            case_note(&typed, cfg!(windows))
        };
        // cr-ecm7-2: the separator after `**/` is the pattern's own. The
        // line showed `Cache` and offered `cache`, which the pattern names
        // neither with the option nor without. It names no entry at all —
        // none is called `/Cache` — so no spelling can be borne out, and
        // the line keeps to words.
        for doubled in ["**//Cache", "//Cache", "Cache//**"] {
            assert_eq!(
                note(&[doubled]),
                Some(format!(
                    "\"{doubled}\" matches exactly as typed; capital letters matter \
                     (--ignore-case reads it otherwise)."
                )),
            );
        }
        // cr-ecm7-3: a pattern the option NARROWS is the one shown,
        // wherever it stands. The line gave `Keep`'s other spellings and
        // only counted the pattern that stops protecting `A.dat`.
        assert_eq!(
            note(&["Keep", "[!a].dat"]).as_deref(),
            Some(
                "\"[!a].dat\" matches exactly as typed, and 1 more pattern matches only as \
                 typed; capital letters matter (--ignore-case changes what they match: [!a] \
                 would stop matching A)."
            )
        );
        assert_eq!(
            note(&["Keep", "*.tmp", "{x,[!a]}.dat"]).as_deref(),
            Some(
                "\"{x,[!a]}.dat\" matches exactly as typed, and 2 more patterns match only as \
                 typed; capital letters matter (--ignore-case changes what they match: [!a] \
                 would stop matching A)."
            )
        );
        // cr-ecm7-4: a pattern the option reads no differently is neither
        // shown nor counted. For `{a,A}` the line offered `{a,a}` and
        // `{A,A}`, both matched already; what changes is `*.Doc`.
        let doc = "\"*.Doc\" matches .Doc only; capital letters matter (--ignore-case would \
                   also match .doc, .DOC).";
        assert_eq!(note(&["{a,A}", "*.Doc"]).as_deref(), Some(doc));
        assert_eq!(note(&["*.Doc", "{a,A}"]).as_deref(), Some(doc));
        for unchanged in [&["{a,A}"][..], &["{a,A}", "{b,B}"], &["*.[Jj][Pp][Gg]"]] {
            assert_eq!(note(unchanged), None, "{unchanged:?}");
        }
        // A spelling the pattern matches already is not offered; one it
        // does not is.
        assert_eq!(
            note(&["{a,A}b"]).as_deref(),
            Some(
                "\"{a,A}b\" matches {a,A}b only; capital letters matter (--ignore-case would \
                 also match {A,A}B)."
            )
        );
        // Every alternative is tried: the letters are in the second.
        assert_eq!(
            note(&["{1,Cache}"]).as_deref(),
            Some(
                "\"{1,Cache}\" matches {1,Cache} only; capital letters matter (--ignore-case \
                 would also match {1,cache}, {1,CACHE})."
            )
        );
        // The option does change it, in a way neither spelling shows
        // (`Ab`): said in words.
        assert_eq!(
            note(&["{ab,AB}"]).as_deref(),
            Some(
                "\"{ab,AB}\" matches exactly as typed; capital letters matter (--ignore-case \
                 reads it otherwise)."
            )
        );
    }

    /// Each check the line rests on, asked directly: the filter is what
    /// answers, whatever the text rules produced.
    #[test]
    fn each_claim_is_checked_against_the_filter() {
        let asked = |pattern: &str| Asked::about(pattern, Role::Exclude).expect("a pattern");
        // The text shown names nothing the pattern does not.
        for (pattern, shown, faithful) in [
            ("**/Cache", "Cache", true),
            ("*Cache", "Cache", true),
            ("Cache*", "Cache", true),
            ("*/Cache", "Cache", false),
            ("Cache/**", "Cache", false),
            ("Cache/*", "Cache", false),
            ("**//Cache", "Cache", false),
            // No entry is called `/Cache`: a text that names nothing is
            // not shown for a pattern either.
            ("**//Cache", "/Cache", false),
            ("sub/Cache", "Cache", false),
        ] {
            assert_eq!(
                asked(pattern).shown_is_faithful(shown, false),
                faithful,
                "\"{pattern}\" shown as {shown}"
            );
        }
        // A spelling is offered only when the option matches it and the
        // pattern as typed does not.
        for (pattern, spelling, offered) in [
            ("*Cache", "cache", true),
            ("*Cache", "CACHE", true),
            ("*Cache", "Cache", false),
            ("{a,A}", "{a,a}", false),
            ("{a,A}", "{A,A}", false),
            ("*/Cache", "cache", false),
            ("photo[!x].jpg", "PHOTO[!x].JPG", false),
            ("photo[!0-9].jpg", "PHOTO[!0-9].JPG", true),
        ] {
            assert_eq!(
                asked(pattern).matched_only_ignoring_case(spelling, false),
                offered,
                "\"{pattern}\" offering {spelling}"
            );
        }
        // A class is said to stop matching a letter only when it does.
        assert!(asked("[!a].dat").stops_matching("[!a].dat", "[!a]", "A", false));
        assert!(asked("photo[!x].jpg").stops_matching("photo[!x].jpg", "[!x]", "X", false));
        assert!(!asked("Keep").stops_matching("Keep", "[!a]", "A", false));
        assert!(!asked("[!0-9]a").stops_matching("[!0-9]a", "[!0-9]", "A", false));
        // And a pattern is spoken of only when the option changes it.
        for (pattern, changes) in [
            ("{a,A}", false),
            ("*.[Jj][Pp][Gg]", false),
            ("{ab,AB}", true),
            ("[A-c]1", true),
            ("Keep", true),
            // However far along the letter that changes stands.
            ("[xX][xX][xX][xX][xX][xX][xX][xX]{ij,IJ}", true),
            ("[xX][xX][xX][xX][xX][xX][xX][xX]{ij,IJ,iJ,Ij}", false),
            // No name to ask with: counted as changed.
            ("**//Cache", true),
        ] {
            assert_eq!(
                asked(pattern).reads_otherwise(pattern, false),
                changes,
                "{pattern}"
            );
        }
        // Names are read off a text's own syntax.
        assert_eq!(
            samples("a{b,{c,D}}*?", false, None).unwrap(),
            ["abxx", "acxx", "aDxx"]
        );
        // A negated class is tried with the other case of a letter it
        // holds as well: the name a spelling goes wrong on.
        assert_eq!(
            samples(r"[!x]\*[q-s]", false, None).unwrap(),
            ["q*q", "X*q"]
        );
        assert_eq!(
            samples("p[!x]q[!x]", false, Some(("[!x]", 'X'))).unwrap(),
            ["pXqq", "pXqX"],
            "the chosen letter, at the first such class"
        );
        assert_eq!(
            samples("{a,b}{c,d}{e,f}{g,h}{i,j}", false, None),
            None,
            "more names than are tried"
        );
        // Only names a walk gives: as a pattern may be written, with `./`
        // before it or a separator after it — and none where a component
        // is empty, `.` or `..`.
        assert_eq!(samples("./Cache/", false, None).unwrap(), ["Cache"]);
        assert_eq!(samples("{/,x}Cache", false, None).unwrap(), ["xCache"]);
        for unwalked in [
            "**//Cache",
            "/Cache",
            "Cache//**",
            "../Cache",
            "a/./b",
            "./",
        ] {
            assert_eq!(
                samples(unwalked, false, None).unwrap(),
                Vec::<String>::new(),
                "{unwalked}"
            );
        }
    }
}
