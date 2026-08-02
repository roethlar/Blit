# cr-ls5-2 — session-stale absent snapshots re-open the alias overwrite

**Severity**: HIGH
**Status**: FIXED, awaiting re-review. Fixed by an opus subagent (owner
instruction: coding via opus/sonnet subagents).
**Source**: `ls6-range` codex dispatch over `7fa18f5b..ba3fc73a`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached,
disposable worktree). **`guard_confirmed: true`** — this round also
VERIFIED-CLOSED cr-ls5-1.
Record: `.review/results/ls6-range.codex.json`.

## The finding, as returned

> The 8.3 fix still trusts session-lifetime AbsentDir and empty-directory
> snapshots. Because local apply runs concurrently while later 128-entry
> chunks are still being diffed, an earlier chunk can create a long-name
> file and its 8.3 alias after the snapshot; a later source entry using
> that alias is still reported Absent rather than sent to authoritative
> stat, so it is planned and can overwrite the earlier file, including
> under --ignore-existing. A Windows probe using a real custom short name
> reproduced the stale Absent verdict.

## Why it is admitted

Correct. cr-ls5-1's fix reasoned about aliases of LISTED entries and kept
the trusted absent where "nothing can be aliased" — absent and empty
directories — but that judgment was made against the sweep-time state,
and the session's own apply invalidates it: verdicts for chunk N are
complete before its payloads write, while chunks N+1… diff concurrently
with those writes. A directory that was absent or empty at sweep time can
hold freshly written files (and their 8.3 aliases, and their case
variants — the folded-name set has the same staleness) by the time a
later chunk's entry is judged. The same-name case is safe on every
platform (each name is judged exactly once, before it is ever written);
the ALIAS and CASE-FOLD channels are not, because a write to one name
creates a different resolvable name.

## Fix (prescribed)

Taint-on-verdict: the moment the diff produces a Transfer verdict for a
path, taint every ancestor directory of that path in the
`DirStatCache`. In a tainted directory, ABSENCE judgments (trusted
absent from a swept listing, `AbsentDir`, or the empty-directory
carve-out) degrade to `Fallback` — the authoritative stat resolves
aliases and case the way the filesystem does. Listed-entry HITS remain
valid: a hit describes the file's own pre-write state, which is the
correct comparison state because every file is judged before it is
written. Verdicts strictly precede writes (chunk N's needs are batched
after its diff completes), so taint-at-verdict is conservative by
construction. All platforms taint — the case-fold channel exists on any
case-insensitive destination, not just Windows.

Perf shape: a converged mirror issues zero Transfer verdicts, so nothing
taints and the 55.57 s path is untouched. A fresh copy pays one stat per
new file in already-written-to directories — bounded by the write count
and dwarfed by the writes themselves.

## Fix (as built)

Taint-on-verdict, as prescribed, with the ordering argument checked
against the code rather than assumed.

- `DirStatCache` gained `tainted: HashSet<PathBuf>` in `CacheInner` (the
  existing `Mutex`) and `pub fn taint(&self, path: &Path)`, which inserts
  every ancestor of `path`. Ancestors are only ever inserted as a whole
  chain, so the walk stops at the first already-present entry. The set is
  deliberately NOT evicted alongside `dirs`: a re-swept snapshot is newer
  but still a snapshot, so the degradation must outlive the listing it
  degrades.
- `lookup` reads the taint AFTER the snapshot, never before — a taint
  recorded while that very sweep was running still precedes the write it
  announces, so a pre-read would miss exactly the write about to go
  stale. In a tainted directory the swept-miss arm, the `AbsentDir` arm
  and the empty-directory carve-out all return `Fallback`; listed-entry
  hits are untouched.
- `destination_needs` (`transfer_session/mod.rs`) was restructured from
  four `Ok(...)` returns into one settled `verdict`, and taints `dst`
  when it is `NeedVerdict::Transfer`. That is the single point where both
  carriers settle a need, so the shared `diff_chunk_verdicts` covers the
  local and wire routes alike. Unconditional, including when
  `repair.enabled` is false: a write-nothing destination (`--dry-run`)
  then pays fallbacks it does not need, which is exactly the round trips
  it made before ls-5 — cheaper than a second condition on the verdict.

**Ordering verified, not assumed.** The manifest loop awaits
`diff_chunk_and_apply_local` per chunk, and that function awaits all of
the chunk's verdicts before `plan_chunk` and `queue`. So every taint from
chunk N is recorded before ANY of chunk N's payloads is queued, and chunk
N+1's diff begins only after chunk N returned. The remaining window is
intra-chunk — a sibling's absence answer may be computed before a peer's
taint lands — and it is harmless by the same ordering: no write occurs
mid-chunk, so those answers describe true pre-write state, which is the
per-file stat's own baseline. Recorded residual: a stale `NonFile` hit
stays trusted per the prescription; it is exact-name-only and a repeat of
an exact path is dropped by the `granted` dedup.

**Guard proof** (byte-identical SHA-256 restores: `dir_stat.rs`
2C4DD995…D7AB, `mod.rs` F34EA160…A5F5):

- Seam: `a_later_chunk_stops_trusting_absence_once_the_session_has_written`
  (`transfer_session/local.rs`) drives `run_local_session` over 130 files
  in one source directory — `DEST_DIFF_CHUNK` is 128, so chunk 2's two
  entries are judged after 128 files landed — asserting
  `copied_files == 130` and `fallbacks() >= 2`. Disabling the `taint`
  call reds it with **"got 0 fallbacks"**: deterministic in both
  directions, since a fresh absent-directory copy otherwise takes zero.
- Cache: `a_tainted_empty_directory_loses_the_carve_out` and
  `a_tainted_absent_directory_falls_back_for_every_child_and_ancestor`
  both red `left: Absent / right: Fallback` when `lookup` ignores the
  taint. `a_tainted_directory_stops_trusting_every_absence_but_keeps_its_hits`
  pins hit-survival (platform-neutral); its miss assertion is
  independently satisfied on Windows by cr-ls5-1's rule, so its
  taint-specific red is a non-Windows one — stated in the test.
- `a_fresh_copy_trusts_the_sweeps_absent_answer` updated, not deleted:
  `fallbacks() == 0` is no longer the right contract for a directory the
  session writes into more than once, so it asserts `copied_files == 4`
  and `hits() >= 1`. Deterministic because a verdict requires a lookup,
  so the session's FIRST lookup cannot be preceded by any taint, and the
  absent directory's sweep answers it.

Verified: `cargo fmt --all -- --check`; clippy `-D warnings`
`--workspace --all-targets` on x86_64-pc-windows-msvc AND cross-target
x86_64-unknown-linux-gnu; workspace **1771 passed / 0 failed** (from
1767). Windows host only — the platform-neutral guards are `cfg`-ungated
and will red on Linux too, but that is code reading, not a run.
