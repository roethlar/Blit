# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows

**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
**Status**: G3 and G4 fixed; fresh complete review pending.

## What

The acceptance harness cannot be reused unchanged for the phase diagnostic.
It retains old/new and push/pull-shaped orchestration, drains Windows even
when q is the destination, keeps one daemon alive across instrumentation-state
changes, discards successful client stderr, and can create a firewall rule.
Those properties either destroy the SOURCE/DESTINATION comparison or make the
new two-endpoint trace uncorrelatable.

## Approach

- Use semantic `source_init` and `destination_init` arms. SOURCE sends and
  DESTINATION receives in both arms; the varied property is only which
  endpoint initiates the one `Transfer` session.
- Pin one canonical source tree per direction and fixture. Both roles read the
  same q or Windows physical path and land into a precreated container of the
  same depth and shape. One session-scoped canonical destination path per
  endpoint is reset and reused by all 128 arms; role-bearing run IDs are kept
  only in evidence names and never enter a measured path. Session scoping
  preserves failed-run endpoint evidence without reintroducing a within-run
  path axis. The harness requires the q and Windows canonical
  relative-path/size manifests to match, pins the one exact `src_<shape>` root,
  and retains an identical manifest and digest for every accepted arm.
- Run a fixed OFF–ON–ON–OFF four-block schedule over
  `wm_tcp_mixed`, `mw_tcp_mixed`, `wm_grpc_mixed`, and `wm_tcp_large`.
  Pair rounds traverse cells forward/reverse/reverse/forward and run the two
  roles adjacently, producing eight pairs per trace state and cell with a
  four/four role-first balance (128 timed transfers).
- Stop and restart both exact daemons for every block, including ON→ON. Each
  block has a common run ID; every TCP client log supplies the 16-hex session
  fingerprint that correlates its peer daemon records. Windows logs are
  retrieved through base64 with SHA-256 verification.
- Fail closed on the exact build, route/interface/IP/MAC/MTU/link speeds,
  direction-specific negotiated MSS, firewall-rule identity, timer
  calibration, load, Time Machine, Spotlight, Windows CPU/disk drain, stale
  processes, PID ownership, port teardown, trace leakage, incomplete trace
  inventory, or landed-tree mismatch. The harness never changes firewall,
  MTU, routing, Time Machine, Spotlight, or unrelated processes.
- Use destination-keyed durability: q file fsync for Windows→q and Windows
  volume flush for q→Windows. Both client locations capture the same q
  monotonic completion anchor: immediate subprocess return on q, or the
  streamed Windows result line as q receives it before SSH teardown. They take
  the same three after-clock samples and wait only to the absolute +250 ms
  deadline before durability. The measured
  settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
  Successful Windows client logs are retrieved only after durability and the
  current landed count/byte verification. Both caches are purged before every arm and
  Windows disk writes must drain. The common first 250 ms of post-client
  observation remains excluded, but every excess settle millisecond is charged
  to the arm's durable total before comparison.
- Compute paired differences `d_i = destination_init_i − source_init_i`, the
  registered split drifts, role-order drift, the full paired range that guards
  the known bimodal fast arm, trace observer bias, and conservative
  `N_resolution`. Reports retain every sorted arm/difference distribution and
  use only per-endpoint monotonic clocks for phase intervals. Cross-host clock
  samples quantify uncertainty and are never silently subtracted.

## Files

- `crates/blit-core/src/transfer_session/data_plane.rs` — SOURCE dial
  trace attachment now follows the matching dial-end marker at epoch zero
  and every resize epoch.
- `crates/blit-core/tests/transfer_session_roles.rs` — both initiator layouts
  pin action-end before attachment on both endpoint roles.
- `scripts/bench_otp12pf_rigw.sh` — q-side registered runner and endpoint
  gates.
- `scripts/otp12pf_rigw_analyze.py` — exact schedule, trace, clock, phase, and
  resolution validator/reporter.
- `scripts/otp12pf_rigw_analyze_test.py` — complete synthetic session and
  fail-closed mutations.
- `.agents/machines.md` — current direction-specific MSS and q SSH endpoint
  fact.

## Tests

