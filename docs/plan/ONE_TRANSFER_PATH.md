# One transfer path — direction-invariant by construction

**Status**: Draft
**Created**: 2026-07-05
**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
(D-2026-07-05-1)
**Decision ref**: D-2026-07-05-1 (directive + pause); Active flip gets
its own entry

## Directive (owner, 2026-07-05, verbatim)

> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
> ANYTHING EVER using anything else because anything else does not
> exist."

> "just make it so that I NEVER see a situation where pull is faster
> than push or vice versa. that CAN NEVER be possible because of
> something blit did. it should be identical if I start the transfer
> from skippy and push to this machine or if I start the transfer on
> this machine and pull from skippy."

> On benchmark methodology: "tmp on one side, spinning rust on the
> other is not a valid test."

Scope, wire, and process were explicitly delegated to the agent
("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
owner's requirement is the invariant; everything below is the
architecture that makes the invariant impossible to violate rather
than merely maintained by discipline.

## Goal

One `TransferSession` implementation owns every byte transfer blit
performs. A transfer has a SOURCE role and a DESTINATION role; which
end initiated, and which CLI verb was used, select roles — they do not
select code. When this plan ships, the per-direction drivers (client
push driver, daemon push-receive, client pull driver, daemon
pull-send, delegated-pull driver, local orchestration) **do not
exist**: for fixed endpoints and dataset, direction/initiator/verb
cannot affect behavior or wall time by blit's doing, because there is
no second code path to differ.

## Non-goals

- Preserving wire compatibility with pre-plan builds. The `Push` and
  `PullSync` RPCs are deleted at cutover; both ends upgrade in
  lockstep (repo precedent: the `PullSyncHeader` removal; the owner
  operates every deployed peer).
- Making different hardware perform identically. If src and dst sit
  on different disks, the two *data directions* still differ by
  physics; the invariant is that the same data direction between the
  same endpoints is identical regardless of who initiates and which
  verb is used.
- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
- New features. This is a consolidation; capability parity with
  today (mirror, filters, resume, fallback, delegation, progress,
  jobs, cancellation) is the bar.

## Constraints

- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
  violated at the choreography layer.
- **Converge up, not down**: per benchmark cell, the unified session
  must match the better of today's two directions (within ±10% run
  noise), not their average. Unification that slows the fast
  direction fails review.
- REV4 invariants carry: byte-identical results, StallGuard,
  cancellation, byte-accounting. Existing pins are ported (not
  dropped) as tests become role-parameterized; test count never
  drops.
- The sf-2 shape-correction behavior (stream count corrects as the
  need list accumulates) becomes the one and only stream policy —
  both directions inherit it by construction; its pins carry over.
- Wire contract discipline (REV4 rule): the unified session's proto —
  messages, field numbers, capability negotiation, transport
  selection — is a reviewed doc+proto slice **before** any behavior
  depends on it.
- Every slice through the codex loop (D-2026-07-04-1); tree green
  after every slice; transitional coexistence of old+new paths is
  scaffolding only — the plan is not Shipped until the deletion slice
  lands and the deletion proof is recorded.
- Windows parity: suite green on the owner's machine + windows-latest
  CI before Shipped.

## Acceptance criteria

- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
      on a symmetric rig (same filesystem class both ends, cold
      caches, disk-to-disk), for each data direction and workload
      (large / 10k-small / mixed): wall time initiating from end A vs
      end B, and via push-verb vs pull-verb, differs only within
      run-to-run noise (±10%). Matrix committed as evidence.
- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
      (driver), daemon `push/control.rs` choreography, daemon
      `pull_sync.rs` choreography, the delegated-pull driver, the
      separate local orchestration path, and the `Push`/`PullSync`
      RPCs no longer exist in the tree; one `TransferSession` and one
      `Transfer` RPC remain. Recorded file-by-file in the final
      slice's finding doc.
- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
      guard), filters, block-resume, gRPC fallback carrier, delegated
      transfer, progress events, jobs/cancel, read-only enforcement —
      each demonstrated by ported tests on the session.
- [ ] Suite green throughout; final test count ≥ pre-plan baseline
      (1483); all REV4 invariant pins and the sf-2 pin pass
      role-parameterized.
- [ ] Benchmark methodology corrected and recorded: symmetric-fs
      cells are the verdict cells; tmpfs cells remain only as
      explicitly-labeled wire-reference rows (never compared across
      directions with asymmetric endpoints).
- [ ] Windows: full suite green (owner machine) + windows-latest CI.

## Design

