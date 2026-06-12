# w2-1-delete-warmup-machinery — honest static tuning table

**Branch**: `master` (owner-authorized branchless session 2026-06-11)
**Commit**: `2a8a490`
**Source findings**: constants-dead-warmup-adaptive-path (reviewer: high),
deadcode-core-warmup-machinery (reviewer: medium) — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

The advertised adaptive tuning was dead code: `analyze_warmup_result` had
zero callers; `determine_tuning`'s only production caller hardwired
`warmup_result = None` (every bandwidth branch unreachable) and then
overwrote the None branch's outputs with its own byte ladder. Deleted all
of it so the tuning table is honestly static.

## Approach

- `auto_tune/mod.rs`: deleted `analyze_warmup_result`, `determine_tuning`,
  their 2 unit tests, and the never-read `TuningParams.warmup_gbps` field
  (no `.warmup_gbps` read exists in the workspace; `TuningParams` was only
  constructed inside the deleted function). Module doc rewritten — it no
  longer claims "warmup probes"; `LocalPlanTuning` / history-derived
  planning is untouched.
- `remote/tuning.rs`: `determine_remote_tuning` now constructs
  `TuningParams` directly. **Production values are bit-identical for every
  tier** (chunk 16/32/64 MiB; streams (4,8)…(24,32); buffers None /
  4 MiB+16 / 8 MiB+32) — this is a pure dead-code deletion, not a tuning
  change. Doc note added that the daemon's push negotiation still runs its
  own winning ladder (single-owner consolidation = w2-2).
- A real warmup probe remains H10b-class future work behind its own plan
  doc (per the slice spec and STATE.md queue item 5).

## Files changed

- `crates/blit-core/src/auto_tune/mod.rs`
- `crates/blit-core/src/remote/tuning.rs`

(No caller changes needed: `determine_remote_tuning`'s signature is
unchanged; the deleted field had no readers.)

## Tests added

4 tier-pinning tests for `determine_remote_tuning` (it previously had
none — partially covering w9-6's "tuning-tier unit tests" ask): floor
tier, 1 GiB mid tier, 10 GiB tier, and the 32 GiB boundary. 2 tests
deleted with `determine_tuning` (covered the deleted branches only —
called out per AGENTS.md §5). Suite 1339 → 1341.

## Known gaps

- The three disagreeing stream ladders (client vs daemon push vs legacy
  pull) remain — that is w2-2 by design.
- w9-6 may still want boundary tests for the chunk/buffer tier edges
  (512 MiB / 8 GiB); only the 32 GiB stream boundary is pinned here.
