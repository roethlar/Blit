# cr-ecm13-4: A retry's own record forgot where the run before it had left an incomplete copy

**Severity**: HIGH — a retry of a retry exits 0 with the destination still holding the incomplete copy
**Status**: Verified — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`, which required no change to it
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `f950e261`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. It completes cr-ecm12-2 (and cr-rework-2, which carried this through one command's passes only).

## Evidence
"A 128 KiB copy and its first saved retry both hit a file-size limit. After I lifted the limit, retrying the second run exited 0 and left the destination at zero bytes. That run had recorded an empty `left_in_place` list. A retry of a reported replacement conflict likewise cleared the failure without copying."

Cause. Which paths hold a run's own incomplete copy is read from each run's own record — from the words of its failures. A retry is a run of its own. Its failure for such a path said something else (a second failed write into a file that was already there said nothing of a copy; cr-ecm12-2's "kept" said it in other words), so its record held no such path, and the retry after it sent the path as any other: the option kept the file standing there, and the failure was cleared.

## Intake
Admitted as the reviewer put it. After cr-ecm13-1 the first half cannot happen as described — no retry writes into a file that is there — and what remains is the second: a path kept where a copy was left must be recorded as one.

## What
What a retry says of a file kept where a copy was left (`A_FILE_WHERE_A_COPY_WAS_LEFT`) is said in the words every record is read for (`INCOMPLETE_LEFT_IN_PLACE`): `an incomplete copy was left at the destination by this run, and a file stands there: --ignore-existing keeps it — remove it and retry, or run without --ignore-existing`. The session's exact sets, the job's record and its log then all hold the path as one where a copy was left, in the retry's own run as in the first — and every later retry sends it apart again, keeps the file, and reports it, until nothing stands there.

## Guard proof
- `job_retry.rs::a_retry_of_a_retry_still_knows_where_an_incomplete_copy_was_left`: a run that left an empty copy; the retry, the retry of the retry and the retry of that each exit 2 with the file untouched, each a run whose own record lists the path as left; with the file removed, a retry of the last copies it.
- `sink::…::a_file_kept_where_a_copy_was_left_is_recorded_as_such`: the words hold the phrase, and a failure recorded with them is in the exact set — by its own bytes for a name that is not text.
- Mutation, restored byte for byte: the kept path told in words no record is read for → both red (the second retry exits 0).

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the words; guard.
- `crates/blit-cli/tests/job_retry.rs` — guard; two assertions on the words.
- `CHANGELOG.md`.

## Known gaps
- What a record holds is still read from the WORDS of its failures; a path is carried because the words match, not because a field says so.
- A path kept this way is sent again by every pass of `--retries`, each to the same end.
- Run on Linux only.
