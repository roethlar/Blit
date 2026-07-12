# otp-12 perf findings — investigate + fix before otp-12c (design)

**Status**: Draft (owner, 2026-07-12: "let's fix the code before
devoting another block of time to testing. plan, reviewloop codex, then
fix once converged" — the flip to Active happens at codex convergence
per that instruction; implementation not before).
**Created**: 2026-07-12
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
say the quiet part: "Unification that slows the fast direction fails
review." otp-12a/b measured exactly two such cells; otp-12c/12d/13 are
deferred until they are fixed or explained at code level.
**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
if an investigation slice needs one, it stops and this doc is amended
through the loop first.

## The two findings (evidence, both committed)

**P1 — destination-initiated TCP mixed transfers pay ~25%**
(`docs/bench/otp12-win-2026-07-12/`): `wm_tcp_mixed` invariance FAIL at
**1.237** (mac_init pull 1127 ms vs win_init push 911 ms, spreads
8.2/3.3%), corroborated independently by block-1 `pull_tcp_mixed` new
1138 vs old-same-session 867 (**1.313**). The signature is sharp:
- carrier: TCP data plane only (wm_grpc_mixed = 1.013 PASS);
- fixture: mixed only (512 MiB + 5k×2 KiB; large 1.023, small 1.011);
- role: only when the DESTINATION end initiates (pull-verb).
Also present in 12a's data? NOT testable there (review 2026-07-12):
zoey's rig anchors converge-up only (12a README), so it has no
mac_init/win_init invariance pair; its pull_tcp_mixed 0.966 is a
new-vs-old check, not a two-layout measurement. P1 was never measured
on zoey — that PASS must not be read as absence or masking evidence.

**P2 — unified small-file push pays ~11–15% vs old push, both rigs**:
zoey `push_tcp_small` 1.105 (RUNS=8, tight), netwatch-01 1.149 (3–4%
spreads); grpc small pushes are AT parity (zoey 1.001, win 0.98-ish per
cells) — so P2 is also TCP-data-plane-specific, source-initiated,
10k×4 KiB. Cross-block note (12b README): block-2 `mw_tcp_small`
mac_init measured 1922 vs block-1 new 2080 in the same session — the
only mechanical difference is block-2's precreated destination
container and per-arm path shapes; the investigation must confirm or
kill that lead.

## Hypotheses (H*, ranked; each cites the recorded mechanism it accuses)

- **H1 (P1)**: data-plane socket-acquisition asymmetry on resize. The
  connection-initiating end DIALS; byte direction is role-set
  (`ONE_TRANSFER_PATH` §Transport facts). For a destination-initiated
  session the SOURCE is the responder: each sf-2 resize epoch is
  ACCEPTED off the source's listener while the DESTINATION dials
  (otp-5b-2: `SourceSockets` Dial/Accept branches;
  `InitiatorReceivePlaneRun.add_dialed_stream`). Mixed is the fixture
  that exercises mid-transfer shape correction hardest (tar-shard small
  half + big-file stream). Suspect: per-epoch accept/dial round-trips
  or serialization in the accept branch that the dial branch does not
  pay, surfacing only when resize fires under a fast source.
- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
  interleave cannot happen — resize begins only after
  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
  layouts drain the same fixed 128-entry destination need loop, so
  batch emission cannot interleave with the resize controller during
  manifest/need emission in either layout. Kept only as a residual: if
  pf-1 timing shows a layout-dependent need-batch delta anyway, the
  mechanism must be re-derived from the trace, not from this text.
- **H3 (P2) — mechanics CORRECTED (review 2026-07-12)**: dest-side
  cost in the receive path that old push didn't pay — but the listed
  candidates were wrong: the small half is tar-sharded and written
  with parallel per-file `create_dir_all`/`fs::write` and NO per-file
  flush, and per-file progress emission to the served push destination
  is disabled (`remote/transfer/sink.rs`); old push used the same
  served sink. So per-file fsync/flush policy and progress emission
  are NOT old/new deltas. Surviving candidates: dest-side directory
  work/handle churn (the 12b cross-block 8% precreated-container lead
  on NTFS) plus whatever the pf-1 trace names; zoey showing 1.105 says
  the residue is not Windows-only.
- **H4 (P2) — NARROWED (review 2026-07-12)**: binary record framing is
  unchanged since `0f922de` (`dial.rs`), and old small push ALSO
  opened at one stream (after its 128-file early flush) then resized
  live — so neither framing nor "fixed-count opening" discriminates.
  What survives of H4 is ramp cadence/shard-boundary timing only, and
  it is subordinate to H5.
