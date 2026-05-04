2026-05-03 remote→remote delegation follow-up after Phase 2 worker implementation.

R37 fixes:
- `RemotePullClient::pull_sync_with_spec` now preserves typed `PullSyncError::Negotiation` for source-side pull-sync refusal before negotiation completes, plus checksum capability negotiation refusal.
- `crates/blit-daemon/src/service/delegated_pull.rs` downcasts `PullSyncError` and emits `DelegatedPullError::NEGOTIATE` instead of generic `TRANSFER` for source refusal.
- `crates/blit-cli/tests/remote_remote.rs` adds `source_refuses_destination_negotiation_does_not_fall_back_to_relay`: real destination daemon + fake source rejecting `pull_sync`; CLI surfaces "source refused delegated pull" and relay counters remain zero.
- `remote_remote_direct.rs` now treats `BytesProgress` as cumulative and converts to deltas before feeding `RemoteTransferProgress`; proto comment documents cumulative semantics.

Phase 3 cleanup/harness:
- `RemoteTransferSource` docstring now says it is the explicit legacy `--relay-via-cli` primitive; default remote→remote uses destination-side `DelegatedPull`.
- Added `scripts/bench_remote_remote.sh` and `docs/perf/remote_remote_benchmarks.md` template. Live benchmark execution is still unchecked/TBD in `REMOTE_REMOTE_DELEGATION_PLAN.md`.
- Review notes appended: Round 38 closes R37; Round 39 accepts Phase 3 cleanup/harness.

Validation run:
- `cargo fmt`; `cargo fmt -- --check`; `cargo test -p blit-core pull_sync_with_spec`; `cargo test -p blit-cli remote_remote_direct`; `cargo test -p blit-cli --test remote_remote`; `cargo test -p blit-daemon delegated_pull`; `cargo test --workspace`; `bash -n scripts/bench_remote_remote.sh`.
- Full workspace passed. Existing unrelated warnings remain: macOS FSEvents deprecation/F14 and unused `preserved` in macOS fs capability test.

Caution:
- `.serena/project.yml` is dirty but was pre-existing/local and should not be committed unless the user requests it.
- Phase 2+R37+Phase3 changes are uncommitted in the worktree.