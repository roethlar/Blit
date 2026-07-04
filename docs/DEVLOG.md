# DEVLOG — append-only journal

**Status**: Active

Append-only history; newest first. ISO timestamp per entry. Never read this
to determine current state — that is `docs/STATE.md`'s job (AGENTS.md §1).
Write to it via the `handoff` procedure; prune STATE.md overflow here.

---

## 2026-07-04T05:45:00Z — D-2026-07-04-1: codex loop goes repo-wide; w4-1 graded and closed

Owner directive mid-session: **"use codex review loop for all code and
plan changes", "NO EXCEPTIONS"** — recorded as D-2026-07-04-1. The
`.review/README.md` async sentinel hand-off is retired as the grading
mechanism (its `findings/`/`results/` records and REVIEW.md stay live);
propagated through `GPT_REVIEW_LOOP.md` (scope + generalized steps),
`PROTOCOL.md` (`slice` rewritten, `plan` gains a codex-review step),
`.review/README.md` banner, `.agents/repo-guidance.md`, STATE.md
(`3ebcc37`, review fixes `10866e4`).

First two codex rounds under the new rule, both NEEDS FIXES, all
findings accepted, none rejected:

- **w4-1 (`65ecb93`)**: 1 Low — the relocated
  `drop_without_consume_aborts_running_task` was vacuous (150ms wait vs
  the task's 500ms completion; inherited from its pull.rs original,
  whose comment contradicted its own code). Fixed `bedfa52` with
  `start_paused` virtual time, mutation-verified (detaching `Drop` now
  fails it deterministically). w4-1 and design-2 flip to `[x]`; the
  pre-decision sentinel deleted.
- **Decision docs (`3ebcc37`)**: 4 stale-text spots the propagation
  missed (STATE process line + Now bullet, REVIEW.md legend, `plan`
  procedure bypass). Fixed `10866e4`.

Process correction, encoded in the loop doc: **codex is the only
reviewer** — a same-model (Claude) review panel started alongside codex
this session was stopped by the owner and its output discarded; Claude's
only grading role is adjudicating codex's findings against source.
Verdicts: `.review/results/{w4-1-abortondrop-family,d-2026-07-04-1-docs}.gpt-verdict.md`.

## 2026-07-04T05:00:00Z — w4-1 landed: AbortOnDrop hoisted, remaining detach-on-drop sites closed

