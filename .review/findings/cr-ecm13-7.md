# cr-ecm13-7: The line printed a whole alternative of which part changes

**Severity**: MEDIUM — the question names `{A,B}` as gained where only `B` is treated otherwise
**Status**: Reviewed, found incomplete — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`. A spelling is still printed where the claim rests on samples: "With `--include 'a?b' --exclude '?1?'`, the note offered `A?B`. Enabling `--ignore-case` added `AxB`, while `A1B` stayed excluded." The remaining work is cr-ecm14-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b7139bd9`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. It completes cr-ecm12-4.

## Evidence
"With `--exclude '[Aa]' --include '{a,b}'`, the prompt said `--ignore-case would also match {A,B}`. Both mirrors kept destination-only `A`. Only the folded mirror deleted `B`. The supported example is `B`."

Cause. cr-ecm12-4 prints the spelling when the name that showed the change is one of the spelling's OWN names. `{A,B}` has two, and the claim it makes is about both.

## Intake
Admitted as the reviewer put it.

## What
The spelling is printed only when EVERY name it reads as is treated otherwise with the option (`Asked::would_also_match`). Where only some are, the name that was checked is printed, and the line does not say what the pattern matches "only".

One line of an existing guard moves with it, and for the same reason: `{1,Cache}` was said to gain `{1,cache}, {1,CACHE}`, which read as `1` too — matched either way. It is now `"{1,Cache}" matches exactly as typed; capital letters matter (--ignore-case would also match cache, CACHE).`

A first attempt — print the spelling only when it IS the name checked — moved seven guards, `Cache/**` among them (`cache/x` for `cache/**`); it was not kept.

Speed (release, medians of 7): unchanged — 64 includes 4.6 ms with the line; 1,000 excludes 8.3 ms.

## Guard proof
- `case_fold::tests::the_case_note_claims_only_what_the_filter_bears_out`: the reviewer's command — `"{a,b}" matches exactly as typed; … (--ignore-case would also match B).`; `{a,b}` alone, every name of it changing, keeps `{A,B}`.
- `case_note.rs::the_line_claims_only_what_the_mirror_then_does`, through the binary: the line, then both mirrors — `B` kept as typed and deleted with the option, `A` kept by both.
- Mutation, restored byte for byte: a text printed when one of its names changes, as before → both red.

## Files changed
- `crates/blit-core/src/case_fold.rs` — `would_also_match`; guards.
- `crates/blit-cli/tests/case_note.rs` — the case.

## Known gaps
- "Every name it reads as" is every name the line's own sampling of the text gives (a `*` as one text, a class by its members up to the sampler's bound) — not every name there is.
- Run on Linux only.
