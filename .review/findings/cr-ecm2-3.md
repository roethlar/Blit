# cr-ecm2-3: Validating before the question compiled every pattern set again and again

**Severity**: MEDIUM — FAST: a command with many patterns took about three times as long as at the base, in the default mode
**Status**: Verified — `exclude-case-r3` (Codex openreview, 2026-11-20) reviewed `3bc3ee47..467773b4`, which holds this fix, with `guard_confirmed: true`, and required nothing further on this finding (its fix left a filter's compiled sets stale on revalidation — cr-ecm3-4)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ce19f2d6`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-slice1-r2` over `3bc3ee47..a1fd6bae` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-slice1-r2.codex.md`.

## Evidence
The reviewer timed a one-file `copy --dry-run` with no case option, release builds, medians of nine runs: 50 excludes 27.6 ms at the base and 46.4 ms at the head; 250 excludes 66.1 ms and 167.5 ms.

Reproduced here before any change (2026-11-20, Linux host `continuum`, release builds of the base `3bc3ee47` and of the head, scratch on tmpfs, medians of 15 runs, patterns `buildNNN*.tmp` and `CacheNNN/**/*.o` alternating):

| | base | head before the fix | after the fix |
|---|---|---|---|
| `copy --dry-run`, 0 excludes | 3.5 ms | 3.5 ms | 3.5 ms |
| `copy --dry-run`, 50 excludes | 12.4 ms | 29.9 ms | 8.0 ms |
| `copy --dry-run`, 250 excludes | 47.3 ms | 135.4 ms | 24.7 ms |
| `mirror --yes --dry-run`, 250 excludes | 51.4 ms | 139.5 ms | 29.7 ms |
| `copy --dry-run --ignore-case`, 250 excludes | (no such option) | 72.7 ms | 9.3 ms |

Cause: compiling a set of patterns is what a filter costs. cr-ecm1-2 made validation compile (each pattern alone, then the sets), slice 1b added a validation before the question, and the filter built at dispatch validated again and then compiled for use; every filter derived from it (the walk's pruning rules, a mirror's deletion scope) compiled once more, as at the base.

## Predicted observable failure
A command with dozens or hundreds of `--exclude` patterns spends most of a short run compiling the same sets several times over.

## Intake
Admitted. The validation order is right and stays (a filter that cannot be used is refused before anything is asked); its cost was not.

## What
One compiled set per distinct list of patterns and case mode, kept and shared (`compiled`, `CompiledCache`, `crates/blit-core/src/fs_enum.rs`). Validation builds the sets through it and hands them to the filter's own cells; the filter built at dispatch, its clones, the pruning rules and the deletion scope find them already built. A pattern is compiled alone only when a set cannot be built, to name the one at fault — the refusal's words are unchanged. A process keeps the 16 newest sets (a command uses four: includes, excludes as whole paths, as names, as subtrees); a failed build is not kept.

The result is below the base, not level with it: the base compiled each set once per derived filter.

## Guard proof
- `fs_enum::tests::a_pattern_set_is_compiled_once_whoever_reads_it`: with patterns no other test uses, in each case mode — an early validation on a filter that is thrown away, the dispatch filter validated and used, a clone, the pruning rules, and a filter never validated — exactly four real compilations: the includes, the excludes as whole paths, as names, as subtrees. Every real compilation in a test process is recorded where it happens (`CompiledPatterns::build`), so a compile that bypasses the cache is counted too. A test process keeps every set, so tests running beside it cannot evict this one's.
- `fs_enum::tests::the_compiled_cache_shares_keys_and_drops_the_oldest`: a hit is the very set kept; the case mode is part of the key in both directions; two builders of one key share the first set; a full cache drops the oldest.
- The refusal guards from cr-ecm1-2 and slice 1b are unchanged and green: a pattern that cannot be compiled is still refused, by name.
- Mutations, each restored byte for byte: nothing compiled is found again → the counting guard red; the name and subtree rules compiled outside the cache → the counting guard red; the case mode dropped from the key → the cache guard red (and thirteen behaviour guards); a full cache dropping the newest → the cache guard red; a second builder's set replacing the first → the cache guard red.

## Files changed
- `crates/blit-core/src/fs_enum.rs` — `CompiledCache`, `compiled`; `CompiledPatterns::compile` and `ExcludeRules` share sets; `validate_globs` compiles once and keeps; two guards.

## Known gaps
- Measured on Linux only, one machine, one pattern shape.
- The cache is process-wide. A daemon serving more than 16 distinct pattern sets at once compiles the 17th's again when it is next read; nothing is matched wrongly, since a set is found only by its exact patterns and mode.
- The deletion scope's own automata (`DestinationNames`) are built once per scope, as before; they are not part of this.
