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
    let all: Vec<String> = patterns
        .iter()
        .map(|&(pattern, _)| pattern.to_string())
        .collect();
    case_note_of(
        &noted,
        Whole {
            excludes: &all,
            includes: &[],
            refused_with_the_option: false,
        },
        windows,
    )
}

/// cr-ecm9-2: the command's patterns as its filter holds them, list by
/// list — what a claim about ONE pattern is judged against. A name the
/// option would add through one pattern is no gain when another pattern
/// names it already (`--exclude a --exclude A`: `A` was offered as an
/// additional match, and both runs kept it), and a name one pattern stops
/// matching is no loss when another still names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Whole<'a> {
    /// The excludes the filter applies (resolved, in order).
    pub excludes: &'a [String],
    /// The includes it applies.
    pub includes: &'a [String],
    /// cr-ecm13-6: the command WITH the option is refused for a pattern
    /// that is in neither list — an absolute exclude that lies outside the
    /// source matches nothing and is in no filter, and is still held to
    /// being a pattern when the command is read. What the two lists make
    /// of the option says nothing of a command that could not run.
    pub refused_with_the_option: bool,
}

/// The run's COMPLETE filter — both lists in one, as the run applies
/// them — as typed and, made only when something has to be asked of it,
/// as `--ignore-case` would read it.
///
/// cr-ecm10-2: one filter and not one for each list. What a run does with
/// a name is decided by the includes AND the excludes: with `--exclude
/// '[!a].bin' --exclude '[!b].txt' --include '*.txt'` the line named the
/// loss of `A` at `[!a].bin`, and both runs left `A.bin` alone, no include
/// naming it — the file the option gave up was `B.txt`.
///
/// The lists are the ones the run's filter holds, so the sets compiled for
/// the run are the ones asked here; nothing is compiled twice.
struct Complete {
    excludes: Vec<String>,
    includes: Vec<String>,
    /// [`Whole::refused_with_the_option`].
    refused_elsewhere: bool,
    typed: crate::fs_enum::FileFilter,
    ignoring: std::cell::OnceCell<crate::fs_enum::FileFilter>,
}

impl Complete {
    fn of(whole: Whole) -> std::rc::Rc<Self> {
        std::rc::Rc::new(Self {
            excludes: whole.excludes.to_vec(),
            includes: whole.includes.to_vec(),
            refused_elsewhere: whole.refused_with_the_option,
            typed: Self::filter(whole.excludes, whole.includes, false),
            ignoring: std::cell::OnceCell::new(),
        })
    }

    /// The pattern alone, as an exclude: what a claim about one pattern
    /// is judged against when it is the whole filter.
    #[cfg(test)]
    fn alone(pattern: &str) -> std::rc::Rc<Self> {
        Self::of(Whole {
            excludes: &[pattern.to_string()],
            includes: &[],
            refused_with_the_option: false,
        })
    }

    fn filter(
        excludes: &[String],
        includes: &[String],
        ignore_case: bool,
    ) -> crate::fs_enum::FileFilter {
        let mut filter = crate::fs_enum::FileFilter::default();
        filter.exclude_files = excludes.to_vec();
        filter.include_files = includes.to_vec();
        filter.ignore_case = ignore_case;
        filter
    }

    /// The filter as the option would read it — or `None` when it holds
    /// more than [`MOST_PATTERNS_WIDENED`] patterns (cr-ecm11-6). It is
    /// the one thing here the run does not build for itself, and building
    /// it costs by the pattern: asked once, about one name, it compiled
    /// 10,000 includes a second time and one line took 120 ms. Past the
    /// bound nothing is asked of it, and what it would have settled is
    /// not claimed.
    fn ignoring(&self) -> Option<&crate::fs_enum::FileFilter> {
        if self.excludes.len() + self.includes.len() > MOST_PATTERNS_WIDENED {
            return None;
        }
        Some(self.ignoring.get_or_init(|| {
            #[cfg(test)]
            WIDENED.with(|built| built.set(built.get() + 1));
            Self::filter(&self.excludes, &self.includes, true)
        }))
    }

    /// Whether the run's filter can be read with the option AT ALL: every
    /// pattern of it is still a pattern when widened (cr-ecm12-5). Where
    /// it is not, nothing is said of what the option would do but the
    /// rule: beside an `s` inside 249 pairs of braces, which the option
    /// cannot widen, `*Cache` was offered `cache, CACHE`, and the command
    /// with `--ignore-case` is refused whole. Where it cannot be told —
    /// past the bound — nothing concrete is said either; a class that
    /// could take less is still named, as one that MAY.
    ///
    /// cr-ecm13-6: nor where the command would be refused for a pattern
    /// that is in neither list.
    fn with_the_option(&self) -> WithTheOption {
        if self.refused_elsewhere {
            return WithTheOption::Refused;
        }
        match self.ignoring() {
            None => WithTheOption::NotKnown,
            Some(filter) if filter.validate_globs().is_ok() => WithTheOption::Reads,
            Some(_) => WithTheOption::Refused,
        }
    }
}

/// Whether a run's filter could be read with `--ignore-case` at all
/// ([`Complete::with_the_option`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WithTheOption {
    /// Every pattern of it is still a pattern when widened.
    Reads,
    /// Some pattern is not: the option would refuse the command.
    Refused,
    /// It holds too many patterns to find out for one line.
    NotKnown,
}

/// The most patterns a run's filter may hold and still be built a second
/// time, as `--ignore-case` would read it, for the line's sake
/// ([`Complete::ignoring`]). Measured: about 12 microseconds a pattern.
const MOST_PATTERNS_WIDENED: usize = 64;

