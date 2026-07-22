# Release readiness

**Status:** Active release ledger
**As of:** local release work through the rel-4 candidate, 2026-07-22

This is the concise release boundary after D-2026-07-22-3. Every known broken
behavior is release work regardless of its internal classification. Optional
performance ceilings and hardware tuning remain post-release work.

## Proven

- Local formatting, strict workspace clippy, workspace tests, and docs checks
  passed at `43f156d` before this docs-only audit.
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
  path. The exact nested push-move test is active on Windows again; current-head
  hosted confirmation remains publication-gated.
- rel-4 defines contract v4 and implements bounded Windows attributes and named
  `$DATA` streams across local, TCP, in-stream, tar, and resume carriers. The
  destination retains and validates manifest descriptors before applying any
  payload, metadata mismatch overrides an ordinary content skip, non-Windows
  destinations refuse before creating a partial file, and ADS bytes count as
  payload. The guards fail under metadata-diff and hash-drift mutations; local
  format, strict clippy, workspace tests, docs, and strict all-target Windows
  cross-compilation pass. Actual Windows filesystem behavior is still pending
  the publication-gated hosted run.

## Release blockers

1. **Hosted Windows confirmation is pending.** Published run `29584631185`
   failed because Windows buffered the guard's 64 MiB loopback handshake.
   rel-1 now exercises the same production timeout through a deterministic
   two-byte/one-byte in-memory blocked writer with local mutation proof. The
   exact fix has not run on hosted Windows because publication is owner-gated.
   The re-enabled nested push-move test and rel-4's single/tar plus local/remote
   metadata guards also need current-head Windows confirmation. Findings:
   `release-win-ci-handshake-stall-test`, `windows-move-tree-hang`, and
   `windows-attrs-and-ads-lost-on-tar-path`.
2. **Temporary-daemon startup is not yet deterministic or diagnosable.** The
   first rel-4 workspace gate had one daemon exit during `admin_verbs` startup;
   the exact test and complete suite then passed, while the harness discarded
   stderr and therefore lost the cause. `tests-harness-stderr-blackhole` must
   capture the process error and any remaining startup failure must be fixed.
3. **Progress reporting is incomplete.** Delegated progress can be silent and
   served-session byte/file totals can remain zero through daemon, RPC, CLI,
   and TUI consumers.
4. **Current release artifacts are unproved.** The exact local head is not on
   GitHub, the latest published release-build jobs were skipped, and install /
   startup smoke checks for the produced CLI and daemon artifacts are not
   recorded.

## Deferred until after release

- ldt-4 causal tuning, unexplained ceiling asymmetries not tied to a known
  broken behavior, MTU follow-ups, and further hardware matrices.
- Mac↔Mac Thunderbolt ceiling testing.
- Small-file and zero-copy performance work, performance refactors, and other
  ceiling optimization.

No new hardware result is required to make the release decision.
