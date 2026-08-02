# cr-ls5-3 — taint matched directory SPELLINGS, not directories

**Severity**: HIGH
**Status**: FIXED, awaiting re-review. Fixed by an opus subagent (owner
instruction: coding via opus/sonnet subagents).
**Source**: `ls6-range` r2 codex dispatch over `ba3fc73a..6c7f67f0`
(reviewer id `parent-alias-taint`). **`guard_confirmed: true`** for
cr-ls5-2's own guards — this is the next hole out, not a vacuous guard.
Record: `.review/results/ls6-range.codex.r2.json`.

## The finding, as returned

> DirStatCache::taint records lexical PathBuf ancestors, while lookup
> checks tainted.contains(dir) using the exact spelling. On Windows, case
> variants and 8.3 directory aliases resolve to the same directory but
> remain different cache keys. An independent probe cached AbsentDir
> through `...\newdirectory`, tainted and created
> `...\NewDirectory\victim.txt`, then still received Absent rather than
> Fallback through the equivalent spelling. A later chunk using an
> alternate parent spelling can therefore trust a stale absence while the
> earlier chunk writes, reopening the overwrite under --ignore-existing
> that cr-ls5-2 was intended to close.

## Why it is admitted

Correct, and it is the third instance of one root error: reasoning about
destination identity through NAMES. cr-ls5-1 was leaf-name aliases,
cr-ls5-2 was snapshot staleness, this is parent-path aliases — every
per-spelling data structure on a case-insensitive, alias-bearing
filesystem has this hole, and canonicalizing is not an answer at verdict
time (the directory may not exist yet, and it costs the round trips the
cache exists to avoid).

## Fix (prescribed)

Stop matching spellings: ONE session-global write taint. The first
transfer verdict of the session sets a flag on `DirStatCache`; from then
on EVERY absence-class answer (swept miss, `AbsentDir`, empty-directory
carve-out) degrades to `Fallback`, in every directory, on every
platform. Listed-entry hits stay valid (pre-write state, judged-once).

Why global is not a perf regression: a CONVERGED mirror issues zero
transfer verdicts — the flag never sets and the 55.57 s path is
byte-for-byte untouched. After the first verdict, absence answers pay
one authoritative stat each — and absence answers belong almost
exclusively to genuinely-new files, whose own copies dwarf one stat.
Per-directory precision bought nothing measurable and carried the
spelling hole; delete it rather than patch it.

## Fix (as built)

Exactly as prescribed — the per-directory precision is deleted, not
patched.

- `CacheInner.tainted: HashSet<PathBuf>` and `DirStatCache::taint(&Path)`
  are GONE. `DirStatCache` carries one session-global
  `write_pending: AtomicBool`, set by `mark_write_pending()`, which takes
  no path because there is no name left to get wrong. Never cleared: the
  degradation has to outlive any listing it degrades, including a
  re-swept one.
- `Release`/`Acquire` rather than `Relaxed`, so the flag stands on its own
  rather than on the surrounding chunk join being the only edge. One
  flag, set once per session.
- `lookup` reads the flag AFTER the snapshot, exactly as cr-ls5-2's taint
  read did, and for the same reason: a mark recorded while this very
  sweep runs still precedes the write it announces. The three degraded
  arms are unchanged in shape — swept miss, `AbsentDir`,
  empty-directory carve-out — and listed-entry HITS still answer from the
  sweep. Side effect: the fast path now takes the cache mutex ONCE per
  lookup instead of twice, since the taint read was a second lock.
- `destination_needs` (`transfer_session/mod.rs`) keeps its single
  `NeedVerdict::Transfer` call site, now `dir_stats.mark_write_pending()`
  with no argument, so both carriers and both routes are still covered by
  one call and the `--dry-run` posture is unchanged.

**Guard proof** (one no-op cycle; byte-identical SHA-256 restores:
`dir_stat.rs` 8E02DB2A…0AD0, `mod.rs` 0CC8EEA1…CAB5):

- New cr-ls5-3 seam: `a_pending_write_degrades_a_directory_the_session_never_named`
  (`transfer_session/dir_stat.rs`) marks a write and then looks up in a
  directory OUTSIDE any written ancestor chain. The old ancestor-keyed
  taint cannot turn that green at all — which is the finding. Both
  directories are EMPTY, so the pre-write answer is the platform-neutral
  empty-directory carve-out rather than cr-ls5-1's Windows-only
  fallback, and the flip is the flag's doing on every platform.
- Emptying `mark_write_pending`'s body reds FOUR guards: the new seam
  (`left: Absent / right: Fallback`), the two renamed cache-level guards
  `a_pending_write_costs_an_empty_directory_its_carve_out` and
  `a_pending_write_falls_back_for_absent_directories_and_their_parents`,
  and the session-level
  `a_later_chunk_stops_trusting_absence_once_the_session_has_written`
  (`transfer_session/local.rs`) at **"got 0 fallbacks"**.
- Both no-write guards stayed GREEN through the same mutation —
  `a_converged_mirror_answers_from_directory_sweeps` and
  `a_fresh_copy_trusts_the_sweeps_absent_answer` — which is the direct
  evidence that the flag-set is a no-op for a session that transfers
  nothing, i.e. the converged fast path is behaviourally byte-identical.
- Recorded platform caveat, unchanged from cr-ls5-2:
  `a_pending_write_stops_trusting_every_absence_but_keeps_its_hits`
  asserts a miss beside LISTED entries, which cr-ls5-1's rule already
  satisfies on Windows, so its flag-specific red is a non-Windows one.

Verified: `cargo fmt --all -- --check`; clippy `-D warnings`
`--workspace --all-targets` on x86_64-pc-windows-msvc AND cross-target
x86_64-unknown-linux-gnu; workspace **1772 passed / 0 failed** (from
1771). Windows host only — the guards are `cfg`-ungated and will red on
Linux too, but that is code reading, not a run.
