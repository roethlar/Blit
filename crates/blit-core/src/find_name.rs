//! `find`'s name tests, matched as the Unix command matches them
//! (`docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` slice 3, AC-4).
//!
//! `-name PATTERN` and `-iname PATTERN` compare an entry's BASE NAME with
//! a shell pattern: `*` is any run of characters (a leading dot
//! included), `?` is any one, `[...]` is one of a set, and `\` takes the
//! next character as itself. The reference is GNU find 4.11 under
//! `C.UTF-8`, and two of its rules are not what the transfer filters do —
//! each verb answers to the tool it is named after:
//!
//! - **Two readings.** A name matches when the pattern fits it read as
//!   CHARACTERS or read as BYTES. So `?.dat` finds `é.dat` (one
//!   character) and `??.dat` finds it too (two bytes). A name or a pattern
//!   that is not valid UTF-8 has only the byte reading.
//! - **`-iname` compares one character at a time by its lowercase form**
//!   — not by case folding. `σ.sig` finds `Σ.sig` and not `ς.sig`, whose
//!   lowercase is itself; `i.x` finds `İ.x` and not `ı.x`.
//!
//! This is not [`crate::fs_enum`]'s glob engine and shares nothing with
//! it: a transfer's `--exclude` follows rsync's rule
//! (`docs/plan/EXCLUDE_CASE_MATCHING.md`), `find` follows find's.

mod reference_classes;

/// A `-name` or `-iname` pattern, ready to be matched against base names.
#[derive(Debug, Clone)]
pub struct NamePattern {
    as_bytes: Vec<Token<u8>>,
    /// The pattern read as characters; `None` when it is not valid UTF-8.
    as_chars: Option<Vec<Token<char>>>,
    ignore_case: bool,
}

impl NamePattern {
    /// `pattern` as typed; `ignore_case` for `-iname`. An error names what
    /// in the pattern this matcher does not read — never silently another
    /// meaning.
    pub fn new(pattern: &[u8], ignore_case: bool) -> Result<Self, String> {
        let as_chars = match std::str::from_utf8(pattern) {
            Ok(text) => Some(compile(&text.chars().collect::<Vec<char>>())?),
            Err(_) => None,
        };
        Ok(Self {
            as_bytes: compile(pattern)?,
            as_chars,
            ignore_case,
        })
    }

    /// Whether the pattern fits `name` — one base name, as its bytes.
    pub fn matches(&self, name: &[u8]) -> bool {
        if let (Some(tokens), Ok(text)) = (&self.as_chars, std::str::from_utf8(name)) {
            let chars: Vec<char> = text.chars().collect();
            if fits(tokens, &chars, self.ignore_case) {
                return true;
            }
        }
        fits(&self.as_bytes, name, self.ignore_case)
    }
}

/// What a pattern and a name are read as: characters, or bytes.
trait Unit: Copy + Ord {
    fn ascii(self) -> Option<u8>;
    /// The lowercase form `-iname` compares by.
    fn lower(self) -> Self;
    fn is(self, class: Class) -> bool;
}

impl Unit for u8 {
    fn ascii(self) -> Option<u8> {
        self.is_ascii().then_some(self)
    }
    fn lower(self) -> Self {
        self.to_ascii_lowercase()
    }
    /// A byte outside ASCII is part of some character and in no class.
    fn is(self, class: Class) -> bool {
        self.is_ascii() && (self as char).is(class)
    }
}

