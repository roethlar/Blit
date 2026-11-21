# cr-ecm6-2: The line's example dropped a trailing `/**`

**Severity**: HIGH — the line said `--ignore-case` would also match a name it does not; a mirror run on that reading deleted it
**Status**: Verified — `exclude-case-r7` (Codex openreview, 2026-11-21) reviewed `3bc3ee47..cbab0c39`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (it found a doubled separator after `**` trimmed the same way: `**//Cache` shown as `Cache` — cr-ecm7-2; and two faults in which pattern the line shows — cr-ecm7-3, cr-ecm7-4)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a7e32b60`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r6` over `3bc3ee47..a13238b6` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r6.codex.md`.

## Evidence
For `--exclude 'Cache/**'`, the prompt said `--ignore-case` would also match `cache`. The reviewer ran that mirror with `--ignore-case` and it deleted the destination-only file `cache`, exiting successfully. It asked that `/**` be kept in the example and the suggested spellings verified against the matcher.

Cause: cr-ecm5-4 kept a single star that is a whole directory level (`*/Cache`, `Cache/*`) and went on trimming `**/` at the start and `/**` at the end, reasoning that both mean "at any depth". They do not mean the same thing: `**/Cache` names an entry called `Cache` at any depth, the top included, so `Cache` shows it; `Cache/**` names what is UNDER `Cache` — it requires `Cache` to be a folder and does not name a file called `Cache`.

The guards were the deeper miss. The check that a spelling shown "is one the option matches" built its names by rule from the spelling itself and asked a bare glob matcher, not the filter; it never put a file named like the folder in front of `Cache/**`.

## Predicted observable failure
`mirror --exclude 'NAME/**'` (or `**/NAME/**`): the line shows `NAME` and reads as protecting a FILE of that name, in either mode; the pattern never names it.

## Intake
Admitted, with the guard the reviewer asked for.

## What
- A trailing `/**` stays in what is shown: `"Cache/**" matches Cache/** only; capital letters matter (--ignore-case would also match cache/**, CACHE/**).` Still trimmed: a star run touching text at either end, and `**/` at the start.
- The rule for trimming is now stated where it is applied: only what leaves the shown text naming no more than the pattern does.

## Guard proof
- `case_fold::tests::every_spelling_the_line_shows_names_only_what_the_filter_names`: for every pattern made of seven beginnings, four middles and seven endings (`*`, `**/`, `*/`, `**/*/`, `sub/`, `*/sub/` × `Cache`, `Ca?he`, `C[a]che`, `Ca*he` × `*`, `/*`, `/**`, `/**/*`, `/x`, `*/**`), against the real `FileFilter` as an exclude, over twenty-six names each taken as a file and as a folder: the text the line shows names nothing the pattern as typed does not, and every other spelling names nothing the pattern does not name with `--ignore-case`.
- `case_note::the_line_keeps_what_is_under_a_folder_apart_from_the_folder`, through the binary: the line for `**/Cache/**` to the letter; with `--ignore-case`, what is under `sub/CACHE/` is kept and the file `cache` deleted — both as the line now shows.
- The two table guards (`…says_what_the_first_lettered_pattern_matches`, `…shows_only_spellings_ignore_case_matches`) pin `Cache/**`, `**/Cache/**`, `**/*Cache*/**` and `Cache\**` under the Windows rule.
- Mutations, each restored byte for byte: a trailing `/**` trimmed again → four guards red; any trailing level trimmed → four red; a leading required level trimmed (cr-ecm5-4's) → three red, the new rule among them.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `without_end_wildcards`; the guards.
- `crates/blit-cli/tests/case_note.rs` — the end-to-end guard.

## Known gaps
- The rule guard covers excludes; an include's spellings are built the same way and are not run through a filter as includes.
- The names tried are twenty-six, on Linux; nothing with letters beyond ASCII goes through that guard (the pairing of such letters has its own).
- Run on Linux only.
