# cr-ecm24-8: a closed record left its raw name and its hold marker

**Severity**: MEDIUM — after a resume record closed, failed or completed, the sink kept its raw-name mapping and its hold marker to the session's end
**Status**: Verified — `combined-r26` (Codex openreview, 2026-11-22) reviewed `19025839..05ff50d7` — the eight `combined-r25` fixes — with `guard_confirmed: true`: nothing further asked of it; a file the source skips is cr-ecm25-6's
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b38564c2`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r25` over `c89b1eb1..19025839` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r25.codex.md`. It completes cr-ecm22-5 (`bdc472dd`) and cr-ecm23-6 (`a3b73d30`).

## Evidence
"After a failed zero-block record with a raw filename and a failed record with a held partial, I measured zero saved baselines and file handles, but one raw-name mapping and one `resume_held` marker. Drain those entries when each record closes."

## Intake
Admitted as the reviewer put it. The raw names the sink keeps are noted only for resume records (`note_raw_name`), and are read only while such a record runs. Its completion is the last of them.

## What
`FsTransferSink::forget_record` drops a record's raw name and its hold marker. It runs when the source fails the record (`fail_resume_record`) and when its completion closes it, completed or failed.

## Guard proof
- `sink::tests::a_closed_record_leaves_nothing_noted_for_it` (Unix): one record failed by its source and one completed, each with a raw name noted and a partial held — nothing noted afterwards.
- Mutations, each restored byte for byte: a record its source failed keeping what was noted (red); a completed record keeping it (red).
- The resume, sink and session tests (312) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `forget_record`, a test accessor, the two closes; guard.

## Known gaps
- A completion that fails with an error ending the session returns before it forgets. The sink goes with the session.