- `SELFTEST=1 bash scripts/bench_otp12pf_rigw.sh` proves the exact block/arm
  inventory and canonical path construction without contacting either rig
  endpoint. Every path assertion has an explicit failure path because macOS
  Bash 3.2 does not reliably apply `set -e` to bare `[[ ... ]]` commands.
- `python3 scripts/otp12pf_rigw_analyze_test.py` builds complete synthetic
  evidence (128 arms, 768 clock samples, split client/daemon phase logs) and
  rejects missing clock rows, missing endpoint trace, trace-off leakage,
  gRPC trace leakage, schedule drift, sequence gaps, and terminal/inventory
  corruption. It pins the split/range/role-order/observer resolution math and
  all exported reports.
- The same self-test runs under q's actual macOS Bash and Python so Bash 3.2
  and platform behavior are exercised, not inferred from nagatha.
- Mutation proof: removing role-order drift and the full paired-range term from
  `N_pair` makes the synthetic diagnostic fail (`N_resolution` falls from 70
  ms to 40 ms); restoring them returns the analyzer suite to green.
- Mutation proof: excluding successful client logs from trace discovery makes
  the synthetic diagnostic fail on a missing SOURCE/DESTINATION endpoint;
  restoring both client and daemon evidence roots returns all tests to green.
- Mutation proof: reducing the clock-row formatter from 12 fields to 11 makes
  the harness self-test fail before analysis; restoring the exact 12-column
  schema returns the local and q/macOS self-tests to green.
- The analyzer rejects a missing `settled_ms` column, non-integer values, and
  values outside `[250,1000)`. Synthetic evidence supplies the lower valid
  bound so every accepted arm proves the registered settle window.
- The analyzer parses each timing component once, requires exact Decimal
  `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`, and uses that
  durable total for every paired median, delta, distribution, observer-bias,
  and resolution-floor value. Only the common first 250 ms remains excluded;
  excess observation latency is charged. Corrupt totals are rejected;
  role-specific flush mutations prove the summaries cannot fall back to the
  pre-durability transfer time, and an equal client-to-durability regression
  proves asymmetric settle/flush partitioning cannot manufacture a role delta.
- All asserted causal phase pairs are endpoint-local and require both producer
  order and nondecreasing monotonic elapsed time. Socket action completion must
  precede trace attachment; attached payload sockets must progress through
  first write/receive before their role's data-plane completion; resize and
  planner prerequisite chains are also pinned. The resize DAG additionally
  requires sent proposal before SOURCE socket acquisition, attachment before
  SOURCE settlement, final settlement/ACK before role-local completion, and
  the exact receive→arm→ready→accept or receive→dial→attach→prepared chain on
  the DESTINATION. Mutations reverse every one of those edges while preserving
  exact contiguous producer sequences and must fail. Swapping completion ahead
  of a first write, swapping attachment ahead of action completion, or
  reversing a causal elapsed interval also makes the analyzer suite fail.
- Mutation proof: restoring SOURCE dial attachment ahead of `socket_dial_end`
  makes the two-initiator Rust phase test fail at epoch zero and resize epoch
  one; restoring end-before-attachment returns it to green. No cross-endpoint
  or concurrent send/ACK ordering is asserted.
- Fixture and landed manifests encode each UTF-8 POSIX relative path in base64
  beside its decimal file size, sort under ordinal/C locale rules, and reject
  nonregular or reparse entries. The analyzer recomputes all digests, requires
  exact q/Windows canonical equality and exactly 128 landed manifest files,
  and rejects swapped per-file sizes, renamed paths, wrong root layout, or a
  forged recorded digest even when file count and total bytes are unchanged.
- The harness atomically claims a never-existing evidence directory before it
  installs the EXIT trap or writes a byte. Existing paths are rejected
  unchanged, with explicit stale `SESSION-COMPLETE`/`SESSION-VOID` diagnostics;
  offline guards also pin rejection of unrelated retained content.
- Every arm resets its exact destination with explicit error propagation,
  verifies deletion landed, and proves the replacement is an empty plain
  directory before draining caches or starting the timer. The q self-test
  mutation makes removal fail under the production `||` call shape and must
  remain rejected; a Windows source-contract guard forbids suppressed removal
  errors and requires absence, directory, reparse, and emptiness checks.
