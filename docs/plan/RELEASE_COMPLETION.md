# Complete the first cross-platform release

**Status**: Shipped
**Created**: 2026-07-22
**Supersedes**: performance work as the pre-release critical path under
D-2026-07-22-1; does not supersede shipped implementation plans
**Decision ref**: D-2026-07-22-3 activates this plan and makes every known
broken behavior release-blocking
**Owner decisions**: Windows attributes and ADS must be preserved across local
and remote transfers in the first release (approved 2026-07-22: "go"); every
known product defect, correctness/robustness gap, progress gap, CI failure,
documentation defect, and packaging/install/startup gap is release-blocking
(approved 2026-07-22: "there's no release until everything is fixed"); P1 must
be identified and fixed from retained evidence and code without another
physical transfer (D-2026-07-22-2); issue classification is internal
bookkeeping and never a reason to defer a known broken behavior (approved
2026-07-22: "all of them need to be fixed")

## Goal

Produce one installable, smoke-tested Blit release from an exact reviewed
commit for macOS, Linux, and Windows. The release boundary is data correctness,
bounded failure, supported-platform CI, packaging, installation, daemon/CLI
startup, and truthful documentation. It does not depend on another performance
or hardware experiment.

## Non-goals

- ldt-4 causal tuning, MTU follow-ups, Thunderbolt ceilings, optional throughput
  ceilings, competitor comparisons, or any other hardware benchmark before
  release. P1 is closed from offline evidence and deterministic guards; P2 is
  a measured regression and therefore remains in release scope.
- Improving throughput as part of a correctness fix.
- Publishing, tagging, pushing, or deleting any ref without a separate exact
  owner approval after the release candidate is complete.
- Treating analyzer, harness, or CI-tool rejection as automatic invalidation of
  complete raw evidence.

## Constraints

- No data-moving hardware benchmark or large SSD-write test. Verification uses
  unit/integration tests and small disposable temporary trees only.
- No new physical P1 transfer or endpoint experiment. Its closure is based on
  retained evidence, exact historical/current code, and mutation-sensitive
  role-parity guards (`docs/bench/p1-evidence-reconciliation-2026-07-22/`).
- The Mac has 16 GiB RAM; RAM-disk designs are not a substitute for bounded
  tests. No test may silently fall back to the internal SSD for a large write.
- One release blocker per commit. Each code fix receives a mutation-sensitive
  guard and the repository verification suite before the next blocker starts.
- Current local `master` is ahead of both published remotes. Outward git actions
  remain separately owner-gated.
- Same-build peers remain mandatory. A Windows metadata wire correction must
  update `docs/TRANSFER_SESSION.md` before code and need not preserve mixed-build
  compatibility.
- Never rerun an experiment merely because an analyzer failed to interpret
  complete, immutable, unambiguous evidence. Repair and reanalyze first.

## Acceptance criteria

- [x] The historical P1 initiator discrepancy has a concrete code cause and
      fix: the old path realized 3 versus 2 workers by role (plus a
      destination-only zero-capacity cap); `a76b785..42b9b38` proves parity,
      and ldt-2 preserves it under the current adaptive controller. No new
      physical transfer was used.
- [x] The Windows handshake-timeout guard deterministically exercises a blocked
      token write on every supported OS; its old Windows-success mutation turns
      the focused guard red.
- [x] Hosted CI is green on the exact release candidate for formatting, strict
      clippy, Linux tests, macOS tests, and Windows tests.
- [x] CLI integration fixtures positively prove their temporary daemon is ready
      before returning; repeated parallel workspace runs have no connection-
      refused startup failures. Daemon stderr is drained concurrently into a
      bounded tail, included in startup failures, and cannot block the child;
      transient early exits retry on fresh ports before the final failure.
- [x] The ignored Windows directory-tree move hang has a root-cause fix; the
      integration test is re-enabled and passes repeatedly on Windows.
- [x] Windows file attributes and ADS are preserved across local and remote
      transfers, including the tar threshold that currently changes fidelity;
      single-file and batched copies produce the same metadata result.
- [x] Delegated and served-session progress is complete: live bytes are emitted,
      daemon rows report accurate bytes, and total/completed file and byte
      denominators are nonzero and correct through GetState, TransferProgress,
      TransferComplete, CLI, and TUI consumers.
- [x] P2's measured unified TCP small-file regression has a named code cause
      and fix, with deterministic, mutation-sensitive software proof and
      retained-evidence reanalysis. It is not deferred as "performance," and
      it does not require a new data-moving hardware matrix.
- [x] Every open `REVIEW.md` item and current-state known issue is reconciled
      against code: fixed with a guard, proven already fixed and closed, or
      classified as a non-defect only by an explicit owner decision. No known
      product defect is deferred merely to reach the release.
