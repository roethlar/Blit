# cr-ecm3-4: A filter validated again after its patterns changed kept the first patterns' matchers

**Severity**: MEDIUM — through the public `FileFilter` API a filter matched patterns it no longer held; no command line reaches it (each command builds its filter once)
**Status**: Verified — `exclude-case-r4` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..0669279f`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `0fa8ac4e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r3` over `3bc3ee47..467773b4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r3.codex.md`.

## Evidence
Through the public API the reviewer validated a filter excluding `old`, replaced the pattern with `*.cfg`, validated again, and matched: `a.cfg` passed. A fresh clone of the same filter excluded it.

Cause: cr-ecm2-3 had `FileFilter::validate_globs` put the sets it compiled into the filter's own cells (`crates/blit-core/src/fs_enum.rs`). A cell takes a value once; the second validation compiled the new patterns, found the cells full, and left the old sets in place. Before cr-ecm2-3 a validation touched nothing in the filter.

## Predicted observable failure
An embedder that builds a filter, validates it, edits its pattern fields and validates again matches by the first patterns.

## Intake
Admitted. It is a consequence of the fix for cr-ecm2-3.

## What
Validating a filter changes nothing in it. The sets it compiles are kept for the process (`compiled`), not in the filter; the filter takes them at its first match — a lookup, not a second compilation — so the speed cr-ecm2-3 bought is unchanged (measured below). What a filter matches is what it holds when it first matches, as before this plan; the struct's own comment now says so, and that another set of patterns wants another filter.

Measured after this fix (2026-11-20, Linux, release build, medians of 15, the same patterns as cr-ecm2-3): `copy --dry-run` with 50 excludes 8.0 ms, with 250 excludes 25.0 ms, `mirror --yes --dry-run` with 250 excludes 29.5 ms, `copy --dry-run --ignore-case` with 250 excludes 9.2 ms — against 8.0 / 24.7 / 29.7 / 9.3 ms before it (one run of the measurement each; the differences are within its noise).

## Guard proof
- `fs_enum::tests::a_filter_validated_again_matches_the_patterns_it_now_holds`: the reviewer's sequence over includes and excludes together (the whole-path glob, the name rule, the subtree rule and the includes each asked), and again for a change of case mode alone.
- `fs_enum::tests::a_pattern_set_is_compiled_once_whoever_reads_it` (cr-ecm2-3) is unchanged and green: still four compilations for every reader.
- Mutations, each restored byte for byte: validation keeping the include set in the filter → red; keeping both exclude sets → red; keeping all three, as it did → red. (Keeping ONE of the two exclude sets alone is not red: the whole-path glob and the name and subtree rules read the same patterns, and the other, current one still names the entry.)

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `validate_globs` no longer fills the cells; the comment on them; the guard.

## Known gaps
- A filter whose pattern fields are edited AFTER it has matched still matches by the earlier patterns. That is how the filter has always worked (its fields are public and its compiled sets are taken once); checking the patterns at every match would cost a comparison of every pattern per file. `clone_without_cache` is the way to a filter with other patterns.
- Run on Linux only.
