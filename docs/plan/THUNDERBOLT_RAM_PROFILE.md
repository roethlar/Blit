# Mac-to-Mac Thunderbolt RAM-path profile

**Status**: Active
**Created**: 2026-07-23
**Supersedes**: nothing
**Decision ref**: D-2026-07-23-1

## Goal

Attribute the exact 0.1.1 candidate's remaining gap between the measured
37.9 Gb/s same-direction TCP ceiling and its 28.6 Gb/s warm large-file result
without another SSD-backed destination or performance matrix. One instrumented
Q-to-Nagatha transfer must distinguish fixed session startup, live-dial
behavior, sender CPU, receiver CPU, and steady data-path backpressure well
enough either to identify the next tuning target or to state exactly what the
existing observer cannot separate.

## Non-goals

- No product, test, harness, analyzer, wire, configuration-default, or tuning
  code change.
- No rsync arm, iperf repeat, reverse direction, A/B matrix, second sample, or
  automatic retry.
- No SSD-backed destination, physical source-fixture copy, durability test,
  small-file conclusion, release verdict, or publication action.
- No tuning conclusion from elapsed time alone. A later code change requires
  its own Active implementation plan and guard proof.

## Constraints

- Candidate identity is `d1f1152dd16b8c2bf8409cb5637135e3f89068c0`.
  Required artifact hashes remain:
  - archive: `d1d7d9e547f703a7b5216cb3227baaf6b2bea848a85599312439cdccff19b726`;
  - `blit`: `dc3cd55ad10903ef695db904f97ea3f6c0c7e6a300e4a163b95e766bced3cca0`;
  - `blit-daemon`: `652a8e641d1211ab9d4a254f56b6f2d9db0626c71a39d8861fee522ebbc74018`.
- Physical direction and route stay Q `172.31.254.2/30` to Nagatha
  `172.31.254.1/30` over `bridge0`, MTU 1500. Re-prove route, peer identity,
  40 Gb/s negotiation, and one source-bound ping; do not rerun iperf while the
  cable and route remain stable.
- Recreate the earlier logical 8 GiB source shape as eight APFS clones of Q's
  retained 1 GiB seed. Every clone must be exactly 1,073,741,824 bytes and
  match SHA-256
  `cb3db617ccc43978fad2e426c45510fcc9df5a5e83bda281d7e8c94c8fae28cc`.
  Hash all clones before timing so their one shared physical extent is warm.
- The destination is a fresh 12 GiB APFS RAM disk on Nagatha. All daemon
  config, PID, and active-run scratch belong on that RAM disk. No benchmark
  payload may land on either SSD.
- APFS clone metadata, one exact client executable if Q has no retained copy,
  and captured text evidence are the only permitted SSD writes. Their combined
  newly allocated size must remain below 32 decimal MB. Stop before timing if
  source allocation indicates physical payload copies rather than clones.
- Enable `BLIT_TRACE_SESSION_PHASES=1` with one session-unique
  `BLIT_TRACE_RUN_ID` on both endpoints. Capture the low-frequency structured
  phase records separately from ordinary stderr. Do not enable
  `--trace-data-plane`: its per-file human-readable output is explicitly
  separate from the performance observer and would add avoidable hot-path I/O.
- Wrap the Q client and Nagatha daemon in macOS `/usr/bin/time -lp`; retain
  user time, system time, elapsed time, maximum resident memory, page faults,
  context switches, and exit status. The daemon must be stopped and reaped so
  its accounting is complete.
- Run no build, review, indexing request, or other deliberate load during the
  timed transfer. Any identity, route, RAM-disk, listener, quietness, trace, or
  allocation failure stops before the transfer and does not authorize a retry.

## Acceptance criteria

- [ ] Exact candidate hashes, host identities, route, link, seed, source-clone
      allocation, RAM-disk identity, free memory, listener state, and run ID
      are recorded before timing.
- [ ] Exactly one 8,589,934,592-byte Blit transfer exits zero, reports eight
      files and `tcp_fallback: false`, and writes only to Nagatha's RAM disk.
- [ ] All eight destination files have the expected size and SHA-256 before
      the RAM disk is detached.
- [ ] Source and destination phase traces correlate on one session ID and
      record connection/startup events, first queue/write/receive events, every
      dial sample actually emitted, all decisions or settlements, and terminal
      state. A zero-sample trace is accepted only when the phase ordering and
      code prove payload admission closed before the first 500 ms tuner tick;
      it is evidence of the controller lifetime, never a reason to repeat.
- [ ] Complete client and daemon `/usr/bin/time -lp` records exist. Analysis
      reports total throughput, fixed pre-first-byte time, sampled steady-state
      bytes and blocked ratios, live/peak/final streams, chunk/prefetch values,
      client and daemon CPU cores consumed, and peak RSS.
- [ ] The verdict names the evidenced dominant class: fixed session overhead,
      sender/user-kernel work, receiver/user-kernel work, socket backpressure,
      tail/work distribution, or unresolved observer gap. It must not name a
      product fix without evidence that excludes the other classes.
- [ ] The exact source clones, staged client copy, daemon, listener, session
      scratch, and RAM disk are removed or stopped. The retained seed, static
      Thunderbolt addresses, candidate artifact in Trash, and prior evidence
      remain untouched.
- [ ] Evidence and limits are committed; `bash scripts/agent/check-docs.sh`
      and `git diff --check` pass. No Rust/proto or other code changes exist.

## Design

Reuse the fixture and direct-link method from
`docs/bench/thunderbolt-macmac-2026-07-22/`, changing only observation. The
candidate daemon receives into RAM exactly as before. Q emits the structured
session-phase trace, which already includes per-tick bytes, socket-write
blocked nanoseconds, elapsed nanoseconds, stream membership, chunk size,
prefetch, TCP buffer, and controller reason. Nagatha emits the corresponding
session and socket milestones. Process resource accounting brackets each
binary without modifying it.

The exact eight-file workload can fit entirely inside the four-worker,
16-payload bounded send pipeline. Retained ldt-4 evidence already showed that
the tuner lifetime follows payload admission rather than the later socket
drain. Therefore this profile pre-registers two valid observer outcomes:
samples emitted before admission closes, or no samples with phase evidence
that admission closed inside 500 ms. The latter identifies a controller-
lifetime limit; it does not authorize reshaping the fixture, extending the
payload, or rerunning.

Analysis first aligns both traces by run and session IDs, then calculates:

1. process start/session open to first queued, first socket write, and first
   receiver payload;
2. aggregate bytes and blocked ratio for every valid busy dial sample;
3. any cheap-dial or stream-membership change and the bytes moved before it;
4. client and daemon CPU seconds divided by wall time; and
5. the unexplained tail between the last full sample and command completion.

The historical 37.9 Gb/s Q-to-Nagatha iperf ceiling and 28.6 Gb/s candidate
result are comparison anchors, not rerun arms. If the default-off observer
lacks a necessary boundary, record that specific missing field as the result;
do not add instrumentation or repeat the transfer inside this plan.

## Slices

1. **tb-ram-profile-1 — plan and safety gate.** Commit this Draft, obtain the
   owner activation decision, and perform no endpoint mutation before Active.
2. **tb-ram-profile-2 — one instrumented RAM run.** Re-prove all gates, create
   only the bounded clone metadata and RAM destinations, run exactly once,
   validate bytes, and clean up exact paths.
3. **tb-ram-profile-3 — attribution record.** Analyze retained traces and
   process accounting, record the bounded verdict and limitations, close the
   plan as Historical, run docs verification, and commit.

## Open questions

- None. Activation is the only owner gate; any product change is outside this
  plan.
