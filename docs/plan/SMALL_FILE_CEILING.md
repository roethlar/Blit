# Small-file transfer to the hardware ceiling

**Status**: Draft
**Created**: 2026-07-05
**Supersedes**: nothing
**Decision ref**: (pending owner approval)

## Principle (owner, 2026-07-05)

blit's guiding principles are **FAST, SIMPLE, RELIABLE** — every
change serves at least one or it's scrapped. blit must be the
fastest way to transfer files in **any** scenario. Goals are
therefore **ceiling-driven, never competitor-relative**: a
"beat tool X by N%" bar embeds a stopping condition and is the wrong
way to engineer this tool. Other tools function only as
**tripwires** — any scenario where any tool measures faster than
blit is, by definition, proof blit is off its hardware ceiling and
is a finding to fix, regardless of margins.

## Goal

For the workload classes where the 2026-07-04/05 10 GbE session
measured blit off its ceiling — many-tiny-file and mixed transfers —
blit's wall time becomes bounded by a **named hardware limit** (wire,
target-filesystem parallel create floor, source enumeration floor),
demonstrated by profile evidence and a stream-scaling curve, not by
blit's own stream policy or per-file overhead.

Measured gap analysis (evidence:
`logs/bench_10gbe_20260704T201804/tool_comparison.csv`, daemon logs,
DEVLOG 2026-07-05):

| cell | blit today | ceiling arithmetic | tripwire |
|---|---|---|---|
| push 10k×4 KiB | 2.4–3.3 s | wire: **34 ms** (40 MiB @ 9.9 Gbit); fs floor: ~150 µs/file proven single-pipe on this ZFS, ÷ parallelism → **~0.2–0.5 s** | rsyncd 1.5 s |
| pull 10k×4 KiB | 446–484 ms | client fs = tmpfs (µs creates); wire+protocol class: **≪ 200 ms** | rsyncd 367 ms |
| push mixed 512 MiB+5k | 1.8–2.2 s | big file alone: ~450 ms wire; small remainder as above | rsyncd 1.24 s |

