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

**What the evidence actually supports — and the confound it does NOT
escape** (corrected, review round 3; an earlier draft of this section
claimed the `mw` cell was a clean control isolating "destination
initiation" as the cause. It is not, and the correction matters because
it re-aims the hypotheses):

Every invariance cell compares two arms that share the same endpoints
and the same data direction, so **within** a cell the initiator is the
only variable — that part is clean. Arm medians (12c-win):

| cell | data direction | dest-initiated arm | source-initiated arm | ratio | spreads |
|---|---|---|---|---|---|
| `wm_tcp_mixed` | Win→Mac | 1221 | 939 | **1.300 FAIL** | 6.4 / 8.4% |
| `mw_tcp_mixed` | Mac→Win | 1477 | 1415 | 1.044 PASS | 20.8 / 20.5% |

The initiator penalty is therefore **real and large in the Win→Mac
direction only**. In Mac→Win the two layouts are within noise, and the
ordering even **flips between sessions** (12b: dest-initiated 1502 was
*faster* than source-initiated 1587), on spreads of 17–25%.

Crossing from `wm` to `mw` is **not** a controlled swap of one variable:
it also swaps the destination filesystem (APFS vs NTFS), the TCP stack,
which host runs the client, and the flush method. So the supported
signature is an **interaction — TCP × mixed × Win→Mac × initiator** —
not "destination initiation" on its own.

Worse, on a two-host rig the failing configuration is **confounded by
construction**: in the slow arm the destination is the Mac (which dials)
*and* the source is Windows (which accepts). With only two hosts, **host
identity IS role** — "Mac-as-dialing-destination" and
"Windows-as-accepting-source" are the same configuration and cannot be
separated by any number of additional runs on this rig.

### THE CONFOUND IS BROKEN — and it breaks toward CODE (2026-07-13)

**Probe: `docs/bench/otp12-perf-2026-07-13/` — magneto↔skippy, Linux on
BOTH ends, real 10 GbE.** The owner offered magneto as a bench end and
confirmed it saturates 10 GbE (unlike zoey, whose CPU is too slow to
partner skippy — `.agents/machines.md`). Same `+f35702a` musl build both
ends; `mixed` fixture; 3 runs/arm:

| data direction | source-initiated | destination-initiated | ratio |
|---|---|---|---|
| skippy → magneto | 950 ms | **1690 ms** | **1.78** |
| magneto → skippy | 1340 ms | unstable (1540–6370) | — |

**P1 reproduces with no Mac and no Windows anywhere in the path** — and
LARGER than rig W's 1.300. Therefore:

- **The platform-residue explanation for P1 is DEAD.** There is no
  macOS/Windows asymmetry left to attribute the gap to, so
  **D-2026-07-12-1's escape hatch does not apply to P1**: it cannot be
  accepted as a destination-write-path residue at the otp-13 walk.
- **P1 is a property of blit's layout — i.e. our code.** H1/H5/H6 are
  live, and a fix is MANDATORY for the parent plan's headline
  invariance criterion. This is no longer a question the owner can
  waive; it is a defect.
- The probe is **not evidence-grade** (no cold caches — magneto lacks the
  `drop_caches` grant; no drains/ABBA/pair-void; RUNS=3). It decides the
  confound; it does not enter the acceptance matrix, and pf-final voids
  it like every other pre-fix row. The magneto→skippy dest-init arm is
  unstable (1540 vs 6370), unexplained, and must not be cited until the
  harness resolves it.

**Promoting magneto to a pf-1 rig needs** (owner): the `NOPASSWD`
`/usr/bin/tee /proc/sys/vm/drop_caches` grant, and the torrent services
quiesced. Then Linux↔Linux becomes pf-1's primary rig — it isolates the
layout with no platform terms at all, which rig W structurally cannot do.

### The residual confound (WHICH code) still needs a counterfactual

Breaking platform-vs-code does NOT tell us *which* layout property costs
the time. On any two-host rig, host identity remains welded to role, so
"the accepting end" cannot be separated from "that host" by more runs:

- **pf-1 must compare all four rig-W arms** (both cells × both
  initiators), not two, and report the interaction — not a single ratio.