- **H5 (P2, prime suspect; added by review 2026-07-12)**: lost
  scan/diff/transfer overlap on the TCP plane — current code withholds
  every TCP payload until `ManifestComplete`
  (`transfer_session/mod.rs`), while old push negotiated and queued
  TCP payloads mid-manifest (`0f922de` `push/client/mod.rs:863-940`).
  gRPC's in-stream carrier did not change comparably — which matches
  the exact signature "TCP regressed, gRPC at parity". NOTE: an H5 fix
  reorders session phases and multi-ADD/pipelined epochs conflict with
  the one-token/one-ADD contract (`TRANSFER_SESSION.md` §Phase
  ordering), so any H5 fix triggers this plan's Contract
  stop-and-amend rule BEFORE implementation.

## Method (the investigation slice — no behavior changes)

1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
   in-process/two-process rigs on the Mac with the otp-2 fixture
   shapes; `--trace-data-plane` + targeted `tracing` spans (added
   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
   ack), need-batch emission times, per-file sink open/write/close in
   the receive path, shard planner in/out timestamps.
2. **A/B the role layouts in one process**: the role suite already
   runs both initiator layouts over identical fixtures (otp-3) — but
   it forces the in-stream carrier (`transfer_session_roles.rs`), so
   the timing-harness variant MUST add a TCP-carrier mode; it reports
   phase timings per layout for mixed and small fixtures. A positive
   layout-dependent delta in a named phase confirms; local ABSENCE
   does not kill H1 (loopback removes the Windows↔Mac topology) — an
   H-kill needs either local reproduction or a rig-side instrumented
   run.
3. **Historical control, then bisect P2**: old push is deleted from
   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   build and run; the control is an old-vs-new run on identical
   fixtures with the same instrumentation. Experiments, corrected per
   review 2026-07-12: (a) precreate-vs-not stays but is
   environmental-only (it cannot attribute code); (b) the flush/
   instrument toggles missed the tar-shard path — instrument the
   tar-shard write path itself; (c) DROPPED — the ramp pin reproduces
   the same one-stream opening old push already had, so it
   discriminates nothing; (d) NEW, for H5: measure the
   manifest-complete→first-TCP-payload gap new vs old (overlap
   experiment); (e) per-member locking/framing timings only if the
   pf-1 trace implicates them.
4. Every experiment lands as a committed probe record under
   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   loop per slice as usual.

## Fix criteria (pre-registered; the owner walks the final numbers)

- Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in an
  acceptance cell passes independently against BOTH its same-session
  reference AND the committed baseline. The listed bars below are
  necessary, not sufficient — no arm may exceed 1.10 against either
  reference even when its counterpart bar passes (closes the
  1.10×1.10 ≈ 1.21 hole; review 2026-07-12).
- P1 fixed ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND `pull_tcp_mixed`
  ≤ 1.10 against BOTH references on the netwatch-01 rig (CELLS
  escalation session, RUNS=8), with `wm_grpc_mixed` and the other
  invariance PASSes unregressed against both references.
- P2 fixed ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
  (same-session AND committed) on BOTH rigs (CELLS sessions), grpc
  small parity unregressed against both.
- No suite regressions; the floor is ≥ the CURRENT count (1484 —
  ≥1483 would permit silently losing a test); any new pins carry
  guard proofs (temporary revert) per the loop.
- If investigation attributes part of a gap to something the plan's
  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
  that residue is RECORDED with its experiment and goes to the owner's
  otp-13 walk — never silently accepted.

## Staging (each through the codex loop)

- **pf-1 (HARD GATE)**: instrumentation + local reproduction harness +
  the two-layout phase-timing report (TCP-carrier mode included) + the
  `0f922de` historical control; probe record committed AND
  codex-reviewed BEFORE any pf-2 branch exists. No fix lands on
  pre-pf-1 evidence.
- **pf-2..n**: one fix slice per confirmed root cause (smallest
  change that moves the phase timing; A/B'd locally before rig time).
- **pf-final**: NOT just the two escalation cells — the final build
  reruns the COMPLETE affected-carrier matrices (all TCP cells + the
  gRPC controls) on BOTH rigs. No mixed-build evidence: every row
  cited for acceptance comes from the final build; pre-fix PASS rows
  are void for acceptance. If any shared controller/planner/sink code
  changed, the gRPC control cells rerun on the final build too.
  Results land in fresh dated evidence dirs; the otp-13 walk
  re-verifies on the full matrix; then otp-12c proceeds on the fixed
  code.

## Known gaps

- H1–H5 were graded against the actual tree by codex review
  2026-07-12 (H2 contradicted, H3 corrected, H4 narrowed, H5 added).
  The old drivers are deleted from HEAD, but the pinned `0f922de`
  source/binaries diff and run fine — historical claims get live
  controls in pf-1, not pin-archaeology.
- zoey never measured P1: its rig anchors converge-up only, so there
  is no invariance pair there — pull_tcp_mixed 0.966 is new-vs-old and
  says nothing about layout asymmetry (review 2026-07-12). pf-1's
  local rig must be fast enough to surface P1 (the Mac's APFS NVMe
  qualifies per the 12b wm numbers).
