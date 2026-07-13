# otp-12 perf findings — investigate + fix before acceptance (design)

**Status**: Draft (owner, 2026-07-12: "let's fix the code before
devoting another block of time to testing. plan, reviewloop codex, then
fix once converged" — the flip to Active happens at codex convergence
per that instruction; implementation not before).
**Created**: 2026-07-12
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
say the quiet part: "Unification that slows the fast direction fails
review." P1 is a miss of the parent's HEADLINE acceptance criterion
(initiator/verb invariance, ±10%) — not a nice-to-have.
**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
if an investigation slice needs one, it stops and this doc is amended
through the loop first.

**Sequencing (corrected 2026-07-13).** This doc originally deferred
otp-12c/12d/13 outright. In fact **otp-12c RAN on 2026-07-13** under a
fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
at the cutover sha `f35702a`; `docs/bench/otp12c-{delegated,win}-2026-07-13/`).
That does not change this plan's standing, and the rows are not lost
work — under `pf-final` they are **pre-fix rows, void for acceptance**,
but they serve two real purposes: (a) an **independent replication** of
both findings at the shipped sha (below), which is exactly the
independent corroboration the round-2 review said P1 lacked; and (b) the
pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
deferred** until P1/P2 are fixed or explained at code level — assembling
an acceptance matrix out of pre-fix rows would build the artifact otp-13
walks from rows this plan declares void.

## The two findings (evidence, both committed)

**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
(`docs/bench/otp12-win-2026-07-12/`, replicated in
`docs/bench/otp12c-win-2026-07-13/`). `wm_tcp_mixed` invariance FAILs in
**two independent sessions**, and got WORSE at the shipped sha:

| session | build | mac_init | win_init | ratio | arm spreads |
|---|---|---|---|---|---|
| 12b (2026-07-12) | `e21cf84` | 1127 | 911 | **1.237** | 8.2 / 3.3% |
| 12c-win (2026-07-13) | `f35702a` (cutover) | 1221 | 939 | **1.300** | 6.4 / 8.4% |

Corroborated by block-1 `pull_tcp_mixed` new-vs-old-same-session:
**1.313** (12b: 1138/867) and **1.247** (12c-win: 1192/956).

**This cannot be re-run away.** Both sessions' arm spreads are far below
D2's 25% escalation trigger, so no escalation session is even available;
the cells stand as measured. (The 12c-win session was a fresh staging on
a different day at a different sha — the round-2 review's objection that
the 1.313 corroboration was "same rig/session, not independent" is now
answered by an independent session reproducing the same cell.)

The signature is razor-sharp, and 12c-win tightens it with the control
that matters most — **the opposite data direction, same carrier, same
fixture, PASSES**:
- **direction**: `wm_tcp_mixed` (dest-initiated) **1.300 FAIL**, while
  `mw_tcp_mixed` (source-initiated, identical carrier + fixture) is
  **1.044 PASS**. The fixture and carrier are therefore not the cause on
  their own — the destination-initiator layout is;
- **carrier**: TCP only — `wm_grpc_mixed` **1.021 PASS** (12b: 1.013);
- **fixture**: mixed only — `wm_tcp_large` **1.039** and `wm_tcp_small`
  **1.027** both PASS;
- **isolation**: in 12c-win, 11 of 12 invariance cells pass at
  1.003–1.044. `wm_tcp_mixed` is the sole outlier, by a wide margin.

Also present in 12a's data? NOT testable there (review 2026-07-12):
zoey's rig anchors converge-up only (12a README), so it has no
mac_init/win_init invariance pair; its pull_tcp_mixed 0.966 is a
new-vs-old check, not a two-layout measurement. P1 was never measured
on zoey — that PASS must not be read as absence or masking evidence.

**P2 — unified small-file push pays ~10–20% vs old push, both rigs**,
`push_tcp_small` new-vs-old-same-session:

| session | build | new | old | ratio |
|---|---|---|---|---|
| 12a zoey (RUNS=8, tight) | `e757dcc` old arm | — | — | **1.105** |
| 12b netwatch-01 (3–4% spreads) | `e21cf84` | 2080 | 1811 | **1.149** |
| 12c-win (2026-07-13) | `f35702a` (cutover) | 1975 | 1644 | **1.201** |

