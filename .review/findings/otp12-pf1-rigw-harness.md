# otp12-pf1-rigw-harness — reduced paired P1 diagnostic on q ↔ Windows

**Slice**: OTP12 performance-finding pf-1, P1 rig harness only.
**Status**: In progress — round-15 Claude accepted exact `8e019ef`; fresh
additive build/staging and live gates remain.

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
  streamed Windows result line as q receives it before SSH teardown. Each
  before/after phase takes the same three independently q-bracketed clock
  samples through one persistent SSH/PowerShell channel, with exact indexed
  request/response handshakes. The after phase then waits only to the absolute
  +250 ms deadline before durability. The measured
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
- One fake SSH process must serve exactly `P|1`→`C|1|<ns>` through
  `P|3`→`C|3|<ns>`, produce three exact 12-column rows, exit, and be reaped.
  Wrong indices, malformed or extra output, a nonzero exit, a partial-line
  timeout, and early EOF all fail closed without a surviving child. Restoring
  the old three-sequential-SSH implementation turns the one-launch guard red;
  restoration returns the complete Bash 3.2 self-test green. Live preflight
  times the complete sample/parse/CSV path and requires all three samples in
  less than the registered 750 ms settle headroom before any daemon starts.
- A delayed fake Windows-result producer emits its exact sentinel and then
  holds the pipe open; the q arrival stamp must predate producer teardown by a
  broad bound. Moving the stamp to EOF or restoring a fresh post-return q
  anchor makes the self-test fail. Reverting q to Python's process-relative
  macOS `time.monotonic_ns()` also fails an explicit cross-process clock guard;
  every carried q timestamp uses `clock_gettime(CLOCK_MONOTONIC)`. Both client
  wrappers carry the q completion stamp as the fourth result field consumed by
  `run_arm`, and live preflight proves the flushed Windows sentinel reaches q
  before the remote producer exits.
- The real q client wrapper executes under deliberately stale trace variables
  in both trace-off and trace-on self-test cases. Trace-off must remove both;
  trace-on must install only `1` and the exact run ID; both must return an exact
  successful completion sentinel. Restoring the empty trace-command array from
  G12 turns the trace-off guard red under `/bin/bash` 3.2 nounset; restoring the
  permanently nonempty command array returns the complete self-test green.
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

- No accepted or graded rig datum has been produced by this slice. Exact
  candidates `d5e9dda`, `d7345f1`, and `1f62ce5` are retired from further live
  use after G13, G14, and G15 voids. Claude accepted the immutable G15
  identity; the full run waits for fresh additive isolated build/stage,
  successful launcher smoke, and green endpoint preflight.
- This four-cell run is the reduced P1 phase diagnostic, not the entire pf-1
  hard gate. The active plan still requires the separately reviewed
  small-fixture/P2 work, phase report, and `0f922de` historical control before
  pf-1 closes.
- q was not quiet during the first read-only readiness sample on 2026-07-15:
  Time Machine AutoBackup was enabled and Spotlight was using substantial CPU.
  The owner later set Time Machine to manual; both live G9 attempts reported
  `AutoBackup=0`, `Running=0`, and passed the quiet gate. The harness did not
  mutate that policy.

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
See `.review/results/otp12-pf1-rigw-harness-r3.*`. G3 was fixed at `27c94b0`;
the complete range still requires fresh review before any rig activity.

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

G4 was fixed at `7e9d2d5`. The full workspace format, strict-clippy, and test
gate; 23 analyzer tests; Bash syntax and self-test; documentation gate; and
diff check are green for both G3 and G4. No endpoint was contacted.

Round-4 mandatory Codex and additive Grok reviewed the complete immutable
range through `6f517ea1bdbea2f7d83f15c086d2bf5f764cf524`. Codex returned
`PASS` with no material finding. Grok returned schema-valid `ACCEPTED`,
`guard_confirmed=true`, exact SHAs, and independently drove the G3 role-path
mutation plus G4 finalization, may-exist, and marker-removal mutations red
before restoring every offline suite green. Its detached worktree ended clean
and was removed. Review is closed; launcher smoke and endpoint preflight remain
required before the registered run.

The first live launcher-smoke attempt on q refused before launching a daemon
or timing a transfer. G5 is accepted as a High instrument-correctness finding:
q legitimately has the Windows peer cached on `en0`, `en1`, and registered
`en8`, but the ARP gate concatenated all three MAC rows. It therefore rejected
the correct peer even though `route -n get` selected `en8`. The failed attempt
is retained as `SESSION-VOID` under
`logs/otp12pf-rigw-20260715T113500Z-launcher` in the isolated q clone. The fix
parses exactly the registered interface, requires one result, and pins the
real three-interface shape in the Bash 3.2 self-test. No daemon started and no
endpoint policy changed. Removing the interface predicate makes the self-test
red on the three-row fixture; restoring it returns the complete self-test to
green.