- SOURCE- and DESTINATION-initiated arms resolve to the same canonical
  endpoint-local destination path and remote module-relative path. The
  self-test pins both direction/role pairs with explicit `|| die` guards and
  rejects any `run_arm` source that lets the role-bearing evidence ID reach a
  measured destination. Adding the initiator role to
  `destination_relative_path` now turns the Bash 3.2 self-test red at the first
  q destination-path assertion; restoring the role-invariant path returns it
  to green.
- The failure handler removes any completion marker, stops only remembered
  identity-checked daemons, appends teardown errors without replacing the
  primary void reason, and never initiates session-tree deletion. HUP, INT,
  and TERM enter that same bounded failure path. Offline process tests exercise
  all three signals and prove both owned teardown paths run while remaining
  evidence paths are reported for inspection.
- Successful finalization first proves no remembered daemon or open port,
  requires analyzer-accepted local evidence, removes and verifies both exact
  Windows trees and the exact q tree, rechecks the port, and only then atomically
  renames `SESSION-COMPLETE.tmp`. Cross-host deletion is not transactional: a
  partial finalization failure keeps the complete local evidence and reports
  remote paths as “may remain,” never as certainly preserved. A zero exit is
  rejected unless the registered marker is a regular one-line file containing
  the exact build SHA with no VOID or temporary marker; preflight-only runs
  cannot create it. Mutations for failed Windows removal, a surviving q tree,
  a pre/post-cleanup open port, missing/wrong completion markers, stale
  preflight markers, and cleanup before completion all fail the self-test.
- Windows launcher and daemon PIDs are numeric and identity-checked before any
  termination: exact executable/name, one anchored block-specific `cmd.exe`
  command line, and daemon parent PID when both processes exist. Startup also
  verifies the same CIM identities immediately. Offline source-contract
  mutations fail if command-line, parent, or validate-before-stop guards move
  or disappear. If startup fails after CIM creation but before either PID file
  is readable, the generated launcher waits on a bounded block-local gate and
  cannot execute the daemon until its PID is atomically placed and read back;
  without that gate it exits on its own. Teardown recovers only the unique
  exact block-specific launcher command and its parented daemon; after stopping
  the launcher it also finds, validates, and stops a child that raced the first
  query. The live daemon smoke remains required to prove CIM quoting.
  Mutations accepting any `cmd.exe`, accepting an unparented daemon, skipping
  the bounded gate wait, opening the gate before PID placement/readback, or
  skipping the late child's exact executable validation each turn the
  self-test red.
- `LAUNCHER_SMOKE=1` is a mutually exclusive standalone live mode. After the
  full provenance and endpoint preflight, it starts only the exact Windows CIM
  launcher and daemon, proves q can reach the registered port, identity-stops
  both processes, proves both endpoint ports closed, and completes strict
  session-tree cleanup. It never registers a run, starts q's daemon, times a
  transfer, invokes the analyzer, or writes `SESSION-COMPLETE`. An offline
  call-order test and source guard pin the start/reach/stop/closed/cleanup
  sequence and keep the smoke branch ahead of registered-run state. Mutations
  removing its pre-start port gate, start, reachability probe, exact stop/log
  collection, block clear, strict cleanup/failure path, or main-branch return,
  and a mutation setting registered state, each turn the self-test red.
- Mutation proof: replacing the absolute-deadline wait with a no-op makes the
  harness self-test fail because it returns before +250 ms. Moving the
  successful Windows client-log fetch ahead of the durability marker makes
  the production-order self-test fail. Restoring both returns the harness and
  analyzer self-tests to green.
- A delayed fake Windows-result producer emits its exact sentinel and then
  holds the pipe open; the q arrival stamp must predate producer teardown by a
  broad bound. Moving the stamp to EOF or restoring a fresh post-return q
  anchor makes the self-test fail. Reverting q to Python's process-relative
  macOS `time.monotonic_ns()` also fails an explicit cross-process clock guard;
  every carried q timestamp uses `clock_gettime(CLOCK_MONOTONIC)`. Both client
  wrappers carry the q completion stamp as the fourth result field consumed by
  `run_arm`, and live preflight proves the flushed Windows sentinel reaches q
  before the remote producer exits.