**gRPC small push did NOT regress — it got materially FASTER**
(correction, review round 2: the earlier "win 0.98-ish per cells" was
wrong against the committed CSVs). `push_grpc_small` new-vs-old:
netwatch-01 **0.801** same-session / **0.835** committed (12b), and
**0.852** / **0.802** (12c-win); zoey is at parity (1.001). So the
honest statement is: **TCP regressed while gRPC did not — and on
Windows the gRPC small push improved materially.** That asymmetry is
the finding's sharpest constraint on mechanism: whatever P2 is, it is
TCP-data-plane-specific, source-initiated, and small-file-heavy
(10k×4 KiB), and it must not implicate code shared with the gRPC
carrier (which got faster on the same fixture).

Cross-block note (12b README): block-2 `mw_tcp_small` mac_init measured
1922 vs block-1 new 2080 in the same session — the only mechanical
difference is block-2's precreated destination container and per-arm
path shapes; the investigation must confirm or kill that lead. It is a
lead, not an attribution (a precreated container is environmental and
cannot attribute code — Method 3(a)).

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
  unchanged since `0f922de` (`remote/transfer/data_plane.rs`; the
  earlier `dial.rs` attribution was wrong), and old small push ALSO
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
- **H6 (P2; added by review round 2, 2026-07-12)**: per-member
  need-claim locking on the TCP receive plane — TCP receive
  (`NeedListSink`) takes a separate mutex/hash-set claim per member
  (`transfer_session/data_plane.rs:1167`), while the gRPC path claims
  a whole shard under one lock (`transfer_session/mod.rs:3047`).
  TCP-only and per-member (so small-file-heavy) — matches the P2
  signature independently of H5. Discriminated by the pf-1 per-member
  locking timings (Method 3(e), now unconditional). Historical
  control (review round 2): check whether the `NeedListSink`
  per-member claim already exists at 0f922de — if it does, H6 must
  name what in the new layout multiplies claim frequency (need-batch
  shape per Method 3(c)), otherwise a pre-existing lock cannot alone
  explain a regression introduced after 0f922de. If H6 is confirmed,
  the P2 fix bar applies unchanged (≤ 1.10 against BOTH references,
  BOTH rigs); no separate bar is granted.

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
   does not kill H1 (loopback removes the Windows↔Mac topology). So
   that H1 stays falsifiable: if the local run is negative, pf-1
   REQUIRES the rig-side instrumented run on netwatch-01 (same spans,
   CELLS fixtures) before pf-1 may close — every hypothesis exits
   pf-1 confirmed or killed, never "unfalsified" (review round 2).
3. **Historical control, then bisect P2**: old push is deleted from
   HEAD but NOT unavailable — the pinned `0f922de` source and binaries
   build and run; the control is an old-vs-new run on identical
   fixtures. The new tracing spans do NOT exist in `0f922de` (review
   round 2), so the control is observed externally — phase boundaries
   from wire + filesystem timestamps and stdout progress, with event
   semantics mapped span-for-span to the new names — or, where that is
   too coarse, a minimal probe backport onto the pinned `0f922de`
   source with identical event names. Either way every timed
   configuration runs an instrumentation-on/off pair to bound observer
   overhead (per-member tracing across ~10k files can perturb a
   double-digit share of the measured gap). Experiments, corrected per
   review 2026-07-12: (a) precreate-vs-not stays but is
   environmental-only (it cannot attribute code); (b) the flush/
   instrument toggles missed the tar-shard path — instrument the
   tar-shard write path itself; (c) REPLACED (review round 2) — the
   ramp pin discriminated nothing (old push also opened at one
   stream), but H4 keeps a code-level counterfactual: a batch-cadence
   replay toggle that processes need batches at the recorded old-push
   shard-boundary cadence; (d) NEW, for H5 — the overlap experiment,
   metric DEFINED (review round 2: "manifest-complete→first-payload
   gap" was underdefined, and for old push the quantity is expected to
   be NEGATIVE, which an unsigned "gap" cannot express). Record, per
   run, on ONE common clock with a SIGNED offset from the
   `ManifestComplete` event, three separately-named events on the
   source side plus one on the destination:
   `t_manifest_complete`; `t_first_payload_queued` (the payload enters
   the send queue); `t_first_socket_write` (first byte handed to the
   TCP data plane); `t_first_payload_received` (destination side —
   requires the two clocks to be reconciled, so record the ssh/NTP
   offset per run and report it with the number, or state that the
   destination event was not usable). The H5 claim is confirmed only if
   `t_first_socket_write − t_manifest_complete` is ≈0-or-positive on
   the new build and provably NEGATIVE on the pinned `0f922de` control
   for the SAME fixture — i.e. old push really did put TCP bytes on the
   wire before its manifest completed, and the new session does not;
   (e) per-member locking/framing timings are now an unconditional pf-1
   measurement (they discriminate H6), not contingent on the trace
   implicating them.
