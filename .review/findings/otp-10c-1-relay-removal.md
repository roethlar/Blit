# otp-10c-1 — `--relay-via-cli` removed; relay read half deleted

**What**: First otp-10c sub-slice. The owner decided (2026-07-11,
D-2026-07-11-1) that the `--relay-via-cli` escape hatch is removed
rather than rebuilt: its read half was the PullSync client's
on-demand per-file remote read, a capability the unified session
deliberately does not have, so PullSync's deletion (otp-10c-2) makes
a streaming relay unrebuildable. Remote→remote is delegated-only;
the CLI is never in the byte path. This slice deletes the flag, the
relay route, every relay-combination gate, `RemoteTransferSource`,
and its instrumentation counter — so otp-10c-2 can delete the four
drivers and the `Push`/`PullSync` RPCs as pure dead code.

**Approach**:

- **Flag + route**: `TransferArgs.relay_via_cli` (cli.rs) deleted;
  `TransferRoute::RemoteToRemoteRelay` variant and the
  `select_transfer_route` relay parameter deleted (dispatch.rs) —
  `(Remote, Remote)` is total on `RemoteToRemoteDelegated`.
- **Gates die with the combination they guarded** (their data-loss
  reasoning is moot once no relay path exists to combine with):
  detach×relay (run_transfer), mirror×relay front gate + in-route
  `debug_assert` (audit-h1 rounds 1–2), move×relay (R50-F1/R51-F2),
  resume×relay (codex otp-10a F4 — the bail in `run_remote_push`).
- **`PushExecution.source`: `Endpoint` → `PathBuf`** — a remote push
  source is now unrepresentable at compile time; this is the slice's
  structural deletion proof for the relay send half. The CLI's
  `run_remote_push_transfer{,_deferred,_inner}` signatures narrow the
  same way; the TUI's `build_f1_push_execution` passes the path
  straight through.
- **`RemoteTransferSource` deleted** (source.rs) with its two
  relay-only helpers `validate_remote_tar_shard_sizes` (F7 bound) and
  `read_remote_entry_bounded` (R11-F1 bound) — those bounds guarded
  the relay's remote-read tar assembly, which no longer exists.
  `FsTransferSource`/`FilteredSource`/`ChecksummingSource` and all
  their tests are untouched.
- **`record_remote_transfer_source_constructed` deleted**
  (instrumentation.rs). The delegated byte-path-isolation pin keeps
  its load-bearing half: `cli_data_plane_outbound_bytes == 0`
  (remote_remote.rs), which observes actual data-plane sends rather
  than constructor calls and doubles as the otp-10 deletion proof's
  CLI half.
- **Delegated error hints**: `relay_fallback_suggestable` and every
  relay hint clause deleted (`DelegatedPullExecution`,
  `map_delegated_error`, the Unimplemented/Unavailable/stream-lost
  arms). CONNECT_SOURCE — the one topology the flag genuinely
  served — now hints the manual two-hop: "if this host can reach
  both daemons, pull to a local path first, then push it".
- **Records**: D-2026-07-11-1 appended to docs/DECISIONS.md;
  `REMOTE_REMOTE_DELEGATION_PLAN.md` (Historical) gets a dated
  relay note per its own header-note precedent;
  `scripts/bench_remote_remote.sh` loses its relay comparison leg.

**Files**:

- `crates/blit-cli/src/cli.rs` — flag deleted.
- `crates/blit-app/src/transfers/dispatch.rs` — variant + param.
- `crates/blit-cli/src/transfers/mod.rs` — gates + route arm; test
  helper `gate_args` loses the relay param.
- `crates/blit-cli/src/transfers/remote.rs` — push signatures narrow
  to `PathBuf`.
- `crates/blit-cli/src/transfers/remote_remote_direct.rs` —
  `relay_fallback_suggestable` threading removed.
- `crates/blit-app/src/transfers/remote.rs` — `PushExecution.source`
  retype; relay-resume bail + Remote arm deleted; hint machinery
  removed; module docs updated.
- `crates/blit-core/src/remote/transfer/source.rs` —
  `RemoteTransferSource` + 2 helpers deleted.
- `crates/blit-core/src/remote/instrumentation.rs` — counter hook.
- `crates/blit-tui/src/exec_plan.rs` — builders follow the retype /
  field removal.
- `docs/DECISIONS.md`, `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`,
  `scripts/bench_remote_remote.sh` — records (above).

**Tests** (suite 1605 → **1585**, exactly the 20 retirements below —
the called-out drop, per the verification rule; gate green: fmt,
clippy -D warnings, `cargo test --workspace` 1585/0):

- **Retired with the code they pinned (20)**: dispatch relay-pick +
  relay-ignored (2); mod.rs gate tests `detach_rejected_with_relay_
  via_cli`, `mirror_rejected_with_relay_via_cli_for_remote_to_remote`,
  `copy_relay_via_cli_does_not_trip_mirror_gate`,
  `mirror_rejected_with_relay_via_cli_when_no_yes` (4);
  source.rs `remote_tar_size_tests` (5) + `remote_bounded_read_tests`
  (5) — bounds on the deleted relay tar assembly;
  `remote_to_remote_explicit_relay_uses_legacy_cli_byte_path` +
  `remote_to_remote_relay_transfers_nested_tree` (2);
  `relay_source_with_resume_is_refused_before_any_connection` (1);
  `remote_to_remote_move_rejects_relay_via_cli` (1).
- **Surviving pins updated, assertions preserved**: the delegated
  isolation pin and the three no-fallback pins keep
  `cli_data_plane_outbound_bytes == 0` and drop only the
  constructed-counter assertion (its counter has no producer left);
  `read_counters` loses the dead field.
- No new tests: the removal is proven at compile time (the flag,
  variant, and type no longer exist), and the surviving delegated
  pins already exercise the only remaining remote→remote route
  against real daemons.

**Known gaps**:

- Workspace test count DROPS by the 20 retired tests above — every
  one pinned behavior that no longer exists (flag parsing gates,
  relay byte-path, relay-only allocation bounds). Nothing they
  guarded survives unguarded: the delegated pins still run against
  real daemons, and the send-half tar bound (`prepare_payload` local
  path) was never relay-specific.
- The old pull driver, `PullSyncOptions`, `enumerate_local_manifest`,
  `apply_pull_mirror_purge`, the old push driver, the daemon
  `push/`/`pull_sync.rs` handlers, and the `Push`/`PullSync` RPCs are
  still in-tree — otp-10c-2 deletes them with their own ported-test
  accounting and the file-by-file deletion proof.
- Comment-level relay mentions in code that otp-10c-2 deletes anyway
  (`remote/pull.rs`, `grpc_fallback.rs` scope lists, the daemon
  `runtime.rs` delegation-gate comment) are left for that slice's
  comment sweep rather than churned twice.
