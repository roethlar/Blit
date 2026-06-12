# Multi-stream pull-sync (w2-3)

**Status**: Draft
**Created**: 2026-06-12
**Supersedes**: nothing
**Decision ref**: pending (doc authorized by D-2026-06-11-2; interview
parameters delegated to the agent by the owner 2026-06-12 — "your call")

## Goal

Pull-sync uses the same multi-stream TCP data plane that push already has:
N parallel connections sized by the single tuning owner, instead of today's
hardcoded 1 stream. Delegated (daemon→daemon) transfers inherit it for free
because they run on the same pull-sync engine. The deprecated Pull RPC's
multi-stream pattern (~500 unreachable daemon lines that are ALSO the only
multi-stream pull code) is harvested before w2-4 deletes it.

## Non-goals

- Runtime bandwidth adaptation (H10b-class; the static table from w2-1/w2-2
  governs stream count).
- Zero-copy receive (D-2026-06-12-1: deleted; revisit gated on the 10 GbE
  benchmarks).
- Touching the gRPC fallback path (stays single-logical-stream by design).

## Constraints

- Wire changes allowed (proto unfrozen, D-2026-06-11-1), but a new client
  against an old daemon (or vice versa) must degrade gracefully to today's
  single-stream behavior — negotiation, not assumption.
- Stream count comes from `determine_remote_tuning` ONLY (w2-2's single
  owner; w2-2 lands first).
- No regression of StallGuard coverage, byte-progress accuracy (design-1
  class), or resume semantics on the multi-stream path.
- Windows parity: new tests ungated unless genuinely platform-specific.

## Sequencing (owner delegated; agent's call)

1. **w2-2** (single ladder owner) — prerequisite, lands first.
2. **w2-3** (this plan).
3. **Adaptive-streams cherry-pick** (D-2026-06-07-2) lands AFTER w2-3: both
   churn `data_plane.rs`, and the cherry-pick already requires hand-resolving
   the StallGuard-vs-`Probe` conflict — doing it once, on the post-w2-3
   tree, localizes all conflict resolution to one event.
4. **w2-4** (delete deprecated Pull RPC) immediately after w2-3 ships.
5. **w3-1** (memory-aware buffer pool) after, sized against the final
   stream topology.

## Acceptance criteria

- [ ] Pull of a many-file tree negotiates >1 stream against a new daemon
      (observable in the daemon's stream lines / tests).
- [ ] New client ↔ old daemon and old client ↔ new daemon both complete
      correctly at 1 stream (compat tests with a fake old-shape peer).
- [ ] Existing remote/parity/resume test suites stay green; new
      multi-stream pull tests cover negotiation, per-stream failure, and
      cancellation mid-transfer.
- [ ] Loopback sanity benchmark shows no regression vs single-stream.
- [ ] **Owner sign-off waits for the 10 GbE rig** (macOS/Windows/Linux/
      TrueNAS matrix — owner, 2026-06-12): pull throughput at parity with
      push on the same hardware, or the gap explained.

## Design

Harvest, don't reinvent: the deprecated Pull RPC already demonstrates
daemon-side multi-connection accept + fan-out (`pull.rs` accept loop,
`pull_stream_count` ladder — the ladder itself dies in w2-2). Pull-sync
gains a stream-count field in its negotiation exchange; the daemon binds
one listener and accepts N token-authenticated connections (same
accept/token timeouts as push — w1-4's constants when that lands); files
fan out across streams; the client runs N `execute_receive_pipeline`
instances exactly like the daemon's push receiver does today. Byte progress
and StallGuard wrap each stream as they do on push. Absent or zero
stream-count in negotiation → single stream (back-compat).

Affected: `blit-core/remote/pull.rs` + `transfer/{data_plane,pipeline}.rs`
(client), `blit-daemon/service/pull_sync.rs` (server), `proto/blit.proto`
(negotiation field), tests in `blit-cli/tests/` + `blit-core/tests/`.

Risks: per-stream partial-failure semantics (one stream dying must fail or
retry the whole transfer deterministically, not silently drop its files);
progress double-count class (design-1) re-checked on the fan-in path.

## Slices

1. `w2-3a-negotiation`: proto field + daemon advertises/honors stream
   count + client requests it; both sides still run 1 stream. Compat tests.
2. `w2-3b-daemon-fanout`: daemon multi-accept + file fan-out across
   streams (harvested Pull pattern), gated on negotiated count.
3. `w2-3c-client-receive`: client opens N connections, parallel receive
   pipelines, fan-in of outcomes/progress; per-stream failure tests.
4. `w2-3d-delegated-inherit`: delegated pull exercises the same path;
   e2e test via the jobs_lifecycle harness pattern.
5. (separate slice, already queued) `w2-4-delete-pull-rpc`.

## Open questions

- None blocking Draft review. Stream-failure policy default (fail-whole vs
  per-stream retry) proposed as fail-whole-with-clean-error in w2-3c;
  owner may override at ratification.