- [x] Release artifacts are produced from one exact commit for
      `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
      `x86_64-pc-windows-msvc`, with embedded version/build identity and
      recorded checksums. Linux/macOS ship `.tar.gz`; Windows ships `.zip`.
- [x] Each artifact installs or runs in a clean temporary environment; CLI
      version/help, daemon startup/health, one small local copy, and one small
      same-build remote copy pass with exact content verification and clean
      teardown.
- [x] User-facing release notes state supported platforms, known limitations,
      compatibility, and deferred performance work.
- [x] The release-candidate tree is clean; all verification evidence names its
      exact commit. Publication remains blocked until the owner approves the
      exact refs and remotes.

## Design

### Release blocker ownership

- `crates/blit-core/src/remote/transfer/socket.rs`: replace the OS socket-buffer
  assumption in `dial_token_write_stall_times_out_bounded_and_retryable` with a
  deterministic blocked-write seam while preserving production timeout and
  retry classification.
- `crates/blit-cli/tests/remote_move.rs` and the unified local-source cleanup
  path: reproduce the Windows directory-tree move hang with bounded diagnostics,
  fix handle lifetime/cleanup ownership, and remove the Windows ignore only
  after the real behavior passes.
- `docs/TRANSFER_SESSION.md`, `proto/blit.proto`, Windows enumeration/copy
  support, and the file/tar payload and sink paths: extend `FileHeader`-owned
  metadata with bounded Windows file attributes and validated named ADS
  descriptors/content. Apply metadata only after the primary file lands;
  reject unsafe stream names, duplicate streams, size/count overruns, and
  incomplete stream records. Preserve identical local/remote semantics and do
  not encode ADS as ordinary destination paths. Do not couple this slice to
  local small-file throughput work.
- Daemon/delegated progress producers and `ActiveJobs`: close all three known
  gaps—live delegated `BytesProgress`, served-session byte counters, and exact
  byte/file denominators—through the RPCs, CLI, and TUI.
- P2's existing source-bookkeeping, claim-lock, tar-shard, and carrier observers
  plus exact historical code: identify the executed-path regression offline,
  replace the responsible code, and guard the corrected operation count or
  ordering directly. Retained timings are evidence; no fresh hardware matrix is
  an acceptance prerequisite.
- `.github/workflows/`, Cargo packaging metadata, release scripts, and operator
  docs: package `blit` and `blit-daemon` for the three existing CI target
  triples as `.tar.gz`/`.zip`, make construction and smoke evidence reproducible
  from one exact commit, and emit SHA-256 checksums.

### Verification boundary

Local development gates are `cargo fmt --all -- --check`, strict workspace
clippy, `cargo test --workspace`, and `bash scripts/agent/check-docs.sh`.
Windows-specific fixes additionally use the existing Windows test entry point
and hosted `windows-latest`. Smoke fixtures stay small and disposable; no
performance ratios are measured or graded.

## Slices

1. **rel-1 — deterministic Windows CI timeout guard `[x]`.** Replace the loopback
   buffer-size assumption, prove the old Windows-success behavior red, run full
   local gates, and obtain a green Windows-focused run before broader work.
1b. **rel-1b — CLI integration daemon readiness `[x]`.** Own the temporary daemon
    lifecycle, replace startup timing with bounded positive readiness, and
    mutation-prove that clients cannot run before the listener is ready.
2. **rel-2 — P2 small-file regression `[x]`.** Reconcile retained timings against the
   exact old/new executed paths and existing observers, name the responsible
   code delta, fix it, and add a direct mutation-sensitive guard. Do not run a
   physical performance matrix or a large-write test.
3. **rel-3 — Windows directory-tree move completion `[x]`.** The
   retained log identified a native-separator need-list stall, not a held
   source handle. `48c5a11` fixed that executed path, and unified-session
   cutover later deleted it. The exact bounded nested-tree test is re-enabled
   and passed on hosted Windows in run `29944148295` at `28cf989`.
4. **rel-4 — Windows metadata fidelity `[x]`.** Contract v5 and
   the first local/remote implementation are committed at `3013e10`. Formal
   Opus review admitted six release-blocking corrections. Aggregate tar-header
   budgeting, durable attribute readback convergence, and per-file ADS error
   isolation are fixed with mutation-sensitive guards. Strict cross-platform
   preflight plus the explicit warned source-side downgrade are also fixed.
   Local file and tar apply now preserve the source's sub-second mtime after
   ADS replacement, and in-stream resume moves the original destination hash
   allocation through metadata hydration. All six review corrections are fixed;
   hosted Windows run `29944148295` at `28cf989` passed the exact local and
   remote single/tar filesystem guards. Records:
   `.review/results/rel-4-windows-metadata-r1.claude.{json,verdict.md}`.
5. **rel-5 — complete progress reporting `[x]`.** Delegated and served-session
   producers now populate one shared job account with exact manifest byte/file
   denominators, completed bytes/files, and final carrier outcome. GetState,
   TransferProgress, TransferComplete, persisted recents, CLI, and TUI retain
   and render those values; focused mutations prove each accounting lane.
6. **rel-6 — full known-issue reconciliation `[x]`.** Audit every unchecked/current
   `REVIEW.md`, STATE, bug-doc, ignored-test, and current-plan item against code.
   Fix each real product defect one commit at a time; close stale records only
   with direct code/test evidence. The release ledger must reach zero unresolved
   defects. `w7-1` is closed: the session mirror and explicit daemon purge use
   one contained deletion executor; the old parallel enumerator no longer
   exists. The dirty-build profile-skew defect is closed: all profiles watch
   and hash the same executable-input set, including untracked source bytes,
   while documentation changes do not churn peer compatibility. `w7-2` is
   closed as stale: its unsafe handler was deleted, and both current session
   roles validate and consume filters through the shared chokepoint. `w7-3` is
   closed: request paths, signed filesystem mtime, and transfer permissions now
   have one blit-core conversion owner with the per-crate twins deleted. `w7-5`
   is closed: CLI and TUI byte/rate output use one binary-unit presenter, and
   the decimal and screen-local formatting ladders are deleted. `w8-1` is
   closed: all recorded abandoned foundation modules/helpers are gone, with
   zero-copy still governed separately by the later FAST decision. `w8-2` is
   closed as stale: otp-10c-2 already deleted the zero-caller control-plane
   payload duplicate with its legacy sinks and fallback module. `w8-3` is
   closed: the inert watch flag, app stubs, unused/misplaced CLI dependencies,
   dead daemon helpers, and stale dead-code suppressions are removed. `w5-3`
   is closed: current daemon filesystem boundaries preserve NotFound and
   PermissionDenied, while other wrapped errors retain full source chains.
   `w5-4` is closed: current daemon response-channel closure is one shared
   `Cancelled` boundary, and live transfer pipelines retain their worker or
   peer-fault preference instead of the deleted drivers' fixed-string errors.
   `w5-5` is closed: the always-no-op production copy logger facade and its
   threaded parameter are deleted; copy failures still propagate as `Result`.
   `w10` is closed: public help now distinguishes retry's re-applied comparison
   from opt-in block resume across every transfer layout, false Phase 2
   shipped claims are historical, and pipeline/whitepaper/audit descriptions
   match the unified session and live dial. A focused generated-help guard was
   red before the wording correction and green afterward; no transfer ran.
   The final unchecked row is closed against `bfefbdc`: the Windows-sensitive
   64 MiB loopback premise was replaced by the deterministic two-byte/one-byte
   blocked writer, with the production timeout helper and retry semantics under
   guard. Hosted Windows execution remains rel-7 evidence, not unfinished code.
7. **rel-7 — reproducible artifacts `[x]`.** Exact run `29949207219` at
   `354f38e` passed the complete test matrix, then built and uploaded all three
   target archives with both binaries, full commit identity, and SHA-256
   sidecars. Missing or empty required output fails construction.
8. **rel-8 — bounded install/startup smoke `[x]`.** `4062947` verifies archive
   checksum/safe extraction, exact CLI/daemon build identity and help, owned
   daemon readiness, tiny local and loopback-remote byte integrity, and bounded
   teardown before upload. Exact run `29951872658` at `6fb4d3f` passed the full
   test matrix and all three target package-smoke jobs after `4927a05` fixed and
   mutation-proved Windows extended-path equivalence. No throughput or
   large-write work.
9. **rel-9 — release-candidate audit `[x]`.** The existing 0.1.0 tag remains
   fixed at its shipped May commit; the candidate is now 0.1.1 from one
   workspace version source, with exact version-plus-commit packaged smoke.
   User-facing notes state the supported triples, same-build requirement,
   security limits, and deferred ceiling work. Exact candidate `d1f1152d`
   passed Docs Gate `29953569556`, CI `29953569652`, and all three packaged
   smoke jobs; the canonical review ledger has no open row. The annotated
   `v0.1.1` tag was pushed to both configured remotes and the public GitHub
   release was published with the six validated archive/checksum assets under
   D-2026-07-23-8.

## Outcome

Blit 0.1.1 was released from exact candidate `d1f1152d` on 2026-07-23:
https://github.com/roethlar/Blit/releases/tag/v0.1.1. The tag peels to the
validated commit on both LAN Gitea and GitHub. The public release is neither a
draft nor a prerelease, and all three archive digests match the release ledger.

## Open questions

- None. This plan is complete.
