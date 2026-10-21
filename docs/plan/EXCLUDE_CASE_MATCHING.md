# Exclude case matching: `--match-case` default, `--ignore-case`, and the one-line prompt

**Status**: Draft (r1, 2026-10-21 — owner requirements stated in chat on 2026-10-21 land here; owner interview continues)
**Created**: 2026-10-21
**Supersedes**: `docs/plan/MIRROR_GLOB_DEST_ONLY.md` (Draft r3, same day — its mechanism options are all withdrawn; its prior-art measurements of robocopy and rsync stand and are referenced below); the first bullet of `docs/RELEASE_NOTES_1_0.md` "Known limitations shipped open", once this ships
**Decision ref**: pending (activation needs a `docs/DECISIONS.md` entry)

## Goal

A user who gives a pattern to a command that can delete is told, in one line and in their own words, how that pattern will match on this run, and chooses. Patterns match exactly as typed by default (`--match-case`, the rsync rule, the same on every machine). `--ignore-case` makes every pattern in the command ignore capital letters, on the source walk and in the destination's deletion scope alike, on every machine. When a delete option is in effect and `-y` is absent, the existing confirmation prompt gains one line that echoes the user's actual pattern and the spellings it will and will not match under the mode in force; `-y` bypasses it as it bypasses the rest of the prompt. Nothing depends on which filesystem the destination has.

## Owner requirements (2026-10-21, verbatim where quoted)

- "FAST, SIMPLE, RELIABLE" — the earlier proposals (disk-detected case mode; regex-style `(?i)` prefix; Unicode-table pairing; filesystem-lookup pairing) were rejected as violating two of the three.
- "it cannot be internally inconsistent" — one reading of a pattern per run, the same for what is copied and what is deleted.
- "I would expect ß to mean that exact character, not transliterated to ss" — no fold or transliteration changes which letters a pattern names. Ignoring case maps one letter to the same letter in another case, never to a different letter or a different number of letters.
- "same as the decades old established tools, unless you have a compelling, real-world reason to subvert people's expectations" — rsync matches exactly; robocopy matches with Windows' own case-insensitive rule; both protect excluded destination-only entries from deletion and both delete a destination-only entry no exclude names (measured and read from source in `MIRROR_GLOB_DEST_ONLY.md` §Prior art).
- "it's a flag users will only look for after they've lost data" — hence the prompt line: the choice is presented at runtime, before the delete, not discovered afterwards.
- "when data loss is possible, this cannot be ambiguous even to novices" — the prompt line says what blit will do, in the user's own pattern, not what a filesystem might do.
- "we have a prompt whenever a delete option is specified without -y, so if -y is not included and a CLI option with a pattern is included, there should be a short, 1 line blurb about the matching issue … with -y bypassing it. --match-case is the default, and --ignore-case does extra work on case sensitive filesystems."
- "that requires us to know what the remote target's filesystem is, so that's a daemon change" — the line therefore states blit's own rule, which is identical everywhere, and needs no probe of the destination's disk.
- "the example needs to be the actual string the user entered."

## Non-goals

- Detecting or depending on the destination's (or source's) case sensitivity. Blit's matching is the same on every machine; the user chooses the mode.
- Transliteration or Unicode normalization (`ß`/`SS`, NFC/NFD `é`): a letter is that letter. The 2026-10-19 APFS reproduction (`*ß` vs `bSS`, `??` vs `s`, `???` vs NFC `É`, each deleted) is EXPECTED behavior under this plan — the exclude did not name those entries — and is re-documented as such, not as a limitation.
- Changing what `?` means (one byte, as today) or any other glob syntax.
- Any per-pattern syntax (regex-style `(?i)`): rejected as requiring knowledge most users do not have.
- Prompting when nothing can be deleted (`copy` without a delete option) or when `-y` is given.

## Constraints

