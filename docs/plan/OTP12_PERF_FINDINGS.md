# otp-12 perf findings — investigate + fix before acceptance (design)

**Status**: Active
**Approved**: D-2026-07-13-1 — owner, 2026-07-13, verbatim:
**"one more round with codex on the plan then just write the code and
reviewloop slice by slice. that converges faster than plans with no
ground truth to test."** The final round ran (round 5, verdict NOT READY,
3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
counterfactual, F3 the inexecutable decision rule); all three are fixed
in this revision, and implementation now proceeds **slice by slice, each
through the codex loop** (D-2026-07-04-1 unchanged). A non-converged plan
verdict is no longer a gate — the plan's earlier "flip to Active at codex
convergence" rule is superseded by D-2026-07-13-1, because rounds 2–5
were increasingly finding defects in the *prose* while the plan's central
factual claim was settled by *measurement* (the same-OS rig refuted a
claim four review rounds had left standing). pf-1 exists to generate
ground truth; it starts now.

**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
assumed by this plan):** P1 has **no escape hatch on the books**.
D-2026-07-12-1 waives a cross-direction converge-up miss only for a cell
that is *already* invariance-passing; P1 is the invariance failure
itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
plan pursues) or the owner must **amend acceptance criterion 1** in a new
decision. pf-1 proceeds either way: it produces the evidence that
decision would rest on.
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

### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)

**Evidence: `docs/bench/otp12-perf-2026-07-13/` — magneto↔skippy, Linux on
BOTH ends, real 10 GbE, full otp-12 methodology** (cold caches both ends,
destination drained, ABBA, pair-void, RUNS=4; 64 runs, 8/8 cells, zero
voided). Harness `scripts/bench_otp12pf_linux.sh`.

**P1 does NOT reproduce.** Its own cell passes with room to spare:

| cell | srcinit | destinit | ratio | outcome |
|---|---|---|---|---|
| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |

**8/8 invariance cells PASS** (`ms_grpc_mixed` via its pre-registered
RUNS=8 escalation → 1.063). There is no destination-initiator penalty at
all when both ends are Linux.

Therefore:

- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
  property of blit's code — a pure layout cost would have appeared here,
  on the same code, same carrier, same fixture.

- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
  earlier revision of this section said D-2026-07-12-1 lets the owner
  accept P1 as a platform residue. **It does not.** That decision excuses
  a **cross-direction converge-up** miss for a cell that has ALREADY
  satisfied its precondition **"(b) is initiator/verb-invariant within
  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
  failure** (`wm_tcp_mixed` 1.300 FAIL) — the precondition it would need
  is the very thing it violates. No decision on the books waives it.
  Therefore exactly two exits exist, and pf-1 must aim at them:
  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
     bar (`ONE_TRANSFER_PATH.md` acceptance criterion 1 is mandatory).
  2. **A NEW OWNER DECISION amending criterion 1** — for which the
     same-OS result is the honest evidence base: criterion 1 asks for
     invariance "on a symmetric rig", Mac↔Windows was designated only
     because no better pair existed, and one now does — magneto↔skippy,
     where blit measures **8/8 invariant**. An owner could reasonably
     rule that criterion 1 is judged on the rig that isolates blit's own
     behaviour, with the Mac↔Windows delta recorded as platform residue.
     **That ruling does not exist. It must not be assumed, and this plan
     must not be written as though it will be granted.**
- **This does NOT fully exonerate the code.** It rules out a pure layout
  property; it does not rule out a code path whose cost only becomes
  material under a particular platform — e.g. a slow accept branch on the
  Windows side, which is exactly what H1 accuses. H1/H5/H6 stay LIVE but
  are now **narrowed to platform-interacting mechanisms**, and only the
  dial/accept inversion counterfactual on rig W can finish the job.
- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
  and no `0f922de` build is staged on these hosts). Nothing here speaks
  to it.

