# Release readiness

**Status:** Active release ledger
**As of:** hosted run `29950306815`; repairs through `2a27c4b`, 2026-07-22

This is the concise release boundary after D-2026-07-22-3. Every known broken
behavior is release work regardless of its internal classification. Optional
performance ceilings and hardware tuning remain post-release work.

## Proven

- Local formatting, strict workspace clippy, workspace tests, and docs checks
  passed for the w10 candidate on 2026-07-22.
- Linux and macOS tests, formatting, and clippy passed on published GitHub head
  `dcf9245` in CI run `29584631185` on 2026-07-17.
- ldt-1 through ldt-3 established one SOURCE-owned adaptive controller,
  acknowledged ADD/REMOVE membership, role-parity tests, and lifecycle closure.
- The historical P1 initiator discrepancy is closed without another transfer:
  the pre-fix code made SOURCE/DESTINATION initiation settle at 3/2 workers,
  `a76b785..42b9b38` mutation-proved parity, post-fix `8e019ef` no longer
  failed the target cell, and ldt-2 retains adaptive role parity. Evidence:
  `docs/bench/p1-evidence-reconciliation-2026-07-22/`.
- Every complete ldt-4 live payload had exact manifest identity and normal
  endpoint restoration. The complete and partial session classification is in
  `docs/bench/ldt4-evidence-audit-2026-07-22/`.
- rel-1 replaced the Windows-sensitive socket-buffer timeout test with a
  deterministic blocked in-memory writer and mutation-proved the timeout arm.
- rel-1b replaced temporary-daemon TCP readiness with a bounded `ListModules`
  identity check against the fixture's unique canonical module root. Its guard
  rejects a foreign listener under mutation proof; all 23 `blit_utils` tests
  pass locally.
- rel-2 reconciled retained P2 evidence to two exact TCP-path deltas: unified
  payload waited for `ManifestComplete` instead of overlapping scan/diff, and
  tar receive claimed the need-list mutex once per file instead of once per
  shard. Both are fixed with mutation-sensitive role/operation-count guards;
  no hardware transfer was rerun. Finding:
  `release-p2-tcp-small-file-regression`.
- rel-3 reconciled the Windows directory-move timeout to the old daemon's
  `nested\c.txt` need-list echo, not a source-delete handle. `48c5a11` fixed
  that native-separator mismatch and the unified session later deleted the old
  path. The exact nested push-move test passed on hosted Windows in run
  `29944148295` at `28cf989`.
- rel-4 defines contract v5 and implements bounded Windows attributes and named
  `$DATA` streams across local, TCP, in-stream, tar, and resume carriers. The
  destination retains and validates manifest descriptors before applying any
  payload, metadata mismatch overrides an ordinary content skip, non-Windows
  destinations refuse before creating a partial file, and ADS bytes count as
  payload. The guards fail under metadata-diff and hash-drift mutations; local
  format, strict clippy, workspace tests, docs, and strict all-target Windows
  cross-compilation pass. Hosted Windows run `29944148295` at `28cf989` passed
  both local and remote single/tar attributes + ADS filesystem guards.
- Hosted run `29944148295` then exposed a terminal live-dial race in both role
  layouts: shutdown could leave a resize claim pending without a driver owner.
  `309f8b6` reconciles that ownerless claim at the unchanged membership after
  the tuner, proposal queue, and wire-owned request are closed. Its new
  deterministic two-layout guard failed before the repair and passes after it.
  `f679a1a` also preserves stdout/stderr when the Linux remote-move guard fails.
- Hosted run `29945332738` passed docs, formatting, strict lint, and Linux. Its
  macOS failure was a source-side scheduling race: the destination could answer
  a queued `ManifestComplete` before the source resumed to mark it sent.
  `d08741b` marks the ordered queue boundary before awaiting the send; a valid
  push and the genuinely-premature-message guard both pass locally. Windows
  then failed a source-initiated 10,000-file role guard after its send pipeline
  closed, but the session replaced the worker cause with that queue symptom.
  `833a859` restores the existing first-error contract; its mutation guard fails
  with the generic symptom and passes when the worker cause is preserved.
- Hosted run `29946548540` confirmed the prior macOS manifest race no longer
  occurred. Linux and macOS instead caught that `833a859` retained the worker
  message but dropped its typed file identity; `2b35c04` preserves both, and the
  exact daemon end-to-end guard passes locally. Windows exposed the worker cause
  as a destination-forced TCP close during tar writes in two SOURCE-initiated
  workloads. `82e3cb0` makes every paired role assertion print both completed
  endpoint results, so the next hosted failure will retain the destination's
  reason instead of stopping at the source result.