- **SIMPLE**: two visible options, `--match-case` (default; present so a user can state it and so a saved job records it) and `--ignore-case`; one added prompt line; one sentence of docs each. Owner asked for these (ruling 2026-08-01 permits a visible option when the owner asks).
- **FAST**: `--match-case` changes nothing. `--ignore-case` costs a case-insensitive glob compile (the glob library supports it natively) and a case-insensitive match per entry; no table, no probe, no extra I/O. No cost at all when the option is absent.
- **RELIABLE / consistent**: one mode per run, carried to the peer at session open so the source walk and the destination's deletion scope read every pattern identically; `--ignore-case` applies to `--exclude` and `--include` patterns (Q2 below asks whether it also applies to the literal lists).
- **Same build only**: the mode is a new wire field; `CONTRACT_VERSION` bumps (8 → 9) and the strict same-build refusal (D-2026-07-05-2) keeps an older peer from reading the field as absent.
- **Letters**: "ignore case" means Unicode simple case mapping — one code point to its other-case code point (`a`/`A`, `ü`/`Ü`, `σ`/`Σ`). `ß` has no single-letter uppercase and maps to itself; `ς` and `σ` are distinct letters and stay distinct. (Q1 asks the owner to confirm this reading of "letter" versus ASCII-only.)
- **Platform truth**: guards for matching run on every OS (the mode is independent of the disk); the end-to-end mirror guards of `exclude_contract.rs` gain the `--ignore-case` destination-only case and run everywhere, no skips.

## Acceptance criteria

- [ ] **AC-1 (default)**: with no option, every pattern matches exactly as typed on every machine, exactly as today (no behavior change; existing guards stay green).
- [ ] **AC-2 (`--ignore-case`)**: `blit mirror --ignore-case --exclude '*cache' src/ dst/` skips `src/aCache/` on the source walk and keeps a destination-only `dst/aCache/` with its contents, on Linux, Windows and macOS alike; an unrelated extraneous entry is still deleted; `--exclude 'Übung'` keeps `übung`. `ß` does not match `SS`; `ς` does not match `σ`.
- [ ] **AC-3 (prompt line, default mode)**: `blit mirror --exclude '*Cache' src/ dst/` without `-y` prints, as part of the existing confirmation, exactly one line of the form `"*Cache" matches Cache only; capital letters matter (--ignore-case would also match cache, CACHE).` echoing the user's pattern as typed, before any delete; answering no deletes nothing.
- [ ] **AC-4 (prompt line, `--ignore-case`)**: the same command with `--ignore-case` prints `"*Cache" matches Cache, cache, CACHE; capital letters are ignored.`
- [ ] **AC-5 (friction)**: several patterns → one line (the first pattern containing a letter, then `and N more patterns`); a pattern with no letters (`??`, `*.1`) → no line; `-y` → no prompt and no line; `copy` with patterns and no delete option → no line.
- [ ] **AC-6 (peer)**: the mode reaches the source daemon and the destination daemon at session open; a `mirror` to a daemon with `--ignore-case` keeps the destination-only `aCache` there too; contract 9 is refused by a contract-8 peer with the existing refusal.
- [ ] **AC-7 (records)**: `--ignore-case` is recorded in the saved job / run record and replayed by `blit jobs run` and `jobs retry`; `--help` documents both options in one sentence each; `docs/RELEASE_NOTES_1_0.md` replaces the pure-glob limitation with the rule and the prompt line; `README.md` and `docs/TRANSFER_SESSION.md` state the rule.

## Design

### Matching