impl Unit for char {
    fn ascii(self) -> Option<u8> {
        self.is_ascii().then_some(self as u8)
    }
    /// The character's simple lowercase mapping — one character for one.
    /// The standard library gives the full mapping, which differs for a
    /// single character: `İ` (U+0130) becomes `i` and a combining dot,
    /// where the simple mapping, and the reference, give `i`.
    fn lower(self) -> Self {
        if self == '\u{130}' {
            return 'i';
        }
        self.to_lowercase().next().unwrap_or(self)
    }
    /// The classes of the reference's locale, `C.UTF-8`, by that
    /// locale's own tables (cr-uv2-6): [`reference_classes`], read from
    /// the C library for every code point, the ones no character is
    /// assigned to among them. They are not the standard library's
    /// predicates of the same names — there a no-break space is a
    /// `space`, an em space no `blank`, a digit of another script no
    /// `alpha`, a title-case letter not `upper` — and they are not
    /// written over this build's Unicode data, as cr-uv1-7 had them:
    /// the standard library cannot say that a code point is unassigned,
    /// so U+0378 was printable here and is not to `find`, and a compiler
    /// with newer tables than the reference's would have moved the
    /// classes with it.
    ///
    /// Four classes are tables. The other eight are those and a few
    /// fixed code points, by identities that hold for every code point
    /// of the reference (guarded on Linux against the library itself).
    fn is(self, class: Class) -> bool {
        use reference_classes::{ALPHA, LOWER, PRINT, UPPER};
        let point = self as u32;
        match class {
            Class::Alpha => within(&ALPHA, point),
            Class::Digit => self.is_ascii_digit(),
            Class::Alnum => self.is(Class::Alpha) || self.is_ascii_digit(),
            Class::Upper => within(&UPPER, point),
            Class::Lower => within(&LOWER, point),
            // No no-break space among them, and not the C1 "next line".
            Class::Space => matches!(
                point,
                0x9..=0xd
                    | 0x20
                    | 0x1680
                    | 0x2000..=0x2006
                    | 0x2008..=0x200a
                    | 0x2028
                    | 0x2029
                    | 0x205f
                    | 0x3000
            ),
            // A space that is no control, and the tab.
            Class::Blank => self == '\t' || (self.is(Class::Space) && !self.is(Class::Cntrl)),
            // The two separators count as controls.
            Class::Cntrl => matches!(point, 0..=0x1f | 0x7f..=0x9f | 0x2028 | 0x2029),
            Class::Xdigit => self.is_ascii_hexdigit(),
            Class::Print => within(&PRINT, point),
            Class::Graph => self.is(Class::Print) && !self.is(Class::Space),
            // Every graphic character that is no letter and no digit.
            Class::Punct => {
                self.is(Class::Graph) && !self.is(Class::Alpha) && !self.is_ascii_digit()
            }
        }
    }
}

/// Whether a character can be printed, by the reference locale's
/// `[:print:]`: what a message shows as itself, and what it must write
/// some other way ([`crate::admin::quote`]).
pub fn is_printable(character: char) -> bool {
    Unit::is(character, Class::Print)
}

/// Whether `point` is in the class `edges` describes: the code points at
/// which membership changes, in order ([`reference_classes`]).
fn within(edges: &[u32], point: u32) -> bool {
    edges.partition_point(|&edge| edge <= point) % 2 == 1
}

/// A POSIX character class, `[:name:]` inside a bracket expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Alpha,
    Digit,
    Alnum,
    Upper,
    Lower,
    Space,
    Blank,
    Cntrl,
    Xdigit,
    Print,
    Graph,
    Punct,
}