Picked up the design-review queue at **w4-1** (`REVIEW.md`, ratified
D-2026-06-11-2) via the `.review/README.md` coder loop. Hoisted
`AbortOnDrop` (R32-F2/R34-F2's abort-not-detach RAII wrapper) out of
`blit-core/src/remote/pull.rs` (`pub(crate)`) into
`blit-core::remote::transfer::abort_on_drop` (`pub`, so `blit-daemon` can
use it too), then wrapped every remaining bare-`JoinHandle` site the
design map flagged:

- `blit-daemon/src/service/push/control.rs`'s `data_plane_handle`
  (design-2's one remaining site — its two `service/pull.rs` sites were
  already deleted with the legacy Pull RPC at `ue-r2-1h`).
- `blit-core`'s push client: `MultiStreamSender::pipeline_handle` and
  `push()`'s `response_task`.
- `blit-daemon/src/service/push/data_plane.rs`'s
  `accept_data_connection_stream`, converted from a bare
  `Vec<JoinHandle>` (first-error detached the survivors) to a `JoinSet`,
  mirroring the resizable sibling path's existing fix from `ue-r2-2`.

Each site got a regression test proving the abort actually fires (each
verified by reverting the fix and confirming the test then fails):
`multi_stream_sender_drop_tests` (blit-core), `data_plane_handle_abort_tests`
and `first_stream_error_aborts_sibling_worker` (blit-daemon, the latter a
real two-TCP-client end-to-end drive of the JoinSet conversion). Commits
`65ecb93` (fix) + `44bf416` (finding doc, REVIEW.md, sentinel). fmt/clippy
clean; `cargo test --workspace` green (blit-core 348, blit-daemon 162, up
from baseline by the new tests; no other crate's count changed). Closes
`design-2-orphaned-daemon-data-planes`'s remaining scope as a byproduct.
Sentinel written to `.review/ready/w4-1-abortondrop-family.json`; awaiting
reviewer verdict.

## 2026-07-03T00:00:00Z — `ue-r2-1g` landed: PullSync multistream through the engine

Backfilled from `docs/STATE.md`'s handoff log (pruned on the next rotation
past 3 entries; full detail lived only there until now). `ue-r2-1g`
(`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` slice) landed end-to-end:
PullSync multistream through the engine, absorbing the `MULTISTREAM_PULL`
design (w2-3). Commit `48e583e` (multistream + engine proposal); codex
review NEEDS FIXES → 2 fixed, plus a self-review panel → 2 fixed / 1
deferred, recorded at `4a2e58d`. fmt/clippy clean; tests 1413/0/2.

## 2026-06-20 — Transfer-core architecture conflict resolved; convergence plan drafted

Design conversation (owner + agent) resolved the 2026-06-14 open question
that had been deferred to "when Claude Fable is available again." Owner
judged ground-up redesign too much; direction is now **convergence**:

- One src/dst-agnostic sequencer owns all four src/dst paths (local↔local,
  push, pull, daemon↔daemon); the already-shared byte-moving leaf stays.
- One live dial (streams + knobs) replaces the three static stream-count
  ladders (`remote/tuning.rs`, `push/control.rs::desired_streams`,
  `pull.rs::pull_stream_count`). Bounded-unilateral: receiver advertises a
  capacity ceiling, sender owns the dial within it. Size-gated: small
  transfers skip the probe entirely.
- A-first (warmup), C-ready by construction: the dial is a mutable object
  read by both ends from day one and the stream-set is elastic (work-stealing
  from adaptive PR2), so continuous mid-transfer adjustment (`ue-2`) is a
  later feed, not a retrofit.
- Substrate salvaged per D-2026-06-07-2: adaptive-streams PR1 (telemetry
  `Probe`) + PR2 (work-stealing queue), cherry-pick up to `eafb187`;
  PR3 WIP `d9d4ec7` excluded.

Recorded as D-2026-06-20-1. Plan doc `docs/plan/UNIFIED_TRANSFER_ENGINE.md`
drafted then **Activated** (D-2026-06-20-2) with slices `ue-1a`–`ue-1e` +
`ue-2` (in scope, sequenced last). Absorbs the mooted incremental work:
w2-2 → `ue-1b`, w2-3/`MULTISTREAM_PULL.md` → `ue-1d`
(`MULTISTREAM_PULL.md` marked Superseded), w2-4 → `ue-1e`, adaptive
cherry-pick → `ue-1a`. The design-review correctness findings (w4-1 etc.)
are independent and unaffected.

Owner then answered the four gating questions and the design shifted
meaningfully (q1 most of all): **no probe-then-go phase** — the engine
starts moving within ~1s at conservative defaults bounded by the receiver
ceiling, and the tuner adjusts dials live from the first byte. This
obviates the small-transfer threshold entirely; the **planner** carries
the workload-shape judgment (file count vs bytes — 100k×10B ≠ 1×20MB) the
old size gate proxied. Capacity profile = rich (more data serves the
ubergoal). Engine type deferred to the agent (recommends new
`TransferEngine` + local adapter). `ue-2` stream-resize in scope, sign-off
via 10 GbE not a gate. The agent flagged an inference — that "start within
1s, planner adjusts on the fly" equals the ratified-but-unbuilt streaming
planner (D-2026-06-04-3 / H10b) and could be folded in — but the **owner
vetoed** that merger (D-2026-06-20-3). D-2026-06-04-3 stands unchanged; the
engine's workload-shape-awareness + 1s-start stand alone, not as the H10b
concept. Owner is **planning only, not ready to code.** No code written;
plan Active and parked; `ue-1a` awaits owner greenlight.