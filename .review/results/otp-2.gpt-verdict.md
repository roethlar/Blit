# otp-2 — codex review adjudication

reviewer: gpt-5.6-sol (codex exec, read-only)
reviewed commit: `e757dcc`
raw output: `.review/results/otp-2.codex.md`
verdict line: NEEDS FIXES (8 findings)
fix commit: (appended after landing)

## F1 (High) — "symmetric baseline" mislabeled; per-direction observations only

**Adjudication: ACCEPTED (the sharpest catch).** D-2026-07-05-1's own
text — cross-direction comparisons valid only on symmetric endpoints —
already governs, and Mac↔zoey (SSD vs pool) is exactly the excluded
shape. The dataset is re-framed everywhere (README title + load-bearing
scope caveat, STATE) as the PER-DIRECTION converge-up reference; the
cross-direction half of the otp-12 bar is an owner question (symmetric
pair vs per-direction-suffices), NOT satisfied and NOT waived here.

## F2 (High) — macOS `sync` does not guarantee durable pull windows

**Adjudication: ACCEPTED.** macOS sync(2) schedules; Linux sync waits —
a real directional bias in the harness. Fixed: pull windows now fsync
every landed file (`fsync_tree`; F_FULLFSYNC deliberately not used —
the Linux side does not pay media flush either, so drive-level flush is
the equivalent depth). The cost is visible and honest: +~150 ms on the
10k-file pull cells vs the pre-review session. Matrix re-run.

## F3 (High) — STATE pre-adjudicated the owner question and advanced to otp-10

**Adjudication: ACCEPTED.** STATE no longer says "gate satisfied" or
"Current: otp-10": the Now/Queue/Blocked entries all read HOLD on the
owner adjudication (options (a) per-direction verdicts / (b) designate
a symmetric pair), with otp-10 following it. The Queue inconsistency
codex flagged (still calling otp-2 current) is fixed in the same pass.

## F4 (Medium) — drain checked quiet before remote sync; timeout silent

**Adjudication: ACCEPTED.** `drain_pool` now syncs FIRST then waits
quiet, and `drop_caches` takes the run label so a DRAIN-TIMEOUT is
recorded per run in drain.log AND warned in bench.log — the final
dataset has exactly one (the expected post-staging first run), visible
in the committed drain.log.

## F5 (Medium) — fixed push destinations; interrupted runs poison reruns

**Adjudication: ACCEPTED.** Destinations now carry a per-invocation
SESSION_TAG and the EXIT trap sweeps them — an interrupted run cannot
leave content a rerun would no-op onto. (Proved useful the same day:
the killed mid-run rerun left dirs that the trap-less path would have
kept.)

## F6 (Medium) — quantitative claims exceeded the CSV evidence

**Adjudication: ACCEPTED.** README rewritten with exact per-probe
numbers (probe-1 spread stated per cell, up to 8.0×, not "4–8×"
blanket); the manual drained probe is now committed
(`probe3-drained-pushes.csv`); pull stability stated as ±6% typical
with the worst single run +21%; push/pull ratios stated as the actual
×1.23–×2.19 range; "physically unreachable regardless of code" replaced
by the D-2026-07-05-1 validity rule (which is the owner's own recorded
ground, not an inference from old-path timings).

## F7 (Low) — median flooring unstated

**Adjudication: ACCEPTED.** Rounding policy stated in both the harness
header and the README (integer ms; even-count median = floor of the
mean of the middle two).

## F8 (Low) — non-monotonic wall time + undocumented python3

**Adjudication: ACCEPTED IN PART.** python3 is now a preflight-checked,
documented prerequisite. The monotonic half was TRIED AND REVERTED with
evidence: start/end stamps are separate processes, and cross-process
`time.monotonic()` has an undefined reference point — the attempt
produced 0/negative windows while daemon logs showed multi-second
transfers (the aborted run4). Wall clock is the correct cross-process
choice here; the harness comment records why.

## Consequence

The matrix was fully re-run under the fixed harness (same commit's
binaries): `summary.csv`/`runs.csv`/`drain.log` are from that run; the
pre-review session is kept as `probe4-prereview-session-runs.csv` for
cross-session corroboration (~10% agreement on most cells).