4. Every experiment lands as a committed probe record under
   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   loop per slice as usual.

## Fix criteria (pre-registered; the owner walks the final numbers)

- **The global rule dominates every bar below** (review round 2 flagged
  a contradiction between "necessary, not sufficient" and the `⇔`
  bars — the `⇔`s are hereby scoped as *definitions of the named
  finding's own bar*, never as a sufficient condition for acceptance).
  Per parent D2 (`OTP12_ACCEPTANCE_RUN.md` §criteria): EVERY arm in
  EVERY acceptance cell passes independently against BOTH its
  same-session reference AND the committed baseline — no arm may exceed
  1.10 against either reference even when its counterpart bar passes
  (closes the 1.10×1.10 ≈ 1.21 hole). A build that satisfies the P1 and
  P2 bars below but regresses any other cell against either reference is
  **not** accepted.
- **P1's bar is met** ⇔ `wm_tcp_mixed` invariance ≤ 1.10 AND
  `pull_tcp_mixed` ≤ 1.10 against BOTH references on the netwatch-01
  rig (CELLS escalation session, RUNS=8), with `wm_grpc_mixed` and the
  other invariance PASSes unregressed against both references. (Meeting
  this bar does not by itself accept the build — see the global rule.)
- **P2's bar is met** ⇔ `push_tcp_small` ≤ 1.10 against BOTH references
  (same-session AND committed) on BOTH rigs (CELLS sessions), with the
  gRPC small-push cells unregressed against both references — note
  those are currently FASTER than old (0.801–0.852), so "unregressed"
  means they must not slide back toward 1.0, not merely stay ≤ 1.10.
- Cross-direction converge-up is a SEPARATE bar (review round 2):
  every final cross-direction row must still meet the parent plan's
  new-vs-old ceiling (`ONE_TRANSFER_PATH.md` acceptance) or satisfy
  the registered platform-residue discriminator — invariance plus the
  per-direction bars alone would pass if a "fix" slowed BOTH layouts
  equally, violating converge-up.
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
  gRPC controls) on BOTH rigs. **No mixed-build evidence: every
  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
  (corrected, review round 2 — "every row" was impossible: the
  same-session `old` arms and the committed baselines are OLD builds by
  construction, which is the entire point of a reference). Pre-fix
  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
  which are **replication and control evidence, not acceptance
  evidence**. If any shared controller/planner/sink code changed, the
  gRPC control cells rerun on the final build too. Results land in fresh
  dated evidence dirs. **Then** otp-12d assembles the matrix from
  final-build rows, and the otp-13 owner walk reads it.

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
- **The 12c-win rows are replication, not acceptance** (2026-07-13).
  They are pre-fix by definition, so `pf-final` voids them for
  acceptance; their value is that they (a) reproduce P1 and P2 on an
  independent session at the shipped sha, (b) supply the
  opposite-direction control (`mw_tcp_mixed` 1.044 PASS vs
  `wm_tcp_mixed` 1.300 FAIL — same carrier, same fixture) that narrows
  P1 to the destination-initiator layout, and (c) serve as the pre-pf-1
  baseline. Both findings got WORSE at the cutover sha (P1 1.237→1.300,
  P2 1.149→1.201), so neither is drifting toward the bar on its own.
- **Rig-D delegated parity is NOT implicated** (2026-07-13): the
  delegated-vs-direct matrix passed 7/7
  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
  measurable cost and is not a suspect for either finding. pf-1 need not
  instrument the delegated trigger path.
