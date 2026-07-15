# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows

**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.

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
  same depth and shape. The harness requires the q and Windows canonical
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
  monotonic completion anchor, take the same three after-clock samples, and
  wait only to the absolute +250 ms deadline before durability. The measured
  settle must remain in `[250,1000)` ms and is retained in `runs.csv`.
  Successful Windows client logs are retrieved only after durability and the
  current landed count/byte verification. Both caches are purged before every arm and
  Windows disk writes must drain.
- Compute paired differences `d_i = destination_init_i − source_init_i`, the
  registered split drifts, role-order drift, the full paired range that guards
  the known bimodal fast arm, trace observer bias, and conservative
  `N_resolution`. Reports retain every sorted arm/difference distribution and
  use only per-endpoint monotonic clocks for phase intervals. Cross-host clock
  samples quantify uncertainty and are never silently subtracted.

## Files

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
  endpoint.
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
  `total_ms = transfer_ms + flush_ms`, and uses that durable total for every
  paired median, delta, distribution, observer-bias, and resolution-floor
  value. The fixed settle window remains excluded. Corrupt totals are rejected;
  role-specific flush mutations prove the summaries cannot fall back to the
  pre-durability transfer time.
- All asserted causal phase pairs are endpoint-local and require both producer
  order and nondecreasing monotonic elapsed time. Socket action completion must
  precede trace attachment; attached payload sockets must progress through
  first write/receive before their role's data-plane completion; resize and
  planner prerequisite chains are also pinned. Swapping completion ahead of a
  first write, swapping attachment ahead of action completion, or reversing a
  causal elapsed interval makes the analyzer suite fail.
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
  or disappear; the live daemon smoke remains required to prove CIM quoting.
- Mutation proof: replacing the absolute-deadline wait with a no-op makes the
  harness self-test fail because it returns before +250 ms. Moving the
  successful Windows client-log fetch ahead of the durability marker makes
  the production-order self-test fail. Restoring both returns the harness and
  analyzer self-tests to green.
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

- No rig datum is produced by this slice. The full live run waits for the
  committed harness, mandatory Codex adjudication, exact isolated builds, live
  launcher smoke, and a green endpoint preflight.
- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
  hard gate. The active plan still requires the separately reviewed
  small-fixture/P2 work, phase report, and `0f922de` historical control before
  pf-1 closes.
- q was not quiet during the first read-only readiness sample on 2026-07-15:
  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
  The harness reports and refuses those conditions; it does not mutate them.

## Reviewer comments

Pending mandatory Codex review after commit.