Round-5 reviewed the complete immutable range through
`06b33228d502c51da24bc2a78fba7eddcf6c0723`. Mandatory Codex independently
confirmed G5, the exact 128-arm schedule, and role-invariant endpoint-local
paths, then returned `NEEDS FIXES` with one separate High finding. G6 is
accepted: the harness runs the endpoint's pre-existing
`D:/blit-test/purge-standby.ps1` by existence and exit status only, rather
than staging and hashing the reviewed repository helper. A stale or no-op
helper could therefore make a warm-cache run look valid. Additive Grok
returned schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` for
G5 after independently driving the ARP interface mutation red and restoring
the Bash 3.2 self-test green. Its detached worktree ended clean and was
removed. No endpoint was contacted. See the round-5 raw reviews and
adjudications under `.review/results/otp12-pf1-rigw-harness-r5.*`.

G6 now takes the purge helper only from the exact clean q checkout. After all
read-only endpoint/fabric/quiet gates pass, the harness reserves a fresh
per-session Windows tree, copies the reviewed helper to a temporary path,
rejects reparse points, verifies SHA-256 before and after the atomic move, and
records the helper hash/path alongside the four executable hashes. Every arm
rechecks that same hash immediately before invocation and requires exactly one
`standby-purged` success line in addition to exit zero. The helper is therefore
covered by the executable snapshot and strict session-tree cleanup rather than
trusted as endpoint state.

The Bash 3.2 self-test functionally mocks both stage and per-arm commands.
Removing the final post-move hash comparison turns it red at the staging
contract; restoring it returns green. Removing the per-arm hash comparison
turns it red before the mocked purge can pass; restoring it returns green. A
separate order guard pins the first remote write after provenance, port,
topology, MSS, firewall, quietness, timer, and result-stream checks. No endpoint
was contacted by the fix or its mutation proofs.

G6 was fixed at `888be4754387311e28e14d687721fd3d1315f82c`.
Format, strict clippy, Bash syntax/self-test, all 23 analyzer tests, the docs
gate, and diff checks passed. The first full workspace test attempt hit the
recorded macOS `blit_utils::test_utils_list_modules` daemon-start race once;
the isolated test then passed, and a complete quiet rerun passed with two
expected ignores. Fresh complete Codex plus additive Grok review is still
required before any build or endpoint contact.

Round-6 reviewed the complete immutable range through
`75a9a33ce600e4707438ed885de2ce0cdf27d946`. Additive Grok returned
schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` after
independently driving both G6 hash mutations red and restoring the Bash 3.2
self-test and worktree clean. Mandatory Codex returned `NEEDS FIXES` with one
new High finding, accepted as G7: G6 derives its expected helper hash from the
working file only after several gates. A concurrent replacement after the
clean-tree check can therefore be adopted as the expected helper rather than
rejected. The expected hash must instead come from the helper blob addressed
by the immutable reviewed commit, with the working file rechecked against it
immediately before copy. No endpoint was contacted. See the round-6 raw
reviews and adjudications under
`.review/results/otp12-pf1-rigw-harness-r6.*`.

G7 derives the expected purge-helper SHA-256 from the blob addressed by
`HEAD_FULL:scripts/windows/purge-standby.ps1`, records that blob identity, and
requires the working file to match it. `stage_purge_helper` rechecks the
working file immediately before SCP; the existing post-move and per-arm checks
therefore compare against the Git-derived value rather than bytes adopted from
the mutable working tree.

The Bash 3.2 self-test commits one helper in a temporary repository, changes
the working file, and requires the binding gate to reject it. Restoring the
committed bytes pins both blob identity and SHA through the staging mock.
Removing the blob/worktree comparison turns the mutable-file guard red;
restoration returns the complete self-test green. A second fixture changes the
working file during remote-session reservation, after binding; the adjacent
pre-SCP recheck refuses it before the copy mock is reached. Removing that
comparison turns the copy-reached guard red, and restoration returns green.
G7 was fixed at `47aaaf0e7784f8cefa3e84d757849bf243bea70a`.
Format, strict clippy, all workspace tests, Bash syntax/self-test, all 23
analyzer tests, the docs gate, and diff checks passed. No endpoint was
contacted. The first final workspace attempt hit the recorded macOS
`blit_utils` daemon-start race once in `test_utils_find_dirs_only`; that
isolated test passed, then a complete quiet workspace rerun passed with two
expected ignores. Fresh complete Codex plus additive Grok review remains
required before rebuild or launcher retry.

