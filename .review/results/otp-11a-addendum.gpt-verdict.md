# otp-11a addendum — codex verdict adjudication

**Reviewed**: commit `d74c1ac` (+ follow-up records `4148705`,
`d173691`) — the journal-unsoundness verdict, regression pin, and
no-op gate re-baseline. Raw review:
`.review/results/otp-11a-addendum.codex.md` (gpt-5.6-sol, VERDICT:
CHANGES REQUESTED, 4 findings). Codex independently CONFIRMED the
core claims: the data-loss is real, no later validation layer catches
the deep-modification case, Windows's changed-USN mtime fallback is
also unsound, and `deep_modification_after_warm_runs_syncs` would
fail under a hypothetical journal-skip reintroduction.
reviewer: gpt-5.5-class (gpt-5.6-sol)

## A1 (Medium) — 610 ms was a single observation; the sound-vs-sound PASS was uncertified

**Accepted — fixed by measurement.** The old binary was re-run with
its `journal_cache.json` removed before each run (probe → `Unknown` →
its SOUND full no-op pass every time), 5 interleaved runs against the
session binary: old median **507 ms**, session median **226 ms** —
session 2.2× faster, gate PASS on real medians. Table recorded in
`docs/bench/otp11-local-2026-07-11/README.md`; every 610 ms/2.8×
citation replaced with the certified figures (bench README, slice doc
D3, STATE).

## A2 (Medium) — STATE's summary line contradicted its own body

**Accepted — fixed.** Line 5 now says the journal question is
resolved, 11b unblocked, suite 1513.

## A3 (Low) — floor arithmetic still 1510-based

**Accepted — fixed.** Slice doc floor section now runs from 1513:
1513 − 71 = 1442 → **≈ +41** real pins to the ≥1483 floor by otp-13.

## A4 (Low) — Linux mechanism imprecise (no global event counter; root CTIME is the first arm)

**Accepted — fixed.** The README intro, the certified-verdict
paragraph, and the pin's doc comment now state the per-platform
mechanism precisely: macOS's event-id arm always differs across runs
so the root-MTIME fallback decides; Linux's FIRST arm is the root
dir's CTIME, which a deep write equally never touches (it never
reaches the fallback); Windows's strict-USN arm requires a
write-quiet volume and otherwise decays to the same mtime fallback.
The unsoundness conclusion is unchanged (codex: "remains correct").

## Disposition

4/4 accepted and fixed (fix sha appended below). The addendum's core
verdict stands confirmed by the independent reviewer; otp-11b remains
unblocked with the certified 2.2× sound-vs-sound margin.
Fix sha: (appended after the gate)
