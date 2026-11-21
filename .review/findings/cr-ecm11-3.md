# cr-ecm11-3: The line printed a spelling other than the name it had checked

**Severity**: HIGH — the question names a file `--ignore-case` would "also match" that both runs treat alike, and not the file the option's mirror deletes
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm11-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r11` over `d3e33acc..bfa537f9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r11.codex.md`. It completes cr-ecm10-5.

## Evidence
"With `--include '*a' --exclude '[Aa]'`, the mirror prompt said `--ignore-case would also match A`. Both mirrors preserved `A`. The file deleted only with `--ignore-case` was `xA`. The code finds a changed name, reduces that result to a boolean, and prints a different spelling. Return and print the verified name."

Cause. cr-ecm10-5 checks a spelling as the whole pattern in that case (`*A`) on the names it denotes (`A`, `xA`), and offers the spelling when one of them is a name the run would treat otherwise. What it then printed was the pattern without its end wildcards — `A` — which reads as one name, and that name was not the one that passed: `[Aa]` leaves `A` out with or without the option.

## Intake
Admitted as the reviewer put it.

## What
- **The check gives back the name** (`Asked::would_also_match`): the one the run would treat otherwise. The first tried is what the text to be printed itself reads as, then the spelling with nothing for a `*`, then with some text — so for `*Cache` the name is `cache`, as it was.
- **The line prints the spelling only when it is that name** — or when the text still holds a wildcard or a class, and so is shown as a pattern, some name of which was checked (`IMG_*.JPG`). Where the text reads as one name and the name checked is another, the NAME is printed.
- **And then the line does not say what the pattern "matches only"**: `"*a" matches exactly as typed; capital letters matter (--ignore-case would also match xA).` Its two halves would not be about the same thing.

For `*.o` beside `??`, which takes `.O` either way: `would also match x.O`, where it said `.O`.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the reviewer's command — the line above, to the letter.
- `case_fold::tests::each_claim_is_checked_against_the_filter`: the name given back for `*Cache` (`cache`, `CACHE`), `*.Txt` (`.TXT`), `a` (`A`), `*.o` beside `??` (`x.O`); `./cache` stands for its own name `cache`; for `*a` beside `[Aa]`, `A` is not gained, `xA` is, and it is not the spelling's own; none beside a pattern that covers every name.
- `case_fold::tests::the_case_note_says_what_the_first_lettered_pattern_matches`: `*.o` beside `??`.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary: `--include '*a' --exclude '[Aa]'` — the line, then both mirrors: `xA` kept as typed and deleted with the option, `A` and `a` kept by both.
- Mutations, each restored byte for byte: the spelling printed whatever name showed the change, as before; a name the spelling denotes taken for the spelling's own; the line still saying what the pattern matches only; the text shown not tried first — four, each red on the guards named for it.

Speed, the two commands of cr-ecm10-6 (release, medians of 9): 10.5 ms and 8.7 ms, as before (10.7, 8.5).

## Files changed
- `crates/blit-core/src/case_fold.rs` — `Asked::would_also_match`, `Spellings::with`, `Spellings::others_are_spellings`, `reads_as_one_name`, the line; the guards.
- `crates/blit-cli/tests/case_note.rs` — the case.
- `CHANGELOG.md`.

## Known gaps
- **A name with `x` in it is an example, not a file anyone has**: `xA` stands for "something, then `A`".
- **A spelling shown as a pattern** (`IMG_*.JPG`) is still a pattern: what was checked is that no name it denotes is one the option does not match, and that one is a name the run would treat otherwise.
- **The first half, `matches Cache only`, is the pattern's text and not a claim checked against the run**: beside an include that names no `Cache`, the run copies no such file either way.
- Names are tried at the top of the tree, with `*` as `x` and as nothing.
- Run on Linux only.
