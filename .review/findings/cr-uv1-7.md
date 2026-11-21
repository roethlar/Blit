# cr-uv1-7: `find`'s `[:class:]` was not the reference locale's

**Severity**: MEDIUM — `-name` with a POSIX class finds names `find` does not, and misses names it does
**Status**: Reviewed, found incomplete — `utility-slices-r2` (Codex openreview, 2026-11-21) reviewed `4841b2d4..3cd49e40`, the seven `utility-slices-r1` fixes, with `guard_confirmed: true`; its own guard was the reviewer's red/green proof. The gap recorded below is now a finding: a code point no character is assigned to (U+0378) is `[:print:]` here and not to `find`. And a start typed as the module's root with a final `.` is given the module's name for `-name`, where `find` sees `.`. The remaining work is cr-uv2-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `2fc6523c`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`.

## Evidence
In direct comparisons, GNU `find` matched an EM SPACE with `[[:blank:]]`; Blit missed it. Blit matched a NO-BREAK SPACE with `[[:space:]]`; GNU excluded it. Use the reference locale's class definitions across platforms and include these cases in the comparison tests. (Slice 3 had recorded the classes as a known gap.)

Cause. The classes were the standard library's predicates of the same names. The reference — GNU find under `C.UTF-8` — uses the C library's classes for that locale, which are defined by other rules.

What those rules are was measured, not read: every code point was asked of this machine's C library (glibc 2.44, `iswctype` under `C.UTF-8`) for each of the twelve classes, and of its `towlower`, and compared with the standard library (`scratchpad` scripts; the result is the guard below). Where they differ, for characters both know:
- `space` is white space without the three no-break spaces (U+00A0, U+2007, U+202F) and without U+0085;
- `blank` is every `space` that is no control, and the tab — 15 characters, not 2;
- `cntrl` also holds the line and paragraph separators (U+2028, U+2029);
- `alpha` also holds every decimal digit of a script other than ASCII's (760 characters in 71 runs; `digit` is the ten of ASCII);
- `upper` also holds the 31 title-case letters (`ǅ`, `ᾈ`), and `lower` the four of those that are digraphs;
- `punct` is exactly `graph` less `alpha` and the ASCII digits, and `graph` exactly `print` less `space` (checked over every code point: no exception);
- `-iname`'s lowercase mapping already equals `towlower` for every character the library knows.

## Predicted observable failure
`blit find -name '*[[:blank:]]*'` misses names holding any blank but a space or tab; `'*[[:space:]]*'` finds names holding a no-break space; `'[[:alpha:]]'` misses a digit of another script; `'[[:upper:]]*'` misses a title-case first letter.

## Intake
Admitted as the reviewer put it.

## What
`find_name` — the classes of a character are written as the locale's rules over this build's Unicode data: the standard library's properties, three small exception sets, and one table (the runs of other decimal digits, as the C library has them). The byte reading is unchanged: a byte outside ASCII is in no class.

## Guard proof
- `find_name::tests::the_classes_are_the_c_librarys_under_c_utf8` (Linux): for EVERY code point the C library's `C.UTF-8` tables know (more than 100,000; asked through `newlocale`/`iswctype_l`, the process's own locale untouched), each of the twelve classes here equals the library's, and the lowercase mapping equals `towlower_l`. A code point the library holds no character for is not compared — this build's Unicode data is newer than the library's. Skips, saying so, on a system with no `C.UTF-8` locale.
- `find_name::tests::a_class_is_the_reference_locales` (every OS): the reviewer's two cases and one character for each rule above — 41 rows — the table's runs, and four patterns as the command is given them.
- `find.rs::find_equals_the_system_find` (Linux): four more names (an em space, a no-break space, a digit of another script, a title-case letter) and eleven more expressions with classes, each compared with the system `find`'s output.
- Mutations, each restored byte for byte: `space` with a no-break space, as before; `blank` the space and the tab only, as before; `alpha` without the other decimal digits; `upper` without title case; `lower` without the digraphs → all three guards red, the comparison with the system `find` among them; `cntrl` without the separators; `punct` by the standard library's "alphanumeric" → the first two.

## Files changed
- `crates/blit-core/src/find_name.rs` — `impl Unit for char`, `OTHER_DECIMAL_DIGITS`, `is_title_case`; the guards.
- `crates/blit-cli/tests/find.rs` — names and expressions.
- `CHANGELOG.md`.

## Known gaps
- **A code point no character is assigned to counts as printable** (`print`, `graph`, `punct`); the reference says no. The standard library cannot be asked whether a code point is assigned.
- The Unicode data is this build's, the reference's is its C library's: a character newer than the library is classed here and unknown there (12,833 letters at this writing), and the reverse will come.
- The table of other decimal digits is as glibc 2.44 has it; a script added later needs a line.
- The guard against the C library runs on Linux only; on macOS and Windows the sampled guard is the evidence, and nothing was run there.
- Equivalence classes and collating symbols (`[=a=]`, `[.a.]`) are still refused, as slice 3 left them.