Round-7 reviewed the complete immutable range through
`a53971574a8badb2ddf4ab952168fc7b2739ff89`. Additive Grok returned
schema-valid `ACCEPTED`, exact SHAs, and `guard_confirmed=true` after
independently driving both G7 worktree-comparison mutations red and restoring
the Bash 3.2 self-test and all 23 analyzer tests green. It also reconfirmed one
`Transfer` RPC, SOURCE-send/DESTINATION-receive semantics, role-invariant
physical paths, and the shared eight-worker target under both initiator
layouts. Mandatory Codex returned `NEEDS FIXES` with one new High finding,
accepted as G8: ordinary Git object lookup honors replacement refs, so a
replacement commit can preserve `HEAD_FULL` and a clean status while making
the reviewed path resolve to substituted helper bytes. G7 would bless that
substituted digest as reviewed. No endpoint was contacted. See the round-7
raw reviews and adjudications under
`.review/results/otp12-pf1-rigw-harness-r7.*`.

G8 disables Git replacement-object interpretation for every reviewed helper
object operation: commit/path resolution, object-type inspection, and
blob-content reads. The provenance HEAD, short build identities, and clean
status use the same command-scoped protection. The expected SHA therefore
derives from the literal object graph named by `HEAD_FULL`; the existing
working-file, pre-SCP, post-move, and per-arm comparisons remain downstream
checks of that immutable value.

The Bash 3.2 self-test installs both commit and blob replacement refs while
leaving the visible HEAD unchanged and ordinary status clean. It proves that
ordinary Git resolves the substituted path and bytes, then requires the
reviewed-object binding to refuse them. Removing only the no-replacement
setting, routing only the commit/path lookup through ordinary Git, or routing
only the blob-content read through ordinary Git each turns the exact
replacement-provenance guard red. Restoration returns the complete self-test
green. An independent audit reproduced the wrapper mutation red-to-green and
found one conditional-context cleanup gap; explicit replacement deletion,
empty-list, checkout, and clean-status checks closed it, and the focused
re-audit passed.

Format, strict clippy, all workspace tests, Bash syntax/self-test, all 23
analyzer tests, the docs gate, and diff checks passed. No endpoint was
contacted.
G8 was fixed at `29d63b7ad45dff21d052a678fff795029b300e6d`.

Round-8 independent Grok reviewed the complete immutable range through
`6fb369e3d70f7633ad1d697afeda35abf5e276cb` and returned schema-valid
`ACCEPTED`, exact SHAs, and `guard_confirmed=true`. It independently drove the
wrapper, commit/path-only, and blob-content-only replacement mutations red and
restored the Bash 3.2 self-test green after each. It reconfirmed the G8 object
coverage and cleanup, one `Transfer` RPC, SOURCE-send/DESTINATION-receive
semantics, role-invariant endpoint-local paths, and the shared 1→8 worker
target under both initiator layouts. Its detached worktree ended clean at the
reviewed SHA and was removed. An attempted same-model Codex review was stopped
and its partial output discarded on the owner's identity correction; it was
not counted as review evidence. No endpoint was contacted. The immutable
reviewed SHA, not this later verdict-record commit, is the only build allowed
into launcher smoke, endpoint preflight, and the registered run.

The exact reviewed candidate `6fb369e3d70f7633ad1d697afeda35abf5e276cb`
was then built from fresh detached clones on the owner Mac and natively on
Windows, staged into new `6fb369e` paths, and verified by embedded clean build
identity plus source/stage SHA-256 equality. q did not build. q's actual Bash
3.2 self-test passed. The live launcher smoke at
`/Users/michael/Dev/blit_v2_6fb369e/logs/otp12pf-rigw-20260715T140346Z-launcher`
then refused before helper staging or daemon launch because q still reported
Time Machine `AutoBackup=1`. The retained `SESSION-VOID` records that exact
reason; both registered ports were closed and the active Windows benchmark
daemon was absent afterward. A read-only follow-up found no second current
quietness blocker: Time Machine was stopped, q load1 was 1.35, Spotlight was
0.0%, Windows CPU was 2.3%, and neither endpoint had a conflicting process.
The harness did not mutate Time Machine or any other endpoint policy.

After the owner set q Time Machine to manual, live gates resumed against exact
candidate `6fb369e3d70f7633ad1d697afeda35abf5e276cb`. q's previously staged
tree and both endpoints' `rigw-module` fixture paths were absent, so fresh q
staging was recreated from the retained immutable bundle and reviewed arm64
binaries without overwriting an existing path; Windows reused the exact
registered fixtures still present under `bench-module/pull_src_*`. q and
Windows canonical shapes were again `5001/547110912` and `1/1073741824`.