> **⚠ A RETRACTED CLAIM LIVED HERE.** An earlier revision of this section
> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
> toward CODE → the fix is mandatory and cannot be waived" — and STATE and
> the acceptance plan were amended to match. That was **WRONG**. It rested
> on a scratch probe (and a first harness revision) that ran the durability
> `sync` inside the INITIATING host's timed bracket: in the push arm the
> initiator is the SOURCE, which only read, so its sync was a no-op and the
> destination's writeback was never paid; in the pull arm the initiator IS
> the destination, so it paid the full writeback. One arm was charged for
> durability the other got free — multi-second on skippy's ZFS — which
> manufactured "failures" on every carrier and fixture, **including the
> gRPC control that is supposed to be clean**. That carrier-independence is
> what exposed it: a real code effect is carrier-specific; an accounting
> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
> never by verb — the otp-2w rule, re-learned). The retraction is recorded
> rather than quietly overwritten because the wrong number was reported to
> the owner and briefly drove this plan.

### The residual confound (WHICH code) still needs a counterfactual

Breaking platform-vs-code does NOT tell us *which* layout property costs
the time. On any two-host rig, host identity remains welded to role, so
"the accepting end" cannot be separated from "that host" by more runs:

- **pf-1 must compare all four rig-W arms** (both cells × both
  initiators), not two, and report the interaction — not a single ratio.
- **The disambiguator is a dial/accept inversion counterfactual, not a
  rig** — but it is **NOT sufficient on its own** (review round 5): the
  inversion swaps the source's `Accept`, the destination's `Dial`, AND
  the epoch-0 topology **simultaneously**, so a positive result implicates
  *the topology pair*, not H1 specifically. It cannot distinguish
  source-accept serialization from synchronous destination dialing
  (`transfer_session/mod.rs:3113`), nor prove the resize-specific claim.
  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
  1. **dial/accept inversion** — same direction, same hosts, same fixture;
     only who dials changes. Implicates the topology pair (or exonerates it).
  2. **no-resize / pre-opened streams** — force the final stream count at
     epoch 0 so no resize epoch ever fires. If the gap survives with zero
     resizes, H1's resize-specific mechanism is **KILLED** regardless of
     what (1) shows (and note `dial.rs:474`: all three fixtures already
     target 8 streams, so resize *count* was never the discriminator).
  3. **per-side ordering** — hold the topology fixed and vary only whether
     the destination's dial-before-ACK is synchronous. Separates the two
     halves the inversion conflates.
  H1 is CONFIRMED only if the wall-time recovery tracks the **accept role**
  across (1) AND survives (2); it is KILLED if the gap persists with no
  resizes, or if (3) shows the cost is the synchronous dial rather than the
  accept branch. Any of these that changes connection topology — (1) and
  (2) do — **trips this plan's Contract stop-and-amend rule**
  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
  written). Same-build-both-ends (D-2026-07-05-2) means no compatibility
  surface is created.
  **H1 is also WEAKENED by the Linux null** (it predicts a layout cost that
  did not appear on a real-network same-OS pair), so pf-1 must be prepared
  to kill it and fall through to H5/H6/H7.
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

## pf-0 — the environmental control (MTU): **KILLED as a material cause of P1** (recorded 2026-07-14)

Executed as pre-registered
(`docs/bench/otp12-jumbo-win-2026-07-13/PREREGISTRATION.md`); evidence + full
adjudication in that directory's `README.md`. **The decision rule, thresholds
and guards were registered in rev 3, before any of the S1–S4 data existed, and
were unchanged by rev 4** (rev 4 re-described the *rig* after the `q` baseline —
so "written before the data" is true of the rule, not of the whole document, and
no threshold was authored around these numbers). Counterbalanced **A-B-B-A**
(9000, 1500, 1500, 9000) on rig W with the `q` Mac end, `RUNS=8`, **256 timed
runs, 0 voided**, MSS gate held at the start AND end of every session (8948
jumbo / 1448 at 1500).

    Δ_9000 = 236 ms    Δ_1500 = 229 ms    N_Δ (measured noise floor) = 78 ms
    r = (Δ_1500 − Δ_9000) / Δ_1500 = −3.1%   →   KILLED (r < 20%, the scale below)