- Exact-head run `29947092127` passed check, Linux, and macOS, including the
  prior manifest and structured-path failures. Windows passed the SOURCE-
  initiated 10,000-file session but caught an interrupted resize settlement in
  the mirrored layout: elastic membership reached 7 while the dial remained at
  6. The manifest scan raced ACK processing against its next header, allowing a
  ready header to cancel the ACK after worker admission but before dial
  settlement. `8fb0a0d` selects only raw inputs and completes the chosen state
  transition outside the race. Its two-byte guard times out with the old
  selection and passes in 0.12 seconds with the fix; no hardware transfer ran.
- Exact-head run `29948151621` at `532ece0` passed check and the complete Linux,
  macOS, and Windows test matrix. Both release executables now expose the exact
  shared session build identity through `--version` at `4bb3389`. Focused CLI
  and daemon parser guards fail when that interface is removed and pass with
  exact output when restored; formatting and strict workspace clippy pass.
- Run `29948702562` passed check, macOS, and Windows. Linux forced a payload
  worker failure to win just before the source sealed membership: the failed
  `Seal` send then replaced the worker's structured `big.bin` error. `fa79f0a`
  joins the already-failed pipeline at that boundary and returns its original
  file/IO fault. The forced-order guard fails with the old masking behavior;
  the guard, exact end-to-end test, formatting, and strict lint pass after the
  fix. Release-build jobs correctly remained blocked after the failed matrix.
- Exact run `29949207219` at `354f38e` passed check, Linux, ARM macOS, and
  Windows, then built and uploaded all three target packages. Linux/macOS are
  `.tar.gz`; Windows is `.zip`; each contains both executables, release
  documents, and `BUILD.txt` with the full commit, and each has an uploaded
  SHA-256 sidecar. Missing binaries or empty output fail the package job.
- `4062947` adds one cross-platform packaged-release runner before artifact
  upload. It verifies the sidecar and safe extraction; full and embedded commit
  identity; CLI/daemon version and help; an owned loopback daemon's module
  readiness; one tiny local and one tiny same-build remote copy with exact byte
  equality; and bounded teardown. Its checksum/path-safety guards pass, and the
  checksum-name guard turns red when mutated. The previously uploaded ARM
  macOS archive passed the full runner locally with SHA-256
  `a1913899649ca1a633306dcb6f5b727f66d254c1fd3763388ae6769828b5364d`.
- Run `29950306815` at `0e61ac8` passed check and all three OS suites. Packaged
  smoke passed on ARM macOS (`b5d9e94a…963e4f42`) and Linux
  (`e962e8a0…4bc26792`). Windows started the packaged daemon and received the
  exact configured module, but its standard canonical `\\?\C:\…` path spelling
  did not string-compare equal to the ordinary `C:\…` spelling. `4927a05`
  normalizes extended drive and UNC prefixes; its exact guard fails when the
  normalization is removed and passes when restored. `2a27c4b` upgrades every
  checkout plus artifact upload to the official Node-24 majors after the same
  hosted logs identified both deprecated actions.
- All six formal rel-4 review corrections are fixed one per commit with focused
  mutation proofs. The final allocation fix moves the destination resume-hash
  vector through metadata hydration and directly into the in-stream block diff.
- Delegated transfers now emit cumulative byte snapshots while the transfer is
  live and once more before the summary when the final count changed. The
  existing RPC consumer converts them to CLI/TUI deltas; disabling the periodic
  producer makes the focused paused-time guard fail.
- Served sessions now feed the same jobs-row byte counter in both responder
  roles: destination writes report directly, while source payload events relay
  into the counter. A two-role loopback guard requires each completed record's
  byte count to match its transfer summary.
- rel-5 now accounts for the declared byte/file denominator, completed
  bytes/files, and final TCP versus in-stream carrier in one shared job row.
  GetState snapshots, periodic and terminal events, persisted recents,
  delegated cumulative updates, CLI JSON/human output, and TUI rows retain and
  render the values. Mutations disabling manifest totals, file completion,
  carrier convergence, or delegated denominator consumption each make a
  focused guard fail.
