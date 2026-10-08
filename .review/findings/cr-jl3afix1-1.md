# cr-jl3afix1-1: a lossy-name collision gives the failed file the wrong bytes

**Severity**: MEDIUM (reviewer: MEDIUM)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `59439838`
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl3afix1-r1 over `7241a572..32280602` (record `.review/results/jl3afix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/run_log.rs:366 — raw identities are collapsed into a first-wins HashMap keyed only by lossy text; crates/blit-cli/src/run_log.rs:299 then recovers failure bytes solely through that text, while crates/blit-core/src/transfer_session/mod.rs:4436 records the second colliding manifest entry as the rejected failure.

## Predicted observable failure
With two distinct Unix filenames that collapse to the same display string, the first entry lands and the second is rejected, but the local RunRecord identifies the failure with the first entry's bytes. The record is forensically false, and future jobs retry can select the already-landed file while leaving the actual failed file absent.

## Reviewer's suggested approach
Add optional raw identity to FileFailed/FileFailure at the point the failure is created, preserve it through summaries and RunTotals, and construct RunRecord failures directly from those structured identities; verify two colliding non-UTF-8 names end to end on Linux.

## Intake
Admitted. When two names collapse to one text, the first entry lands and the second is rejected as a duplicate, but the record's lookup by text gives that failure the first entry's bytes. The failure must carry its own bytes from where it is made. The plan's jl-3a known-gap line about raw bytes is also stale since cr-jl3a-3.

## What
A failure carries its own name bytes from where it is made. `FileFailure` gains `raw_relative_path`, set by `record_named_failure` at every site that holds the entry: the duplicate-manifest rejection (the second entry's bytes, not the first's), a name the destination cannot store, a source's skip as received, a retry scan's failures, and the source's own skip frames and prepare failures. The wire `FileFailure` carries them (contract 7, field 3; dropped whole past the path bound), `RunTotals.failures` keeps whole failures, the log's closing lines and the record's failures use a failure's own bytes and fall back to the log's map by text only when it has none. The plan's stale known-gap line is replaced.

## Guard proof
`{in_stream,data_plane}_duplicate_manifest_path_is_reported_once_first_wins` (blit-core `source_side_containment`): the duplicate entry now names other bytes, and its failure must arrive over each carrier with them. `each_failure_is_logged_by_its_own_bytes` (blit-core run_log): the log names the failure by its bytes although the text maps to the first entry's. `a_record_keeps_a_failed_names_exact_bytes` (CLI): two failures of one text keep their own bytes. Five mutations each red alone — the rejection recording no bytes, `to_wire` dropping them, `from_wire` dropping them, the log preferring the text map, the record preferring the text map — restored green. Not run: two colliding non-UTF-8 files end to end on Linux (macOS cannot hold such names); the manifest-rewriting test exercises the same rejection path with synthesized entries.
