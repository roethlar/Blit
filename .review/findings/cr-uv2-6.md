# cr-uv2-6: Two of `find`'s tests chose otherwise than `find`

**Severity**: MEDIUM — `blit find` selects a different set of paths than `find` for the same expression
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv2-6`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r2` over `4841b2d4..3cd49e40` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r2.codex.md`. It completes cr-uv1-7.

## Evidence
"`find HOST:/module/. -maxdepth 0 -name .` omitted the start, while `-name module` selected it. Use the explicitly typed final `.` for that predicate. Separately, `[[:print:]]` selected a filename consisting of U+0378, an unassigned code point that GNU `find` omitted under `C.UTF-8`. Pin character-class membership to the reference data and include unassigned code points in the comparison."

Cause.
- **The start's name.** cr-uv1-1 named a start `.` when it was typed with a final `.` — and only when the path also ended in a name (`dir/.`). The module itself typed so has no name in its path, and fell through to the module's.
- **The classes.** cr-uv1-7 wrote the reference locale's classes as RULES over the standard library's Unicode predicates. Those predicates cannot say that no character is assigned to a code point, so the rule for `print` was "no control" — true of every unassigned code point — and `graph` and `punct` followed it. cr-uv1-7 recorded this as a known gap, and its guard left such code points out. Rules over this build's data have a second weakness: a compiler with newer Unicode tables than the reference's moves the classes with it.

Measured against GNU find 4.11.0 in a directory `m` (start × `-name`, `-maxdepth 0`): `m/.`, `m/./`, `m/.//`, `m/./.` each differed for `.`, `m` and `.*` — twelve differences, none after.

## Predicted observable failure
`blit find HOST:/module/. -maxdepth 0 -name .`, or any script that starts at `MODULE/.` and prunes or selects the start by `-name '.*'`. `blit find … -name '*[![:print:]]*'`, written to list names with unprintable characters, passes over a name holding an unassigned code point.

## Intake
Admitted as the reviewer put it.

## What
- **A start typed with a final `.` is named `.`** (`start_name`, `blit-daemon`), whatever the rest of its path — the module's root included.
- **The classes are the reference's own tables** (`find_name/reference_classes.rs`): for `alpha`, `upper`, `lower` and `print`, the code points at which membership changes, read from the C library (glibc 2.44, `C.UTF-8`) for EVERY code point, the unassigned among them. The other eight classes are those four and a few fixed code points, by identities checked against the library for every code point (`graph` = `print` without `space`; `punct` = `graph` without letters and digits; `alnum`; `blank`). No class now depends on the compiler's Unicode data. A code point no character is assigned to is in no class.
- **The tables are generated** by `scripts/gen_find_classes.py`, which reads the C library of the machine it runs on; it is in the tree for the day the reference moves.

Not measured: speed. A class is now a binary search of at most 1,612 numbers where it was a predicate; only a pattern holding `[:class:]` asks.

## Guard proof
- `find.rs::find_names_its_start_and_reads_a_class_as_the_system_find_does` (Linux), through the binary and beside the system `find`: nine ways of typing the start — the module with `.`, `./`, `.//`, `./.`, and below it — × seven `-name` patterns, whether the start is printed and the exit status; then ten classes and their negations over names that are one character each (U+0378, U+E0080, a letter, the Greek question mark, a control, a no-break space), the same set printed.
- `find_name::tests::the_classes_are_the_c_librarys_under_c_utf8` (Linux): all twelve classes for every code point, 1,112,064 of them, against the C library — asked in full when this machine's library is the one the tables were read from (it is: glibc 2.44), and otherwise only where both hold a character, saying so.
- `find_name::tests::the_reference_tables_are_whole`: each table in order, holding the count of code points written beside it, `within` agreeing with the runs at every edge; the four counts of glibc 2.44 pinned.
- `find_name::tests::a_class_is_the_reference_locales`: U+0378, U+0379, U+E0080 and U+10FFFF in none of the twelve; through a pattern, `[[:print:]]` and `[![:print:]]`.
- `blit-daemon` `start_name_tests::the_start_is_named_as_find_names_it`: eleven ways of typing a start.
- Mutations, each restored byte for byte: the module typed with a final `.` given the module's name, as before → the daemon and end-to-end guards red; whatever is no control printable, as before → the class, library and end-to-end guards; one edge of the `print` table moved by one code point → those three and the table guard.

## Files changed
- `crates/blit-daemon/src/service/core.rs` — `start_name`; guard.
- `crates/blit-core/src/find_name.rs` — `Unit for char::is`, `within`; the rules' tables removed; guards.
- `crates/blit-core/src/find_name/reference_classes.rs` — new, generated.
- `scripts/gen_find_classes.py` — new.
- `crates/blit-cli/tests/find.rs` — guard.
- `CHANGELOG.md`.

## Known gaps
- **`-iname`'s lowercase is still the standard library's**, compared with the C library's `towlower` wherever that library holds a character; it is not pinned to a table.
- **The reference is one C library's tables** (glibc 2.44). A `find` on a machine with another C library can differ from it, and so from blit, for code points assigned since.
- `HOST:/module` with no `/` after the module is refused by `find`, `du`, `df` and `rm` as a local path — the endpoint grammar's rule, older than this plan; the message is cr-uv2-5's.
- The end-to-end guard compares with whatever `find` and C library the machine has. Run on Linux only.