// How many times a run's whole filter was built as the option would read
// it, on this thread (cr-ecm11-6).
#[cfg(test)]
thread_local! {
    static WIDENED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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
/// - a pattern whose reading the option does not change at all is not
///   shown. (cr-ecm11-5: it is COUNTED with the others, by its text; it
///   was left out of the count only when it had been asked about, which
///   depended on where it stood.)
///
/// cr-ecm10-1 — an eighth round, and what the seven before it show. Each
/// made the line check more of what it says against the filter, and each
/// check was sound for the cases it was written for and unsound beside
/// them: what a set of globs matches over every path is not settled by
/// trying names. So the line no longer CONCLUDES anything; it only shows,
/// and what it cannot show it says in words that are true of every
/// pattern with a letter in it:
/// - an example — a spelling the option would also match, a letter a
///   class would stop matching — is shown only when ONE CONCRETE NAME was
///   put to the run's complete filter, both lists, in both modes, and the
///   run would treat it otherwise ([`Asked::gained`], [`Asked::lost`]);
/// - a pattern is left out of the line only when its OWN two readings are
///   proved the same (`{a,A}`), never because another pattern seems to
///   cover it: `--exclude a --exclude ./A` got no line, `./A` covering
///   `A` at the root, and a mirror deleted `sub/A`, which the option keeps;
/// - where a class could take less and no name shows it, the warning
///   stays, and says `may`: with `Keep`, `[!a]*.txt` and `Ax.txt` the one
///   name tried was one the third pattern keeps, the line spoke of `Keep`,
///   and the option gave up `Ay.txt`;
/// - anything else is the plain rule.
///
/// cr-ecm10-6: and the whole is bounded. [`MOST_PATTERNS_ASKED`] patterns
/// are asked about for what to show, however many the command holds —
/// with 1,000 excludes each covered by another pattern, every one was
/// asked about in turn, and the line cost 443 ms.
pub(crate) fn case_note_of(patterns: &[Noted], whole: Whole, windows: bool) -> Option<String> {
    let whole = Complete::of(whole);
    // By the text alone, and cheaply, for every pattern: the ones the
    // option could read otherwise. Only the ones looked at for showing are
    // checked against the filter.
    let candidates: Vec<&Noted> = patterns
        .iter()
        .filter(|noted| widen_case(noted.read_as, windows) != noted.read_as)
        .collect();
    fn read<'a>(
        noted: &Noted<'a>,
        windows: bool,
        whole: &std::rc::Rc<Complete>,
    ) -> Option<Spellings<'a>> {
        Spellings::of_role(noted.read_as, noted.role, windows, whole)
    }
    let mut shown: Option<(usize, Spellings)> = None;
    // cr-ecm12-5: only of a run the option could make. Where the filter
    // as the option would read it does not validate, or is too long to
    // find out, no pattern is asked about and the line is the rule.
    let with_the_option = if candidates.is_empty() {
        WithTheOption::Refused
    } else {
        whole.with_the_option()
    };
    let asked_about = match with_the_option {
        WithTheOption::Reads => MOST_PATTERNS_ASKED,
        WithTheOption::Refused | WithTheOption::NotKnown => 0,
    };
    // A loss first: a pattern the option would NARROW, wherever it stands
    // — borne out by a name if one can be, and said with `may` if not.
    let mut may_narrow: Option<(usize, Spellings)> = None;
    let narrowing = candidates
        .iter()
        .enumerate()
        .filter(|(_, noted)| has_narrowed_class(noted.read_as, windows))
        .take(asked_about);
    for (at, noted) in narrowing {
        let Some(spellings) = read(noted, windows, &whole) else {
            continue;
        };
        if spellings.narrowed.is_some() && spellings.narrowing_borne_out {
            shown = Some((at, spellings));
            break;
        }
        if spellings.narrowed.is_some() {
            may_narrow.get_or_insert((at, spellings));
        }
    }
    shown = shown.or(may_narrow);
    // Else the first pattern with a spelling the run would also match;
    // else the first the option reads otherwise, in words.
    if shown.is_none() {
        let mut in_words: Option<(usize, Spellings)> = None;
        for (at, noted) in candidates.iter().enumerate().take(asked_about) {
            // A pattern whose own two readings are proved the same
            // (`{a,A}`) has nothing to show.
            let Some(spellings) = read(noted, windows, &whole) else {
                continue;
            };
            if !spellings.others.is_empty() {
                shown = Some((at, spellings));
                break;
            }
            in_words.get_or_insert((at, spellings));
        }
        shown = shown.or(in_words);
    }
    // Not known whether the option could run at all: no name is asked
    // about, and a class that would take less BY ITS TEXT is still named,
    // with `may` — a warning that protection may shrink is kept wherever
    // it cannot be ruled out (cr-ecm10-1).
    if shown.is_none() && with_the_option == WithTheOption::NotKnown {
        shown = candidates
            .iter()
            .enumerate()
            .filter(|(_, noted)| has_narrowed_class(noted.read_as, windows))
            .take(MOST_PATTERNS_ASKED)
            .find_map(|(at, noted)| {
                let mut by_text = Spellings::with(noted.read_as, windows, None)?;
                by_text.narrowed.as_ref()?;
                by_text.narrowing_borne_out = false;
                Some((at, by_text))
            });
    }
    // Past what is asked about, nothing is known of a pattern but that it
    // holds a letter: the plain rule, about the first of them.
    if shown.is_none() {
        shown = candidates
            .iter()
            .enumerate()
            .skip(asked_about)
            .map(|(at, noted)| (at, Spellings::in_words(noted.read_as)))
            .next();
    }
    let (shown_at, spellings) = shown?;
    let (pattern, file) = (candidates[shown_at].typed, candidates[shown_at].file);
    // The patterns that are only counted: EVERY other one the option
    // could read otherwise, by its text — which is what was typed, and
    // costs nothing to tell.
    //
    // cr-ecm11-5: by the text and nothing else. A pattern whose two
    // readings are the same (`{a,A}`) used to be left out of the count —
    // when it had been asked about, and only so many are: with 33
    // patterns the count was "1 more" or "2 more" by where `{z,2}`
    // stood among them. What is counted is how many more patterns the
    // rule is about, and the rule is about each of them.
    let noted = candidates
        .iter()
        .enumerate()
        .filter(|(at, _)| *at != shown_at)
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
        // cr-ecm10-1: `would` for a letter a name bore out; `may` for one
        // the class would lose by its text, which no name tried showed.
        let would = if spellings.narrowing_borne_out {
            "would"
        } else {
            "may"
        };
        format!(
            "\"{pattern}\"{shown_from} matches exactly as typed{more}; capital letters matter \
             (--ignore-case changes what {they}: {class} {would} stop matching {lost})."
        )
    } else if spellings.others.is_empty() {
        // cr-ecm8-2: the rule, and no more. No spelling the option would
        // also match could be borne out, and that it would match none is
        // not established either — so nothing is said about THIS pattern
        // beyond how it is read, and what the option does.
        format!(
            "\"{pattern}\"{shown_from} matches exactly as typed{more}; capital letters matter \
             (--ignore-case ignores them)."
        )
    } else if !spellings.others_are_spellings {
        // cr-ecm11-3: a name the run would treat otherwise, which is not
        // the text the pattern would be shown as: nothing is said of what
        // the pattern matches "only".
        format!(
            "\"{pattern}\"{shown_from} matches exactly as typed{more}; capital letters matter \
             (--ignore-case would also match {}).",
            spellings.others.join(", ")
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
    /// cr-ecm4-3: a negated class `--ignore-case` would make take less,
    /// with the letters it would stop matching (`[!x]`, `X`). When there
    /// is one, `others` is empty: no spelling of the pattern says what the
    /// option does to it.
    narrowed: Option<(&'a str, String)>,
    /// cr-ecm10-1: a name was found that the run stops naming there — the
    /// letters are the ones borne out. Otherwise they are the ones the
    /// class would lose by its text, and the line says `may`.
    narrowing_borne_out: bool,
    /// cr-ecm11-3: `others` are spellings of the text shown as `typed`,
    /// each the very name that showed the change (or a pattern, when a
    /// wildcard is left in it). Where one is instead ANOTHER name the
    /// spelling denotes, the line does not say what the pattern "matches
    /// only": the two halves would not be about the same thing.
    others_are_spellings: bool,
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
        Self::of_role(pattern, Role::Exclude, windows, &Complete::alone(pattern))
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
    ///   and stand for a name the RUN treats otherwise with the option
    ///   ([`Asked::gained`]) — else it is dropped;
    /// - a class is said to stop matching a letter when a name shows the
    ///   run stops naming it, where that class stands in the pattern
    ///   ([`Asked::lost_here`]);
    /// - and a pattern is left out (`None`) only when it is ESTABLISHED
    ///   that the option changes nothing it names ([`Asked::compared`]:
    ///   `{a,A}` already takes both). cr-ecm8-2: it was left out whenever
    ///   the names tried showed no difference, which is not the same —
    ///   `{Σ,σ}` got no line, and `--ignore-case` also takes `ς`. When
    ///   neither a difference nor its absence can be shown, the line
    ///   states the rule and claims nothing more.
    ///
    /// Checked with the filter itself ([`FileFilter::probing`]), over
    /// names read off each text's own syntax ([`samples`]). Under the
    /// other platform's path rule nothing can be asked of this machine's
    /// filter, and the rules stand unchecked. That is the one case in
    /// which they do.
    fn of_role(
        pattern: &'a str,
        role: Role,
        windows: bool,
        whole: &std::rc::Rc<Complete>,
    ) -> Option<Self> {
        if windows != cfg!(windows) {
            return Self::with(pattern, windows, None);
        }
        match Asked::about(pattern, role, whole) {
            Some(asked) => Self::with(pattern, windows, Some(&asked)),
            // cr-ecm11-4: the pattern cannot be put to the filter in one
            // of its two readings, so nothing can be asked — and nothing
            // is shown of it but the rule. It was shown BY RULE, unchecked:
            // an `s` inside 249 pairs of braces is a pattern as typed, the
            // line offered `S` for it, and the same command with
            // `--ignore-case` is refused, the pattern too deep to widen.
            None => (widen_case(pattern, windows) != pattern).then(|| Self::in_words(pattern)),
        }
    }

    /// What the rules alone make of an exclude, nothing checked: so that a
    /// rule that goes wrong is seen to, and not only covered for.
    #[cfg(test)]
    fn by_rule(pattern: &'a str, windows: bool) -> Option<Self> {
        Self::with(pattern, windows, None)
    }

    /// A pattern nothing was asked about: shown whole, and nothing said of
    /// it but the rule.
    fn in_words(pattern: &'a str) -> Self {
        Self {
            typed: pattern,
            others: Vec::new(),
            narrowed: None,
            narrowing_borne_out: false,
            others_are_spellings: true,
        }
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
        let (lower, upper) = in_both_cases(typed, windows);
        // cr-ecm10-5: what is CHECKED for a spelling is the whole pattern
        // in that case — what the text shown stands for — and the names
        // that pattern itself denotes. The text shown (`.O` for `*.O`) was
        // checked beside names taken from the pattern and re-cased, which
        // a spelling holding a class does not denote: `Q[b-~]` was offered
        // on the strength of `QB`, and is the very glob typed beside it.
        let (whole_lower, whole_upper) = if typed == pattern {
            (lower.clone(), upper.clone())
        } else {
            in_both_cases(pattern, windows)
        };
        // cr-ecm8-2: every negated class of the pattern that would take
        // less, and the first of them the filter bears out WHERE IT STANDS.
        // The first such class was credited when any name at all stopped
        // matching: for `{[!a].1,A.1,[!b].2}` that was `[!a]` and `A`,
        // though `A.1` stays matched by the alternative beside it and it
        // is `B.2` that is lost.
        let candidates = narrowed_classes(pattern, windows);
        // A spelling made by rule copies such a class as it is written,
        // and so reads as admitting the letters the option would refuse
        // there (cr-ecm4-3). None is offered for a pattern that holds one,
        // whether or not the narrowing itself is borne out.
        let takes_less_somewhere = !candidates.is_empty();
        // cr-ecm9-2: the claim is the letters that ARE lost there — one is
        // enough. cr-ecm10-1: and where no name shows any, the class that
        // would lose some by its text is still named, as one that MAY: a
        // name that happened to be kept by another pattern is no proof
        // that every name is.
        let borne_out = asked.and_then(|asked| {
            candidates.iter().find_map(|&(at, class, _)| {
                let lost = asked.lost_here(pattern, at, class, windows);
                (!lost.is_empty()).then(|| (class, written(&lost)))
            })
        });
        let narrowing_borne_out = borne_out.is_some() || asked.is_none();
        let narrowed: Option<(&'a str, String)> = borne_out.or_else(|| {
            candidates
                .into_iter()
                .next()
                .map(|(_, class, lost)| (class, lost))
        });
        let mut others = Vec::new();
        let mut others_are_spellings = true;
        if !takes_less_somewhere {
            for (other, whole) in [(lower, whole_lower), (upper, whole_upper)] {
                if other == typed {
                    continue;
                }
                // cr-ecm11-3: what is printed is what was checked. The
                // spelling stands when it is itself the name the run would
                // treat otherwise (`cache`), or is a pattern some name of
                // which is (`IMG_*.JPG`). Where it reads as one name and
                // the name that showed the change is another, THAT name
                // is printed: beside `--exclude '[Aa]'`, `--include '*a'`
                // was said to gain `A`, which both runs leave out — the
                // name checked, and the file the option took, was `xA`.
                let shown =
                    match asked {
                        None => Some(other),
                        Some(asked) => asked.would_also_match(&whole, &other, windows).map(
                            |(name, its_own)| {
                                // cr-ecm12-4: its own name, and that alone
                                // (cr-ecm13-7: and only where every name
                                // the text reads as is one).
                                // A text that still holds a class was kept
                                // as "a pattern": `[A]` was printed for
                                // `*[a]`, which as a name is `A` — and the
                                // name checked was `xA`.
                                if its_own {
                                    other
                                } else {
                                    others_are_spellings = false;
                                    name
                                }
                            },
                        ),
                    };
                if let Some(shown) = shown.filter(|shown| !others.contains(shown)) {
                    others.push(shown);
                }
            }
        }
        if let Some(asked) = asked {
            // cr-ecm10-6: only a pattern that spells out alternatives can
            // read the same both ways, or differ where neither spelling
            // reaches — and comparing costs up to a thousand spellings.
            // Plain text is not put through it for an example nobody typed.
            if others.is_empty() && narrowed.is_none() && pattern.contains('{') {
                match asked.compared(pattern, windows) {
                    Compared::Same => return None,
                    // Names the run would treat otherwise with the option.
                    Compared::Differs(names) => others = names,
                    Compared::Unknown => {}
                }
            }
        }
        Some(Self {
            typed,
            others,
            narrowed,
            narrowing_borne_out,
            others_are_spellings,
        })
    }
}

/// `text` with its letters all lower, and all upper: each letter changed
/// for the one `--ignore-case` pairs it with, a class for the class of its
/// letters' other case.
fn in_both_cases(text: &str, windows: bool) -> (String, String) {
    let (mut lower, mut upper) = (String::new(), String::new());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let class = if c == '[' {
            bracket_class_len(rest)
        } else {
            None
        };
        let len = match class {
            Some(len) => {
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
    (lower, upper)
}

/// Every negated class of `pattern` that `--ignore-case` would make take
/// less — by its text alone ([`narrowed_class`]) — each with the byte it
/// begins at, the class as written, and the letters it would lose. A class
/// inside braces is one of them.
fn narrowed_classes(pattern: &str, windows: bool) -> Vec<(usize, &str, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(c) = pattern[at..].chars().next() {
        let rest = &pattern[at..];
        let len = match c {
            '[' => match bracket_class_len(rest) {
                Some(len) => {
                    if let Some(lost) = narrowed_class(&rest[..len]) {
                        found.push((at, &rest[..len], lost));
                    }
                    len
                }
                None => 1,
            },
            '\\' if !windows => 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
            c => c.len_utf8(),
        };
        at += len;
    }
    found
}

/// Whether `pattern` holds such a class; cheap enough to ask of every
/// pattern.
fn has_narrowed_class(pattern: &str, windows: bool) -> bool {
    !narrowed_classes(pattern, windows).is_empty()
}

/// How many of a run's counted patterns are asked about (the ones holding
/// braces or a class), beyond the one shown.
const MOST_PATTERNS_ASKED: usize = 16;

/// One pattern as the filter holds it, to ask what it names: as typed,
/// and as `--ignore-case` would read it.
struct Asked {
    role: Role,
    /// The pattern's own text, as typed (cr-ecm15-2).
    typed: String,
    as_typed: crate::fs_enum::FileFilter,
    ignoring_case: crate::fs_enum::FileFilter,
    /// The run's complete filter (cr-ecm9-2, cr-ecm10-2): what the option
    /// changes is judged by what the RUN does with a name, not by what
    /// one pattern, or one list, says of it.
    whole: std::rc::Rc<Complete>,
}

/// What a filter says of one name: taken as a file, and taken as a folder.
type Named = (bool, bool);

/// What a `*` is tried as when names are read off a pattern: some text,
/// and none. cr-ecm10-1: one text in its place found `Ax.txt` for
/// `[!a]*.txt`, a name another pattern kept, and never `A.txt`.
const STARS: [&str; 2] = ["x", ""];

// How many questions have been put to a filter on this thread: the
// line's cost, counted where it is spent (cr-ecm10-6).
#[cfg(test)]
thread_local! {
    static QUESTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl Asked {
    fn about(pattern: &str, role: Role, whole: &std::rc::Rc<Complete>) -> Option<Self> {
        let probing = |ignore_case| {
            crate::fs_enum::FileFilter::probing(pattern, role == Role::Exclude, ignore_case)
        };
        Some(Self {
            role,
            typed: pattern.to_string(),
            as_typed: probing(false)?,
            ignoring_case: probing(true)?,
            whole: std::rc::Rc::clone(whole),
        })
    }

    /// Whether `filter` NAMES `name` the way a pattern of `role` does: an
    /// exclude leaves it out, an include lets it in. Asked of a filter
    /// holding one pattern, that is what the pattern matches; asked of
    /// the complete filter, it is what the run does.
    fn named(role: Role, filter: &crate::fs_enum::FileFilter, name: &str) -> Named {
        #[cfg(test)]
        QUESTIONS.with(|asked| asked.set(asked.get() + 1));
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

    /// The RUN would treat `name` otherwise with the option, through this
    /// pattern: the pattern names it with the option, and the complete
    /// filter — both lists (cr-ecm10-2) — names it with the option and not
    /// as typed.
    ///
    /// An exclude that names an entry leaves it out whatever else the
    /// filter holds, so only the filter as typed is asked: the sets the
    /// run compiled already. An include lets an entry in only if no
    /// exclude, read the same way, takes it out again, and the filter as
    /// the option would read it is asked — made then, and not before.
    fn gained(&self, name: &str) -> bool {
        let by_pattern = Self::named(self.role, &self.ignoring_case, name);
        if !(by_pattern.0 || by_pattern.1) {
            return false;
        }
        let typed = Self::named(self.role, &self.whole.typed, name);
        let with = match self.role {
            Role::Exclude => by_pattern,
            // cr-ecm11-6: past the bound it cannot be asked, and nothing
            // is claimed.
            Role::Include => match self.whole.ignoring() {
                Some(filter) => Self::named(self.role, filter, name),
                None => return false,
            },
        };
        (by_pattern.0 && with.0 && !typed.0) || (by_pattern.1 && with.1 && !typed.1)
    }

    /// The RUN stops naming the FILE `name` with the option, through this
    /// pattern: the pattern names it as typed and not with the option, and
    /// so does the complete filter — both lists (cr-ecm10-2).
    ///
    /// A file, and not a folder of that name. An include says nothing of
    /// folders, so beside one an exclude can stop naming a FOLDER the run
    /// goes on leaving every file of alone; that is no loss to show before
    /// a file's (`A.bin` beside `--include '*.txt'`).
    fn lost(&self, name: &str) -> bool {
        let named = |filter| Self::named(self.role, filter, name).0;
        named(&self.as_typed)
            && !named(&self.ignoring_case)
            && named(&self.whole.typed)
            // cr-ecm11-6: not shown past the bound; the class is then
            // one that MAY take less.
            && self.whole.ignoring().is_some_and(|filter| !named(filter))
    }

    /// `spelling` — the WHOLE pattern with its letters all lowered, or all
    /// raised — names only what the option would match through this
    /// pattern, and some name it denotes is one the run would treat
    /// otherwise with the option ([`Self::gained`]): that NAME, or `None`.
    ///
    /// cr-ecm10-5: the names are the ones the spelling itself denotes,
    /// with each `*` tried as some text and as none.
    ///
    /// cr-ecm11-3: and the name is given back, to be printed. It was
    /// reduced to "there is one", and the text printed was the spelling —
    /// which, read as a name, can be one the run treats the same either
    /// way. The first tried is what `shown`, the spelling as the line
    /// would print it, reads as; then the spelling with nothing for a
    /// `*`; then with some text.
    fn would_also_match(
        &self,
        spelling: &str,
        shown: &str,
        windows: bool,
    ) -> Option<(String, bool)> {
        let filter =
            crate::fs_enum::FileFilter::probing(spelling, self.role == Role::Exclude, false)?;
        let mut names = samples_with(shown, windows, None, "x").unwrap_or_default();
        // Those are the text's OWN names (`cache` for `./cache`): found
        // among them, the name is the text shown.
        let own = names.len();
        for star in ["", "x"] {
            let denoted = samples_with(spelling, windows, None, star)?;
            for name in &denoted {
                let by_spelling = Self::named(self.role, &filter, name);
                let ignoring = Self::named(self.role, &self.ignoring_case, name);
                if (by_spelling.0 && !ignoring.0) || (by_spelling.1 && !ignoring.1) {
                    return None;
                }
            }
            names.extend(denoted);
        }
        let changes = |name: &String| {
            let by_spelling = Self::named(self.role, &filter, name);
            (by_spelling.0 || by_spelling.1) && self.gained(name)
        };
        // cr-ecm13-7: the text shown stands for ALL the names it reads
        // as, so it is the text to print only when every one of them is
        // treated otherwise. `{A,B}` was printed for `{a,b}` beside
        // `[Aa]`: `B` is one of its names and is gained, `A` is another
        // and is left out by both runs. Where only some change, the name
        // that was checked is what is printed.
        //
        // cr-ecm14-5: and only when the names tried are all it can be
        // answered for. That is so for a text of letters and alternatives
        // — its names are the ones tried — and for any text when this
        // pattern is the run's WHOLE filter: there is then no other
        // pattern to treat one of its names otherwise. Beside other
        // patterns a text that still holds a wildcard or a class reads as
        // names nobody tried: `A?B` was printed for `--include 'a?b'`
        // beside `--exclude '?1?'` on the strength of `AxB`, and `A1B`,
        // which it reads as too, is left out by both runs. There the name
        // that was tried is what is printed.
        //
        // cr-ecm15-2: and even the run's whole filter has one pattern
        // that can treat such a name the same — ITSELF, as typed. `1*A*2`
        // was printed for a sole `--include '1*a*2'`: it reads as `1aA2`,
        // which the pattern has as typed (its `a`, and the `A` in a `*`).
        // The text stands for a sole pattern only where no name it reads
        // as is one of those; otherwise the name that was checked,
        // `1xAx2`, is what is printed.
        //
        // cr-ecm16-4: and that is PROVED, not searched for. A search
        // through `*` alone missed `{A?,?B}` for `--include '{a?,?b}'`: it
        // reads as `Ab`, which `?b` has as typed. The spelling is printed
        // only where the two texts cannot name one path
        // ([`cannot_share_a_name`]).
        let sole = self.whole.excludes.len() + self.whole.includes.len() == 1;
        let whole_text = names[..own].iter().all(changes)
            && (only_text_and_alternatives(shown, windows)
                || (sole && cannot_share_a_name(&self.typed, spelling, windows)));
        names
            .into_iter()
            .enumerate()
            .find(|(_, name)| changes(name))
            .map(|(at, name)| (name, at < own && whole_text))
    }

    /// The letters the class that begins at byte `at` of the pattern is
    /// SHOWN to stop matching: each of those it would lose by its text for
    /// which some name with that letter where this class stands — read
    /// through the alternative that holds the class, and no other — is one
    /// the run stops naming with the option ([`Self::lost`]). cr-ecm10-1:
    /// each `*` on the way is tried as some text and as none.
    fn lost_here(&self, pattern: &str, at: usize, class: &str, windows: bool) -> Vec<char> {
        let Some(letters) = lost_letters(class) else {
            return Vec::new();
        };
        letters
            .into_iter()
            .filter(|&letter| {
                STARS.iter().any(|star| {
                    samples_with(pattern, windows, Some((at, letter)), star)
                        .is_some_and(|names| names.iter().any(|name| self.lost(name)))
                })
            })
            .collect()
    }

    /// cr-ecm8-2: both readings of the pattern, asked about EVERY name
    /// they could differ on — which can be done for a pattern of plain
    /// text and alternatives, with no wildcard and no class. Such a
    /// pattern names a finite set of names, the option adds every other
    /// case of each of their letters and nothing else, and all of those
    /// spellings can be put to the two filters. Then either no name is
    /// read differently ([`Compared::Same`]: `{a,A}`), or some are
    /// ([`Compared::Differs`]), and the ones the RUN would treat
    /// otherwise are the examples: `ς` for `{Σ,σ}`, which no change to
    /// lower or upper case reaches.
    ///
    /// cr-ecm10-1: the pattern's OWN readings, and no other pattern's. A
    /// pattern was called the same when the list as typed named every
    /// such spelling — at the root, where the names were tried.
    ///
    /// For any other pattern, and past [`MOST_SPELLINGS`], the answer is
    /// [`Compared::Unknown`] — never "the same" on the strength of the
    /// names that happened to be tried.
    fn compared(&self, pattern: &str, windows: bool) -> Compared {
        if !only_text_and_alternatives(pattern, windows) {
            return Compared::Unknown;
        }
        let Some(names) = samples(pattern, windows, None).filter(|names| !names.is_empty()) else {
            return Compared::Unknown;
        };
        // Each name's letters, each with every case the option pairs it
        // with — and how many spellings that makes in all.
        let mut spelled: Vec<Vec<Vec<char>>> = Vec::with_capacity(names.len());
        let mut total = 0usize;
        for name in &names {
            let letters: Vec<Vec<char>> = name
                .chars()
                .map(|c| {
                    let mut variants = case_variants(c);
                    if !variants.contains(&c) {
                        variants.push(c);
                    }
                    variants
                })
                .collect();
            let spellings = letters
                .iter()
                .try_fold(1usize, |all, variants| all.checked_mul(variants.len()));
            total = match spellings.and_then(|spellings| total.checked_add(spellings)) {
                Some(total) if total <= MOST_SPELLINGS => total,
                _ => return Compared::Unknown,
            };
            spelled.push(letters);
        }
        let mut gained: Vec<String> = Vec::new();
        let mut differs = false;
        for letters in &spelled {
            let spellings: usize = letters.iter().map(Vec::len).product();
            for mut nth in 0..spellings {
                let name: String = letters
                    .iter()
                    .map(|variants| {
                        let letter = variants[nth % variants.len()];
                        nth /= variants.len();
                        letter
                    })
                    .collect();
                let (typed, ignoring) = (
                    Self::named(self.role, &self.as_typed, &name),
                    Self::named(self.role, &self.ignoring_case, &name),
                );
                if typed == ignoring {
                    continue;
                }
                differs = true;
                if gained.len() < MOST_EXAMPLES && !gained.contains(&name) && self.gained(&name) {
                    gained.push(name);
                }
            }
        }
        if differs {
            Compared::Differs(gained)
        } else {
            Compared::Same
        }
    }
}

/// What asking both readings of a pattern about every name they could
/// differ on found ([`Asked::compared`]).
#[derive(Debug, PartialEq, Eq)]
enum Compared {
    /// No name is read differently: the option changes nothing here.
    Same,
    /// Some name is read differently; these are names among them that the
    /// run would treat otherwise with the option — none, when every one
    /// of the few looked at is a name the filter already names.
    Differs(Vec<String>),
    /// Not every such name could be asked about.
    Unknown,
}

/// The most spellings [`Asked::compared`] puts to the filters for one
/// pattern; past it the comparison is not made.
const MOST_SPELLINGS: usize = 1024;

/// The most names it gives back as examples.
const MOST_EXAMPLES: usize = 2;

/// cr-ecm16-4, cr-ecm17-4: whether `typed` and `spelling` — one pattern,
/// and that pattern with the case of its letters changed — can be shown
/// to MATCH no path in common. `false` where it cannot be shown: the
/// caller then prints a name it checked, which is true whatever the
/// answer.
///
/// What a run's filter matches, measured through the filter itself:
/// - every pattern ends where the path ends;
/// - a `*` crosses separators (`*/Cache/*` matches `x/Cache/cache/saved`,
///   and so does `*/cache/*`);
/// - a pattern with a separator in it (a trailing one aside) starts where
///   the path starts — it is anchored at the root — for no entry's own
///   name holds one. cr-ecm19-5: a separator it MUST match, that is — a
///   `/` itself; a class that may hold one (`[x/]`) may hold another
///   character instead, and the pattern then matches an entry's own name
///   at any depth (`a[x/]*1` matches `A/ax1` by its name, as `A[X/]*1`
///   matches it by its whole path). cr-ecm20-5: nor does a `/` in one
///   alternative of a brace group (`a{/,x}*1`), which the other
///   alternatives need not hold;
/// - a pattern of one component matches an entry's own name at any depth,
///   AND the whole path from the root (`IMG_*.jpg` matches `d/IMG_x.jpg`
///   and `IMG_a/b.jpg`) — so where it starts is not fixed.
///
/// So the two can be told apart a fixed way from the END — after the last
/// place whose width varies — and, for a pattern anchored at the root, a
/// fixed way from the START: at such a place one holds a character the
/// other's does not (a letter of the other case, or a class that does not
/// hold it). A separator is a place like any other. (What an EXCLUDE takes
/// with a folder — everything below it — is the folder's doing, not what
/// the pattern matches, and is no part of the claim, as it is not for a
/// pattern of plain text.)
fn cannot_share_a_name(typed: &str, spelling: &str, windows: bool) -> bool {
    let (ours, theirs) = (places(typed, windows), places(spelling, windows));
    if ours.len() != theirs.len() {
        return false;
    }
    let anchored = must_part(typed, windows);
    let apart_here = |pairs: &mut dyn Iterator<Item = (&Place, &Place)>| {
        pairs
            .take_while(|(x, y)| !matches!(x, Place::Varies) && !matches!(y, Place::Varies))
            .any(|(x, y)| places_apart(x, y))
    };
    apart_here(&mut ours.iter().rev().zip(theirs.iter().rev()))
        || (anchored && apart_here(&mut ours.iter().zip(&theirs)))
}

/// Whether every path `pattern` matches has a separator the pattern
/// itself puts there — one outside every class and every brace group (a
/// trailing one aside): such a pattern matches the whole path, from the
/// root, and never an entry's own name (cr-ecm19-5, cr-ecm20-5).
fn must_part(pattern: &str, windows: bool) -> bool {
    let separator = |c: char| c == '/' || (windows && c == '\\');
    let body = pattern.strip_suffix(separator).unwrap_or(pattern);
    let mut depth = 0usize;
    let mut rest = body;
    while let Some(c) = rest.chars().next() {
        let len = match c {
            '[' => bracket_class_len(rest).unwrap_or(1),
            '\\' if !windows => match rest[1..].chars().next() {
                Some('/') if depth == 0 => return true,
                Some(next) => 1 + next.len_utf8(),
                None => 1,
            },
            '{' => {
                depth += 1;
                1
            }
            '}' => {
                depth = depth.saturating_sub(1);
                1
            }
            c if separator(c) && depth == 0 => return true,
            c => c.len_utf8(),
        };
        rest = &rest[len..];
    }
    false
}

/// One place of a pattern, as [`cannot_share_a_name`] reads it: a
/// character (a separator among them), a class, any one character, or a
/// place whose width varies.
#[derive(Debug)]
enum Place<'a> {
    Char(char),
    Class(&'a str),
    Any,
    Varies,
}

fn places(pattern: &str, windows: bool) -> Vec<Place<'_>> {
    let mut out = Vec::new();
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        let (place, len) = match c {
            '*' | '{' | '}' | ',' => (Place::Varies, 1),
            '?' => (Place::Any, 1),
            '[' => match bracket_class_len(rest) {
                Some(len) => (Place::Class(&rest[..len]), len),
                None => (Place::Char('['), 1),
            },
            '\\' if !windows => match rest[1..].chars().next() {
                Some(next) => (Place::Char(next), 1 + next.len_utf8()),
                None => (Place::Char('\\'), 1),
            },
            // Either separator is the one place a path has there.
            '\\' => (Place::Char('/'), 1),
            c => (Place::Char(c), c.len_utf8()),
        };
        out.push(place);
        rest = &rest[len..];
    }
    out
}

/// The characters two classes are tried on: every one below 128 —
/// control characters too (cr-ecm17-4: a class holding a tab shared it
/// with its other spelling, and the tab was never tried), and `/`
/// (cr-ecm18-4: a class matches it in a path, as a `*` does, and `1[a/]2`
/// and `1[A/]2` both match `1/2`) — and one beyond (a class holding a
/// letter beyond ASCII is refused, but a negated class holds them all).
fn class_trials() -> impl Iterator<Item = char> {
    (1u8..=0x7f).map(char::from).chain(std::iter::once('é'))
}

fn places_apart(a: &Place, b: &Place) -> bool {
    let holds = |class: &str| {
        globset::Glob::new(class)
            .ok()
            .map(|glob| glob.compile_matcher())
    };
    match (a, b) {
        (Place::Char(x), Place::Char(y)) => x != y,
        (Place::Char(x), Place::Class(class)) | (Place::Class(class), Place::Char(x)) => {
            holds(class).is_some_and(|class| !class.is_match(x.to_string()))
        }
        (Place::Class(one), Place::Class(other)) => match (holds(one), holds(other)) {
            (Some(one), Some(other)) => !class_trials().any(|c| {
                let c = c.to_string();
                one.is_match(&c) && other.is_match(&c)
            }),
            _ => false,
        },
        _ => false,
    }
}

/// Whether `pattern` is plain text and alternatives only: no wildcard and
/// no bracket class, so the names it can match are the ones written in it.
fn only_text_and_alternatives(pattern: &str, windows: bool) -> bool {
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        let len = match c {
            '*' | '?' => return false,
            '[' if bracket_class_len(rest).is_some() => return false,
            '\\' if !windows => 1 + rest[1..].chars().next().map_or(0, char::len_utf8),
            c => c.len_utf8(),
        };
        rest = &rest[len..];
    }
    true
}

/// The most names [`samples`] reads off one text.
const MOST_SAMPLES: usize = 16;

/// Names `text` matches, read off its own syntax — the names a claim
/// about `text` is checked with: a wildcard run stands for `x`, a class
/// for one of its members, an alternation for each of its alternatives in
/// turn. `None` when the text holds more alternatives than
/// [`MOST_SAMPLES`] names can cover, or a class no member can be read off.
///
/// `pick` puts a chosen character where ONE class stands — the class
/// that begins at that byte of `text` — and then only the names that pass
/// through that class are given: the ones read through the alternative
/// that holds it. (cr-ecm8-2: every alternative's names were given, and a
/// name from another alternative was taken for evidence about the class.)
///
/// Only names a walk can give ([`walked`]): what a pattern says of any
/// other text is nothing a run will ever see, and a claim resting on it
/// would be about no file. `**//Cache` read off as `/Cache` — the text
/// matches, and no entry is ever called that — leaves no name at all.
fn samples(text: &str, windows: bool, pick: Option<(usize, char)>) -> Option<Vec<String>> {
    samples_with(text, windows, pick, "x")
}

/// [`samples`] with each run of `*` standing for `star` — some text, or
/// none ([`STARS`]).
fn samples_with(
    text: &str,
    windows: bool,
    pick: Option<(usize, char)>,
    star: &str,
) -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for (name, through) in expand(text, 0, windows, pick, star)? {
        if pick.is_some() && !through {
            continue;
        }
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

/// [`samples`] for the part of the whole text that begins at byte `base`:
/// each name with whether it passes through the picked class.
fn expand(
    text: &str,
    base: usize,
    windows: bool,
    pick: Option<(usize, char)>,
    star: &str,
) -> Option<Vec<(String, bool)>> {
    let mut names = vec![(String::new(), false)];
    let mut rest = text;
    let push = |names: &mut Vec<(String, bool)>, c: char| {
        names.iter_mut().for_each(|(name, _)| name.push(c))
    };
    while let Some(c) = rest.chars().next() {
        let at = base + (text.len() - rest.len());
        let len = match c {
            '[' => match bracket_class_len(rest) {
                Some(len) => {
                    let (members, picked) = match pick {
                        Some((place, letter)) if place == at => (vec![letter], true),
                        _ => (class_members(&rest[..len])?, false),
                    };
                    names = names
                        .iter()
                        .flat_map(|(name, through)| {
                            members
                                .iter()
                                .map(move |member| (format!("{name}{member}"), *through || picked))
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
                names.iter_mut().for_each(|(name, _)| name.push_str(star));
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
                        let inner = at + (alternative.as_ptr() as usize - rest.as_ptr() as usize);
                        for (ending, picked) in expand(alternative, inner, windows, pick, star)? {
                            for (name, through) in &names {
                                next.push((format!("{name}{ending}"), *through || picked));
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
    let written: Vec<String> = lost_runs(class)?
        .into_iter()
        .map(|(lo, hi)| {
            let mut range = String::new();
            push_range(&mut range, lo, hi);
            range
        })
        .collect();
    Some(written.join(", "))
}

/// The letters [`narrowed_class`] writes, each run of neighbours as one
/// range.
fn lost_runs(class: &str) -> Option<Vec<(char, char)>> {
    let (negated, ranges) = parse_class(class)?;
    if !negated || widen_class(class) == class {
        return None;
    }
    let holds = |c: char| ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi);
    // The letters the class does not name yet.
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
    (!lost.is_empty()).then_some(lost)
}

/// Letters as a class would write them, each run of neighbours as one
/// range: `B-C, X`.
fn written(letters: &[char]) -> String {
    let mut runs: Vec<(char, char)> = Vec::new();
    for &letter in letters {
        match runs.last_mut() {
            Some(run) if run.1 as u32 + 1 == letter as u32 => run.1 = letter,
            _ => runs.push((letter, letter)),
        }
    }
    let written: Vec<String> = runs
        .into_iter()
        .map(|(lo, hi)| {
            let mut range = String::new();
            push_range(&mut range, lo, hi);
            range
        })
        .collect();
    written.join(", ")
}

/// Those letters one by one, for asking the filter about each; `None`
/// when the class would lose none, or more than are asked about.
fn lost_letters(class: &str) -> Option<Vec<char>> {
    const MOST: usize = 64;
    let letters: Vec<char> = lost_runs(class)?
        .into_iter()
        .flat_map(|(lo, hi)| lo..=hi)
        .take(MOST + 1)
        .collect();
    (letters.len() <= MOST).then_some(letters)
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
        // (cr-ecm16-4: `**/Cache/**` is no longer one of these. Past its
        // `**`, `**/cache/**` names `Cache/cache/x` too, as it does: the
        // spelling is not printed for it, and the name checked is —
        // `cache/x` — with nothing said of what it matches "only".)
        assert_eq!(
            note(&["**/Cache/**"]).as_deref(),
            Some(
                "\"**/Cache/**\" matches exactly as typed; capital letters matter \
                 (--ignore-case would also match cache/x, CACHE/x)."
            )
        );
        assert_eq!(
            note(&["Cache/**"]),
            line("\"Cache/**\" matches Cache/**", "", "cache/**, CACHE/**")
        );
        // cr-ecm17-4: `img_*.jpg` and `IMG_*.jpg` both match
        // `IMG_a/img_b.jpg` (a pattern of one component also matches the
        // whole path, its `*` crossing the separator): the name checked is
        // printed for the one, and nothing said of what is matched "only".
        assert_eq!(
            note(&["IMG_*.jpg"]).as_deref(),
            Some(
                "\"IMG_*.jpg\" matches exactly as typed; capital letters matter \
                 (--ignore-case would also match img_x.jpg, IMG_*.JPG)."
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
            note(&["??", "*.1", "Thumbs.db", "*.2"]),
            line(
                "\"Thumbs.db\" matches Thumbs.db",
                "",
                "thumbs.db, THUMBS.DB"
            )
        );
        // cr-ecm9-2: beside a pattern that names everything, the option
        // adds nothing through this one that a name bears out: the filter
        // keeps `thumbs.db` already, and no spelling is offered.
        // cr-ecm10-1: the line stays, in words — that `*` covers every
        // spelling everywhere is not what one name at the root shows.
        assert_eq!(
            note(&["??", "*.1", "Thumbs.db", "*"]).as_deref(),
            Some(
                "\"Thumbs.db\" matches exactly as typed; capital letters matter \
                 (--ignore-case ignores them)."
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
        // cr-ecm11-3: the name printed is the one that was checked. `.O`
        // is a name `??` takes either way; what the option adds through
        // `*.o` is `x.O`, and the line says that — and so does not say
        // what the pattern matches "only".
        assert_eq!(
            note(&["*.o", "*.tmp", "??", "Build"]).as_deref(),
            Some(
                "\"*.o\" matches exactly as typed, and 2 more patterns match only as typed; \
                 capital letters matter (--ignore-case would also match x.O)."
            )
        );
    }

    /// cr-ecm15-2: a spelling is printed for what `--ignore-case` would
    /// also match only where no name it reads as is one the pattern has
    /// AS TYPED. `1*A*2` was printed for a sole `--include '1*a*2'`, and
    /// it reads as `1aA2` — which that pattern has as typed, its `a` and
    /// then the `A` in a `*`. The name that was checked, `1xAx2`, is one
    /// the option changes, and is what is printed. A pattern whose
    /// letters begin or end the name, or stand a fixed way from either,
    /// has no such name, and its spelling is printed as before.
    #[test]
    fn a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed() {
        let others = |pattern: &str| {
            Spellings::of(pattern, false)
                .map(|spellings| spellings.others)
                .unwrap_or_default()
        };
        // The reviewer's.
        assert_eq!(others("1*a*2"), ["1xAx2"]);
        // cr-ecm16-4: through another alternative, with no `*` — `{A?,?B}`
        // reads as `Ab`, which `?b` has as typed.
        assert_eq!(others("{a?,?b}"), ["Ax"]);
        // Letters of two cases as typed: either spelling reads as a name
        // the pattern has (`1abaB2`, `1ABaB2`).
        assert_eq!(others("1*aB*2"), ["1xabx2", "1xABx2"]);
        // Two runs of letters, each between wildcards.
        assert_eq!(others("*a*b*"), ["AxB"]);
        // Letters that begin the name, end it, or do both: no name is had
        // both ways, and the spelling stands.
        assert_eq!(others("IMG_*.jpg"), ["img_x.jpg", "IMG_*.JPG"]);
        assert_eq!(others("a*b"), ["A*B"]);
        assert_eq!(others("x*{a,b}*y"), ["X*{A,B}*Y"]);
        assert_eq!(others("dir/*a*"), ["DIR/*A"]);
        // No `*` to hold the other case: every letter stands where it is.
        assert_eq!(others("?a?"), ["?A?"]);
        // cr-ecm16-4: a class is a place too. `[abc]` and `[ABC]` hold no
        // letter in common; `[A-C_]x` and `[a-c_]x` both hold `_x`, and
        // the name checked is printed; `[A-C_]X` is apart by its `X`.
        assert_eq!(others("[abc]"), ["[ABC]"]);
        assert_eq!(others("[A-C_]x"), ["ax", "[A-C_]X"]);
        // A path pattern is anchored at the root: its start keeps two
        // spellings apart, and so does its end. cr-ecm17-4: a `*` crosses
        // separators, so between two `*` nothing does — `*/cache/*` and
        // `*/Cache/*` both match `x/Cache/cache/saved`, and the name
        // checked is shown.
        assert_eq!(others("Cache/*"), ["cache/*", "CACHE/*"]);
        assert_eq!(others("**/*/Cache"), ["*/cache", "*/CACHE"]);
        assert_eq!(others("*/Cache/*"), ["x/cache/x", "x/CACHE/x"]);
        // A pattern of one component also matches the whole path from the
        // root, so only its end tells two spellings apart: `img_*.jpg`
        // and `IMG_*.jpg` both match `IMG_a/img_b.jpg`.
        assert_eq!(others("IMG_*.jpg"), ["img_x.jpg", "IMG_*.JPG"]);
        // A class holding a tab holds it in both spellings: alone, the
        // two classes share it, and the name checked is shown; beside a
        // letter, the letter tells them apart.
        assert_eq!(others("[a\t]"), ["A"]);
        // cr-ecm18-4: and one holding `/` holds it in both: `1[a/]2` and
        // `1[A/]2` both match the path `1/2`.
        assert_eq!(others("1[a/]2"), ["1A2"]);
        // cr-ecm19-5: and a `/` a class may hold anchors nothing — the
        // class may hold another character, and the pattern match an
        // entry's own name: `a[x/]*1` matches `A/ax1` by its name, and
        // `A[X/]*1` matches it by its whole path. The name checked is
        // shown, not that spelling.
        assert_eq!(others("a[x/]*1"), ["AXx1"]);
        // cr-ecm20-5: nor does a `/` in one alternative of a brace group:
        // `a{/,x}*1` matches `A/ax1` by its name, `A{/,X}*1` by its path.
        assert_eq!(others("a{/,x}*1"), ["A/x1"]);
        for (pattern, shown) in [("a[x/]*1", "AXx1"), ("a{/,x}*1", "A/x1")] {
            for (name, as_typed, with_the_option) in [(shown, false, true), ("A/ax1", true, true)] {
                let taken = |ignore_case: bool| {
                    let filter = crate::fs_enum::FileFilter::probing(pattern, false, ignore_case)
                        .expect("a pattern");
                    Asked::named(Role::Include, &filter, name).0
                };
                assert_eq!(taken(false), as_typed, "{pattern}: {name} as typed");
                assert_eq!(
                    taken(true),
                    with_the_option,
                    "{pattern}: {name} with --ignore-case"
                );
            }
        }
        assert_eq!(others("[a\t]x"), ["[A\t]X"]);

        // And what is printed is true of the run: the name shown is taken
        // with the option and not without, and the name the spelling
        // read as is taken either way.
        let run = |ignore_case: bool| {
            crate::fs_enum::FileFilter::probing("1*a*2", false, ignore_case).expect("a pattern")
        };
        for (name, as_typed, with_the_option) in [("1xAx2", false, true), ("1aA2", true, true)] {
            let taken =
                |filter: &crate::fs_enum::FileFilter| Asked::named(Role::Include, filter, name).0;
            assert_eq!(
                (taken(&run(false)), taken(&run(true))),
                (as_typed, with_the_option),
                "{name}"
            );
        }
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
        // cr-ecm16-4: `[a-c_]x` holds `_x`, which `[A-C_]x` has as typed:
        // the name checked is printed for it.
        assert_eq!(others("[A-C_]x"), ["ax", "[A-C_]X"]);
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
            // cr-ecm17-4: a `*` crosses separators, and both spellings
            // match `x/Cache/cache/x`: the names checked are shown.
            ("*/Cache/*", false, "*/Cache/*", &["x/cache/x", "x/CACHE/x"]),
            // (cr-ecm16-4 showed names here, on the belief that two
            // spellings share `x/Cache/cache` through its folder. What a
            // pattern itself matches ends where the path ends — measured,
            // cr-ecm17-4 — so these two match nothing in common, and the
            // spellings stand.)
            ("**/*/Cache", false, "*/Cache", &["*/cache", "*/CACHE"]),
            // cr-ecm15-2: `cache*/**` reads as `cacheCache/x`, which the
            // pattern has as typed — so the names that were checked are
            // what is shown beside it.
            (
                "**/*Cache*/**",
                false,
                "Cache*/**",
                &["cachex/x", "CACHEx/x"],
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
                 ignores them)."
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
        let as_held = |pattern: &str| vec![pattern.to_string()];
        let (data, digits_held) = (as_held("./Data"), as_held("./Data/2024"));
        let whole = |excludes| Whole {
            excludes,
            includes: &[],
            refused_with_the_option: false,
        };
        assert_eq!(
            case_note_of(&[absolute], whole(&data), false).as_deref(),
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
        assert_eq!(case_note_of(&[digits], whole(&digits_held), false), None);
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
                     (--ignore-case ignores them)."
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
        // cr-ecm7-4: a pattern the option reads no differently is not
        // shown. For `{a,A}` the line offered `{a,a}` and `{A,A}`, both
        // matched already; what changes is `*.Doc`. (cr-ecm11-5: it is
        // counted, as every other pattern with a letter in it is.)
        let doc = "\"*.Doc\" matches .Doc only, and 1 more pattern matches only as typed; \
                   capital letters matter (--ignore-case would also match .doc, .DOC).";
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
        // cr-ecm13-7: and what is printed is what changes — `{1,cache}`
        // was, which reads as `1` too, and `1` is matched either way.
        assert_eq!(
            note(&["{1,Cache}"]).as_deref(),
            Some(
                "\"{1,Cache}\" matches exactly as typed; capital letters matter (--ignore-case \
                 would also match cache, CACHE)."
            )
        );
        // The option does change it, in a way neither rule-made spelling
        // shows: the names themselves, each asked of both readings.
        assert_eq!(
            note(&["{ab,AB}"]).as_deref(),
            Some(
                "\"{ab,AB}\" matches {ab,AB} only; capital letters matter (--ignore-case would \
                 also match aB, Ab)."
            )
        );
        // cr-ecm8-2: a pattern is left out only when it is ESTABLISHED
        // that the option changes nothing it names. `{Σ,σ}` got no line:
        // lowered and raised it is itself, and the option also takes `ς`.
        assert_eq!(
            note(&["{Σ,σ}"]).as_deref(),
            Some(
                "\"{Σ,σ}\" matches {Σ,σ} only; capital letters matter (--ignore-case would also \
                 match ς)."
            )
        );
        assert_eq!(note(&["{Σ,σ,ς}"]), None, "every case of it is written");
        // `{[a-c],A,B}` got none either, and the option also takes `C`.
        // With a class in it not every name can be asked about, so
        // nothing is concluded: the line states the rule.
        assert_eq!(
            note(&["{[a-c],A,B}"]).as_deref(),
            Some(
                "\"{[a-c],A,B}\" matches exactly as typed; capital letters matter (--ignore-case \
                 ignores them)."
            )
        );
        // And it is counted, where it was dropped from the count.
        assert_eq!(
            note(&["Keep", "{[a-c],A,B}", "{a,A}"]).as_deref(),
            Some(
                "\"Keep\" matches Keep only, and 2 more patterns match only as typed; capital \
                 letters matter (--ignore-case would also match keep, KEEP)."
            )
        );
        // A narrowing is credited to the class that narrows: `A.1` stays
        // matched by the alternative beside `[!a].1`; `B.2` is what is lost.
        assert_eq!(
            note(&["{[!a].1,A.1,[!b].2}"]).as_deref(),
            Some(
                "\"{[!a].1,A.1,[!b].2}\" matches exactly as typed; capital letters matter \
                 (--ignore-case changes what it matches: [!b] would stop matching B)."
            )
        );
        // cr-ecm9-2: one real loss is enough to show, and it is shown
        // first. `A.txt` stays kept by the alternative beside the class;
        // `B.txt` does not — and the line spoke of `Keep`.
        assert_eq!(
            note(&["Keep", "{[!ab].txt,A.txt}"]).as_deref(),
            Some(
                "\"{[!ab].txt,A.txt}\" matches exactly as typed, and 1 more pattern matches \
                 only as typed; capital letters matter (--ignore-case changes what they match: \
                 [!ab] would stop matching B)."
            )
        );
        assert_eq!(
            note(&["{[!a-c].x,A.x}"]).as_deref(),
            Some(
                "\"{[!a-c].x,A.x}\" matches exactly as typed; capital letters matter \
                 (--ignore-case changes what it matches: [!a-c] would stop matching B-C)."
            )
        );
        // cr-ecm9-2: a gain is judged against the whole filter. `A` was
        // offered for `a`, and the pattern beside it keeps `A` already.
        // cr-ecm10-1: no spelling is offered — and the line stays. It was
        // dropped, as if the option changed nothing these two name; with
        // `./A` in place of `A` that is false one level down (`sub/A`),
        // and a name at the root cannot tell the two commands apart.
        let in_words = "\"a\" matches exactly as typed, and 1 more pattern matches only as \
                        typed; capital letters matter (--ignore-case ignores them).";
        assert_eq!(note(&["a", "A"]).as_deref(), Some(in_words));
        assert_eq!(note(&["a", "./A"]).as_deref(), Some(in_words));
        // A pattern whose own two readings are the same is still not
        // shown; with nothing else to show there is no line.
        assert_eq!(
            note(&["a", "A", "{b,B}"]).as_deref(),
            Some(
                "\"a\" matches exactly as typed, and 2 more patterns match only as typed; \
                 capital letters matter (--ignore-case ignores them)."
            )
        );
        assert_eq!(note(&["{b,B}", "[Cc]"]), None);
        let doc = "\"*.Doc\" matches .Doc only, and 2 more patterns match only as typed; \
                   capital letters matter (--ignore-case would also match .doc, .DOC).";
        assert_eq!(note(&["a", "A", "*.Doc"]).as_deref(), Some(doc));
        // cr-ecm10-5: and the count is the same whatever the order — it
        // was two with `*.Doc` first and none with it last.
        for order in [
            ["*.Doc", "a", "A"],
            ["a", "*.Doc", "A"],
            ["a", "A", "*.Doc"],
        ] {
            assert_eq!(note(&order).as_deref(), Some(doc), "{order:?}");
        }
        // And with a pattern whose two readings are the same: it was
        // counted when typed before the pattern shown and not when typed
        // after, only the later ones being asked.
        let narrowed = "\"[!a].dat\" matches exactly as typed, and 1 more pattern matches only \
                        as typed; capital letters matter (--ignore-case changes what they match: \
                        [!a] would stop matching A).";
        for order in [["{b,B}", "[!a].dat"], ["[!a].dat", "{b,B}"]] {
            assert_eq!(note(&order).as_deref(), Some(narrowed), "{order:?}");
        }
        // cr-ecm11-5: the count is of the other patterns with a letter in
        // them, by their text — whatever was asked about, and however many
        // there are. The reviewer's 33: one name, thirty-one patterns that
        // read the same both ways, and `{z,2}`, which does not. With
        // `{z,2}` last it was past what is asked about and counted
        // unasked, the others asked and left out: "1 more"; second, it was
        // asked and counted, and more of the others were not: "2 more".
        let same: Vec<String> = ('a'..='z')
            .chain('α'..='ε')
            .map(|c| format!("{{{c},{}}}", c.to_uppercase()))
            .collect();
        assert_eq!(same.len(), 31);
        let counted = "\"Visible.txt\" matches Visible.txt only, and 32 more patterns match \
                       only as typed; capital letters matter (--ignore-case would also match \
                       visible.txt, VISIBLE.TXT).";
        for at in [1, 17, same.len() + 1] {
            let mut patterns: Vec<&str> = std::iter::once("Visible.txt")
                .chain(same.iter().map(String::as_str))
                .collect();
            patterns.insert(at, "{z,2}");
            assert_eq!(patterns.len(), 33);
            assert_eq!(
                note(&patterns).as_deref(),
                Some(counted),
                "`{{z,2}}` at {at}"
            );
        }
        // cr-ecm10-5: a spelling is offered for the names it denotes.
        // `Q[b-~]` was offered for `q[b-~]` beside the pattern typed
        // exactly so: what the option adds is `QB`, which that glob does
        // not denote, and every name it does denote is kept already.
        assert_eq!(
            note(&["q[b-~]", "Q[b-~]"]).as_deref(),
            Some(
                "\"q[b-~]\" matches exactly as typed, and 1 more pattern matches only as \
                 typed; capital letters matter (--ignore-case ignores them)."
            )
        );
        // cr-ecm10-1: where a class could take less and no name shows it,
        // the warning stays, and says `may`. `A.dat` is kept by the second
        // pattern either way — at the root, which is where it was tried.
        assert_eq!(
            note(&["[!a].dat", "A.dat"]).as_deref(),
            Some(
                "\"[!a].dat\" matches exactly as typed, and 1 more pattern matches only as \
                 typed; capital letters matter (--ignore-case changes what they match: [!a] \
                 may stop matching A)."
            )
        );
        for pattern in ["{[!a].1,A.1}", "x{[!a],A}"] {
            assert_eq!(
                note(&[pattern]),
                Some(format!(
                    "\"{pattern}\" matches exactly as typed; capital letters matter \
                     (--ignore-case changes what it matches: [!a] may stop matching A)."
                )),
            );
        }
        // cr-ecm10-1: a `*` is tried as some text and as none. With
        // `Ax.txt` kept by the third pattern, `A.txt` shows the loss the
        // line missed — it spoke of `Keep`, and the option gave up
        // `Ay.txt`.
        assert_eq!(
            note(&["Keep", "[!a]*.txt", "Ax.txt"]).as_deref(),
            Some(
                "\"[!a]*.txt\" matches exactly as typed, and 2 more patterns match only as \
                 typed; capital letters matter (--ignore-case changes what they match: [!a] \
                 would stop matching A)."
            )
        );
        // cr-ecm10-2: both lists, as the run applies them. The line named
        // the loss of `A` at `[!a].bin`; no include names `A.bin`, so the
        // run leaves it alone either way. What the option gives up is
        // `B.txt`.
        let listed = |patterns: &[&str], role: Role| -> (Vec<String>, Vec<Noted<'static>>) {
            let owned: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
            let noted = patterns
                .iter()
                .map(|p| {
                    let typed: &'static str = Box::leak(p.to_string().into_boxed_str());
                    Noted {
                        typed,
                        read_as: typed,
                        file: None,
                        role,
                    }
                })
                .collect();
            (owned, noted)
        };
        let both = |excludes: &[&str], includes: &[&str]| {
            let (excluded, mut noted) = listed(excludes, Role::Exclude);
            let (included, more) = listed(includes, Role::Include);
            noted.extend(more);
            case_note_of(
                &noted,
                Whole {
                    excludes: &excluded,
                    includes: &included,
                    refused_with_the_option: false,
                },
                cfg!(windows),
            )
        };
        assert_eq!(
            both(&["[!a].bin", "[!b].txt"], &["*.txt"]).as_deref(),
            Some(
                "\"[!b].txt\" matches exactly as typed, and 2 more patterns match only as \
                 typed; capital letters matter (--ignore-case changes what they match: [!b] \
                 would stop matching B)."
            )
        );
        // cr-ecm11-3: what is printed is the name that was checked. The
        // line said `--ignore-case would also match A`; both runs leave
        // `A` out, by `[Aa]`, and the file the option takes in is `xA`.
        assert_eq!(
            both(&["[Aa]"], &["*a"]).as_deref(),
            Some(
                "\"*a\" matches exactly as typed; capital letters matter (--ignore-case would \
                 also match xA)."
            )
        );
        // cr-ecm12-4: and so where the text to print still holds a class.
        // `[A]` was printed for `*[a]` — the name it reads as is `A`.
        assert_eq!(
            both(&["[Aa]"], &["*[a]"]).as_deref(),
            Some(
                "\"*[a]\" matches exactly as typed; capital letters matter (--ignore-case \
                 would also match xA)."
            )
        );
        // A text with a wildcard left in it is printed when a name of its
        // OWN showed the change.
        assert_eq!(
            both(&["img_*.jpg"], &[]).as_deref(),
            Some(
                "\"img_*.jpg\" matches img_*.jpg only; capital letters matter (--ignore-case \
                 would also match IMG_*.JPG)."
            )
        );
        // cr-ecm14-5: beside ANOTHER pattern, a text with a wildcard left
        // in it is not printed on the strength of the names tried — it
        // reads as names nobody tried. `A?B` was printed for `a?b` beside
        // `--exclude '?1?'`: `AxB` is let in by the option, and `A1B`,
        // which `A?B` reads as too, is left out by both runs. The name
        // tried is what is printed. Alone, the pattern is the whole
        // filter, and its spelling stands.
        assert_eq!(
            both(&["?1?"], &["a?b"]).as_deref(),
            Some(
                "\"a?b\" matches exactly as typed; capital letters matter (--ignore-case \
                 would also match AxB)."
            )
        );
        assert_eq!(
            both(&[], &["a?b"]).as_deref(),
            Some(
                "\"a?b\" matches a?b only; capital letters matter (--ignore-case would also \
                 match A?B)."
            )
        );
        // A class is a wildcard here, and a star in the middle of a text.
        assert_eq!(
            both(&["?1?"], &["a[xy1]b"]).as_deref(),
            Some(
                "\"a[xy1]b\" matches exactly as typed; capital letters matter (--ignore-case \
                 would also match AXB)."
            )
        );
        assert_eq!(
            both(&["*1*"], &["img_*.jpg"]).as_deref(),
            Some(
                "\"img_*.jpg\" matches exactly as typed; capital letters matter (--ignore-case \
                 would also match IMG_x.JPG)."
            )
        );
        // A text of letters and alternatives is all the names it reads
        // as, whatever stands beside it.
        assert_eq!(
            both(&["?1?"], &["{axb,ayb}"]).as_deref(),
            Some(
                "\"{axb,ayb}\" matches {axb,ayb} only; capital letters matter (--ignore-case \
                 would also match {AXB,AYB})."
            )
        );
        // cr-ecm13-7: and where the text to print reads as SEVERAL names
        // of which only some change. `{A,B}` was printed for `{a,b}`: `A`
        // is left out by both runs, and only `B` is let in by the option.
        assert_eq!(
            both(&["[Aa]"], &["{a,b}"]).as_deref(),
            Some(
                "\"{a,b}\" matches exactly as typed; capital letters matter (--ignore-case \
                 would also match B)."
            )
        );
        // Every name of it changing, the text stands.
        assert_eq!(
            both(&[], &["{a,b}"]).as_deref(),
            Some(
                "\"{a,b}\" matches {a,b} only; capital letters matter (--ignore-case would also \
                 match {A,B})."
            )
        );
        // An include the option widens is offered what it would let in —
        // unless an exclude, read the same way, takes it out again.
        assert_eq!(
            both(&[], &["*.Txt"]).as_deref(),
            Some(
                "\"*.Txt\" matches .Txt only; capital letters matter (--ignore-case would also \
                 match .txt, .TXT)."
            )
        );
        assert_eq!(
            both(&["*.[Tt][Xx][Tt]"], &["*.Txt"]).as_deref(),
            Some(
                "\"*.Txt\" matches exactly as typed; capital letters matter (--ignore-case \
                 ignores them)."
            )
        );
    }

    /// cr-ecm10-6: the line costs the same however many patterns the
    /// command holds. With 1,000 excludes whose every spelling another
    /// pattern covers, each was asked about in turn — and a plain name
    /// was put to the filters in every case of its letters, a thousand
    /// spellings — so one line took 443 ms where the whole command had
    /// taken 31. A fixed number of patterns are asked about, only one that
    /// spells out alternatives is compared spelling by spelling, and past
    /// that the line is the plain rule. Counted in questions put to a
    /// filter.
    /// cr-ecm11-6: the line never builds a long filter a second time. One
    /// question about one include built the run's whole filter as the
    /// option would read it — 10,000 includes compiled again, 160 ms where
    /// the command takes 38. Past [`MOST_PATTERNS_WIDENED`] patterns it is
    /// not built, and the line says only what it can without it.
    #[test]
    fn the_line_does_not_build_a_long_filter_a_second_time() {
        let includes =
            |n: usize| -> Vec<String> { (0..n).map(|at| format!("name{at:05}.dat")).collect() };
        let line = |includes: &[String], excludes: &[String]| {
            let noted: Vec<Noted> = includes
                .iter()
                .map(|pattern| (pattern, Role::Include))
                .chain(excludes.iter().map(|pattern| (pattern, Role::Exclude)))
                .map(|(pattern, role)| Noted {
                    typed: pattern,
                    read_as: pattern,
                    file: None,
                    role,
                })
                .collect();
            WIDENED.with(|built| built.set(0));
            let line = case_note_of(
                &noted,
                Whole {
                    excludes,
                    includes,
                    refused_with_the_option: false,
                },
                cfg!(windows),
            );
            (line, WIDENED.with(|built| built.get()))
        };
        // The reviewer's command: 10,000 includes. The rule, and a count.
        let (said, built) = line(&includes(10_000), &[]);
        assert_eq!(built, 0, "the whole filter was built again for one line");
        assert_eq!(
            said.as_deref(),
            Some(
                "\"name00000.dat\" matches exactly as typed, and 9999 more patterns match only \
                 as typed; capital letters matter (--ignore-case ignores them)."
            )
        );
        // Within the bound it is built, once, and the example is shown.
        let (said, built) = line(&includes(MOST_PATTERNS_WIDENED), &[]);
        assert_eq!(built, 1);
        assert!(
            said.is_some_and(|said| said.contains("would also match NAME00000.DAT")),
            "an include's example is asked of the whole filter"
        );
        // A class that could take less, beside more patterns than the
        // bound: the warning stays, and says `may` — no name was asked.
        let (said, built) = line(&includes(MOST_PATTERNS_WIDENED), &["[!a]*.dat".to_string()]);
        assert_eq!(built, 0);
        assert!(
            said.is_some_and(|said| said.contains("[!a] may stop matching A")),
            "what cannot be shown is still warned of"
        );
    }

    /// cr-ecm11-4: a pattern that cannot be put to the filter as the
    /// option would read it gets the rule, and no example made by rule.
    #[test]
    fn a_pattern_the_option_cannot_read_gets_the_rule_and_no_example() {
        let pattern = format!("{}s{}", "{".repeat(249), "}".repeat(249));
        let probing =
            |ignore_case| crate::fs_enum::FileFilter::probing(&pattern, true, ignore_case);
        assert!(probing(false).is_some(), "as typed it is a pattern");
        assert!(probing(true).is_none(), "with the option it is refused");
        assert_eq!(
            case_note(&[(pattern.as_str(), None)], cfg!(windows)),
            Some(format!(
                "\"{pattern}\" matches exactly as typed; capital letters matter \
                 (--ignore-case ignores them)."
            ))
        );
        // cr-ecm12-5: and beside it NO pattern gets an example. `*Cache`
        // was offered `cache, CACHE` here — and the same command with
        // `--ignore-case` is refused whole, for the pattern beside it.
        // Whichever is typed first, the line is the rule.
        for (first, second) in [(pattern.as_str(), "*Cache"), ("*Cache", pattern.as_str())] {
            assert_eq!(
                case_note(&[(first, None), (second, None)], cfg!(windows)),
                Some(format!(
                    "\"{first}\" matches exactly as typed, and 1 more pattern matches only as \
                     typed; capital letters matter (--ignore-case ignores them)."
                ))
            );
        }
        // cr-ecm13-6: and where the pattern that refuses the option is in
        // NEITHER list — an absolute exclude outside the source — the line
        // is the rule just the same.
        let cache = ["*Cache".to_string()];
        let said = |refused_with_the_option| {
            case_note_of(
                &[Noted {
                    typed: "*Cache",
                    read_as: "*Cache",
                    file: None,
                    role: Role::Exclude,
                }],
                Whole {
                    excludes: &cache,
                    includes: &[],
                    refused_with_the_option,
                },
                cfg!(windows),
            )
        };
        assert_eq!(
            said(true).as_deref(),
            Some(
                "\"*Cache\" matches exactly as typed; capital letters matter (--ignore-case \
                 ignores them)."
            )
        );
        assert!(said(false).is_some_and(|line| line.ends_with("would also match cache, CACHE).")));
        // Alone, `*Cache` has its example.
        assert!(case_note(&[("*Cache", None)], cfg!(windows))
            .is_some_and(|line| line.ends_with("(--ignore-case would also match cache, CACHE).")));
        // Few enough levels, and both readings compile: an example, checked.
        let shallow = format!("{}s{}", "{".repeat(8), "}".repeat(8));
        assert!(
            case_note(&[(shallow.as_str(), None)], cfg!(windows)).is_some_and(
                |line| line.ends_with("(--ignore-case would also match {{{{{{{{S}}}}}}}}).")
            )
        );
    }

    #[test]
    fn the_line_asks_a_bounded_number_of_questions() {
        /// A few hundred are asked for the patterns looked at; a thousand
        /// patterns asked about, or sixteen compared in full, are tens of
        /// thousands.
        const MOST_QUESTIONS: usize = 2000;
        let mut patterns: Vec<String> = (0..1000).map(|n| format!("archive{n:03}.tmp")).collect();
        patterns.push("*.[Tt][Mm][Pp]".to_string());
        let typed: Vec<(&str, Option<&str>)> =
            patterns.iter().map(|p| (p.as_str(), None)).collect();
        QUESTIONS.with(|asked| asked.set(0));
        let line = case_note(&typed, cfg!(windows));
        let asked = QUESTIONS.with(|asked| asked.get());
        assert!(
            asked <= MOST_QUESTIONS,
            "{asked} questions were put to a filter for one line"
        );
        eprintln!("{asked} questions with the covering glob");
        // Nothing the first of them could show is borne out, and the last
        // reads the same both ways: the rule, and a count of the rest.
        assert_eq!(
            line.as_deref(),
            Some(
                "\"archive000.tmp\" matches exactly as typed, and 999 more patterns match only \
                 as typed; capital letters matter (--ignore-case ignores them)."
            )
        );
        // And with nothing to cover them: the rule too (cr-ecm12-5). A
        // thousand patterns are more than is read a second time for one
        // line, so it is not known that the option could run at all, and
        // nothing concrete is said of it.
        QUESTIONS.with(|asked| asked.set(0));
        let line = case_note(&typed[..1000], cfg!(windows));
        let asked = QUESTIONS.with(|asked| asked.get());
        assert!(asked <= MOST_QUESTIONS, "{asked} questions");
        eprintln!("{asked} questions without it");
        assert_eq!(
            line.as_deref(),
            Some(
                "\"archive000.tmp\" matches exactly as typed, and 999 more patterns match only \
                 as typed; capital letters matter (--ignore-case ignores them)."
            )
        );
    }

    /// Each check the line rests on, asked directly: the filter is what
    /// answers, whatever the text rules produced.
    #[test]
    fn each_claim_is_checked_against_the_filter() {
        let asked = |pattern: &str| {
            Asked::about(pattern, Role::Exclude, &Complete::alone(pattern)).expect("a pattern")
        };
        // The same pattern among others: the lists are the filter.
        let within = |pattern: &str, role: Role, excludes: &[&str], includes: &[&str]| {
            let owned = |list: &[&str]| -> Vec<String> {
                list.iter().map(|other| other.to_string()).collect()
            };
            let whole = Complete::of(Whole {
                excludes: &owned(excludes),
                includes: &owned(includes),
                refused_with_the_option: false,
            });
            Asked::about(pattern, role, &whole).expect("a pattern")
        };
        let among = |pattern: &str, others: &[&str]| {
            let mut all = vec![pattern];
            all.extend(others);
            within(pattern, Role::Exclude, &all, &[])
        };
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
        // A spelling — the whole pattern in another case — is offered only
        // when it denotes nothing the option does not match, and some name
        // it denotes is one the run would treat otherwise with the option.
        for (pattern, spelling, offered) in [
            ("*Cache", "*cache", true),
            ("*Cache", "*CACHE", true),
            ("{a,A}", "{a,a}", false),
            ("{a,A}", "{A,A}", false),
            ("*/Cache", "*/cache", true),
            // A text that names what the pattern does not, even ignoring
            // case, is not offered: a level the pattern requires is gone.
            ("*/Cache", "cache", false),
            ("photo[!x].jpg", "PHOTO[!x].JPG", false),
            ("photo[!0-9].jpg", "PHOTO[!0-9].JPG", true),
            ("q[b-~]", "Q[b-~]", true),
        ] {
            assert_eq!(
                asked(pattern)
                    .would_also_match(spelling, without_end_wildcards(spelling, false), false)
                    .is_some(),
                offered,
                "\"{pattern}\" offering {spelling}"
            );
        }
        // cr-ecm11-3: and what is given back is the NAME that showed it —
        // the text the line would print, when that is such a name.
        let offered = |asked: &Asked, spelling: &str| {
            asked
                .would_also_match(spelling, without_end_wildcards(spelling, false), false)
                .map(|(name, _)| name)
        };
        // The text shown stands for its own name, whatever anchors it;
        // another name the spelling denotes is not the text shown.
        assert_eq!(
            asked("./Cache").would_also_match("./cache", "./cache", false),
            Some(("cache".to_string(), true))
        );
        assert_eq!(
            within("*a", Role::Include, &["[Aa]"], &["*a"]).would_also_match("*A", "A", false),
            Some(("xA".to_string(), false))
        );
        assert_eq!(
            offered(&asked("*Cache"), "*cache").as_deref(),
            Some("cache")
        );
        assert_eq!(
            offered(&asked("*Cache"), "*CACHE").as_deref(),
            Some("CACHE")
        );
        assert_eq!(offered(&asked("*.Txt"), "*.TXT").as_deref(), Some(".TXT"));
        // The reviewer's: `A` is left out by `[Aa]` either way, and is
        // not the name; `xA` is the one the option lets in.
        let beside = within("*a", Role::Include, &["[Aa]"], &["*a"]);
        assert!(!beside.gained("A") && beside.gained("xA"));
        assert_eq!(offered(&beside, "*A").as_deref(), Some("xA"));
        // A class is said to stop matching a letter only when it does —
        // the class that stands at that place in the pattern, by a name
        // read through the alternative that holds it (cr-ecm8-2).
        let lost = |pattern: &str, at: usize, class: &str| {
            asked(pattern).lost_here(pattern, at, class, false)
        };
        assert_eq!(lost("[!a].dat", 0, "[!a]"), ['A']);
        assert_eq!(lost("photo[!x].jpg", 5, "[!x]"), ['X']);
        assert_eq!(lost("Keep", 0, "[!a]"), Vec::<char>::new());
        assert_eq!(lost("[!0-9]a", 0, "[!0-9]"), Vec::<char>::new());
        // `A.1` stays matched by the alternative beside `[!a].1`; it is
        // `[!b]` that stops matching, `B.2`.
        let three = "{[!a].1,A.1,[!b].2}";
        assert_eq!(lost(three, 1, "[!a]"), Vec::<char>::new());
        assert_eq!(lost(three, 12, "[!b]"), ['B']);
        // cr-ecm9-2: the letters that ARE lost are the claim, however many
        // the class would lose: of `A-C`, `A` stays matched here, and `B`
        // and `C` do not.
        assert_eq!(lost("{[!a-c].x,A.x}", 1, "[!a-c]"), ['B', 'C']);
        assert_eq!(lost("{[!ab].txt,A.txt}", 1, "[!ab]"), ['B']);
        assert_eq!(lost("[!a-c].x", 0, "[!a-c]"), ['A', 'B', 'C']);
        assert_eq!(written(&['A', 'B', 'C', 'X']), "A-C, X");
        // And against the whole filter: a name another pattern still keeps
        // with the option is not lost.
        assert_eq!(
            among("[!a].dat", &["A.dat"]).lost_here("[!a].dat", 0, "[!a]", false),
            Vec::<char>::new()
        );
        assert_eq!(
            among("[!a].dat", &["Keep"]).lost_here("[!a].dat", 0, "[!a]", false),
            ['A']
        );
        // cr-ecm10-1: a name is tried with each `*` as some text and as
        // none. With `Ax.txt` kept by another pattern, `A.txt` still shows
        // what `[!a]*.txt` gives up.
        assert_eq!(
            among("[!a]*.txt", &["Keep", "Ax.txt"]).lost_here("[!a]*.txt", 0, "[!a]", false),
            ['A']
        );
        // cr-ecm10-2: and against BOTH lists. No include names `A.bin`, so
        // the run leaves it alone either way and nothing is lost at
        // `[!a]`; `B.txt` is let in by `*.txt` once `[!b]` refuses it.
        let (excludes, includes) = (["[!a].bin", "[!b].txt"], ["*.txt"]);
        assert_eq!(
            within("[!a].bin", Role::Exclude, &excludes, &includes)
                .lost_here("[!a].bin", 0, "[!a]", false),
            Vec::<char>::new()
        );
        assert_eq!(
            within("[!b].txt", Role::Exclude, &excludes, &includes)
                .lost_here("[!b].txt", 0, "[!b]", false),
            ['B']
        );
        // An include the option widens lets a name in only if no exclude,
        // read the same way, takes it out again.
        assert!(within("*.Txt", Role::Include, &[], &["*.Txt"]).gained("x.txt"));
        assert!(!within("*.Txt", Role::Include, &["*.TXT"], &["*.Txt"]).gained("x.txt"));
        // So with a gain: a spelling another pattern names already is not
        // offered.
        assert_eq!(offered(&asked("a"), "A").as_deref(), Some("A"));
        assert_eq!(offered(&among("a", &["A"]), "A"), None);
        assert_eq!(offered(&among("a", &["./A"]), "A"), None);
        // cr-ecm10-5: nor the very glob typed beside it. The name the
        // option adds, `QB`, is none that `Q[b-~]` denotes.
        assert_eq!(offered(&among("q[b-~]", &["Q[b-~]"]), "Q[b-~]"), None);
        assert!(among("q[b-~]", &["Q[b-~]"]).gained("QB"));
        // `.O` alone is a name `??` takes; `*.O` denotes `x.O` as well,
        // which it does not.
        assert_eq!(
            offered(&among("*.o", &["??"]), "*.O").as_deref(),
            Some("x.O")
        );
        assert_eq!(offered(&among("*.o", &["*"]), "*.O"), None);
        // cr-ecm10-1: whether a pattern's readings are THE SAME is asked of
        // the pattern alone. Beside `A` — or beside `./A`, which keeps `A`
        // at the root and nowhere else — `a` still reads otherwise; there
        // is only no example of it the run bears out.
        assert_eq!(
            among("a", &["A"]).compared("a", false),
            Compared::Differs(vec![])
        );
        assert_eq!(
            among("a", &["./A"]).compared("a", false),
            Compared::Differs(vec![])
        );
        assert_eq!(
            among("a", &["b"]).compared("a", false),
            Compared::Differs(vec!["A".to_string()])
        );

        // A pattern is left out only when it is ESTABLISHED that the
        // option changes nothing it names: both readings asked about every
        // spelling of every name it holds. Where that cannot be done — a
        // wildcard, a class, too many spellings — nothing is concluded.
        let compared = |pattern: &str| asked(pattern).compared(pattern, false);
        for same in ["{a,A}", "{ab,AB,aB,Ab}", "{x/a,x/A,X/a,X/A}", "{Σ,σ,ς}"] {
            assert_eq!(compared(same), Compared::Same, "{same}");
        }
        assert_eq!(
            compared("{Σ,σ}"),
            Compared::Differs(vec!["ς".to_string()]),
            "a letter that is neither the lower nor the upper case of the ones written"
        );
        for (differs, one) in [("{ab,AB}", "Ab"), ("Keep", "keep"), ("sub/{a,A}", "SUB/a")] {
            match compared(differs) {
                Compared::Differs(names) => assert!(
                    !names.is_empty() && names.len() <= MOST_EXAMPLES,
                    "{differs}: {names:?}"
                ),
                other => panic!("{differs}: {other:?}"),
            }
            // Whatever is given back, this one is among what differs.
            let asked = asked(differs);
            assert_ne!(
                Asked::named(Role::Exclude, &asked.as_typed, one),
                Asked::named(Role::Exclude, &asked.ignoring_case, one),
                "{differs}: {one}"
            );
        }
        for unknown in [
            "{[a-c],A,B}",
            "*.[Jj][Pp][Gg]",
            "[A-c]1",
            "x?{a,A}",
            "**//Cache",
            "//Cache",
            "abcdefghijk",
        ] {
            assert_eq!(compared(unknown), Compared::Unknown, "{unknown}");
        }
        // Names are read off a text's own syntax — a `*` as some text, or
        // as none.
        assert_eq!(
            samples("a{b,{c,D}}*?", false, None).unwrap(),
            ["abxx", "acxx", "aDxx"]
        );
        assert_eq!(
            samples_with("a{b,{c,D}}*?", false, None, "").unwrap(),
            ["abx", "acx", "aDx"]
        );
        assert_eq!(samples_with("*.o", false, None, "").unwrap(), [".o"]);
        // A negated class is tried with the other case of a letter it
        // holds as well: the name a spelling goes wrong on.
        assert_eq!(
            samples(r"[!x]\*[q-s]", false, None).unwrap(),
            ["q*q", "X*q"]
        );
        assert_eq!(
            samples("p[!x]q[!x]", false, Some((1, 'X'))).unwrap(),
            ["pXqq", "pXqX"],
            "the chosen letter, at the class that stands there"
        );
        assert_eq!(
            samples("p[!x]q[!x]", false, Some((6, 'X'))).unwrap(),
            ["pqqX", "pXqX"]
        );
        // With a place chosen, only the names read through it.
        assert_eq!(
            samples("{[!a].1,A.1,[!b].2}", false, Some((1, 'A'))).unwrap(),
            ["A.1"]
        );
        assert_eq!(
            samples("{[!a].1,A.1,[!b].2}", false, Some((12, 'B'))).unwrap(),
            ["B.2"]
        );
        assert_eq!(
            samples("{[!a].1,A.1,[!b].2}", false, Some((8, 'Z'))).unwrap(),
            Vec::<String>::new(),
            "no class stands there"
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
