# cr-jl4fix4-2: a record from the raw-list era is read as text-only

**Severity**: LOW (reviewer: LOW)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `2f2cd1ad`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix4-r1 over `beab73c9..e522afe8` (record `.review/results/jl4fix4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
.review/findings/cr-jl4fix2-1.md:22 — base-era writers already stored exact identities in `left_in_place_raw`; crates/blit-core/src/job_record.rs:188 — the absent new marker defaults false, and line 516 deserializes v1 directly without a field-presence migration.

## Predicted observable failure
A record written by the base-era binary can be treated as text-only; if a raw-named failure shares its lossy display text with a UTF-8 leftover, the retry is unnecessarily refused despite the record already containing exact list semantics.

## Reviewer's suggested approach
During v1 record loading, interpret an absent `left_in_place_exact` as true when the serialized `left_in_place_raw` key is present, and as false only when both the marker and raw-list key are absent; pin this with JSON fixtures from both formats.

## Intake
Admitted. Records written between the raw list and the marker already keep identities; read now, the absent marker makes them legacy and a collision is refused needlessly. A record whose raw-list key is present is exact; only one with neither key is text-only.

## What
`read_record` reads the record untyped as well: when the exact marker is absent but the `left_in_place_raw` key is present, the record is from between the raw list and the marker and keeps identities, so it is read as exact; a record with neither key is text-only, and a marker present is taken as written.

## Guard proof
`a_records_leftover_lists_are_exact_by_marker_or_by_the_raw_list` (core): four JSON fixtures — neither key (not exact), the raw list without the marker (exact), the marker false (not exact), the marker true (exact). Mutation — the migration dropped — makes it fail; restored, green.
