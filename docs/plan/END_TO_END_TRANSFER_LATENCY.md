# End-to-end transfer latency attribution

**Status**: Active
**Created**: 2026-07-23
**Supersedes**: nothing
**Decision ref**: D-2026-07-23-3

## Goal

Make the fixed end-to-end latency around remote transfers observable without
changing transfer policy. One default-off lifecycle trace must divide client
runtime entry, argument/route preparation, control connection, Transfer RPC
opening, HELLO/OPEN/ACCEPT establishment, the existing session/data body,
summary return, result rendering, and command/runtime exit closely enough to
name the next optimization target for 10, 25, and 40 Gb/s links. After the
instrument is mutation-proved, one Q-to-Nagatha RAM-destination run must
attribute the 0.448-second interval the 2026-07-23 Thunderbolt profile left
outside its measured payload body.

## Non-goals

- No stream-count, buffer, dial, planner, carrier, filesystem, wire-contract,
  retry, or transfer-policy change.
- No claim that every external nanosecond is observable. Dynamic loading,
  Tokio runtime construction before async `main`, runtime destruction after
  return, and OS process teardown remain a reported residual unless evidence
  later justifies a separate design.
- No rsync, iperf, SSD-backed payload, reverse direction, A/B matrix,
  durability run, automatic retry, or second hardware sample.
- No throughput or release acceptance verdict. A latency fix, if the trace
  earns one, requires its own Active plan and guard proof.
- No always-on telemetry, public CLI flag, metrics surface, protocol field, or
  persisted user history.

## Constraints

- Preserve the existing `BLIT_TRACE_SESSION_PHASES=1` /
  `BLIT_TRACE_RUN_ID` opt-in contract. With tracing disabled there must be no
  writer thread, event serialization, output, or per-payload work.
- Add a separate structured lifecycle record rather than weakening the
  existing schema-1 `SessionPhaseEvent` contract. Lifecycle and session
  records correlate by the unique run ID and wall timestamp; after
  establishment the lifecycle record also carries the derived session ID.
- One explicitly propagated trace context owns a monotonic origin and sequence
  for an initiating process. Do not use process-global mutable state: CLI,
  library, TUI, retry, and delegated-daemon callers must remain concurrency
  safe even when only the CLI success path is graded on hardware.
- Record only low-frequency control boundaries. No event may be emitted per
  file, chunk, payload, socket write, or progress update; the existing session
  trace remains the data-body observer.
- Direct remote push and pull must share the same lifecycle trace code and
  event vocabulary. The delegated destination daemon must carry the same core
  connection/RPC/establishment boundaries when it initiates a session; it has
  no CLI-entry or result-render stages to invent.
- Success, refusal, and error paths must emit one terminal lifecycle outcome
  and flush already-queued records without masking or changing the product
  result. Trace writer failure remains diagnostic-only and cannot fail a
  transfer.
- No proto or wire change. Existing trace consumers must continue to parse
  `[session-phase]` schema 1 unchanged.
- Build, test, and review scratch must use a RAM-backed `CARGO_TARGET_DIR` on
  Nagatha where practical. Stable dependency caches may be read, but avoid a
  fresh SSD-backed target tree.
- The one hardware validation uses the same physical direction and logical
  shape as `docs/bench/thunderbolt-ram-profile-2026-07-23/`: Q
  `172.31.254.2` SOURCE to Nagatha `172.31.254.1` DESTINATION, eight warm APFS
  clones of the retained 1 GiB seed, and a fresh Nagatha APFS RAM disk.
- The hardware run may allocate only APFS clone metadata, one staged exact
  client binary, and text evidence on SSD, together below 32 decimal MB. It
  may write no benchmark payload to SSD. A clone allocation delta, route/link
  failure, listener conflict, hash mismatch, or RAM-disk failure stops before
  timing and does not authorize a retry.
- The owner approved inclusion of exactly one later 8 GiB RAM-destination
  validation and activated this plan on 2026-07-23.

## Acceptance criteria

- [ ] A default-off lifecycle trace emits compact structured records with one
      run ID, producer sequence, wall timestamp, monotonic elapsed time, event,
      and terminal outcome; session ID and initiator role appear once known.
- [ ] The initiating timeline distinguishes, at minimum: async-main entry,
      argument parse/context completion, transfer dispatch/route selection,
      control connect begin/end, Transfer RPC open begin/end, session
      establishment begin/end, session body return, result render begin/end,
      and command terminal. The external-time residual before the first and
      after the last event is calculated rather than silently assigned.
- [ ] Push and pull use one event vocabulary and explicit trace propagation.
      Delegated remote-to-remote initiation emits the applicable core spans.
- [ ] Existing `[session-phase]` schema-1 output and trace-on/off behavior are
      byte-shape compatible; no proto, transfer decision, or payload hot path
      changes.
- [ ] Deterministic injected-emitter tests prove lifecycle order, one terminal
      result on success/refusal/error, run/session correlation, trace-off
      silence, and flush behavior for both initiator roles. Every new guard is
      mutation-proved by reverting its production change, observing failure,
      restoring it, and observing the complete suite pass.
- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --
      -D warnings`, `cargo test --workspace`,
      `bash scripts/agent/check-docs.sh`, and `git diff --check` pass with test
      counts not reduced. Product code differs from `d1f1152d` only by this
      diagnostic instrumentation and any admitted review fixes.
- [ ] One exact instrumented build is identified and hashed, then exactly one
      8,589,934,592-byte Q-to-Nagatha transfer writes to a fresh RAM disk,
      reports eight files with `tcp_fallback: false`, and produces matching
      size/SHA-256 for every destination file. No comparison or repeat runs.
- [ ] The retained evidence reports every lifecycle span, the existing
      data-body span, process CPU/memory, the external residual, observer
      limits, and SSD allocation. It identifies the dominant reducible class
      or states that the remaining gap is outside product-observable code.
- [ ] The exact clones, staged client, daemon, listener, build RAM disk,
      destination RAM disk, and session scratch are removed or stopped. The
      retained seed, static Thunderbolt addresses, prior candidate artifact,
      and all earlier evidence remain untouched.
- [ ] Instrumentation, review records, validation evidence, plan closure, and
      current state are committed one coherent slice at a time. Nothing is
      pushed, tagged, or published without a separate exact owner approval.

## Design

### Lifecycle trace

Add a small `TransferLifecycleTrace` beside the existing session-phase module
in `blit-core`. It owns an optional asynchronous emitter, a run ID, one
monotonic origin, and a producer sequence. Its production constructor consults
the existing phase-trace environment contract; its injected constructor lets
tests capture records without environment or stderr. A lifecycle event uses a
new prefix and schema so `SessionPhaseEvent` stays unchanged. Optional fields
carry initiator role, derived session ID, and terminal outcome only when they
are known.

The trace context is passed explicitly from the initiating surface through
`blit-app` execution options and `blit-core` session-client options. The CLI
captures the earliest practical stamp inside async `main`, then marks parsing,
context, route, output, and terminal boundaries. `session_client` owns connect
and bidi-RPC boundaries. `run_source` / `run_destination` own establishment
boundaries and attach the derived session ID once the accept is available. The
existing bound session trace continues to own manifest, data-plane, tuner, and
summary events. The delegated daemon creates the same lifecycle context at its
outbound session boundary and emits only stages that actually exist there.

Every begin boundary has an outcome-bearing end boundary. A small terminal
guard or single return funnel records success/error exactly once and flushes
off the async runtime's hot path. The trace never changes the returned result;
writer startup, serialization, or flush failure disables or truncates only the
diagnostic output.

### Attribution

The lifecycle trace uses monotonic elapsed time within the initiating process.
Its wall timestamps align it with the independently-originated session traces
on both endpoints. The analysis partitions external `/usr/bin/time -lp` wall
time into observable lifecycle spans, the existing first-record-to-data-plane-
complete interval, and an explicit pre-entry/post-terminal residual. It does
not infer a narrower cause when timestamps cannot separate one.

The hardware validation is one observation, not an optimization comparison.
The instrumented build is compiled and verified with target scratch on a
Nagatha RAM disk, then only its client binary is staged on Q. Q's source names
are APFS clones whose free-block delta must remain zero; Nagatha receives on a
separate RAM disk. Integrity, resource accounting, exact hashes, cleanup, and
the no-repeat rule follow the completed 2026-07-23 RAM profile.

### Affected code and tests

- `crates/blit-core/src/remote/transfer/`: lifecycle trace schema/emitter and
  client connection/RPC propagation.
- `crates/blit-core/src/transfer_session/`: establishment boundaries and
  session-ID handoff, without touching payload policy.
- `crates/blit-app/src/transfers/`: explicit trace-context carriage through
  shared remote push/pull execution.
- `crates/blit-cli/src/main.rs` and `crates/blit-cli/src/transfers/`: earliest
  practical command, route, render, and terminal boundaries.
- `crates/blit-daemon/src/service/delegated_pull.rs`: applicable outbound
  lifecycle spans for daemon-initiated sessions.
- Existing core role/integration tests and CLI route tests: injected records,
  order, error terminality, disabled behavior, and push/pull/delegated parity.

## Slices

1. **etl-1 — trace primitive `[x]`.** Add the default-off lifecycle record,
   asynchronous emitter, explicit context, session-ID attachment, flush, and
   deterministic unit tests without wiring it into product transfer calls.
2. **etl-2 — shared core boundaries `[x]`.** Carry the trace through remote push,
   pull, and delegated initiation; emit connect, RPC, establish, session-body,
   and terminal spans; add role-parity and failure-path integration guards.
3. **etl-3 — CLI lifecycle `[x]`.** Carry one trace from async-main entry through
   parse/context, route dispatch, result rendering, and command terminal;
   mutation-prove the complete successful timeline and trace-off silence.
4. **etl-4 — verification and review closure.** Run the RAM-backed full
   workspace gates, adjudicate selected review findings one per commit, and
   prove the final instrument differs from `d1f1152d` only in diagnostic scope.
5. **etl-5 — one RAM validation and attribution.** Build/hash the exact
   instrumented candidate, execute the one approved 8 GiB RAM-destination run,
   validate integrity and write allocation, clean up, record the bounded
   attribution, and close this plan as Historical.

## Open questions

- None.
