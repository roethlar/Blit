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
/// single letter `--ignore-case` pairs it with, so every spelling shown is
/// one the option really matches (`straße` is shown `STRAßE`, never
/// `STRASSE`). A bracket class is copied as it is: it already says which
/// letters it takes.
///
/// `None` when no pattern holds a letter with another case (`??`, `*.1`):
/// no case mode would change what such patterns match. Further patterns
/// that do hold one are counted, never listed — the line is one line
/// however many patterns there are.
///
/// Slice 3b (AC-8): each pattern comes with the pattern file it was read
/// from, if any, and the line says so — `"*Cache" (from skip.txt) matches
/// …` for the pattern shown, `and 2 more patterns (from skip.txt)` for the
/// ones counted: the operator did not type those on the command line.
pub(crate) fn case_note(patterns: &[(&str, Option<&str>)], windows: bool) -> Option<String> {
    let mut noted = patterns
        .iter()
        .filter_map(|(pattern, file)| Some((*pattern, *file, Spellings::of(pattern, windows)?)));
    let (pattern, file, spellings) = noted.next()?;
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
    Some(format!(
        "\"{pattern}\"{shown_from} matches {} only{more}; capital letters matter \
         (--ignore-case would also match {}).",
        spellings.typed,
        spellings.others.join(", ")
    ))
}

/// A pattern's text as [`case_note`] shows it, and the other spellings of
/// it that `--ignore-case` would match.
struct Spellings<'a> {
    typed: &'a str,
    others: Vec<String>,
}

impl<'a> Spellings<'a> {
    /// `None` when no letter of `pattern` outside a class has another case.
    fn of(pattern: &'a str, windows: bool) -> Option<Self> {
        let typed = without_end_wildcards(pattern, windows);
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
                    lower.push_str(&rest[..len]);
                    upper.push_str(&rest[..len]);
                    len
                }
                // globset's escape under the Unix rule: the next character
                // is itself — a `[` after it opens no class.
                None if c == '\\' && !windows => {
                    let escaped = rest[1..].chars().next();
                    lower.push(c);
                    upper.push(c);
                    if let Some(escaped) = escaped {
                        lower.push(paired(escaped, escaped.to_lowercase()));
                        upper.push(paired(escaped, escaped.to_uppercase()));
                    }
                    1 + escaped.map_or(0, char::len_utf8)
                }
                None => {
                    lower.push(paired(c, c.to_lowercase()));
                    upper.push(paired(c, c.to_uppercase()));
                    c.len_utf8()
                }
            };
            rest = &rest[len..];
        }
        let mut others = Vec::new();
        for other in [lower, upper] {
            if other != typed && !others.contains(&other) {
                others.push(other);
            }
        }
        (!others.is_empty()).then_some(Self { typed, others })
    }
}

/// `c` in another case — `mapped`, the standard library's lower or upper
/// form of it — when that is ONE letter and one `--ignore-case` pairs with
/// `c` ([`case_variants`]); `c` itself otherwise (`ß`, whose upper form is
/// two letters the option never matches).
fn paired(c: char, mut mapped: impl Iterator<Item = char>) -> char {
    match (mapped.next(), mapped.next()) {
        (Some(one), None) if case_variants(c).contains(&one) => one,
        _ => c,
    }
}

/// `pattern` without the wildcards at its ends, for showing: `*Cache`,
/// `Cache*` and `**/Cache/**` are all shown `Cache`. A wildcard inside the
/// pattern stays (`IMG_*.jpg`), and so does an escaped `*`, which is a
/// star and no wildcard.
fn without_end_wildcards(pattern: &str, windows: bool) -> &str {
    let separator = |c: char| c == '/' || (windows && c == '\\');
    let mut text = pattern;
    let start = text.trim_start_matches('*');
    if start.len() < text.len() {
        text = start.trim_start_matches(separator);
    }
    let end = text.trim_end_matches('*');
    if end.len() < text.len() && (windows || !end.ends_with('\\')) {
        text = end.trim_end_matches(separator);
    }
    text
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
    let holds = |c: char| ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
    // The other case of each run of ASCII letters the class holds.
    let mut added: Vec<(char, char)> = Vec::new();
    for &(lo, hi) in &ranges {
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
        for pattern in ["Cache*", "*Cache*", "**/Cache/**", "**/Cache", "Cache/**"] {
            assert_eq!(
                note(&[pattern]),
                line(&format!("\"{pattern}\" matches Cache"), "", "cache, CACHE"),
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
            note(&["/srv/Data"]),
            line(
                "\"/srv/Data\" matches /srv/Data",
                "",
                "/srv/data, /SRV/DATA"
            )
        );
        // AC-5: no letter, no line.
        for patterns in [
            &["??"][..],
            &["*.1"],
            &["??", "*.1", "*", "**", "[abc]", "{1,2}"],
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
        // `ß` has no single upper letter: it stays, and `SS` is never shown.
        assert_eq!(others("straße"), ["STRAßE"]);
        // The sigmas pair one to one; the final sigma lowers to itself.
        assert_eq!(others("σ*"), ["Σ"]);
        assert_eq!(others("Σ"), ["σ"]);
        assert_eq!(others("Übung"), ["übung", "ÜBUNG"]);
        // A class is copied as it is; a pattern whose only letters are in
        // one gets no note.
        assert_eq!(others("[Cc]ache"), ["[Cc]ACHE"]);
        assert!(others("[abc]").is_empty());
        // An escaped character under the Unix rule is itself: an escaped
        // star at the end is no wildcard, an escaped bracket opens no class.
        assert_eq!(others(r"Cache\*"), [r"cache\*", r"CACHE\*"]);
        assert_eq!(others(r"\[Ab]"), [r"\[ab]", r"\[AB]"]);
        // On Windows `\` separates; the wildcard ends are trimmed with it.
        assert_eq!(
            Spellings::of(r"**\Cache\**", true).map(|s| s.typed),
            Some("Cache")
        );

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
}
