# DEVLOG — append-only journal

**Status**: Active

Append-only history; newest first. ISO timestamp per entry. Never read this
to determine current state — that is `docs/STATE.md`'s job (AGENTS.md §1).
Write to it via the `handoff` procedure; prune STATE.md overflow here.

---

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