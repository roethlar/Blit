# SMALL_FILE_CEILING plan draft — codex adjudication

**Plan commit**: `78eabfd` (Draft; plan-doc + STATE queue entry)
**Raw review**: `.review/results/small-file-ceiling-plan.codex.md`
(codex verdict: **NEEDS FIXES**, 1 High / 3 Medium / 1 Low)
**reviewer: gpt-5.5**

All five findings **ACCEPTED**; fixed in `811a3f2`.

1. **Medium — evidence not reviewable from repo records** (plan cited
   gitignored `logs/` + daemon stderr for the ceiling arithmetic).
   Fixed: `docs/bench/10gbe-2026-07-05/` committed — DIAGNOSIS.md
   (daemon-log extracts, per-file arithmetic, CPU measurements,
   methodology) + all session CSVs; plan cites the tracked paths.
2. **Medium — acceptance tripwire set broader than the sf-1 harness**
   (rsync-ssh / rclone-best / `cp -a` in acceptance, only rsyncd in
   the harness). Fixed: sf-1 commits the full-matrix
   `bench_tripwires.sh`; acceptance now states harness and tripwire
   list are the same set by construction.
3. **Medium — sf-3 oversized and vague** (profile + unspecified cuts
   across two paths in one slice). Fixed: split into sf-3a
   (analysis-only limiter profile naming each cut + expected saving;
   w8-1b precedent) and sf-3b… (one cut per slice, each with its own
   proxy pin).
4. **High — sf-6 wire gate lacked REV4 wire-compat deliverables.**
   Fixed: the gate now names the deliverable set — proto
   fields/numbers, capability negotiation, old/new peer behavior
   both directions, and mixed-version tests landing before any
   behavior depends on the shard lane.
5. **Low — residual competitor-relative wording** ("currently-winning
   cells"). Fixed: regression criterion now speaks of baseline
   matrix cells vs the committed baseline.

Docs gate (`check-docs.sh`) green on both commits. Plan remains
**Status: Draft** — no code until the owner flips Active
(PROTOCOL.md plan step 6).