impl Class {
    fn named(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"alpha" => Self::Alpha,
            b"digit" => Self::Digit,
            b"alnum" => Self::Alnum,
            b"upper" => Self::Upper,
            b"lower" => Self::Lower,
            b"space" => Self::Space,
            b"blank" => Self::Blank,
            b"cntrl" => Self::Cntrl,
            b"xdigit" => Self::Xdigit,
            b"print" => Self::Print,
            b"graph" => Self::Graph,
            b"punct" => Self::Punct,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token<U> {
    /// `*`
    Run,
    /// `?`
    One,
    Literal(U),
    Set {
        negated: bool,
        members: Vec<Member<U>>,
    },
    /// A pattern ending in a lone `\`: the reference matches nothing.
    Nothing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Member<U> {
    One(U),
    Range(U, U),
    Class(Class),
}

fn is<U: Unit>(unit: U, ascii: u8) -> bool {
    unit.ascii() == Some(ascii)
}

fn compile<U: Unit>(pattern: &[U]) -> Result<Vec<Token<U>>, String> {
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(&unit) = pattern.get(at) {
        at += 1;
        let token = if is(unit, b'*') {
            // A run of stars is one.
            if tokens.last() == Some(&Token::Run) {
                continue;
            }
            Token::Run
        } else if is(unit, b'?') {
            Token::One
        } else if is(unit, b'\\') {
            match pattern.get(at) {
                Some(&escaped) => {
                    at += 1;
                    Token::Literal(escaped)
                }
                None => Token::Nothing,
            }
        } else if is(unit, b'[') {
            match set(pattern, at)? {
                Some((token, after)) => {
                    at = after;
                    token
                }
                // Never closed: a `[` like any other character.
                None => Token::Literal(unit),
            }
        } else {
            Token::Literal(unit)
        };
        tokens.push(token);
    }
    Ok(tokens)
}

/// The bracket expression whose `[` stands just before `from`: the token
/// and where the pattern goes on after its `]`; `None` when it is never
/// closed.
fn set<U: Unit>(pattern: &[U], from: usize) -> Result<Option<(Token<U>, usize)>, String> {
    let mut at = from;
    let negated = pattern
        .get(at)
        .is_some_and(|&unit| is(unit, b'!') || is(unit, b'^'));
    if negated {
        at += 1;
    }
    let mut members = Vec::new();
    let first = at;
    loop {
        let Some(&unit) = pattern.get(at) else {
            return Ok(None);
        };
        at += 1;
        // A `]` closes the set, unless it stands first in it.
        if is(unit, b']') && at - 1 != first {
            return Ok(Some((Token::Set { negated, members }, at)));
        }
        let start = if is(unit, b'\\') {
            let Some(&escaped) = pattern.get(at) else {
                return Ok(None);
            };
            at += 1;
            escaped
        } else if is(unit, b'[') && pattern.get(at).is_some_and(|&next| is(next, b':')) {
            // `[:name:]`, when it is closed; else `[` is a member.
            let name: Vec<u8> = pattern[at + 1..]
                .iter()
                .map_while(|unit| unit.ascii().filter(u8::is_ascii_lowercase))
                .collect();
            let end = at + 1 + name.len();
            let closed = pattern.get(end).is_some_and(|&c| is(c, b':'))
                && pattern.get(end + 1).is_some_and(|&c| is(c, b']'));
            if closed {
                let class = Class::named(&name).ok_or_else(|| {
                    format!(
                        "[:{}:] is not a character class",
                        String::from_utf8_lossy(&name)
                    )
                })?;
                members.push(Member::Class(class));
                at = end + 2;
                continue;
            }
            unit
        } else if is(unit, b'[')
            && pattern
                .get(at)
                .is_some_and(|&next| is(next, b'=') || is(next, b'.'))
        {
            return Err(
                "an equivalence class ([=a=]) or collating symbol ([.a.]) in a bracket \
                 expression is not supported"
                    .to_string(),
            );
        } else {
            unit
        };
        // `a-c`, when something other than the closing `]` follows the `-`.
        let ranged = pattern.get(at).is_some_and(|&dash| is(dash, b'-'))
            && pattern.get(at + 1).is_some_and(|&end| !is(end, b']'));
        if !ranged {
            members.push(Member::One(start));
            continue;
        }
        let mut end = pattern[at + 1];
        at += 2;
        if is(end, b'\\') {
            let Some(&escaped) = pattern.get(at) else {
                return Ok(None);
            };
            at += 1;
            end = escaped;
        }
        members.push(Member::Range(start, end));
    }
}

impl<U: Unit> Token<U> {
    /// Whether this token takes `unit` as the one character it stands for.
    fn takes(&self, unit: U, ignore_case: bool) -> bool {
        let fold = |unit: U| if ignore_case { unit.lower() } else { unit };
        match self {
            Token::One => true,
            Token::Literal(literal) => fold(*literal) == fold(unit),
            Token::Set { negated, members } => {
                let folded = fold(unit);
                let held = members.iter().any(|member| match member {
                    Member::One(one) => fold(*one) == folded,
                    Member::Range(low, high) => fold(*low) <= folded && folded <= fold(*high),
                    // A class is asked about the character as it is
                    // written, in either mode — as the reference asks.
                    Member::Class(class) => unit.is(*class),
                });
                held != *negated
            }
            Token::Run | Token::Nothing => false,
        }
    }
}

fn fits<U: Unit>(tokens: &[Token<U>], name: &[U], ignore_case: bool) -> bool {
    let (mut token, mut unit) = (0, 0);
    // Where to go back to: the token after the last `*`, and how much of
    // the name that star has taken so far.
    let mut star: Option<(usize, usize)> = None;
    loop {
        match tokens.get(token) {
            Some(Token::Run) => {
                star = Some((token + 1, unit));
                token += 1;
                continue;
            }
            Some(Token::Nothing) => return false,
            Some(one) if unit < name.len() && one.takes(name[unit], ignore_case) => {
                token += 1;
                unit += 1;
                continue;
            }
            None if unit == name.len() => return true,
            _ => {}
        }
        match star {
            Some((after, taken)) if taken < name.len() => {
                star = Some((after, taken + 1));
                token = after;
                unit = taken + 1;
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names `pattern` finds among `names`.
    fn found<'a>(pattern: &str, ignore_case: bool, names: &[&'a str]) -> Vec<&'a str> {
        let pattern = NamePattern::new(pattern.as_bytes(), ignore_case).expect("a pattern");
        names
            .iter()
            .copied()
            .filter(|name| pattern.matches(name.as_bytes()))
            .collect()
    }

    const NAMES: [&str; 38] = [
        "a.dat",
        "ab.dat",
        "abc.dat",
        "é.dat",
        "éa.dat",
        "€.dat",
        ".hidden",
        "A.DAT",
        "x]y",
        "x-y",
        "x^y",
        "x!y",
        "x*y",
        "x?y",
        "x\\y",
        "[lit]",
        "ÜBUNG.txt",
        "übung.txt",
        "Σ.sig",
        "σ.sig",
        "ς.sig",
        "ǅ.x",
        "ǆ.x",
        "Ǆ.x",
        "İ.x",
        "i.x",
        "ı.x",
        "I.x",
        "ß.x",
        "ẞ.x",
        "K.x",
        "k.x",
        "\u{212a}.x",
        "0",
        "9",
        "b",
        "B",
        "t",
    ];

    /// cr-uv1-7: the classes are the reference locale's. The reviewer's
    /// two — an em space is a `blank` (it was not), a no-break space is no
    /// `space` (it was) — and one character for each rule that is not the
    /// standard library's.
    #[test]
    fn a_class_is_the_reference_locales() {
        use Class::*;
        for (character, class, member) in [
            ('\u{2003}', Blank, true),
            ('\u{2003}', Space, true),
            ('\u{a0}', Space, false),
            ('\u{a0}', Blank, false),
            ('\u{a0}', Punct, true),
            ('\u{a0}', Graph, true),
            ('\u{2007}', Space, false),
            ('\u{202f}', Space, false),
            ('\u{85}', Space, false),
            ('\u{85}', Cntrl, true),
            ('\u{2028}', Space, true),
            ('\u{2028}', Cntrl, true),
            ('\u{2028}', Blank, false),
            ('\u{2028}', Print, false),
            ('\t', Blank, true),
            ('\t', Cntrl, true),
            ('\n', Space, true),
            ('\n', Blank, false),
            (' ', Blank, true),
            (' ', Print, true),
            (' ', Graph, false),
            // A digit of another script is a letter, and no digit.
            ('٣', Alpha, true),
            ('٣', Digit, false),
            ('٣', Alnum, true),
            ('٣', Punct, false),
            ('３', Alpha, true),
            // A superscript two is a number and no decimal digit.
            ('²', Alpha, false),
            ('²', Punct, true),
            // Title case: upper; and the digraphs lower too.
            ('ǅ', Upper, true),
            ('ǅ', Lower, true),
            ('ᾈ', Upper, true),
            ('ᾈ', Lower, false),
            ('𝐀', Upper, true),
            ('ª', Lower, true),
            ('ß', Lower, true),
            ('é', Alpha, true),
            ('€', Punct, true),
            ('!', Punct, true),
            ('a', Punct, false),
            ('7', Punct, false),
            ('7', Alnum, true),
        ] {
            assert_eq!(
                character.is(class),
                member,
                "U+{:04X} {class:?}",
                character as u32
            );
        }
        // cr-uv2-6: a code point no character is assigned to is in no
        // class — U+0378 was printable, a graphic character and
        // punctuation here, and is none of them to `find`. Its neighbours
        // are letters.
        for class in [
            Alpha, Digit, Alnum, Upper, Lower, Space, Blank, Cntrl, Xdigit, Print, Graph, Punct,
        ] {
            for unassigned in ['\u{378}', '\u{379}', '\u{e0080}', '\u{10ffff}'] {
                assert!(
                    !unassigned.is(class),
                    "U+{:04X} {class:?}",
                    unassigned as u32
                );
            }
        }
        assert!('\u{377}'.is(Print) && '\u{377}'.is(Alpha) && '\u{37a}'.is(Print));
        // And through a pattern, as the command is given it.
        let found = |pattern: &str, name: &str| {
            NamePattern::new(pattern.as_bytes(), false)
                .expect(pattern)
                .matches(name.as_bytes())
        };
        assert!(found("*[[:blank:]]*", "em\u{2003}sp"));
        assert!(!found("*[[:space:]]*", "nb\u{a0}sp"));
        assert!(found("*[[:punct:]]*", "nb\u{a0}sp"));
        assert!(found("d[[:alpha:]]", "d٣") && !found("d[[:digit:]]", "d٣"));
        assert!(!found("[[:print:]]", "\u{378}") && found("[[:print:]]", "\u{377}"));
        assert!(
            found("[![:print:]]", "\u{378}"),
            "one character, in no class"
        );
    }

    /// cr-uv2-6: the tables are the ones that were generated — each in
    /// order, each holding the count of code points written beside it —
    /// so an edit to one does not pass for the reference.
    #[test]
    fn the_reference_tables_are_whole() {
        use reference_classes::*;
        for (name, table, members) in [
            ("alpha", &ALPHA[..], ALPHA_MEMBERS),
            ("upper", &UPPER[..], UPPER_MEMBERS),
            ("lower", &LOWER[..], LOWER_MEMBERS),
            ("print", &PRINT[..], PRINT_MEMBERS),
        ] {
            assert!(table.len().is_multiple_of(2), "{name}");
            assert!(table.windows(2).all(|pair| pair[0] < pair[1]), "{name}");
            assert!(table.last().is_some_and(|&last| last <= 0x110000), "{name}");
            let held: u32 = table.chunks(2).map(|run| run[1] - run[0]).sum();
            assert_eq!(held, members, "{name}");
            // `within` and the runs agree at every edge.
            for run in table.chunks(2) {
                assert!(within(table, run[0]) && within(table, run[1] - 1), "{name}");
                assert!(!within(table, run[1]), "{name}");
                assert!(run[0] == 0 || !within(table, run[0] - 1), "{name}");
            }
        }
        // The counts of glibc 2.44, the reference these guards were
        // measured against; a regenerated table changes them on purpose.
        assert_eq!(READ_FROM, "glibc 2.44");
        assert_eq!(
            (ALPHA_MEMBERS, UPPER_MEMBERS, LOWER_MEMBERS, PRINT_MEMBERS),
            (148_181, 2_037, 2_599, 297_267)
        );
    }

    /// cr-uv1-7 and cr-uv2-6, where the reference is at hand: for EVERY
    /// code point — the ones no character is assigned to among them —
    /// each class here is the C library's under `C.UTF-8`. That is asked
    /// when this machine's library is the one the tables were read from;
    /// a library of another version is compared only where it and the
    /// tables both hold a character, and the test says so. `-iname`'s
    /// lowercase, which is the standard library's, is compared with
    /// `towlower` wherever the library holds a character.
    ///
    /// cr-uv3-8: for glibc only. It asks the library its version by a
    /// call no other C library has, and on a Linux built against another
    /// (musl) this module's tests did not compile. The tables' own guards
    /// above run everywhere.
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn the_classes_are_the_c_librarys_under_c_utf8() {
        use std::ffi::{c_char, c_int, c_ulong, CString};
        extern "C" {
            fn wctype_l(name: *const c_char, locale: libc::locale_t) -> c_ulong;
            fn iswctype_l(character: u32, class: c_ulong, locale: libc::locale_t) -> c_int;
            fn towlower_l(character: u32, locale: libc::locale_t) -> u32;
        }
        let name = CString::new("C.UTF-8").unwrap();
        // SAFETY: a NUL-terminated name and no base locale; the result is
        // checked before it is used and freed below.
        let locale =
            unsafe { libc::newlocale(libc::LC_ALL_MASK, name.as_ptr(), std::ptr::null_mut()) };
        if locale.is_null() {
            eprintln!("skipping: this system has no C.UTF-8 locale");
            return;
        }
        let described = |class: &str| {
            let class = CString::new(class).unwrap();
            // SAFETY: a NUL-terminated class name and the locale made above.
            unsafe { wctype_l(class.as_ptr(), locale) }
        };
        let classes: Vec<(Class, c_ulong)> = [
            "alpha", "digit", "alnum", "upper", "lower", "space", "blank", "cntrl", "xdigit",
            "print", "graph", "punct",
        ]
        .iter()
        .map(|name| {
            (
                Class::named(name.as_bytes()).expect("a class"),
                described(name),
            )
        })
        .collect();
        let (print, cntrl) = (described("print"), described("cntrl"));
        // SAFETY: a scalar value, a description from this locale, and the
        // locale itself, for each call.
        let theirs = |point: u32, class: c_ulong| unsafe { iswctype_l(point, class, locale) } != 0;
        // SAFETY: the C library's own version string, static and
        // NUL-terminated.
        let library = format!(
            "glibc {}",
            unsafe { std::ffi::CStr::from_ptr(libc::gnu_get_libc_version()) }.to_string_lossy()
        );
        let the_reference = library == reference_classes::READ_FROM;
        if !the_reference {
            eprintln!(
                "this machine's C library is {library}; the tables are {}'s: compared only \
                 where both hold a character",
                reference_classes::READ_FROM
            );
        }
        let (mut known, mut compared, mut wrong) = (0u32, 0u32, Vec::new());
        for point in 0..=0x10FFFF {
            let Some(character) = char::from_u32(point) else {
                continue;
            };
            let held_there = theirs(point, print) || theirs(point, cntrl);
            let held_here = character.is(Class::Print) || character.is(Class::Cntrl);
            known += u32::from(held_there);
            if !the_reference && !(held_there && held_here) {
                continue;
            }
            compared += 1;
            for &(class, description) in &classes {
                if character.is(class) != theirs(point, description) {
                    wrong.push(format!("U+{point:04X} {class:?}"));
                }
            }
            if !held_there {
                continue;
            }
            // SAFETY: as above.
            if Unit::lower(character) as u32 != unsafe { towlower_l(point, locale) } {
                wrong.push(format!("U+{point:04X} lowercase"));
            }
        }
        // SAFETY: made above, used no more.
        unsafe { libc::freelocale(locale) };
        assert!(known > 100_000, "the library knew {known} code points");
        if the_reference {
            assert_eq!(compared, 0x110000 - 0x800, "every code point");
        }
        assert!(
            wrong.is_empty(),
            "{} differ from the C library, the first: {:?}",
            wrong.len(),
            &wrong[..wrong.len().min(24)]
        );
    }

    /// AC-4: every line here is what GNU find 4.11.0 printed for the same
    /// names under `C.UTF-8` (run 2026-11-20 on Linux; the comparison with
    /// the system's own `find` is in the CLI's end-to-end guards). Two
    /// readings: `é` is one character and two bytes, `€` one and three.
    #[test]
    fn a_name_matches_as_the_reference_matches_it() {
        for (pattern, expected) in [
            ("?.dat", &["a.dat", "é.dat", "€.dat"][..]),
            ("??.dat", &["ab.dat", "é.dat", "éa.dat"]),
            ("???.dat", &["abc.dat", "éa.dat", "€.dat"]),
            (".*", &[".hidden"]),
            (
                "*.dat",
                &["a.dat", "ab.dat", "abc.dat", "é.dat", "éa.dat", "€.dat"],
            ),
            ("[ab]", &["b"]),
            ("[!ab]", &["0", "9", "B", "t"]),
            ("[^ab]", &["0", "9", "B", "t"]),
            ("[a-c]", &["b"]),
            ("[0-9]", &["0", "9"]),
            ("x[]]y", &["x]y"]),
            ("x[!]]y", &["x-y", "x^y", "x!y", "x*y", "x?y", "x\\y"]),
            ("x[*]y", &["x*y"]),
            ("x\\*y", &["x*y"]),
            ("x\\\\y", &["x\\y"]),
            ("x[\\\\]y", &["x\\y"]),
            ("[[]lit]", &["[lit]"]),
            ("\\[lit\\]", &["[lit]"]),
            ("[lit]", &["t"]),
            ("x[-!^]y", &["x-y", "x^y", "x!y"]),
            ("x[!-]y", &["x]y", "x^y", "x!y", "x*y", "x?y", "x\\y"]),
            ("[[:upper:]]", &["B"]),
            ("[[:digit:]]", &["0", "9"]),
            ("[é€].dat", &["é.dat", "€.dat"]),
            // Never closed: `[` is a character, and no name here is `[a`.
            ("[a", &[]),
            // A lone `\` at the end matches nothing.
            ("a.dat\\", &[]),
            // A base name holds no `/`.
            ("sub/inner.dat", &[]),
            ("t", &["t"]),
        ] {
            assert_eq!(found(pattern, false, &NAMES), expected, "-name {pattern}");
        }
        assert_eq!(
            found("*", false, &NAMES).len(),
            NAMES.len(),
            "a leading dot too"
        );
        assert_eq!(found("[a", false, &["[a", "a"]), ["[a"]);
        // Not even a name that ends in one.
        assert_eq!(
            found("a.dat\\", false, &["a.dat\\", "a.dat"]),
            Vec::<&str>::new()
        );
    }

    /// AC-4, `-iname`: one character at a time by its lowercase form — the
    /// reference's rule, which is not case folding. Final sigma's
    /// lowercase is itself; dotless `ı` is nobody's lowercase but its own.
    /// A class is asked about the character as written.
    #[test]
    fn iname_compares_by_lowercase_one_character_at_a_time() {
        for (pattern, expected) in [
            ("übung*", &["ÜBUNG.txt", "übung.txt"][..]),
            ("ÜBUNG*", &["ÜBUNG.txt", "übung.txt"]),
            ("σ.sig", &["Σ.sig", "σ.sig"]),
            ("Σ.sig", &["Σ.sig", "σ.sig"]),
            ("ς.sig", &["ς.sig"]),
            ("ǆ.x", &["ǅ.x", "ǆ.x", "Ǆ.x"]),
            ("ǅ.x", &["ǅ.x", "ǆ.x", "Ǆ.x"]),
            ("Ǆ.x", &["ǅ.x", "ǆ.x", "Ǆ.x"]),
            ("i.x", &["İ.x", "i.x", "I.x"]),
            ("I.x", &["İ.x", "i.x", "I.x"]),
            ("İ.x", &["İ.x", "i.x", "I.x"]),
            ("ı.x", &["ı.x"]),
            ("ß.x", &["ß.x", "ẞ.x"]),
            ("ẞ.x", &["ß.x", "ẞ.x"]),
            ("k.x", &["K.x", "k.x", "\u{212a}.x"]),
            ("\u{212a}.x", &["K.x", "k.x", "\u{212a}.x"]),
            ("[a-c]", &["b", "B"]),
            ("[A-C]", &["b", "B"]),
            ("a.DAT", &["a.dat", "A.DAT"]),
            ("[[:upper:]]", &["B"]),
            ("[[:lower:]]", &["b", "t"]),
        ] {
            assert_eq!(found(pattern, true, &NAMES), expected, "-iname {pattern}");
        }
        // Without it, a letter is itself.
        assert_eq!(found("a.DAT", false, &NAMES), Vec::<&str>::new());
    }

    /// AC-12: a name that is not valid text has the byte reading alone —
    /// `?` is one of its bytes — and a pattern that is not valid text
    /// matches by bytes too.
    #[test]
    fn a_name_that_is_not_text_is_matched_by_its_bytes() {
        let name = b"bad\xff.dat";
        for (pattern, matched) in [
            ("bad*.dat", true),
            ("bad?.dat", true),
            ("bad??.dat", false),
            ("*", true),
            ("BAD?.DAT", false),
        ] {
            let pattern_read = NamePattern::new(pattern.as_bytes(), false).unwrap();
            assert_eq!(pattern_read.matches(name), matched, "{pattern}");
        }
        assert!(NamePattern::new(b"BAD?.DAT", true).unwrap().matches(name));
        assert!(NamePattern::new(b"bad\xff.*", false).unwrap().matches(name));
        assert!(!NamePattern::new(b"bad\xfe.*", false).unwrap().matches(name));
    }

    /// What this matcher does not read is refused, by name — never given
    /// another meaning in silence.
    #[test]
    fn what_is_not_read_is_refused() {
        for pattern in ["[[=a=]]", "[[.a.]]", "x[[:nosuch:]]"] {
            let refused = NamePattern::new(pattern.as_bytes(), false).expect_err(pattern);
            assert!(
                refused.contains("not supported") || refused.contains("not a character class"),
                "{pattern}: {refused}"
            );
        }
        // An unclosed class name is three characters of a set.
        assert_eq!(
            found("[[:x]", false, &["[", ":", "x", "a"]),
            ["[", ":", "x"]
        );
    }

    /// A star gives back what it took: the pattern after it is tried at
    /// every place, and a second star starts afresh.
    #[test]
    fn a_star_takes_as_much_as_the_rest_needs() {
        for (pattern, name, matched) in [
            ("*a*b", "xaxbxab", true),
            ("*a*b", "xaxbxa", false),
            ("a*", "a", true),
            ("*a", "ba", true),
            ("**a**", "a", true),
            ("*?", "", false),
            ("*", "", true),
            ("", "", true),
            ("", "a", false),
            ("a*b*c", "abcabc", true),
            ("a*b*c", "abcab", false),
        ] {
            assert_eq!(
                NamePattern::new(pattern.as_bytes(), false)
                    .unwrap()
                    .matches(name.as_bytes()),
                matched,
                "{pattern} against {name:?}"
            );
        }
    }
}