**What this licenses — exactly the registered outcome, and no more.** Raising
the MTU **did not improve these cells under the observed packetization**: the
point estimate of the MTU contribution to P1 is ~0. The null is **not vacuous**
— the manipulation demonstrably reached the wire (`wm_tcp_large` ran **3–4%
faster at jumbo on both arms**, and both `wm_tcp_mixed` arms sped up slightly) —
and the benefit is **symmetric**, which is why it cannot explain an
**asymmetry**. P1 FAILED in all four sessions (1.237–1.362) regardless of MTU;
all controls passed in all four.

**What it does NOT license (do not restate this result as more than it is).**
- **The wire is not exonerated, and "P1 is code-shaped" is NOT established
  here.** MTU is *one* environmental variable. Segment **fill** is unmeasured
  (8948 is the MSS *ceiling*), so underfilled segments, a bottleneck elsewhere,
  or a smaller wire contribution are all still live. This result kills **MTU**,
  not "the environment".
- **It is not powered to exclude a CONTRIBUTING-size MTU effect.** The
  CONFIRMED-CONTRIBUTING threshold is 20% of Δ_P1 ≈ **46 ms**, which is
  **below the rig's measured between-session noise floor of 78 ms**. So the
  experiment can exclude a **DOMINANT** effect (50% ≈ 114 ms, comfortably above
  the floor) but **cannot exclude a contributing-size one** — a 46 ms effect
  could be swamped. The registered rule returns KILLED on the point estimate,
  and that grade stands as registered; the *resolution limit* is stated here so
  the grade is never read as a stronger exclusion than the data supports.
- It confirms no hypothesis. pf-1 still owns attribution.

**`Δ_P1(rig W)` is re-estimated, and the noise floor constrains how pf-1 may
grade.** The `282 ms` above is a **single nagatha session**; four sessions on
the `q` pairing give **Δ_P1 ≈ 230 ms** (229 at 1500, 236 at 9000).

- **Between-session grading of a counterfactual is now definitively ruled out**
  on this rig: a 46 ms (20%) recovery is smaller than the 78 ms between-session
  floor, so an unpaired before/after across sessions cannot separate
  CONTRIBUTING from KILLED.
- **This does NOT prove the interleaved design has enough resolution** — that is
  a different (paired, within-session) variance, and pf-0 did not measure it.
  **pf-1 must measure its own paired within-session noise floor on the
  unmodified build and register a resolution check** (its smallest reportable
  recovery must exceed that floor) *before* grading any hypothesis. A pf-1
  recovery quoted without its paired floor is uninterpretable.
- **The noise is not diffuse — it is a bistable fast arm.** The `win_init` runs
  are **bimodal** (roughly ~730 ms and ~840 ms clusters); S1 drew 6 low/2 high
  and S4 drew 2 low/6 high **at the same MTU**, and that mixture — not MTU — is
  what produced the 72 ms `win_init` replicate spread and hence N_Δ. The
  `mac_init` arm is by contrast stable to **5–6 ms**. **Trap for pf-1: a
  counterfactual that merely shifts the mode mixture would masquerade as a
  recovery.** Grade on the run distribution, not the median alone. (The MTU
  verdict is robust to this: pooling all 16 runs per condition gives
  Δ_9000 = 232, Δ_1500 = 221.5, r = −4.7% — same KILLED grade.)

**RESOLVED — the committed baselines are RE-RECORDED at MTU 9000
(D-2026-07-14-1, owner, 2026-07-14).** The exposure pf-0 surfaced: the fabric now
runs MTU 9000 while the committed anti-drift ceilings were recorded at **MTU
1500**, and pf-0 measured jumbo making **both arms 3–4% faster** — so grading a
jumbo NEW arm against a 1500-recorded ceiling is **LENIENT, not conservative**:
the MTU gain flatters the ratio and a real regression could pass unseen.