Diagnosis (from the session's daemon logs): the 10k push rode **one
stream** — `engine::initial_stream_proposal` is byte-weighted, so
40 MiB proposes a single stream despite 10,000 files — and paid
~215 µs/file sequentially on the daemon. The parallel machinery
(elastic streams, work-stealing, mid-transfer resize) exists and
negotiated 8 connections for the 1 GiB push in the same session.
This is a policy gap plus per-file overhead, not missing machinery.

## Non-goals

- Competitor-relative targets of any kind (see Principle).
- WAN/latency-shaped tuning (separate scenario class; gets its own
  ceiling analysis when a rig exists).
- Non-Linux rig ceiling targets (no measurement hardware this plan
  can bind to; Windows/macOS must not regress — suite + CI guard).
- Encrypted-transport scenarios (ssh-wrapped tools measured only as
  tripwires; blit's transport security model is unchanged by this
  plan).

## Constraints

- Every slice serves FAST without violating SIMPLE (dial stays the
  single tuning owner; no second engine, no special-case paths that
  survive past their measured need) or RELIABLE (REV4 invariants:
  byte-identical, StallGuard, cancellation, byte accounting).
- No wire-visible protocol change without a dedicated owner gate on
  the wire design before code (sf-6); mixed-version peers keep
  working via existing negotiation.
- No measured cell regresses beyond run-to-run noise (±10%),
  guarded by the committed baseline.
- Test count never drops; every slice through the codex loop
  (D-2026-07-04-1).
- Small-file parallel writes must respect the receiver capacity
  profile (spinning-pool receivers bound their own parallelism —
  the existing bounded-unilateral dial contract, D-2026-06-20-1).

## Acceptance criteria

- [ ] For each cell above: a recorded **limiter analysis** (profile
      + stream-scaling curve, committed with the slice records)
      demonstrating wall time is bound by a named hardware limit,
      not by stream policy or blit-controlled per-file overhead.
- [ ] Scaling evidence: files/s rises with stream count until the
      named limiter binds — the curve flattens at hardware, not at
      policy.
- [ ] **Tripwires clean**: no tested tool (rsyncd, rsync-over-ssh,
      rclone in its best measured config, `cp -a` locally) measures
      faster than blit on any cell of the committed matrix.
- [ ] All currently-winning cells stay within noise of today's
      baseline.
- [ ] The comparison + scaling harness is committed and the owner
      can rerun it against any daemon host in one command.

## Design

Levers, cheapest first, measuring between each — sequencing exists
to find the ceiling with the least machinery, not to stop early:

1. **File-count-aware stream proposal** (blit-core `engine/`):
   `initial_stream_proposal` (and the pull-side equivalent) weight
   file count alongside bytes so many-tiny-file manifests open
   multiple streams; work-stealing spreads per-file cost across
   daemon workers. Push knows counts from enumeration, pull from
   the manifest.
2. **Per-file cost to the syscall floor** (daemon receive + client
   pull write paths): profile first (`strace -c`/`perf` during a
   small transfer), then cut — candidates: temp-file+rename
   pattern, separate set-times/set-perms syscalls, per-file
   need-list echo. The profile, not intuition, names the cuts.
3. **Resize-on-file-backlog**: feed the existing ue-2 resize
   machinery a backlog signal so a stream drowning in tiny files
   triggers mid-transfer ADD — this is also the organic resize
   trigger byte-bound workloads can never produce.
4. **Tar-shard push lane** (wire-visible, own owner gate): bundle
   tiny files into shard frames on the push wire as the local
   engine and delegated lane already do — amortizes both protocol
   roundtrips and daemon syscalls. Reached when the limiter
   analysis shows per-file framing itself is the binding cost.

Risks: parallel small-file writes can seek-storm spinning pools —
bounded by the receiver capacity profile (constraint above); lever 2
touches platform-sensitive syscall paths — Windows suite must stay
green; lever 4 adds wire complexity — SIMPLE requires the limiter
analysis to prove it earns its keep before design review.

## Slices

1. **sf-1 harness + baseline**: `RSYNC_COMPARE=1` tripwire section
   in `scripts/bench_10gbe.sh` (spins rsyncd on the daemon host,
   same fresh-target matrix) + a stream-scaling probe mode; commit
   the 2026-07-05 baseline CSVs under `docs/bench/`. No production
   code.
2. **sf-2 dial file-count weighting**: proposal-table unit pins
   (10k tiny → multi-stream; 1×1 GiB unchanged; mixed →
   intermediate) + loopback e2e pin that a 10k-file push opens >1
   data-plane connection.
3. **sf-3 per-file cost profile + trim**: profiling evidence in the
   finding doc, then the cheapest cuts; loopback per-file-cost
   proxy pin so CI catches gross regressions without the rig.
4. **sf-4 rig re-measure + limiter analysis**: rerun sf-1 harness on
   the 10 GbE rig; record the limiter analysis per cell. Hardware-
   bound everywhere + tripwires clean → acceptance review with the
   owner. Otherwise the analysis names what binds; continue.
5. **sf-5 resize-on-backlog feed** (if sf-4 names stream count
   under load as a binder, or the owner wants the ue-2 organic
   trigger regardless — flagged at sf-4).
6. **sf-6 tar-shard push lane** (if sf-4/sf-5's analysis names
   per-file wire framing as the binder): wire-compat design section
   to the owner **before any code**; then implement.
7. **sf-7 verdict**: final rig run, limiter analyses committed,
   acceptance checklist walked with the owner; plan → Shipped or
   the remaining gap gets its own named follow-on.

## Open questions

- **sf-6 wire gate** (standing): the tar-shard lane's wire design
  needs explicit owner sign-off at execution time — recorded here
  so no session treats sf-6 as pre-authorized code. — owner
