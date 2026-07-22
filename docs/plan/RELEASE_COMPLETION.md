# Complete the first cross-platform release

**Status**: Draft
**Created**: 2026-07-22
**Supersedes**: performance work as the pre-release critical path under
D-2026-07-22-1; does not supersede shipped implementation plans
**Decision ref**: D-2026-07-22-1 establishes release-first scope; activation
decision pending
**Owner decisions**: Windows attributes and ADS must be preserved across local
and remote transfers in the first release (approved 2026-07-22: "go"); every
known product defect, correctness/robustness gap, progress gap, CI failure,
documentation defect, and packaging/install/startup gap is release-blocking
(approved 2026-07-22: "there's no release until everything is fixed"); P1 must
be identified and fixed from retained evidence and code without another
physical transfer (D-2026-07-22-2)

## Goal

Produce one installable, smoke-tested Blit release from an exact reviewed
commit for macOS, Linux, and Windows. The release boundary is data correctness,
bounded failure, supported-platform CI, packaging, installation, daemon/CLI
startup, and truthful documentation. It does not depend on another performance
or hardware experiment.

## Non-goals

- ldt-4 causal tuning, MTU follow-ups, Thunderbolt ceilings, small-file
  performance, zero-copy performance, or any other hardware benchmark before
  release. P1 is already closed from offline evidence and deterministic guards.
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
- [ ] The Windows handshake-timeout guard deterministically exercises a blocked
      token write on every supported OS; its old Windows-success mutation turns
      the focused guard red.
- [ ] Hosted CI is green on the exact release candidate for formatting, strict
      clippy, Linux tests, macOS tests, and Windows tests.
- [ ] The ignored Windows directory-tree move hang has a root-cause fix; the
      integration test is re-enabled and passes repeatedly on Windows.
- [ ] Windows file attributes and ADS are preserved across local and remote
      transfers, including the tar threshold that currently changes fidelity;
      single-file and batched copies produce the same metadata result.
- [ ] Delegated and served-session progress is complete: live bytes are emitted,
      daemon rows report accurate bytes, and total/completed file and byte
      denominators are nonzero and correct through GetState, TransferProgress,
      TransferComplete, CLI, and TUI consumers.
- [ ] Every open `REVIEW.md` item and current-state known issue is reconciled
      against code: fixed with a guard, proven already fixed and closed, or
      classified as a non-defect only by an explicit owner decision. No known
      product defect is deferred merely to reach the release.
- [ ] Release artifacts are produced from one exact commit for the supported
      target set, with embedded version/build identity and recorded checksums.
- [ ] Each artifact installs or runs in a clean temporary environment; CLI
      version/help, daemon startup/health, one small local copy, and one small
      same-build remote copy pass with exact content verification and clean
      teardown.
- [ ] User-facing release notes state supported platforms, known limitations,
      compatibility, and deferred performance work.
- [ ] The release-candidate tree is clean; all verification evidence names its
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
- `.github/workflows/`, Cargo packaging metadata, release scripts, and operator
  docs: make artifact construction and smoke evidence reproducible from one
  exact commit.

### Verification boundary

Local development gates are `cargo fmt --all -- --check`, strict workspace
clippy, `cargo test --workspace`, and `bash scripts/agent/check-docs.sh`.
Windows-specific fixes additionally use the existing Windows test entry point
and hosted `windows-latest`. Smoke fixtures stay small and disposable; no
performance ratios are measured or graded.

## Slices

1. **rel-1 — deterministic Windows CI timeout guard.** Replace the loopback
   buffer-size assumption, prove the old Windows-success behavior red, run full
   local gates, and obtain a green Windows-focused run before broader work.
2. **rel-2 — Windows directory-tree move completion.** Reproduce with a bounded
   small tree, identify the held handle or cleanup deadlock, fix it, re-enable
   the Windows test, and prove the fix red/green.
3. **rel-3 — Windows metadata fidelity.** Amend the transfer contract first;
   then enumerate, transport, validate, and apply bounded attributes/ADS for
   local and remote file/tar carriers. Guard single versus ≥32-file behavior,
   ADS contents, attributes, unsafe/duplicate stream refusal, cancellation,
   and failure without partial success reporting.
4. **rel-4 — complete progress reporting.** Implement the three recorded
   daemon/delegated progress gaps end to end, with producer, RPC, state-table,
   CLI, and TUI guards.
5. **rel-5 — full known-issue reconciliation.** Audit every unchecked/current
   `REVIEW.md`, STATE, bug-doc, ignored-test, and current-plan item against code.
   Fix each real product defect one commit at a time; close stale records only
   with direct code/test evidence. The release ledger must reach zero unresolved
   defects.
6. **rel-6 — reproducible artifacts.** Build the supported target matrix from
   one exact commit, preserve build identity, generate checksums, and fail if a
   required artifact is absent.
7. **rel-7 — bounded install/startup smoke.** In clean temporary environments,
   verify CLI and daemon startup plus small local and remote integrity transfers
   on macOS, Linux, and Windows. Retain logs and exact artifact hashes; no
   throughput or large-write work.
8. **rel-8 — release-candidate audit.** Reconcile the ledger against exact CI,
   artifacts, smoke evidence, known limitations, and open findings. Prepare the
   exact refs/remotes for owner approval; do not publish.

## Open questions

- Beyond the now-closed P1 defect, does "everything" include P2 and unresolved
  performance acceptance criteria, or are optional ceiling experiments still
  post-release? — owner: Michael
- Which concrete target triples and archive/install formats constitute the
  required macOS, Linux, and Windows artifact set? — owner: Michael
