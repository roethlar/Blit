# live-dial-tuning-plan — restore telemetry-driven stream control

**Severity**: HIGH — production transfers cannot adapt stream count to live
conditions and cannot scale down, contrary to the settled transfer design.
**Status**: Reopened — D-2026-07-16-2 activated the reviewed plan; activation
round 1 found three stale secondary references to correct before ldt-1.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `554d080839e1419c2242921e444d40d02c947815` (round-1 candidate),
`a78d553` and `41dcb4d` (fixes), `b99637f` (round-2 candidate),
`acd368f` (activation candidate)

## Evidence

- `docs/DECISIONS.md:77-85` requires one sender-owned live dial, tuning from
  the first byte, with mid-transfer stream add/drop.
- `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:109-158` requires conservative
  start, telemetry-driven adjustment, and no surviving static
  size-to-streams table.
- `crates/blit-core/src/dial.rs:676-705` carries the retired daemon's static
  workload table; 10,000 files or 512 MiB maps to 8.
- `crates/blit-core/src/transfer_session/data_plane.rs:831-834` explicitly
  defers the live tuner; production sockets use the no-probe constructors and
  `spawn_dial_tuner_with_resize` has no production caller.
- `crates/blit-core/src/transfer_session/mod.rs:3556-3575` accepts ADD only and
  rejects REMOVE.
- `docs/plan/ONE_TRANSFER_PATH.md` and `docs/TRANSFER_SESSION.md` had narrowed
  sf-2's static shape correction into the sole policy, conflicting with the
  decisions above.

## Predicted observable failure

A 10,000-file transfer converges to eight workers regardless of whether live
statistics say more capacity is useful or the receiver is backpressured. It
cannot grow past the shape target, cannot shrink at all, and opens tail workers
only to satisfy the predetermined count. Exact-8 tests can therefore pass in
both initiator layouts while the approved adaptive behavior is absent.

## What

Create a self-contained implementation plan that restores one SOURCE-owned,
receiver-bounded, telemetry-driven controller to unified `TransferSession`.
Correct the active parent plan and wire contract so static shape parity is not
misreported as adaptive worker parity. No runtime code changes belong to this
finding.

## Approach

`docs/plan/LIVE_DIAL_TUNING.md` makes `SourceDataPlane` the single controller
ownership boundary for both connection layouts. It removes workload shape as
worker-count authority, wires the existing live probes/tuner, specifies exact
ADD and REMOVE membership settlement, preserves the receiver safety ceiling,
and requires deterministic role/lifecycle guards plus quiet Mac↔Mac evidence.
The parent plan, session contract, and STATE now name current HEAD as drift
rather than claiming exact eight completes the design.

## Files changed

- `docs/plan/LIVE_DIAL_TUNING.md` — Draft correction and implementation slices.
- `docs/plan/ONE_TRANSFER_PATH.md` — restore REV4 live-dial intent and record
  current implementation drift.
- `docs/TRANSFER_SESSION.md` — correct the one-stream-policy contract and
  describe current drift.
- `docs/STATE.md` — queue the correction and narrow the exact-8 claim.
- `REVIEW.md` — reviewloop status row.
- `.review/findings/live-dial-tuning-plan.md` — this evidence/review record.
- `DEVLOG.md` — candidate chronology.

## Guard proof

Documentation-only gate:

- `bash scripts/agent/check-docs.sh`
- `git diff --check`

Semantic review guard, to be independently executed in the retained review
worktree:

1. On reviewed bytes, assert all three documents establish SOURCE-only
   ownership, live non-idle telemetry, both ADD and REMOVE, both initiator
   layouts, a receiver safety ceiling, no static terminal target, and the
   exact-8 claim narrowed to static parity.
2. Restore only `docs/plan/ONE_TRANSFER_PATH.md` and
   `docs/TRANSFER_SESSION.md` from the dispatched base SHA. The assertions
   must fail on their shape-only policy text.
3. Restore the exact reviewed blobs. The same assertions and docs gate must
   pass, and the worktree must end clean at the reviewed SHA.

## Coder dispute (if any)

None.

## Known gaps

- This finding changes design records only. D-2026-07-16-2 activated the plan;
  production remains static ADD-only until the reviewed code slices land.
- The plan deliberately separates TCP stream tuning from local filesystem
  apply workers and the one-lane in-stream carrier. They share session
  choreography but do not share a meaningful socket-blocking signal.
- The receiver's default absolute stream limit remains a defensive constant,
  not a performance target. Making receiver capacity itself load-derived is a
  separate decision if live backpressure proves insufficient.

## Reviewer comments

Claude Code `2.1.211`, `claude-fable-5`, effort `max`; reviewed
`35d7d1307d7a2a455756b372d3bf637f2a5a382c..554d080839e1419c2242921e444d40d02c947815`;
`guard_confirmed=true`; verdict `REOPENED`; recorded
`2026-07-16T06:09:18Z`. The direct attempt returned exit zero, a schema-valid
payload, exact dispatched SHAs, and left the retained worktree clean.