- Every trace-on TCP session must prove the complete seven-epoch one-stream
  ramp from one to eight live sockets on both roles, including exact proposal,
  preparation, ACK, settlement, attachment, and role-local ordering evidence.
  Removing epoch 7 makes the targeted analyzer guard fail; disabling exact
  target/live validation makes all four final-epoch SOURCE and DESTINATION
  mutations fail. Restoring both guards returns the analyzer suite to green.
- The build-identity self-test accepts the exact 12-character clean marker and
  mutation-proves that the same marker with `.dirty` is rejected. Live q and
  Windows gates apply that positive-and-negative check to both binaries.
- The repository gate is green: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, the documentation gate, analyzer tests, and shell
  syntax checks all passed.

## Known gaps

- No rig datum is produced by this slice. The full live run waits for fresh
  mandatory Codex adjudication, exact isolated builds, a successful live
  launcher smoke, and a green endpoint preflight.
- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
  hard gate. The active plan still requires the separately reviewed
  small-fixture/P2 work, phase report, and `0f922de` historical control before
  pf-1 closes.
- q was not quiet during the first read-only readiness sample on 2026-07-15:
  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
  The harness reports and refuses those conditions; it does not mutate them.

## Reviewer comments

Initial Codex review (`gpt-5.6-sol`, `xhigh`, codex-cli 0.144.4) reviewed
`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..0fb8237c2e6f63feb9cfc613d8af1602730061b0`
and returned `NEEDS FIXES` with three High findings. All three were accepted
and fixed independently: destination reset fail-closed at `661cf75`, excess
settle accounting at `1617546`, and the complete resize causal-edge audit plus
emitter alignment at `2dd977e`. See the raw review and adjudication under
`.review/results/otp12-pf1-rigw-harness.*`.

Round-2 Codex reviewed the complete immutable range through `8fbd486` and
returned `NEEDS FIXES`: it independently confirmed F1–F3 closed, then found two
new High defects. F4 is an uncharged Windows-client interval before q captures
the settle anchor. F5 is the role-bearing `rid` selecting different physical
destination paths for paired arms, contrary to the only-initiator-varies
contract. Both were accepted and fixed in order: F5 at `1231e42`, then F4 at
`6ba5408`. A separate runbook audit found the missing standalone launcher mode,
fixed at `18d3cde`; follow-up safety audit found the pre-PID-journal CIM race,
fixed at `454ebce`. The additive Grok second eye returned a schema-valid
`ACCEPTED` verdict with three independent red-to-green guards, but it does not
override the mandatory Codex findings. See the round-2 raw and adjudication
records under `.review/results/otp12-pf1-rigw-harness-r2.*`. Fresh review of
the complete fixed range is pending; no rig run is authorized yet.

Round-3 Codex reviewed
`4c7c7544db69289cf2e5fc0cf21093b40f00bc0d..53bb5e56a864abe0ee2d2b00c411846a1e7d24d5`
and returned `PASS` with no findings. The additive Grok review of the same
immutable range returned schema-valid `REOPENED`, `guard_confirmed=false`.
G3 is accepted: production role-invariant path construction is correct, but
the path-construction/parity assertions are bare `[[ ... ]]` commands that can
survive failure under macOS Bash 3.2. Grok's role-in-path mutation produced
different physical destinations while `SELFTEST=1` still exited zero. The
timing-anchor and launcher-journal mutations independently went red-to-green.
See `.review/results/otp12-pf1-rigw-harness-r3.*`. G3 must be fixed and the
complete range reviewed again before any rig activity.

Coder follow-up audit admitted G4 as a separate High instrument-correctness
finding. Destination-type, finalization-state, strict-cleanup-state,
completion-marker-removal, and signal-cleanup checks still used bare
`[[ ... ]]` assertions that macOS Bash 3.2 can let fall through to a later
successful command. A regression could therefore leave an unsafe destination
type, false cleanup state, or stale completion marker while the offline
self-test still exited zero. G4 gives each material lifecycle assertion an
explicit failure path and seeds the signal test with a completion marker, so
that its absence check is not vacuous. Final-command subshell predicates and
intentional predicate returns are unchanged. Removing the production
`SESSION_FINALIZED=1`, retaining `Q_SESSION_MAY_EXIST=1` after successful
cleanup, or conditionally skipping completion-marker removal for a received
signal each turns the Bash 3.2 self-test red at the intended assertion;
restoring all three returns it to green.