- **The disambiguator is a dial/accept inversion counterfactual, not a
  rig.** Today the initiator always dials and the responder accepts, so
  role and host are welded together. pf-1 adds a **debug-flag that flips
  which end dials** for a given source/destination assignment, then runs
  the SAME data direction, SAME hosts, SAME fixture, changing only who
  accepts. If the ~30% follows the **accept role**, H1 is CONFIRMED; if
  it stays with the **platform** regardless of who accepts, H1 is KILLED
  and the residue is a TCP-stack/write-path property (→ the D-2026-07-12-1
  discriminator and the owner's walk). This changes connection topology
  even behind a debug flag, so it **trips this plan's Contract
  stop-and-amend rule** (`TRANSFER_SESSION.md` amended through the loop
  BEFORE the flag is written). Same-build-both-ends (D-2026-07-05-2)
  means no compatibility surface is created.
- **The same-platform loopback run is a ONE-WAY test** (corrected — an
  earlier draft of this section had it backwards). A dest-initiator
  penalty that still appears on Mac↔Mac loopback proves **pure layout**
  (code). Its ABSENCE proves **nothing**: loopback has no NIC, near-zero
  RTT and a huge MTU, so it erases exactly the per-epoch accept/dial
  round-trip cost H1 accuses. A negative local result is **INCONCLUSIVE**
  and never reads as "no code bug" — it escalates to the inversion
  counterfactual and the rig-side instrumented run (Method 2).

This refines rather than weakens H1: H1 accuses the **source's accept
branch** under resize, and the source in the slow arm is Windows —
consistent. But consistency is not confirmation, and the confound above
is exactly why pf-1 exists.

The rest of the signature is unchanged and sharp:
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

**gRPC small push did NOT regress** (correction, review round 2: the
earlier "win 0.98-ish per cells" was wrong against the committed CSVs;
range corrected again in round 3). `push_grpc_small` new-vs-old,
same-session / committed:

| rig | same-session | committed |
|---|---|---|
| zoey | **1.001** | 0.907 |
| netwatch-01 (12b) | **0.801** | 0.835 |
| netwatch-01 (12c-win) | **0.852** | 0.802 |

So the cross-rig range is **0.801–1.001**: gRPC small push is at parity
on zoey and materially FASTER on Windows. The honest statement is **"TCP
regressed while gRPC did not"** — not "gRPC is uniformly faster".

That asymmetry is the finding's sharpest constraint on mechanism:
whatever P2 is, it is TCP-data-plane-specific, source-initiated, and
small-file-heavy (10k×4 KiB). **But it is a constraint, not a proof of
innocence** (review round 3): an aggregate gRPC *improvement* cannot
exclude a shared regression on both carriers that a larger
gRPC-specific gain simply masks. Shared controller/planner/sink code is
therefore NOT exonerated by the gRPC numbers, and pf-1 must attribute
the TCP gap to a named delta rather than infer "TCP-only ⇒ not shared".

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
  `InitiatorReceivePlaneRun.add_dialed_stream`). Suspect: per-epoch
  accept/dial round-trips or serialization in the accept branch that the
  dial branch does not pay.
  **H1's fixture rationale is FALSIFIED (review round 4)**: the claim
  was "mixed exercises resize hardest", but **all three fixtures target
  eight streams before clamping** (`src/dial.rs:474`) — so resize
  *count* cannot explain mixed-only behaviour, and H1 must name what
  about mixed differs (shard-boundary timing? the tar-shard small half
  interleaving with the big-file stream at the moment epochs fire?) or
  be killed. **H1 also names the wrong half without proof**: it accuses
  `Accept` while the destination's **synchronous dial-before-ACK** path
  (`transfer_session/mod.rs:3113`) is an equally good suspect. pf-1 must
  separate them with the dial/accept inversion counterfactual below —
  "consistent with H1" is not confirmation.
- **H2 (P1) — CONTRADICTED by code (review 2026-07-12)**: the claimed
  interleave cannot happen — resize begins only after
  `ManifestComplete` (`transfer_session/mod.rs` resize gate), and both
  layouts drain the same fixed 128-entry destination need loop, so
  batch emission cannot interleave with the resize controller during
  manifest/need emission in either layout. Kept only as a residual: if
  pf-1 timing shows a layout-dependent need-batch delta anyway, the
  mechanism must be re-derived from the trace, not from this text.
