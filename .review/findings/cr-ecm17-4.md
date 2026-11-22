# cr-ecm17-4: the case line's proof assumed a `*` stops at a separator, and never tried a class on a tab

**Severity**: MEDIUM — the line names, as what `--ignore-case` would add, a spelling some paths of which are matched either way
**Status**: Reviewed, found incomplete — `combined-r19` (Codex openreview, 2026-11-22) reviewed `ca377805..17f4eb97` — the eight `combined-r18` fixes — with `guard_confirmed: true`. "The CLI advertised `1[A/]2` as an additional match for `--include '1[a/]2'`. I ran both spellings case-sensitively and both copied `1/2`. The proof omits `/`, which a class can match in a relative path." The remaining work is cr-ecm18-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c5077057`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-ecm16-4 (`9723cef9`).

## Evidence
"`--include '*/Cache/*'` advertised `*/cache/*` as an additional match. I copied `x/Cache/cache/saved` with either pattern as typed. A class containing `a` and a literal tab also shared a match with its advertised uppercase spelling. The component comparison misses separator-crossing `*`, and the class comparison omits tabs."

cr-ecm16-4 compared a path pattern component by component, which holds only if a `*` stays inside its component. It does not. And the characters two classes were tried on were the printable ones only.

What the filter matches, measured through it (a probe of the filter in this build, Linux; Include and Exclude alike for what the pattern itself matches):
- every pattern ends where the path ends; a `*` crosses separators (`x*/y` matches `xa/b/y`; `*/Cache/*` matches `x/y/Cache/z`);
- a pattern with a separator in it is anchored at the root (`Cache/*` does not match `y/Cache/x`);
- a pattern of one component matches an entry's own name at any depth (`d/IMG_x.jpg`) AND the whole path from the root, its `*` crossing (`IMG_a/b.jpg`; `a*b` matches `a/x/b`).

## Intake
Admitted as the reviewer put it. The third point is new and goes further than the reviewer's case: for a pattern of one component, its START tells nothing — `img_*.jpg` and `IMG_*.jpg` both match `IMG_a/img_b.jpg`.

## What
`cannot_share_a_name` reads each text as one row of places — a separator a place like any other — and tells the two apart:
- a fixed way from the END, after the last place whose width varies, for every pattern;
- a fixed way from the START, before the first such place, only for a pattern anchored at the root (a separator in it, a trailing one aside).
Classes are tried on every character from 1 to 127 but `/`, and on one beyond.

What an exclude takes with a folder — everything below it — is the folder's doing, not what the pattern matches, and is no part of the claim; it is not for a pattern of plain text either.

What changes on the line: `IMG_*.jpg` shows `img_x.jpg` and `IMG_*.JPG` (was `img_*.jpg`, `IMG_*.JPG`) and no longer says what it matches "only"; `*/Cache/*` shows `x/cache/x`, `x/CACHE/x`; a class alone holding a tab (`[a\t]`) shows `A`. And `**/*/Cache` shows `*/cache`, `*/CACHE` again: cr-ecm16-4 had changed it to names on the belief that the two share `x/Cache/cache` through its folder — what a pattern itself matches ends where the path ends, so they share nothing.

## Guard proof
- `case_fold::tests::a_spelling_that_reads_as_what_the_pattern_has_as_typed_is_not_printed` (extended): the reviewer's path; a one-component pattern whose start differs; a class alone holding a tab, and beside a letter; `**/*/Cache` back to its spellings.
- `the_case_note_shows_only_spellings_ignore_case_matches` and `the_case_note_says_what_the_first_lettered_pattern_matches` — rows changed for `*/Cache/*`, `IMG_*.jpg` and `**/*/Cache`, each with a comment saying why.
- Mutations, each restored byte for byte: a one-component pattern told apart by its start too (two guards red); places past a `*` compared (three red); classes tried on printable characters only (red); no spellings told apart by their end (six red).
- `case_note` (16) and `pattern_files` (8) pass unchanged.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `cannot_share_a_name`, `places`, `class_trials`; `components_apart` removed; guards.
- `CHANGELOG.md`.

## Known gaps
- The rules rest on a measurement of this build's filter on Linux. A change to how the filter anchors or crosses would need the proof changed with it; the guards that compare the line with the filter (`every_spelling_the_line_shows_names_only_what_the_filter_names` and others) would see some of that, not all.
- A trailing separator is taken not to anchor a pattern (rsync's rule); it was not measured here.
- This is the line's sixteenth round. The owner's question — keep the line's design, or state the plain rule and no example — is open.
