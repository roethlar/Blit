# cr-jl4fix2-1: a lossy-name collision gives left-in-place status to the wrong file

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix2-r1 over `e3a8bde0..47827f24` (record `.review/results/jl4fix2-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-core/src/job_record.rs:824 — left-in-place membership is stored only by display path and retained while any failure has that path; crates/blit-cli/src/jobs.rs:299 — every raw identity sharing that path is consequently classified as Blit's own leftover.

## Predicted observable failure
If two byte-distinct names collapse to the same text, one leaves an incomplete copy and later succeeds while the other remains failed, `jobs retry` misclassifies the remaining file as Blit's leftover. For an original `--ignore-existing` job it disables that protection and can overwrite a pre-existing destination file.

## Reviewer's suggested approach
Store the optional raw bytes with each left-in-place entry, remove and partition entries by exact composite identity, migrate existing text-only records conservatively, and prove the collision through detached settlement and retry execution.

## Intake
Admitted. Failures carry their own bytes since cr-jl3afix1-1, but the left-in-place set — in the engine's summary, the record and the daemon-log fold — is keyed by text alone, so two names of one text share the status. The status must be kept by the same (text, bytes) identity as the failure it belongs to, through the wire and the record, and the `--ignore-existing` split must use it.

## What
The left-in-place status is kept by the failed entry's identity everywhere it lives. In the engine, `SinkOutcome` keeps a text set for entries named by their text and a raw set (`left_in_place_raw`) for the rest; at the end of a session, `named_raw` names every failure recorded by its text alone from the manifest the session granted (one entry per text, since a second entry of a text is refused at intake) and moves its status to the raw set — one pass before the summary, covering every carrier and the local route. The wire carries the raw set (contract 7: `TransferSummary.left_in_place_raw = 14`, `DelegatedPullSummary.left_in_place_raw = 15`; the text list then holds only text-named entries; the truncation flag covers both). `LocalMirrorSummary`, `RunTotals` and `RunRecord` (`left_in_place_raw`, escaped like `Failure::raw`; an older record classifies no raw-named file as left in place) carry it through; the daemon-log fold (`LogRead`) keeps identities and removes exactly the identity a later event landed; `jobs retry` classifies each failure by its own identity (`is_left_in_place`) — text against the text list, bytes against the raw list, or what this machine's log said for that identity.

## Guard proof
`left_in_place_status_follows_the_entrys_own_bytes` (engine: a marker failure with bytes lands in the raw set; one recorded by text is named and moved by `named_raw`; the wire form and `merge_failures` carry it), `reading_a_log_notes_what_it_lost` (daemon-log fold: a copy of another entry of the same text does not clear the status; the record's two lists), `left_in_place_status_is_the_entrys_own` (CLI: the reviewer's case — the entry that left its copy and landed had other bytes; the still-failed one is not the run's leftover), and `a_raw_named_write_is_left_in_place_by_its_bytes` (session end to end, Linux only — runs in CI). Four mutations each red alone — the engine filing a raw-named entry by text, `named_raw` moving nothing, the fold removing by text, the classifier matching any raw — restored green. Not proven here: the session's naming pass itself (its test cannot run on macOS).
