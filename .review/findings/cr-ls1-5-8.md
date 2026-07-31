# cr-ls1-5..8 — The parallel destination diff, and why it was reverted rather than repaired

**Severity**: 4 × MEDIUM
**Status**: VERIFIED-CLOSED. Round-4 dispatch over `81c53255..0c708f72`
returned **`verdict: clean`, `guard_confirmed: true`, zero findings** — the
revert plus the round-trip-elimination replacement was independently
re-verified, including the reviewer running the red/green itself. Record:
`.review/results/ls-1-range.codex.r4.json`.
**Source**: `ls-1-range` round-3 codex dispatch over `35948e70..81c53255`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached, disposable
worktree); codex-cli 0.146.0; **`guard_confirmed: false`**.
Record: `.review/results/ls-1-range.codex.r3.json`.

## The findings, as returned

**cr-ls1-5 `phase-overlap` (MEDIUM)** — "one wall-clock Compare span, while
every repair duration is recorded separately and their sum is subtracted.
After the diff parallelises repairs, overlapping durations can exceed wall
time, making `saturating_sub` report Compare as zero and hide real
stat/metadata cost on repair-heavy runs."

**cr-ls1-6 `shared-rayon` (MEDIUM)** — "blocking SMB metadata calls run on
rayon's global CPU pool. The concurrent apply path uses that same pool for
tar-shard writes, and daemon sessions share it too. A slow diff can stall
apply or unrelated transfers; the recorded 24× per-call latency inflation
reinforces this risk."

**cr-ls1-7 `parallel-effects` (MEDIUM)** — "filesystem mutations inside a
fallible parallel map. Rayon does not deterministically return the first of
multiple errors, contrary to the comment, and duplicate manifest paths are
deduplicated only afterward, so two closures can repair the same destination
concurrently."

**cr-ls1-8 `guard-vacuous` (MEDIUM)** — "the added parallel-diff test checks
only the resulting file set. Replacing `into_par_iter` with `into_iter` left
the intended test green; after exact-byte restoration it remained green."

## Adjudication: reverted, not repaired

All four are correct. Three of them (5, 6, 7) are defects *introduced by* the
parallelisation, and the fourth says the guard for it never existed.

The parallelisation is reverted rather than fixed, because the measurement
that motivated it also argues against it. It bought **1.34×** on the owner's
SMB share while producing **26× effective concurrency** — the destination
metadata path saturates, so the binding constraint is the NUMBER of round
trips per file, not the order they are issued in
(`docs/bench/ls1-phase-2026-07-31/compare-subphase-and-parallel.md`).

Weighing a 1.34× win against: blocking I/O monopolising a pool shared with
the apply path and with concurrent daemon sessions; non-deterministic error
selection; a real concurrent-mutation hazard on duplicate manifest paths; and
a broken timing instrument — the win does not justify the exposure, and none
of those costs buy back speed. Repairing all three would mean a bounded
dedicated destination-I/O executor plus pre-diff path deduplication plus a
re-derived compare span, which is a design, not a patch.

**cr-ls1-6 is the one that would have mattered most in production**: nothing
in the 1.34× measurement would have revealed a diff stalling an unrelated
daemon transfer, because the benchmark ran one session at a time.

## What replaced it

Round-trip elimination, which is what the measurement actually points at.
`destination_needs` stats each destination file and then asked
`destination_verdict` to re-read the same file's attributes with
`GetFileAttributesW`. On Windows the stat ALREADY carries the attribute
DWORD, so that second round trip was pure waste — measured at ~1 ms of the
~4.7 ms per-file destination cost, paid 46,041 times on the owner's
converged mirror. `destination_verdict_using` now accepts the mask the
caller already holds.

This is strictly more consistent than the old code, not less: attributes and
size/mtime now come from ONE observation of the file instead of two, which
closes a TOCTOU window rather than opening one.

**Guard**: `a_supplied_attribute_mask_matches_reading_it` pins that supplied
and read produce the same verdict across four quadrants (converged/diverged
× supplied/read).

**Guard proof, and a third vacuous guard caught before landing**: the first
version of that test used a plain temp file whose raw attributes already sat
inside `WINDOWS_PRESERVED_ATTRIBUTE_MASK`, so masking was a no-op and
dropping the mask left it GREEN. The fixture now sets NOT_CONTENT_INDEXED
(0x2000) — outside the preserved mask — and asserts the fixture carries a
non-preserved bit at all. With that, removing the mask on the supplied path
reds the test. Byte-identical SHA-256 restore, green after.

## Recorded honestly

That is three vacuous guards in this slice (cr-ls1-2, cr-ls1-8, and the
above, caught pre-landing). The pattern is the same each time: an assertion
written against a fixture that cannot distinguish the fixed code from the
broken code. Running the revert is the only thing that has caught any of
them — reasoning about the assertion has caught none.