- **H3 (P2) — RETIRED as a code hypothesis (review round 3)**. Round 2
  already killed its named candidates (the small half is tar-sharded and
  written with parallel per-file `create_dir_all`/`fs::write`, NO
  per-file flush; per-file progress emission to the served push
  destination is disabled — `remote/transfer/sink.rs`; and old push used
  the same served sink, so fsync/flush policy and progress emission are
  NOT old/new deltas). What was left — "dest-side directory work/handle
  churn" — **names no old/new code delta at all**, and its only probe
  (precreate-vs-not) is explicitly environmental and cannot attribute
  code (Method 3(a)). A hypothesis that cannot be confirmed *or* killed
  by pf-1 is not a hypothesis; keeping it would let pf-1 close with a
  shrug. It is therefore retired, and its one code-attributable
  descendant — a per-member cost on the TCP receive path that old push
  did not pay — lives on as **H6**, which names an executed-path delta.
  H3 may only be revived if the pf-1 trace names a concrete old/new
  delta in the destination directory/handle path; the 12b cross-block
  precreated-container lead (8%, NTFS) is recorded as an environmental
  lead for that trace, not as an attribution.
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
  the exact signature "TCP regressed while gRPC did not" (zoey gRPC at
  parity 1.001, Windows gRPC faster; NOT "gRPC uniformly at parity" —
  review round 3). NOTE: an H5 fix
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
  locking timings (Method 3(e), now unconditional).
  **Historical control — corrected (review round 3): test the EXECUTED
  path, not source presence.** `NeedListSink` *exists* in the tree at
  `0f922de`, so "does the symbol exist there" is the wrong question and
  would wrongly force H6 into a "multiplied claim frequency" story. What
  matters is what old push actually RAN: at `0f922de` the served push
  data plane goes `socket → StallGuard → execute_receive_pipeline →
  FsTransferSink → disk`
  (`crates/blit-daemon/src/service/push/data_plane.rs:185-206`) —
  it **bypasses `NeedListSink` entirely** and takes no per-member claim.
  So H6's claim is precise and falsifiable: the unified TCP receive path
  introduced a per-member lock/hash-set claim on a path whose old
  counterpart took none. pf-1 confirms it by (a) reading the executed
  old path (done — cited above) and (b) the per-member locking timings;
  it is KILLED if those timings do not scale with member count or do not
  account for a material share of the P2 gap. If H6 is confirmed, the P2
  fix bar applies unchanged (≤ 1.10 against BOTH references, BOTH rigs);
  no separate bar is granted.

- **H7 (P2; added by review round 4 — the SHARED-controller candidate
  the gRPC caveat predicted)**: HEAD's need/manifest bookkeeping is
  heavier than old push's per entry. The unified source keeps a
  **mutex-protected sent-manifest map** with per-entry insertion and
  removal, and routes each need through a **per-need event-channel hop**
  (`transfer_session/mod.rs:1038`, `:1123`, `:1350`); old push used a
  **task-local map and handled need batches inline**, with no lock and no
  channel hop per entry. This is **per-entry**, so it scales with FILE
  COUNT — exactly P2's 10k×4 KiB signature — and, critically, it is
  **shared by BOTH carriers**. That is the precise class the round-3
  gRPC caveat warned about: a shared regression can hide under gRPC's
  larger carrier-specific gain, so "TCP-only symptom" does NOT exonerate
  shared code. No prior hypothesis tested it. Discriminated by: per-entry
  bookkeeping timings scaled against file count, plus the wall-time
  counterfactual (a task-local/batch-inline path behind a debug flag).
  H7 and H6 are independent and may BOTH contribute.

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
   destination event was not usable). The overlap DIFFERENCE is
   established only if `t_first_socket_write − t_manifest_complete` is
   ≈0-or-positive on the new build and provably NEGATIVE on the pinned
   `0f922de` control for the SAME fixture — i.e. old push really did put
   TCP bytes on the wire before its manifest completed, and the new
   session does not.
   **That timestamp proves ORDERING, not CAUSATION, so it cannot confirm
   H5 (review round 3).** H5 is confirmed only by a causal
   counterfactual: a debug-flag toggle that restores mid-manifest TCP
   payload queueing (queueing/ordering only — if it cannot be done
   without a wire change, this plan's Contract stop-and-amend rule fires
   FIRST) and measures WALL TIME on the same fixture and rig,
   interleaved old-vs-new. Pre-registered: H5 is CONFIRMED iff the
   toggle closes ≥ half of the new-vs-old-same-session P2 delta, and
   KILLED if it restores the old ordering but does not move wall time —
   which would prove the lost overlap is real and irrelevant, and hand
   P2 to H6;
   (e) per-member locking/framing timings are now an unconditional pf-1
   measurement (they discriminate H6), not contingent on the trace
   implicating them.
4. **Rig fallback applies to P2 as well as P1 (review round 3).** The
   local rig is Mac↔Mac loopback: it removes the very platform terms P1
   is confounded with, and it may equally fail to surface P2 (whose
   Windows arms are the sharpest). So the rule is symmetric — **if a
   finding does not reproduce locally, pf-1 REQUIRES the rig-side
   instrumented run** (netwatch-01 for P1; netwatch-01 AND zoey for P2,
   since P2 was measured on both) with the same spans and the CELLS
   fixtures, before pf-1 may close. Every hypothesis exits pf-1
   confirmed or killed — never "did not reproduce, moving on".
