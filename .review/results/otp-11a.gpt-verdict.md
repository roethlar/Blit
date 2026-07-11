# otp-11a — codex verdict adjudication

**Reviewed**: commit `dfdddd6` (local transfers ride the session).
Raw review: `.review/results/otp-11a.codex.md` (gpt-5.6-sol via codex
v0.144.1, VERDICT: FAIL, 9 findings; the reviewer pinned itself to the
commit object while later docs-only commits landed — correct posture).
Codex confirmed the shared remote paths unchanged, the files/dirs
recombination sound, and the suite delta honest (1488 → 1510, none
removed).
reviewer: gpt-5.5-class (gpt-5.6-sol)

## F1 (High) — LocalApply "branches before the sink seam into duplicated diff/planning/dispatch — a second subpath"

**Accepted in part.** The duplication half was real scaffolding, not
semantics: the local diff twin repeated the wire diff's blocking-pool
fold around the same `destination_needs` core. Fixed by extraction —
`diff_chunk_verdicts` is now the ONE diff core both carriers call;
`diff_chunk_and_send_needs` (grant to source) and
`diff_chunk_and_apply_local` (plan + apply in-process) are thin
dispatchers, which IS the carrier difference. The "second subpath"
conclusion is disputed on the grounds adjudicated at the design round
(`.review/results/otp-11-design.gpt-verdict.md` F1): the carrier delta
is now stated in the slice doc, every semantic layer is shared code,
and the bypassed structures (`NeedBatch`/`outstanding`/source payload
work) are precisely the wire-carrier mechanics that have no meaning
when no byte crosses a transport. Session resume records: local resume
is the carrier's sink-level block phase (design F5 adjudication),
pinned by the new resume test.

## F2 (High) — local work starts only after 128 headers or ManifestComplete; immediate-start pin not ported

**Accepted as fact, rejected as regression-to-fix.** Diff batching is
session-uniform: the WIRE session has granted needs in
`DEST_DIFF_CHUNK` batches since otp-4 — the local carrier inherits the
session's start behavior rather than the old engine's 3-header
eagerness, which is the one-path goal (making local eager-er than
remote would create exactly the per-topology divergence this plan
deletes). For trees > one chunk, first work still lands before
enumeration completes; the overlap property ports as a pin at 11b (the
slice doc's floor plan names it). Empirically the A/B tree/small cells
sat within the gate. No code change; doc states the batching
explicitly.

## F3 (High) — bare `JoinHandle`: cancellation/early error detaches the pipeline

**Accepted — fixed.** `LocalApplyRun` now aborts the pipeline task on
drop (a run not consumed by `finish()` — session error or cancelled
future — stops at the next payload boundary; the in-flight
`spawn_blocking` write completes, queued payloads are dropped). Same
bound as the data-plane receive's AbortOnDrop discipline.

## F4 (High) — apply-time unreadables slip past the ManifestComplete mirror guard

**Accepted — fixed.** Verified against the old engine: R46-F2 refused
mirror deletions on ANY unreadable entry (engine/mod.rs:695-727,
including per-file open failures recorded during the pipeline). The
session's local carrier now carries the same posture: at SourceDone,
after the apply pipeline joins and before any deletion, a non-empty
unreadable accumulator refuses the mirror. Deterministic pin: the
in-crate vanishing-source test (a clean scan whose availability check
drops an entry — a mode-000 fixture is caught at scan time and lands
in the existing scan-incomplete pin instead). Guard-proven by
mutation. Note the blast radius was already bounded: extraneous-entry
deletion candidates come from the manifest diff, so a manifested-but-
vanished file's destination copy was never a deletion candidate — the
guard closes the "success with silently incomplete apply + deletions"
report, the old path's exact refusal.

## F5 (High) — bench harness swallows binary failures

**Accepted — fixed, and the rerun surfaced a real regression.**
`run_cell` now propagates a non-zero binary exit as `FAIL` and the
caller aborts the gate on any non-numeric timing. Rerun (fixtures
reused): huge/tree/small PASS; the 33-file noop cell flipped to a
noise-bound "FAIL" (20→23 ms, startup jitter — the first run had NEW
winning), so a focused 10k-file noop cell was run for a real signal —
and found the change-journal skip engaging on the OLD path's steady
state: old ~21 ms (journal-warm) vs new ~219 ms (full enumerate+diff,
which beats the old path's own non-journal pass at 610 ms). The
regression is precisely the D3 journal retirement, quantified. Per the
slice doc's own gate rule this BLOCKS 11b pending an owner decision
(recorded in docs/STATE.md; full table in
`docs/bench/otp11-local-2026-07-11/README.md`). Codex design F8's
concern is thereby confirmed with data — the design verdict's
"fair fight" reading held only for cold/fresh trees, not warm
repeated runs.

## F6 (High) — finding doc claimed committed A/B evidence absent from `dfdddd6`

**Accepted — record fixed.** The evidence landed at `631255b` (after
the slice commit; the gate had in fact run before the finding doc was
committed, but the doc cited a path not yet in history). The finding
doc now cites the evidence commit and the rerun.

## F7 (Medium) — `options.workers` ignored

**Accepted — fixed.** `LocalApply.sink_workers` maps the hidden
`--workers` debug limiter (which always sets `debug_mode`) to the
apply pipeline's worker count; the default stays 1 — the old streaming
pipeline's exact shape (`vec![sink]`), which is what the A/B measured.

## F8 (Medium) — outcome synthesis uses the fast-path-shape gate, not the doc's count-based rule

**Accepted as doc defect — impl kept.** The implementation replicates
the OLD outcome reachability exactly (mirror/checksum/SizeOnly runs
always reported `Transferred` on the old streaming leg; only fast-path
shapes could say `UpToDate`/`SourceEmpty`) — 11a's bar is parity, and
the parity gate also guarantees mirror deletion lines never hide
behind an early-return outcome. The slice doc's D2 said count-based;
the doc was wrong and is amended.

## F9 (Medium) — null-sink history tagged "session" contaminates Real profiling

**Accepted — fixed.** Verified: `RunKind` derivation keys on
`fast_path == Some("null_sink")` (perf_history.rs:199). `--null` runs
keep the `null_sink` tag; real runs record `session`.

## Disposition

7 accepted-and-fixed (F1-dedup, F3, F4, F5, F6, F7, F9), 1 accepted as
doc defect (F8 — doc amended, impl kept for parity), 1 rejected as a
regression while accepting the fact (F2 — session-uniform batching,
overlap pin ports at 11b). Fix sha: `e445e8d` (gate green: fmt,
clippy -D warnings, cargo test --workspace 1512/0, 2 ignored — after a
consistent clean rebuild; two earlier full-suite "failures" were
BUILD_MISMATCH artifacts of e2e binaries carrying different
dirty-tree digests from mid-edit builds, i.e. D-2026-07-05-2 refusing
correctly). Suite 1510 → 1512 (+2 fix-round pins). Guard proofs this
round: apply-time mirror guard mutation → vanishing-source pin FAILS
→ restored → passes.