The first retained retry at
`logs/otp12pf-rigw-20260715T152231Z-launcher` refused while discovering the
missing Windows canonical source. After the guarded fixture copy, the second
retained retry at `logs/otp12pf-rigw-20260715T152457Z-launcher` passed build,
fabric, direction-specific MSS, firewall, quiet-host, timer, and streamed
Windows-result gates, then exposed G9 before helper staging or daemon launch.
`write_win_tree_manifest` embedded PowerShell `` `n`` escapes inside a Bash
double-quoted `wssh` payload. Bash therefore treated them as command
substitution, emitted `n) + : command not found`, and delivered a syntactically
corrupt manifest program to Windows. No transfer was timed.

G9 emits LF with `[string][char]10` and joins/appends that literal PowerShell
variable, leaving no Bash command-substitution delimiter in the rendered
payload. The Bash 3.2 self-test now executes `write_win_tree_manifest` through
a capturing `wssh` mock, requires the exact rendered LF expression, and rejects
any grave accent in the payload. Restoring the exact live-failing backtick
expression reproduces Bash's command-not-found error and turns the new guard
red; restoring G9 returns the complete self-test green. Format, strict clippy,
all workspace tests, Bash syntax/self-test, all 23 analyzer tests, the docs
gate, and diff checks passed. The first workspace run hit the recorded macOS
daemon-start race once in `test_utils_find`; that isolated test passed and the
complete quiet rerun passed with two expected ignores. G9 was fixed at
`f7ef1d7184574639adb823513c17ebf94f720292`.

Round-9 independent Grok reviewed the immutable
`6fb369e3d70f7633ad1d697afeda35abf5e276cb..d57a86ef4070a8852067ae0b8c6bad91010ec98e`
range and returned schema-valid `ACCEPTED`, exact SHAs, and
`guard_confirmed=true`. In its detached worktree it ran the Bash 3.2 self-test
green, restored the exact live-failing backtick expression and reproduced both
the Bash command-substitution error and the literal-LF guard failure, then
restored the reviewed bytes and returned the complete self-test green. The
worktree ended clean at the exact reviewed SHA and was removed. Review is
closed; the exact reviewed candidate `d57a86e`, not the later verdict-record
commit, is the only build allowed into launcher smoke, endpoint preflight, and
the registered run. No endpoint was contacted during review.

The exact `d57a86e` launcher smoke at
`/Users/michael/Dev/blit_v2_d57a86e/logs/otp12pf-rigw-20260715T155415Z-launcher`
passed build identity, fabric, direction-specific MSS, firewall, quiet-host,
timer, and streamed Windows-result gates. It then exposed G10 in preflight,
before helper staging, daemon launch, run registration, or a timed transfer:
under macOS Bash 3.2 with `set -u`, `fetch_win_file` declared `local_path` and
derived `tmp="$local_path.base64"` in the same `local` command, so the right-hand
side expanded before `local_path` existed. Bash 3.2 passed status zero to the
EXIT trap for this nounset failure; the harness correctly rejected that
impossible success because strict success cleanup had not run and exited one.
The Windows session and q evidence remain retained by the registered failure
policy; no cleanup or endpoint-policy mutation was attempted.

G10 splits argument assignment from the following dependent declaration in
`fetch_win_file`. A class-wide audit of all production `local` declarations found and
fixed the only three additional same-command dependencies: block-log path,
q-daemon block path, and run-block identity. Those three happened to inherit
caller variables through Bash dynamic scope in current call chains, which could
hide the defect or substitute stale caller state. The Bash 3.2 self-test now
executes each real function in isolation with the inherited names unset and
pins exact output/path/identity behavior. Rejoining each of the four declarations
individually turns its targeted guard red with the expected unbound-variable or
wrong-derivation failure; restoring each split returns the complete self-test
green. Bash syntax, format, strict clippy, all workspace tests, all 23 analyzer
tests, the docs gate, and diff checks passed. G10 was fixed at
`b1cfde74a5ffbd8413aa9dc69e4b1abe9b9118e9`.

Round-10 independent Grok reviewed the immutable
`d57a86ef4070a8852067ae0b8c6bad91010ec98e..5a7e7ec3dcaa4965ba7fe2bce57686f5acb05549`
range and returned schema-valid `ACCEPTED`, exact SHAs, and
`guard_confirmed=true`. In its detached worktree it ran syntax and the complete
Bash 3.2 self-test green, then independently rejoined each of the four local
declarations. The executed guards failed respectively on unset `local_path`,
block-log `block`, q-daemon `block`, and run-block `block`. It restored the
exact reviewed bytes and reran the complete self-test green after every
mutation. A final lexical audit found no remaining same-command dependency;
final syntax/self-test passed and the worktree ended clean at the reviewed SHA.
Review is closed; exact candidate `5a7e7ec`, not the later verdict-record
commit, is the only build allowed into launcher smoke, endpoint preflight, and
the registered run. No endpoint was contacted during review.

Exact candidate `5a7e7ec` then passed launcher smoke at
`/Users/michael/Dev/blit_v2_5a7e7ec/logs/otp12pf-rigw-20260715T162815Z-launcher`
and complete preflight at
`/Users/michael/Dev/blit_v2_5a7e7ec/logs/otp12pf-rigw-20260715T162853Z-preflight`.
The registered session at
`/Users/michael/Dev/blit_v2_5a7e7ec/logs/otp12pf-rigw-20260715T162938Z`
started exact block-1 daemons and executed its first arm,
`b1_wm_tcp_mixed_p1_source_init`, but voided before durability or a
`runs.csv` append: its post-client settle was 1247 ms, outside `[250,1000)`.
The CSV remains header-only, so no datum was accepted, analyzed, or graded.
Identity-scoped teardown closed both registered ports and left no benchmark
process. Failure evidence remains at that q path, q module session
`/Users/michael/blit-bench-work/rigw-sessions/20260715T162944Z.13808`, and
Windows session `D:/blit-test/rigw-pf1/20260715T162944Z.13808`.

G11 is instrument overhead, not transfer behavior. The three sequential
after-clock SSH/PowerShell probes each took roughly 360–400 ms RTT and consumed
about 1.1 seconds after the client anchor by themselves. G11 keeps three
independent q-before/Windows-clock/q-after samples but carries them over one
indexed persistent channel per phase. Binary nonblocking reads enforce one
overall deadline and bounded output while draining stderr; exact EOF, clean
exit, and child reaping are required. The preflight gate now times the complete
recording path and refuses it unless all three samples fit the 750 ms settle
headroom. Bash 3.2 self-tests reject wrong indices, malformed/extra output,
nonzero exit, partial-line timeout, early EOF, and surviving children. Restoring
the exact three-SSH implementation turns the one-launch guard red; restoration
is green. Syntax, format, strict clippy, all workspace tests, all 23 analyzer
tests, and diff checks pass. External Grok or Claude review remains before a
new exact candidate can be staged. No retained evidence, Time Machine setting,
or mount state was changed.
G11 was fixed at `d1686a009a301e01c9253f055524708d0bbc9a11`.

Round-11 independent Grok reviewed the immutable
`5a7e7ec3dcaa4965ba7fe2bce57686f5acb05549..aa0785c6f2bd1e3133bf288dabffd67930496440`
range. Its first response claimed acceptance and guard confirmation in about
six seconds without using a tool or running the required proof, so it was
discarded fail-closed. The authoritative retry ran the syntax gate, complete
Bash 3.2 self-test, and all 23 analyzer tests green; replaced only
`record_clock_samples` with the exact base three-SSH body and observed the
one-channel guard fail; then restored the exact reviewed bytes and returned
the complete gates green. It audited the sampler failure/reaping paths, Bash
3.2 row validation, full-path 750 ms preflight gate, and unchanged placement
and accounting, and returned schema-valid `ACCEPTED` with exact SHAs and
`guard_confirmed=true`. The retained detached worktree ended clean at
`aa0785c` with script SHA-256
`af13f4d5dace4ad1933d85acee1950e6030302b154881f6f22c55643dab39562`.
No endpoint was contacted. Exact candidate `aa0785c`, not the later review
record, is the only build allowed into additive staging, launcher smoke,
preflight, and the registered retry.

Exact candidate `aa0785c` was built and staged additively from clean detached
clones. q's real Bash 3.2 self-test passed; launcher smoke at
`/Users/michael/Dev/blit_v2_aa0785c/logs/otp12pf-rigw-20260715T171555Z.15522`
and standalone preflight at
`/Users/michael/Dev/blit_v2_aa0785c/logs/otp12pf-rigw-20260715T171647Z-preflight`
passed, with the complete batched clock path at 396 ms and 403 ms. The
registered session at
`/Users/michael/Dev/blit_v2_aa0785c/logs/otp12pf-rigw-20260715T171727Z`
completed `b1_wm_tcp_mixed_p1_source_init` at transfer/settle/flush/total
`1027/447/86/1310` ms and appended its provisional row. The paired
trace-off q-client arm then completed drain and its three before-clock samples
but failed before launching Python or blit: macOS Bash 3.2 with `set -u`
rejects expansion of the empty `trace_env` array. The EXIT trap recorded
`SESSION-VOID`; `runs.csv` contains one of 128 rows and `clock-samples.csv`
contains nine of 768 samples. The analyzer and finalizer never ran, no
`SESSION-COMPLETE` exists, and zero session data was accepted, analyzed, or
graded. Both registered ports are closed, no benchmark process remains, and
the q and Windows session evidence is retained under session tag
`20260715T171727Z.16108`.

G12 makes the invoked command array permanently nonempty by placing `env -u`
and both trace-variable names in its fixed prefix; trace-on appends the two
exact assignments. The executed Bash 3.2 guard poisons both parent variables,
proves trace-off removes them and trace-on replaces them exactly, and checks
the real wrapper sentinel and child status. Restoring only the live-failing
empty-array implementation reproduces `trace_env[@]: unbound variable` and
turns the new guard red; restoration returns syntax, the complete harness
self-test, and all 23 analyzer tests green. A whole-script audit found no
second empty user array; `SSH_MUX` is nonempty at every assignment. No retained
evidence, Time Machine setting, or mount state was changed.
G12 was fixed at `cd78ab945372c8fd3bfeed8f2502c13b6a919d5c`.

Round-12 independent Grok reviewed exact immutable range
`aa0785c6f2bd1e3133bf288dabffd67930496440..d5e9ddadc766534cdb498a9f55a550dbf22bc5e8`.
The first invocation was mistakenly interrupted after completing its green,
red-mutation, restoration, and final green/clean proof but before returning a
verdict. At the owner's direction the same retained session resumed and
returned a schema-valid `ACCEPTED` with exact SHAs and
`guard_confirmed=true`. It reproduced the Bash 3.2 nounset failure by
restoring only the exact old empty-array body, restored the reviewed bytes,
and returned syntax, the complete self-test, and all 23 analyzer tests green.
It audited the whole-script array class and found no second empty expansion,
and confirmed the range does not alter roles, endpoint-local measured paths,
workers, schedule, timing, trace schema, or analyzer math. The detached
worktree ended clean at `d5e9dda`; script Git blob was `d3f2fb3` and SHA-256
was `5e3f3aa802b9b9bd92f9673b0b31ce7166046fa00e2e5d8cd9aef6a0f2559c95`.
Exact candidate `d5e9dda`, not the later verdict-record commit, is cleared for
additive build/staging and SSH-driven launcher, preflight, and registered-run
gates.

Exact candidate `d5e9dda` was bundled and built from a clean detached clone,
then staged only under new SHA-named q and Windows paths. q's actual Bash 3.2
self-test passed. Launcher smoke at
`/Users/michael/Dev/blit_v2_d5e9dda/logs/otp12pf-rigw-20260715T180301Z.17538`
and standalone preflight at
`/Users/michael/Dev/blit_v2_d5e9dda/logs/otp12pf-rigw-20260715T180338Z-preflight`
passed exact provenance, fabric, quietness, lifecycle, and fixture gates; the
complete clock path took 414 ms and 404 ms.

The registered run at
`/Users/michael/Dev/blit_v2_d5e9dda/logs/otp12pf-rigw-20260715T180425Z`
(session `20260715T180421Z.18113`) cleared G12 live: all eight pair-1 arms,
including both q-client destination-initiated arms, completed with exit zero,
verified drain/durability, and landed manifests. Immediately before pair 1,
q load1 was 1.79 with Spotlight at 0.0%, Time Machine disabled/stopped, and no
known conflicting process. The eight owned transfers then ran for 74 seconds;
the pair-2 entry gate sampled the one-minute load history at 3.19 and killed
the session against the fixed 3.0 ceiling. The session is `SESSION-VOID` with
eight of 128 rows and 48 of 768 clock samples. No analyzer or finalizer product
exists, no `SESSION-COMPLETE` exists, and nothing was accepted, analyzed, or
graded. Both ports closed, the exact daemon PIDs are absent, and q plus both
Windows evidence trees remain retained.

G13 is an instrument defect: the same immediate one-minute load gate used in
preflight was also called after every eight owned transfers, so it treated the
benchmark's own runnable/I/O history as new external contamination. The fix
keeps the registered 3.0 ceiling and every per-round check. Preflight still
fails immediately above the ceiling; runtime checks now poll every five
seconds for at most 120 seconds, rechecking conflicting processes, Time
Machine, and Spotlight on every sample. Self-generated load history can decay
outside every timed arm, while persistent load still fails closed at the same
bar. Executed Bash 3.2 guards prove a 3.19→3.05→2.95 sequence takes exactly two
polls, immediate mode rejects 3.19, persistent 3.19 exhausts the bound, every
run-block quiet check requests runtime recovery, and the end gate does too.
Restoring only the exact old one-shot `q_quiet_gate` body makes the transient
recovery guard fail with `runtime q load gate did not wait through
self-generated load history`; restoring the bounded implementation returns
syntax and the complete self-test to green. Format, strict clippy, all
workspace tests, all 23 analyzer tests, the docs gate, and diff checks pass.
G13 was fixed at `0cbb16a295dd7a2104355af1799cb35b3c325701`.

Round-13 Claude Fable 5/max reviewed exact immutable range
`2c8e8d9284fc9ab5d6511f506de3b611c5b12e40..d7345f19299e1d90af0325894f4de497b0a1e88c`
in retained detached worktree `/tmp/blit-review-g13-d7345f1` and returned a
schema-valid `accepted` verdict with exact SHAs and `guard_confirmed=true`.
Syntax, the complete Bash 3.2 self-test, all 23 analyzer tests, docs, and diff
checks were green. Claude kept every new G13 test and replaced only
`q_quiet_gate` with the byte-exact base implementation; the complete self-test
failed on the targeted `runtime q load gate did not wait through self-generated
load history` guard. It restored exact reviewed bytes, verified script
SHA-256 `febc195282feafd9e6bd25fe8a00aec153e8a515d3f8aa809590d4f8d1ab3a9c`,
reran syntax and the complete self-test green, and ended clean at `d7345f1`.
No benchmark endpoint was contacted. Exact candidate `d7345f1`, not the later
verdict-record commit, is the only build allowed into additive staging and the
live retry. Full record:
`.review/results/otp12-pf1-rigw-harness-r13.claude.json`.

Exact accepted candidate `d7345f1` was bundled and cloned only under fresh
additive paths. Bundle SHA-256 was
`6600f2fe1e5c121d36e4dee0781d452fdf0803304b7a08bef95692b9bea12509`
on local, q, and Windows. Local/q arm64 binaries matched at
`afc075131192d76e88f0bb67a03fd46ecdbc1821f9e74cbc629547efcf4ed6cd`
and `0054ad7575d4318108c5e430b642a4224172c938869e367222d9a787afe69377`;
native Windows staged binaries were
`293af62459a900b61ca7ced375538cead4f837f213ad144c9cc5c6df6e9396c3`
and `7224a5e49ba26718bcf105d34d10c32d5d681ee5abaff3ba4b28f1af52ab13ef`.
Every clone was clean at exact HEAD with no replacement ref, all binaries
embedded clean `+d7345f19299e`, and Windows' CRLF checkout filtered back to
the exact reviewed script blob `eb0db17`.

q's real Bash 3.2 self-test passed. Launcher smoke at
`/Users/michael/Dev/blit_v2_d7345f1/logs/otp12pf-rigw-20260715T191455Z.20128`
and standalone preflight at
`/Users/michael/Dev/blit_v2_d7345f1/logs/otp12pf-rigw-20260715T191532Z.20431`
passed exact provenance, fabric, lifecycle, quietness, fixtures, and 397/401
ms clock batches.

Two fresh registered sessions then reproduced a second runtime gate defect at
the same boundary. Session `20260715T191605Z.20751` completed pairs 1 and 2,
with q Spotlight at 0.0% before pair 1 and 1.0% before pair 2, then voided at
pair 3 on 58.7%. After teardown, six consecutive five-second samples were
0.0–0.1% Spotlight with load1 1.62–1.76. Fresh session
`20260715T192044Z.22145` again completed exactly pairs 1 and 2, with Spotlight
0.0% then 1.2%, and voided at pair 3 on 87.3%. Each retained `SESSION-VOID`
has 16 of 128 rows and 96 of 768 clock samples; no analyzer/finalizer ran and
nothing was accepted, analyzed, or graded. Both teardowns closed both ports
and left no daemon/cargo/rustc process; all q and Windows evidence trees remain.

G14 is the runtime recovery gap exposed by delayed Spotlight work following
the harness's own q destination-tree writes. The repeated schedule boundary
and direct filesystem mechanism make
that operational classification high-confidence; the retained logs do not
provide per-path attribution or an exhaustive unrelated-process inventory, so
they do not prove exclusive causality. The fixed 10% ceiling remains unchanged.
Preflight still rejects an above-ceiling sample immediately; runtime checks
outside timed arms may now
use the same existing five-second/120-second recovery budget when either load
history or Spotlight is high. Conflicting processes, Time Machine, load
parsing, and Spotlight are rechecked every sample, and persistent or malformed
contamination still fails closed. Bash 3.2 guards separately cover transient
Spotlight, staggered load/Spotlight, immediate rejection, malformed samples,
exact bounded persistence, exact poll cadence, and Time Machine or conflicting
processes emerging on a recovery poll. Restoring the exact G13
immediate-Spotlight gate while keeping the new tests makes the complete
self-test fail on `runtime q Spotlight gate did not wait through post-transfer
indexing`; delaying process checks past the next poll and over-advancing
elapsed recovery time each make their targeted guards fail. Exact restoration
returns syntax and the complete self-test green. No
role, endpoint-local path, worker, schedule, measured-arm timing, trace schema,
analyzer math, threshold, retained evidence, Time Machine setting, or mount
state changed.
G14 was fixed at `942c88e601ea2d27f0a1da52aa5408b763ee61f4`.

Round-14 Claude Fable 5/max reviewed exact immutable range
`35f160f03d6dedc6beb841a3033076dab2b6e07c..1f62ce564f127e18f5e0f044dd9fd4605f3a610c`.
The first proxy-routed invocation returned no verdict. At the owner's
instruction, the exact stale process was terminated after the proxy was
removed; its worktree, prompt, and debug log remain retained as
non-authoritative evidence. A fresh invocation in retained detached worktree
`/tmp/blit-review-g14-1f62ce5-r2` returned exit zero and a schema-valid
`accepted` verdict with the exact SHAs and `guard_confirmed=true`.

Syntax, the complete Bash 3.2 self-test, all 23 analyzer tests, docs, and diff
checks were green. Claude kept every new G14 test through three independent
production-only mutations: exact G13 `q_quiet_gate` failed the post-transfer
Spotlight guard; ten-second offender polling failed the next-poll offender
guard; and over-advanced elapsed accounting failed the exact-bound guard.
Exact restoration after each mutation returned syntax and the complete
self-test green. Final script blob
`f3a6195e121eaae74eb715a798e5e2d1aef70edb`, SHA-256
`21f4b686fdcd5ddfb47e8478a8666e5d10d0f9e838db2bf2f7227d0da75852d8`,
matched reviewed HEAD byte-for-byte; the worktree ended clean. No benchmark
endpoint was contacted and no retained artifact was deleted. Exact candidate
`1f62ce5`, not the later verdict-record commit, is the only build allowed
into additive staging and the live retry. Full record:
`.review/results/otp12-pf1-rigw-harness-r14.claude.json`.

Exact Claude-accepted candidate `1f62ce5` was bundled and staged under fresh,
additive q and Windows paths. q's real Bash 3.2 self-test, SSH-driven launcher
smoke, and standalone preflight all passed exact provenance, 10 GbE/MTU/MSS,
quietness, lifecycle, fixture, and clock gates. Registered session
`20260715T203809Z.25897` then completed all 32 block-1 arms with exit zero,
verified drain and durability, matching landed manifests, and all 192 expected
clock samples. G14 worked live: it recovered once from 44.1% Spotlight and
again from load1 4.61 plus 51.5% Spotlight, holding until both values were
simultaneously below the unchanged ceilings. The end fabric and quiet gates
passed.

The session nevertheless became `SESSION-VOID` when the analyzer correctly
rejected 32 rows against the registered 128-row schedule. The controller fed
the four-line block schedule to a `while` loop through standard input, while
ordinary noninteractive `wssh` calls inherited that same descriptor. The
first Windows quiet probe in block 1 therefore consumed the three remaining
schedule records; the loop reached EOF normally and invoked the full-session
analyzer after only block 1. This stdin-ownership bug originated with the
initial harness commit and every prior registered candidate carried it, but
all earlier live sessions stopped before the first block boundary. The
analyzer remained correctly fail closed. No `SESSION-COMPLETE` or analyzer
report exists, and nothing was accepted, analyzed, or graded. Local evidence
and both endpoint session trees remain retained; both ports are closed and no
benchmark process remains.

G15 makes the ordinary Windows command wrapper invoke `ssh -n`, detaching
stdin without changing the separately batched clock sampler's intentional
pipe or `scp`. Its executed Bash 3.2 guard runs a main-shaped four-record loop
against an SSH shim that drains stdin unless `-n` is present and requires
blocks 1, 2, 3, and 4 to survive. Removing only `-n` makes the guard fail on
`noninteractive Windows SSH consumed the registered block schedule`; exact
restoration returns syntax and the complete self-test green. Format, strict
clippy, the full workspace suite, all 23 analyzer tests, docs, and diff checks
pass. No transfer role, endpoint-local path, worker policy, schedule, measured
arm, trace schema, analyzer rule, threshold, retained artifact, Time Machine
setting, or mount state changed. Final script blob is `ae60920`; SHA-256 is
`85cda14fbeecb9446b1ad2462f938e5cf397f2f042a0d106690b6332c295a05d`.
G15 was fixed at `7bdaf8bda5919f2ed03a17709baf6d4aefabe8e0`.

Round-15 Claude Fable 5/max reviewed exact immutable range
`af2ffb5fb4c57bfd4b4f18371fd9ace5d7431b39..8e019ef5e948b94a7aca7cb3a8d0be41204742af`
in retained detached worktree `/tmp/blit-review-g15-8e019ef` and returned exit
zero plus a schema-valid `accepted` verdict with exact SHAs and
`guard_confirmed=true`. It confirmed the schedule-stdin diagnosis, the
analyzer's correct 32/128 fail-closed rejection, and that `-n` applies only to
ordinary noninteractive `wssh`; `wscp` and the separately piped batched clock
sampler are unchanged. Its whole-loop audit found no second schedule-stdin
consumer.

Syntax, the complete Bash 3.2 self-test, all 23 analyzer tests, docs, and diff
checks were green. Keeping every G15 test, Claude removed only the production
`-n`; the complete self-test failed exactly on `noninteractive Windows SSH
consumed the registered block schedule`. It restored exact reviewed bytes,
reran syntax and the complete self-test green, verified script blob
`ae60920a45322f6ad4fa550408118d4d17cb56dc` and SHA-256
`85cda14fbeecb9446b1ad2462f938e5cf397f2f042a0d106690b6332c295a05d`,
and ended clean at exact `8e019ef`. No endpoint was contacted and no retained
artifact was deleted. Exact candidate `8e019ef`, not the later verdict-record
commit, is the only build allowed into additive staging and the live retry.
Full record: `.review/results/otp12-pf1-rigw-harness-r15.claude.json`.