- The consolidated CLI integration harness now drains each temporary daemon's
  stderr concurrently into a bounded 256 KiB tail. Startup failure kills and
  reaps the child, waits for the drain, and includes the actual process stderr
  in the panic; unrelated test panics also print retained daemon diagnostics.
  A transient early exit retries on two fresh ports before failing. The first
  full gate exposed the previously hidden cause (`Address already in use`);
  after the retry fix, the complete suite passed. The invalid-option guard
  fails if stderr capture is omitted or the retry budget is reduced to one.
- Session mirror and explicit daemon purge now use one contained deletion
  executor. It preserves filtered-mirror scope and cancellation while making
  recursive purge clear Windows read-only trees before deletion. Opposite-mode
  mutations make the existing filtered-mirror and recursive-purge guards fail.
- Same-tree CLI and daemon builds now derive one stable handshake ID even when
  Cargo profiles rebuild separately. The identity is scoped to executable
  inputs, watches the complete input set on every build, and hashes both tracked
  diffs and actual untracked-file bytes. Documentation-only changes no longer
  cause false `BUILD_MISMATCH` failures. Both rules fail under mutation, and the
  real three-case containment suite passes from one dirty tree without forcing
  either profile to resample.
- The recorded filter-normalization split is no longer present. Both session
  roles validate peer globs at open through `filter_from_spec`, filtered mirror
  deletion converts through that helper, and the old hand-mapped push handler
  was deleted at the one-path cutover. Existing malformed-glob and two-role
  filter/mirror guards pass.
- Filesystem-to-wire metadata and request-path conversion now have one owner in
  blit-core. Transfer manifests, daemon admin responses, delegated requests,
  app listings, and TUI local rows share the same signed-mtime and POSIX-path
  rules; Unix transfer permissions share the same owner as well. The old
  per-crate converters and manual slash joins are deleted.
- Byte counts and throughput now use one binary-unit presenter across jobs
  watch, local-transfer summaries, and all five TUI surfaces. The decimal jobs
  formatter and private screen ladders are deleted; reverting the shared
  binary divisor makes its boundary guard fail.
- The abandoned tar streamer, old delete planner, chunked-copy variant, and
  dead enumeration helpers and exports are deleted. The earlier otp-11b slice
  had already removed parallel-copy/stat leftovers. A source-structure guard
  fails if any retired path or public name returns.
- The recorded zero-caller control-plane payload sender is already absent:
  otp-10c-2 deleted it with the legacy gRPC sinks, fallback module, and four
  old drivers. Current payload code contains only session-used preparation and
  planning helpers.
- Jobs watch no longer accepts the inert polling-interval option; its help now
  describes stream updates. Unused/runtime-misplaced CLI dependencies, the
  empty app transfer module, unused perf report wrapper, genuinely dead daemon
  helpers, and stale dead-code suppressions are removed. Restoring the retired
  flag makes its parser guard fail.
- Daemon filesystem errors now retain actionable gRPC codes: missing paths are
  NotFound and permission failures are PermissionDenied. Other wrapped errors
  keep their complete source chains. Both mapping rules fail under mutation,
  and current admin handlers prove the missing-path behavior end to end.
- A dropped admin response stream is now a `Cancelled` request with one honest
  peer-disconnect message, not an `Internal` daemon failure. Both current
  stream handlers prove that boundary, and the status-code mutation fails.
- The copy engine no longer accepts a logger that was always a production
  no-op. Errors still return with caller context; the unused file logger and
  its blit-core dependency are gone and guarded against reintroduction.
- Public reliability help now states the actual unified-session contract:
  retry re-applies the selected destination comparison, while `--resume`
  continues eligible partial files block-wise for local, push, pull, and
  remote-to-remote. A generated-help test failed before the correction and
  passes after it. False
  Phase 2 shipped claims, the nonexistent `FileStream` payload description,
  and the deleted static-tuning path are corrected to current or explicitly
  historical truth. No transfer or hardware test was used.

## Release blockers

1. **Hosted install/startup smoke is pending.** ARM macOS and Linux passed the
   full runner at `0e61ac8`; `4927a05` fixes the Windows-only canonical-path
   comparison. The next exact head must pass all three before each upload.
2. **The final release-candidate head is pending.** The smoke implementation
   and truthful release notes must land, then the complete check/test/package/
   smoke matrix must pass at that exact clean commit.

## Deferred until after release

- ldt-4 causal tuning, unexplained ceiling asymmetries not tied to a known
  broken behavior, MTU follow-ups, and further hardware matrices.
- Mac↔Mac Thunderbolt ceiling testing.
- Small-file and zero-copy performance work, performance refactors, and other
  ceiling optimization.

No new hardware result is required to make the release decision.