5. Every experiment lands as a committed probe record under
   `docs/bench/otp12-perf-<date>/` (timings + the flag matrix), codex
   loop per slice as usual.

## pf-1 decision rule — UNIFORM, pre-registered (added round 5)

Round-4 review: individual hypotheses had no shared decision threshold —
H1 accepted any positive phase delta, H4's cadence replay had no
threshold, H5 left a 1–49% recovery undecided, H6 left "material share"
undefined. A phase-timing delta is **descriptive**; only wall time
decides. So ONE rule governs every hypothesis (H1, H4, H5, H6, H7):

- Each hypothesis must have a **wall-time counterfactual**: a debug-flag
  variant that removes or restores exactly the accused mechanism, run
  interleaved against the unmodified build on the same rig and fixture.
  A hypothesis with no counterfactual **cannot be confirmed** — it is
  carried as UNTESTED and pf-1 does not close.
- Let `Δ` = the measured new-vs-old-same-session gap for that finding.
  The counterfactual's wall-time recovery `r` (as a share of `Δ`) is
  graded on a **pre-registered scale**, no post-hoc bands:
  - `r ≥ 50%` → **CONFIRMED DOMINANT** (fix it first)
  - `20% ≤ r < 50%` → **CONFIRMED CONTRIBUTING** (fix it, but it is not
    the whole story — keep hunting)
  - `r < 20%` → **KILLED** as a material cause (recorded, not pursued)
- **pf-1 closes only when the confirmed contributions account for ≥ 70%
  of `Δ`** for each finding. If they do not, the residue is unexplained
  and pf-1 **stays open** with the shortfall stated in the probe record —
  never "several hypotheses were consistent, moving on".
- Every measurement runs instrumentation-on/off pairs (per-member tracing
  across ~10k files can itself perturb a double-digit share of `Δ`).

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
  gRPC small-push cells unregressed. **"Unregressed" is given a
  reference and a tolerance (review round 3)**: each gRPC small-push
  cell must stay ≤ 1.10 against both of its own references AND must not
  worsen by more than **10% against its own pre-fix median on the same
  rig** (zoey 4731 ms; netwatch-01 2264 ms at 12c-win). The second
  clause exists because those cells currently range 0.801–1.001 — a fix
  that dragged Windows gRPC from 0.85 back to 1.05 would still pass a
  bare ≤1.10 bar while having eaten a real, measured win.
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
  gRPC controls) on **all THREE rigs: Z (zoey), W (netwatch-01) and
  D (delegated, netwatch-01↔skippy)**. **No mixed-build evidence: every
  NEW/UNIFIED arm cited for acceptance comes from the final fix build**
  (corrected, review round 2 — "every row" was impossible: the
  same-session `old` arms and the committed baselines are OLD builds by
  construction, which is the entire point of a reference). Pre-fix
  new-arm rows are void for acceptance — including otp-12a/12b/12c's,
  which are **replication and control evidence, not acceptance
  evidence**.
  **Rig D is included even though it is not a suspect (review round
  3).** Voiding otp-12c's pre-fix rows while re-running only Z and W
  would leave the parent plan's **delegated-parity bar**
  (`OTP12_ACCEPTANCE_RUN.md` D2, a hard bar) with *no* final-build
  evidence at all. "Not implicated" scopes what pf-1 must
  *instrument* — it does not waive an acceptance bar. Rig D's TCP
  verdict cells (+ the gRPC smoke) therefore rerun on the final build;
  both arms are new-build by construction there (rig D has no old
  baseline), so the whole cell is re-measured.
  **Every gRPC row the acceptance method requires reruns
  UNCONDITIONALLY on the final build** (corrected, review round 4 — the
  earlier "if shared code changed, the gRPC cells rerun too" left the
  decision to the author's own judgement of what counts as shared, which
  is exactly the loophole H7 exploits: a shared regression can hide under
  a gRPC-specific gain). `OTP12_ACCEPTANCE_RUN.md` D2 requires the
  complete Z/W gRPC converge and invariance rows, so those are
  final-build rows, full stop — no conditional. Results land in fresh
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
- **Rig-D delegated parity is not a SUSPECT, but it is still an
  ACCEPTANCE bar** (2026-07-13; scoped correctly at review round 3): the
  delegated-vs-direct matrix passed 7/7
  (`docs/bench/otp12c-delegated-2026-07-13/`), so delegation adds no
  measurable cost and pf-1 need not instrument the delegated trigger
  path. That is a statement about *where to look for the bug* — it does
  **not** waive the parent plan's delegated-parity bar, whose evidence
  is pre-fix and therefore void under pf-final. Rig D reruns on the
  final build (see pf-final).