**What already is one code** (kept, becomes the session's engine):
`remote/transfer/` — pipeline, sink/source abstractions, data plane,
diff planner, tar-shard, stall guard, progress, `operation_spec` (the
REV4 unified contract), and the engine dial (stream policy incl. sf-2
shape correction). The defect layer is above it: four driver loops
choreograph these pieces differently per direction.

**The one choreography** (roles, not directions):

1. Initiator opens the single bidi `Transfer` RPC and sends the
   operation spec: which end is SOURCE, which is DESTINATION, path/
   module, filters, mirror/resume flags, capabilities.
2. SOURCE enumerates and **streams** its manifest immediately (no
   buffered-enumeration phase — this generalizes push's fast start;
   pull's full-enumeration-then-negotiate slow start is deleted, which
   absorbs the "pull 1s-start" residue item).
3. DESTINATION diffs incrementally against its own filesystem and
   returns need-list batches (one diff owner, always the end that
   owns the target fs — push's proven model; pull_sync's
   source-side diff is deleted).
4. The data plane opens at the dial floor immediately; stream count
   shape-corrects as the need list accumulates (sf-2 mechanism, now
   the only policy, both roles).
5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
   the one pipeline into the data plane; DESTINATION writes through
   the one receive path.
6. Mirror: DESTINATION computes deletions from the completed source
   manifest it received (filter-scoped, scan-complete-guarded) and
   executes them locally. One rule, no per-direction delete
   choreography.
7. Resume: optional block-hash phase inside the same session, same
   messages regardless of roles.
8. Summary/byte-accounting: one record shape.

**Transport facts vs choreography**: the connection-initiating end
dials TCP data-plane sockets (NAT reality) — byte direction within a
socket is set by role, not by who dialed. The gRPC-fallback lane
becomes a *byte-carrier option* inside the same session (control-
stream frames instead of TCP sockets), selected at negotiation — not
a separate transfer path. Resize keeps its controller-at-sender rule.

**Delegated transfer**: a daemon receiving a delegated request simply
becomes an initiator of the same session against the other daemon
(destination role on its module fs). The bespoke delegated-pull
driver is deleted; the delegation *gate* (authorization) stays.

**Local transfers**: the same session driver over an in-process
transport (both roles in one process, no wire). The engine underneath
is already shared; the separate local orchestration path is deleted
in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
guard the migration.

**Affected crates**: `blit-core` (new `transfer_session` module;
`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
`blit-tui` (progress/jobs consume the same events).

**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
mitigated by strangler slices with the tree green throughout and a
non-optional deletion slice. Per-cell regression risk on today's
faster direction — mitigated by the converge-up constraint and
baseline parity pins per slice. Wire break — lockstep upgrade,
owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
Progress/jobs/TUI integration churn — the session emits the existing
event contract (w6-1) at the same boundaries.

## Slices

One coherent, testable change per slice — sized for the `.review/`
loop. Tree green after every slice; old paths keep working until
otp-9 deletes them.

1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   `Transfer` RPC and message set — roles, phases, field numbers,
   capability negotiation, transport selection, resume/mirror
   phases, error/cancel semantics. Full REV4 wire-contract
   deliverable set; codex-reviewed before any code consumes it.
2. **otp-2 TransferSession core (blit-core)**: role-parameterized
   state machine over the existing engine with an in-process
   transport; unit/e2e tests run BOTH role assignments over the same
   fixtures — the invariance property enters the test suite here.
3. **otp-3 daemon serves `Transfer`, client initiates as SOURCE**
   (remote push-equivalent rides the session); A/B parity pins vs
   old push (byte-identical trees, summary parity, sf-2 pin ported).
4. **otp-4 roles swapped: client initiates as DESTINATION** (pull-
   equivalent) — the same code with roles flipped; the parity suite
   reruns with no per-direction test code.
5. **otp-5 mirror + filters** on the session (one delete rule).
6. **otp-6 resume** block phase.
7. **otp-7 fallback byte-carrier** (control-stream frames) as the
   session's alternate transport.
8. **otp-8 delegated transfer** = daemon-initiated session; bespoke
   delegated-pull driver retired behind the existing gate.
9. **otp-9 cutover + deletion**: CLI/app/TUI route every remote
   operation through the session; `Push`/`PullSync` and all four
   drivers deleted from the tree and the proto; ported-test
   accounting proves count never dropped. Deletion proof recorded.
10. **otp-10 local transfers** ride the in-process transport; the
    separate local orchestration is deleted; local perf pins hold.
11. **otp-11 symmetric-rig acceptance run**: sf-1 harness matrix
    corrected (same-fs disk-to-disk verdict cells, cold caches,
    tmpfs as labeled wire-reference only) + the initiator/verb
    invariance A/B matrix; committed as this plan's acceptance
    evidence.
12. **otp-12 verdict**: acceptance checklist walked with the owner;
    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
    against the unified baseline — owner call at that point.

## Open questions

- None requiring owner input now — scope, wire, and process were
  delegated (Directive section). Slice-level unknowns (exact proto
  shapes, resume edge semantics, TUI event wiring) are settled inside
  their slices through the codex loop. — owner