- `crates/blit-core/src/fs_enum.rs`: `FileFilter` gains `ignore_case: bool`. `CompiledPatterns` and the audit-19 `ExcludeRules` compile each glob with `globset::GlobBuilder::case_insensitive(ignore_case)`; literal-name and subtree rules compare under the same mode (simple case fold of both sides when set). `--include` patterns follow the same bit.
- `crates/blit-core/src/transfers/filter.rs`: `DestinationNames` (the deletion scope) reads pure-glob rules through the same compiled set, so with `--ignore-case` a destination-only `aCache` is in the scope of `*cache`; literal-name lookups through `stored_name` stay as they are (they already use the destination's own pairing for literal names — unchanged by this plan).
- Simple case mapping only: globset's case-insensitive mode uses Unicode simple case folding on the regex side; a guard pins `ß`/`SS` and `ς`/`σ` as NON-matches.

### CLI and prompt

- `crates/blit-cli/src/cli.rs`: `--match-case` (default, `conflicts_with = "ignore_case"`) and `--ignore-case` on `copy`, `mirror`, `move` (the pattern-bearing transfer verbs), one sentence of help each.
- `crates/blit-cli/src/transfers/mod.rs` `confirm_destructive_operation` (the existing `mirror`/`move` prompt, `:347`, `:802`): when any `--exclude`/`--include` pattern is present, add one line built from the first pattern containing a letter: the pattern as typed in quotes, then the spellings — under `--match-case`: `matches <as typed> only; capital letters matter (--ignore-case would also match <lower>, <UPPER>)`; under `--ignore-case`: `matches <as typed>, <lower>, <UPPER>; capital letters are ignored`. `<lower>`/`<UPPER>` are the pattern's literal text with its letters lowered/uppered (wildcards unchanged); whichever equals the typed form is omitted. Further patterns: `and N more patterns`. No line when no pattern holds a letter. `-y` skips the whole prompt as today.
- No probe of any disk anywhere in this path.

### Wire

- `crates/blit-core/proto/blit.proto` `FilterSpec`: `bool ignore_case` (new field number). `CONTRACT_VERSION` 8 → 9 (`docs/TRANSFER_SESSION.md` frame/contract table, `CHANGELOG.md`). `operation_spec.rs` normalizes it; `NormalizedTransferOperation::from_spec` and the daemon's `validate_spec` carry it into `FileFilter`.

### Records

- `blit_core::job_record` / `JobSpec`: `ignore_case` field, serialized, replayed by `jobs run` and `jobs retry`.

### Docs

- Release notes: remove the pure-glob limitation bullet; add the rule ("Excludes match exactly as typed. `--ignore-case` ignores capital letters. Before a mirror or move with patterns, blit shows how your pattern will match and asks.") and the two example lines. README and TRANSFER_SESSION likewise. Re-state the 2026-10-19 cases as expected behavior.

### Risks

- A contract bump forces a same-build upgrade of every daemon (already the standing rule, D-2026-07-05-2).
- globset's case-insensitive mode folds via the regex engine's Unicode simple folding; the guard in AC-2 pins the exact extent so a library change would be caught.
- The prompt line adds one line to a prompt users already see; AC-5 bounds it to one line in every case.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop.

1. **Matching + option, local**: `FileFilter.ignore_case`, compiled patterns, deletion scope, `--match-case`/`--ignore-case` on the CLI (local transfers only), unit guards for AC-1/AC-2 on every OS plus the `ß`/`ς` non-match guard, `exclude_contract.rs` end-to-end guard for the destination-only `aCache` kept under `--ignore-case` on every OS.
2. **Prompt line**: `confirm_destructive_operation` gains the line; guards for AC-3/AC-4/AC-5 through the CLI's dispatch tests (prompt text captured; no-letter and multi-pattern cases; `-y` and `copy` suppress).
3. **Wire**: `FilterSpec.ignore_case`, contract 9, daemon source walk and destination scope honor it; daemon end-to-end guard (AC-6).
4. **Records + docs**: job spec field and replay (AC-7), `--help`, release notes, README, TRANSFER_SESSION, CHANGELOG; `MIRROR_GLOB_DEST_ONLY.md` marked Superseded; STATE/RELEASE_1_0 G3b updated: the pure-glob item is closed by this rule, not by pairing.

## Open questions

- **Q1 — what is a letter**: Unicode simple case mapping (`ü`/`Ü`, `σ`/`Σ` fold; `ß`, `ς`/`σ` do not) [recommended], or ASCII `a-z`/`A-Z` only. — owner
- **Q2 — scope of `--ignore-case`**: `--exclude` and `--include` patterns only [recommended], or also the literal lists (`--files-from`). — owner
- **Q3 — `copy`**: `copy` never deletes at the destination, so no prompt line there [recommended]; or show it anyway when patterns are present. — owner
- **Q4 — the 2026-10-19 cases**: confirm they are closed as expected behavior under `--match-case` (documented, not fixed), so the G3b blocker entry is retired on activation. — owner
