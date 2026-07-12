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
Also present in 12a's data in weaker form? No — zoey pull_tcp_mixed
PASSed (0.966) — so the cost needs the fast-NVMe/Windows-source rig or
is masked by zoey's pool; the investigation must say which.

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
- **H2 (P1)**: need-list/diff cadence differs by initiator layout for
  the tar-shard planner — the destination diffs incrementally and
  returns need batches; when the destination is also the session
  initiator, batch emission may interleave differently with the resize
  controller (controller-at-sender), delaying shard planning for the
  small half of mixed.
- **H3 (P2)**: per-file dest-side cost in the receive path that old
  push didn't pay — candidate mechanics: per-file fsync/flush policy,
  directory-handle churn, or per-file progress emission (w6-1
  `SourceInstruments`/dest instruments) synchronous with the write
  loop. The 12b cross-block 8% delta (precreated container) points at
  dest-side directory work as a real component on NTFS; zoey showing
  1.105 says the rest is not Windows-only.
- **H4 (P2)**: tar-shard boundaries/stream ramp on the TCP plane —
  grpc-at-parity means the in-stream carrier's shard handling is fine;
  the TCP path's binary record framing or its dial-floor ramp
  (`stream count corrects as the need list accumulates`) may start
  slower than old push's fixed-count opening for 10k tiny files.

## Method (the investigation slice — no behavior changes)

1. **Reproduce locally-instrumented, not on the rigs**: two-daemon
   in-process/two-process rigs on the Mac with the otp-2 fixture
   shapes; `--trace-data-plane` + targeted `tracing` spans (added
   behind a debug flag, kept) around: resize epochs (arm→accept/dial→
   ack), need-batch emission times, per-file sink open/write/close in
   the receive path, shard planner in/out timestamps.
2. **A/B the role layouts in one process**: the role suite already
   runs both initiator layouts over identical fixtures (otp-3) — add a
   timing harness variant that reports phase timings per layout for
   mixed and small fixtures; the P1 signature must reproduce as a
   layout-dependent delta in a named phase, or H1/H2 die.
3. **Bisect P2 against old push mechanics**: old push is deleted, but
   its recorded per-phase behavior (sf-2 pins, otp-2 baselines) and
   the block-2 container lead give three testable deltas: (a)
   precreate-vs-not (pure fs experiment on NTFS + APFS), (b) per-file
   flush/instrument cost (toggle via debug flag), (c) ramp (fix the
   initial stream count to old push's opening as an experiment flag).
4. Every experiment lands as a committed probe record under
   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   loop per slice as usual.

## Fix criteria (pre-registered; the owner walks the final numbers)

- P1 fixed ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND `pull_tcp_mixed`
  same-session converge ≤ 1.10 on the netwatch-01 rig (CELLS
  escalation session, RUNS=8), with `wm_grpc_mixed` and the other
  invariance PASSes unregressed.
- P2 fixed ⇔ `push_tcp_small` same-session ≤ 1.10 on BOTH rigs (CELLS
  sessions), grpc small parity unregressed.
- No suite regressions; the ≥1483 floor stands; any new pins carry
  guard proofs (temporary revert) per the loop.
- If investigation attributes part of a gap to something the plan's
  Non-goals exclude (e.g. NTFS directory semantics no code can dodge),
  that residue is RECORDED with its experiment and goes to the owner's
  otp-13 walk — never silently accepted.

## Staging (each through the codex loop)

- **pf-1**: instrumentation + local reproduction harness + the
  two-layout phase-timing report; probe record committed. No fix.
- **pf-2..n**: one fix slice per confirmed root cause (smallest
  change that moves the phase timing; A/B'd locally before rig time).
- **pf-final**: the two CELLS escalation sessions above (rig time is
  cheap here: ~10 min each, not another full matrix); results appended
  to the otp-12 evidence dirs; then otp-12c proceeds on the fixed
  code.

## Known gaps

- The hypotheses cite recorded mechanisms, not yet-verified code paths
  (the deleted old drivers can't be diffed live; their behavior is
  known from pins + baselines). pf-1 exists to kill or confirm them —
  a codex reviewer should grade H1–H4 against the actual tree.
- zoey's P1 absence (pull_tcp_mixed PASSed there) is itself evidence:
  whatever P1 is, a slow-pool destination masks it. pf-1's local rig
  must be fast enough to surface it (the Mac's APFS NVMe qualifies per
  the 12b wm numbers).
