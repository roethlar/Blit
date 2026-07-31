# cr-ls1-4 — Failed sessions report truncated phases as fast ones

**Severity**: MEDIUM
**Status**: FIXED — awaiting reviewer verification
**Source**: `ls-1-range` round-2 codex dispatch over `a0b5d83d..35948e70`
Reviewer: codex / gpt-5.6-sol / xhigh / workspace-write (detached, disposable
worktree); codex-cli 0.146.0. Record:
`.review/results/ls-1-range.codex.r2.json`.

## The finding, as returned

> `local.rs:770` deliberately emits before checking the session faults, but
> `mod.rs:4966` applies `?` before closing the Compare span and line 4993
> applies `?` before recording Plan; source enumeration has the same
> record-after-success shape. A scan, comparison, or plan that fails after
> substantial work therefore emits that phase as zero or incomplete with no
> failure/partial marker, contradicting the stated purpose of timing slow
> failed runs and potentially selecting the wrong phase.

## Why it is admitted

The slice explicitly claimed the opposite. The emit site carries the comment
"emit before the fault match, so a failed session still yields its breakdown
— a run that died slowly is exactly the one worth timing", and then three
call sites discard exactly that timing via `?`. A run that spent four
minutes enumerating before dying would report `ENUMERATE: 0` — which does
not read as "missing", it reads as "instant", and would push attribution
onto whichever phase did manage to close. Same failure class as cr-ls1-1,
reached through the error path.

## Fix

Three sites restructured to close before propagating:

- `diff_chunk_and_apply_local`: `let verdicts = …await; compare_span.finish();
  let needed = verdicts?;`
- the same shape for the PLAN span.
- `spawn_manifest_task`: the walk's ENUMERATE record moves above the `?` on
  the scan outcome.

And the report gains `session_failed`, set from
`dest_result.is_err() || source_result.is_err()`. The flag matters as much
as the spans: a truncated phase is a FLOOR, not a measurement, and a reader
(or a later attribution slice) that cannot tell the difference will draw the
wrong conclusion confidently.

**Guards**: `a_failed_session_is_marked_in_the_report` and
`a_clean_session_is_not_marked_failed`.

**Guard proof**: hard-coding `session_failed: false` at the construction
site reds the failed-session test; byte-identical SHA-256 restore
(`0F190352…3B1D`), green after.

## Known gap

The guards prove the FLAG is plumbed and honest. They do not prove that a
mid-scan failure records partial enumerate time, which would need a fixture
that fails a walk deterministically after measurable work. The span
restructuring is a code-read argument, not a measured one, and is recorded
as such rather than claimed as proven — the mistake cr-ls1-2 was about.