- **MEDIUM, admitted:** the fault rule for an accepted ADD/REMOVE contradicts
  the required need-completion race. Specify that draining/completion plus
  normal END retirement satisfies an already accepted transition; reserve a
  fault for a still-live pipeline that refuses or errors on it.
- **LOW, admitted:** growing above 8 does not prove the retired 16-worker table
  maximum is absent. Make the deterministic guard cross 16 or reach a lower
  advertised receiver ceiling.
- **Observation, accepted without a design change:** the frozen blocked-ratio
  policy can grow to the receiver ceiling on a source-bound transfer. Keep it
  bounded and make the observer/evidence report the signature.
- The remaining architecture questions passed: shape authority is deleted,
  SOURCE ownership is shared across layouts, epoch zero is receiver-bounded,
  REMOVE fits the existing substrate, the cutover has no dual-policy window,
  and local/in-stream exclusions are not direction-specific worker paths.
- Independent proof: docs/diff gates plus 33 semantic assertions were green;
  restoring only the two contract docs to base made 17 expected assertions
  red; restoring exact reviewed blobs returned green and clean at the head SHA.

Raw record:
`.review/results/live-dial-tuning-plan-r1.claude.json`. Adjudication:
`.review/results/live-dial-tuning-plan-r1.claude-verdict.md`. The earlier
proxy-routed error is retained separately and is non-authoritative.

### Round 1 response

- The MEDIUM is corrected by `a78d553`: accepted versus unaccepted operations
  are distinct at need completion; accepted ADD completes authentication then
  immediately follows normal no-payload END retirement, accepted REMOVE may be
  satisfied only by the named member's normal retirement, and only a live
  pipeline refusal/error faults. The plan now requires a terminal member ledger
  and no socket/probe/member/receive-task leak in either layout.
- The LOW is corrected by `41dcb4d`: the deterministic role guard must use a
  receiver ceiling of at least 17, emit the same sequence through target 17,
  and turn red under either an 8- or 16-worker production clamp.
- The source-bound observation remains a bounded policy characteristic. The
  existing observer and hardware evidence criteria expose its sample/reason/
  count trace; it is not silently promoted into a correctness failure.

Round 2 reviews the complete corrected Draft and the incremental range from the
round-1 reviewed SHA.

### Round 2 attempt 1 — excluded

The first round-2 dispatch was interrupted without a verdict on 2026-07-16
when the owner replaced the prompt policy with D-2026-07-16-1. Its prompt named
expected defects and checks, so it is substantively steered under the new rule
and cannot count as review evidence.
The exact CLI error envelope is retained at
`.review/results/live-dial-tuning-plan-r2.claude-attempt1-steered-error.json`;
the prompt and worktree remain retained under `/tmp`. Round 2 restarts from the
same immutable plan candidate with a neutral best-way prompt.

### Round 2 — accepted

Claude Code `2.1.211`, `claude-fable-5`, effort `max`; reviewed
`554d080839e1419c2242921e444d40d02c947815..b99637fe34eff5407a50f8f07bf0d2a6b67525ad`;
`guard_confirmed=true`; verdict `ACCEPTED`; recorded
`2026-07-16T06:40:21Z`. The prompt followed D-2026-07-16-1: one neutral goal
and the best-way question, with no prior finding, expected fix, checklist, or
preferred outcome supplied.

Claude independently found both round-1 corrections coherent and sufficient,
verified the plan uses existing repository substrate, found the slice order and
scope sound, and reported no new material issue. Its self-chosen 35-assertion
guard was green on reviewed bytes, red on 23 correction assertions when only
the plan was restored to the round-1 blob, and green after exact restoration.
The retained worktree ended clean at exact reviewed SHA.

Raw record: `.review/results/live-dial-tuning-plan-r2.claude.json`.
Adjudication:
`.review/results/live-dial-tuning-plan-r2.claude-verdict.md`.

### Activation round 1 — reopened

Claude Code `2.1.211`, `claude-fable-5`, effort `max`; reviewed
`8f08546b181efa00a07365eaccfd6725a4064b43..acd368f338089a32e8d810fcecd4f580f572816a`;
`guard_confirmed=true`; verdict `REOPENED`; recorded
`2026-07-16T13:23:12Z`. The prompt was neutral under D-2026-07-16-1.

The activation itself and all primary records are correct. Three live secondary
references remain stale: this finding still named owner approval as pending,
`docs/plan/ONE_TRANSFER_PATH.md` called the correction a Draft, and
`docs/TRANSFER_SESSION.md` called it a Draft. All three findings are admitted as
one bounded consistency fix. The historical-pair warning from
`.review/check-state.sh` is pre-existing and is not admitted against this change.

Claude's independently chosen 26-assertion proof was green on the activation,
red on all 20 activation assertions after restoring the five changed files to
base, locally red on six plan assertions after restoring only the plan, and
green/clean after exact restoration. Raw result:
`.review/results/live-dial-activation-r1.claude.json`. Adjudication:
`.review/results/live-dial-activation-r1.claude-verdict.md`.
