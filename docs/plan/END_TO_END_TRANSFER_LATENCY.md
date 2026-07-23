# End-to-end transfer latency attribution

**Status**: Historical
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

- [x] A default-off lifecycle trace emits compact structured records with one
      run ID, producer sequence, wall timestamp, monotonic elapsed time, event,
      and terminal outcome; session ID and initiator role appear once known.
- [x] The initiating timeline distinguishes, at minimum: async-main entry,
      argument parse/context completion, transfer dispatch/route selection,
      control connect begin/end, Transfer RPC open begin/end, session
      establishment begin/end, session body return, result render begin/end,
      and command terminal. The external-time residual before the first and
      after the last event is calculated rather than silently assigned.
- [x] Push and pull use one event vocabulary and explicit trace propagation.
      Delegated remote-to-remote initiation emits the applicable core spans.
- [x] Existing `[session-phase]` schema-1 output and trace-on/off behavior are
      byte-shape compatible; no proto, transfer decision, or payload hot path
      changes.
- [x] Deterministic injected-emitter tests prove lifecycle order, one terminal
      result on success/refusal/error, run/session correlation, trace-off
      silence, and flush behavior for both initiator roles. Every new guard is
      mutation-proved by reverting its production change, observing failure,
      restoring it, and observing the complete suite pass.
- [x] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --
      -D warnings`, `cargo test --workspace`,
      `bash scripts/agent/check-docs.sh`, and `git diff --check` pass with test
      counts not reduced. Product code differs from `d1f1152d` only by this
      diagnostic instrumentation and any admitted review fixes.
- [x] One exact instrumented build is identified and hashed, then exactly one
      8,589,934,592-byte Q-to-Nagatha transfer writes to a fresh RAM disk,
      reports eight files with `tcp_fallback: false`, and produces matching
      size/SHA-256 for every destination file. No comparison or repeat runs.
- [x] The retained evidence reports every lifecycle span, the existing
      data-body span, process CPU/memory, the external residual, observer
      limits, and SSD allocation. It identifies the dominant reducible class
      or states that the remaining gap is outside product-observable code.
- [x] The exact clones, staged client, daemon, listener, build RAM disk,
      destination RAM disk, and session scratch are removed or stopped. The
      retained seed, static Thunderbolt addresses, prior candidate artifact,
      and all earlier evidence remain untouched.
- [x] Instrumentation, review records, validation evidence, plan closure, and
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
4. **etl-4 — verification and review closure `[x]`.** Run the RAM-backed full
   workspace gates, adjudicate selected review findings one per commit, and
   prove the final instrument differs from `d1f1152d` only in diagnostic scope.
5. **etl-5 — one RAM validation and attribution `[x]`.** Build/hash the exact
   instrumented candidate, execute the one approved 8 GiB RAM-destination run,
   validate integrity and write allocation, clean up, record the bounded
   attribution, and close this plan as Historical.

## etl-4 closure evidence

Exact instrument head `dd1ac0adf029b3f9c72f17acf13c6f423aac9264`
passed the RAM-backed workspace format, strict all-target Clippy, complete
test, docs, and diff gates. Formal Opus 4.8/max reviews accepted etl-1, etl-2,
and etl-3 with independent red/green guards and no findings.

The product diff from release candidate `d1f1152d` is confined to the affected
code and tests named by this plan: the lifecycle trace primitive, explicit
trace-context propagation, low-frequency lifecycle boundaries, structured
diagnostic outcome preservation, and their guards. There is no diff in the
proto or Cargo manifests and no change to the payload data plane, stream or
worker policy, buffers, retry policy, carrier choice, filesystem behavior, or
wire contract. The CLI route/result rewrites were independently checked as
behavior-preserving funnels for balanced boundaries and one terminal flush.

## etl-5 closure evidence

Exact candidate `a3be4a64fbfb7a7ff3e867d40a3b75ba582a1517`, whose
product tree is identical to reviewed instrument head `dd1ac0ad`, completed the
one 8 GiB Q-to-Nagatha RAM-destination transfer as run
`etl5-20260723T054219Z-a3be4a64`. The client exited zero with eight files,
8,589,934,592 bytes, and `tcp_fallback: false`; all source and destination
sizes and SHA-256 values match.

The historical 0.448-second outside-body interval did not recur. The exact
session data body was 3.587845 seconds and the complete client process reported
3.60 seconds. The new trace observed 13.645 ms outside the data span; control
connect, RPC open, and session establishment together consumed 4.687 ms, while
render-through-terminal consumed 0.041 ms. External timer rounding bounds the
unobserved process residual at 0–3.449 ms. The earlier gap is therefore not in
the measured fixed product stages and remains either cold pre-main work,
external wrapper/environment state, or a non-reproducing condition.

The successful body rate was 19.153 Gb/s, below the prior 35.578 Gb/s sample.
The authorized one-observation design cannot classify that difference or
select a tuning change. Complete attribution, the zero-payload macOS firewall
setup incident, integrity, 11,112,448-byte Q SSD allocation, and cleanup proof
are retained in `docs/bench/end-to-end-transfer-latency-2026-07-23/`.

## Open questions

- None.