The owner's resolution is to **re-record each rig's committed baseline with its
ORIGINAL OLD build at MTU 9000**, then re-freeze it. The freeze principle is
unchanged (a baseline is immutable once recorded; no run may re-point its own
ceiling) — only the *pin* moves, once. The 2026-07-10 baselines are retained as
historical MTU-1500 records.

**This is a prerequisite slice for `pf-final`, and it affects BOTH rigs** (each
harness hardcodes its own reference, and both predate the fabric-wide jumbo
raise): rig W `bench_otp12_win.sh:105` → `otp2w-baseline-2026-07-10/`; rig Z
`bench_otp12_zoey.sh:102` → `otp2-baseline-2026-07-10/`. Rig D has no old
baseline and is unaffected. Constraints (same old build per rig,
manifest-verified; `BASELINE_SUMMARY` stays override-free and is re-pointed by a
reviewed source edit; the pf-0 start-AND-end MSS gate applies, since a baseline
recorded at an unverified MTU is the very defect being fixed) are in
D-2026-07-14-1 and are not restated here.

Same-session references (`old_session`) are MTU-matched by construction and were
never at risk.

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
  **H6's WALL-TIME counterfactual (added round 5 — timings alone would
  strand pf-1 under the uniform decision rule):** behind a debug flag,
  claim the whole tar shard under ONE lock on the TCP receive path —
  i.e. give TCP the same batch-claim shape the gRPC path already uses
  (`transfer_session/mod.rs:3047`), rather than a per-member claim
  (`data_plane.rs:1167`). This is safe and wire-neutral (it changes only
  the granularity of a local mutex/hash-set claim, not any frame), so it
  does NOT trip the Contract rule. Grade its recovery against `Δ_P2` on
  the uniform scale. If per-member claiming is the cost, batch-claiming
  recovers it; if not, H6 dies with a number rather than a shrug.

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
- **`Δ` is defined per finding and per rig — it is NOT one number**
  (review round 5: the earlier text left it ambiguous between P1's
  layout gap and P2's old/new gap, which are different quantities):
  - **`Δ_P1(rig)`** = `destinit_median − srcinit_median` for
    `wm_tcp_mixed` on THAT rig (an invariance gap: new-vs-new, no old
    build involved). On rig W it is 1221 − 939 = **282 ms** — a **single
    nagatha session**; §pf-0 re-estimates it from four sessions on the `q`
    pairing, rules out **between-session** grading of any counterfactual, and
    requires pf-1 to measure its own **paired within-session** floor before
    grading. Read §pf-0 before grading any recovery against `Δ_P1`. On
    magneto↔skippy it is ~0 (8/8 pass) — so
    **P1 counterfactuals are graded on rig W only**; a Linux-rig recovery is
    meaningless against a gap that does not exist there.
  - **`Δ_P2(rig)`** = `new_median − old_same_session_median` for
    `push_tcp_small` on THAT rig (a converge gap, requires the `0f922de`
    build on that rig). netwatch-01: 1975 − 1644 = **331 ms**; zoey:
    4033 − 3636 = **397 ms**.
  Every reported recovery names its `Δ` and its rig. A counterfactual run
  on a rig whose `Δ` is ~0 proves nothing and is not reported as a kill.
- **Overlapping causes are attributed SEQUENTIALLY, never summed**
  (review round 5: H4/H7, and H6/H7, can each recover the same
  milliseconds, so independent recoveries would double-count and could
  "explain" >100% of `Δ`). Procedure: grade each hypothesis's recovery
  ALONE against the unmodified build; then, for every confirmed
  hypothesis in descending order of solo recovery, measure the
  **incremental** recovery of adding it to the already-applied set. The
  ≥70% closure test below is evaluated on the **cumulative combined**
  build, not on the sum of solo recoveries.
- The counterfactual's wall-time recovery `r` (as a share of the named
  `Δ`) is graded on a **pre-registered scale**, no post-hoc bands:
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
