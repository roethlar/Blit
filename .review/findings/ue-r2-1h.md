# ue-r2-1h: delete the deprecated Pull RPC; port the relay onto PullSync

**Slice**: ue-r2-1h — eighth slice of `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`
**Status**: Coded, pending GPT review
**Branch**: master (no agent branches — AGENTS.md §8)
**Commits**: slice commit (this doc rides with it) + two separate
preceding commits: the Windows-host clippy baseline fixes and the
win-1 push-separator fix (see Environment note below).
**Commit-boundary erratum** (severity upgraded by the self-review
panel): the `service/pull.rs` deletion was staged early (`git rm`) and
rode into the clippy-baseline commit. Consequence: the two
intermediate commits (`9f37a7a`, `48c5a11`) **do not build** — their
trees delete `pull.rs` while `mod.rs`/`core.rs` still reference it —
so `git bisect` must skip them, and their commit messages' gate claims
describe the full working tree at commit time, not those exact trees.
HEAD is correct and fully gated. Left as-is rather than rewriting
history (AGENTS.md git-safety); flagged to the owner, who may approve
a history fix if bisectability matters more than the no-rewrite rule.

## What

Delete the deprecated `Pull` RPC end-to-end — proto (`rpc Pull`,
`PullRequest`, `PullChunk`), daemon handler (`service/pull.rs`, whole
file, including the `pull_stream_count` ladder at its :829), the
`core.rs` handler/`PullStream` type, the `PullSender`/`PullPayload`
aliases, and the client's dead `RemotePullClient::pull` — and port the
RPC's two surviving consumers, the remote→remote relay's
`scan_remote_files` + `open_remote_file`, onto PullSync sessions.
`PullEntry` + `collect_pull_entries_with_checksums` (+ their private
dep `build_file_header` and their two unit tests) relocate into
`pull_sync.rs`, their only remaining consumer — closing the 1g Known
gap and removing the `accept_and_wrap_sinks` borrow-back import in the
same move. With this, the last of the three static ladders is gone:
`determine_remote_tuning` (1e), `desired_streams` (1f),
`pull_stream_count` (1h, untested and unlamented — no test anywhere
pinned it).

## Interpretation of scope (stated for review)

The REV4 slice text says "delete the deprecated Pull RPC (and its
pull_stream_count ladder) … and tests cover the replacement, including
old/new peer pairs". It does not name the relay. The owner-ratified
w2-4 row (`REVIEW.md:55`, D-2026-06-11-2) does: "Delete deprecated
Pull RPC after w2-3 harvest (owner-decided, wire-breaking OK); **port
scan_remote_files**". So the relay survives, ported — not deleted.
That port is the slice's only design surface:

- **No existing wire could carry it.** The relay scan needs recursive
  enumeration with full `FileHeader`s (size/mtime/**permissions** — it
  builds tar shards from them). `FindEntry`/`FileInfo` lack
  permissions; `FileList` (the need-list) is paths-only; PullSync's
  `ManifestBatch` is counts-only. The deprecated RPC's
  `metadata_only` flag was the only header-scan wire.
- **Chosen design**: add `bool metadata_only = 13` to
  `TransferOperationSpec` (additive, spec_version 2 unchanged — the
  `receiver_capacity` precedent; the proto comment documents why the
  no-bump bar is met: an old daemon ignoring it degrades to
  correct-but-wasteful, never unsafe). The daemon's pull_sync handler
  answers a metadata-only spec with one bare `file_header` frame per
  enumerated entry (existing ServerPullMessage field 6) + summary —
  after the R47-F3 incomplete-scan refusal, before comparison; no
  need-list, no delete-list, no data plane, no bytes.
- `open_remote_file` becomes a single-file `force_grpc` PullSync
  session read through a new `RemoteFileStream` (AsyncRead over
  `file_data` frames, EOF at `summary`). Two structural guarantees
  make the frame shape total: the planner never tar-shards a single
  file (`transfer_plan.rs` "2+ files" rule) and the session's
  `client_capabilities` truthfully advertises no tar support; tar
  frames arriving anyway are a hard read error, not silent garbage.
- Both session kinds share `open_relay_session` (spec + empty
  manifest + manifest_done, then read frames) and
  `build_relay_session_spec` (no filter, default compare, no
  mirror/resume, `force_grpc: true` always — so even an old daemon
  never steers the session toward a TCP data plane these clients will
  not dial; a non-fallback `negotiation` frame is a hard error).

## Files changed

- `proto/blit.proto` — rpc + 2 messages deleted (BlitAuth-style dated
  tombstones; no `reserved` needed — nothing removed from a surviving
  message); `TransferOperationSpec.metadata_only = 13` added; stale
  comments fixed (CancelJob list, receiver_capacity tombstone, detach,
  ActiveTransfer.path, TransferKind::PULL history note).
- `crates/blit-daemon/src/service/pull.rs` — **deleted** (953 lines).
  Relocated to `pull_sync.rs`: `PullEntry` (minus its deprecated-only
  `absolute_path` impl), `collect_pull_entries_with_checksums` (now
  private), `build_file_header`, `mod single_file_filter_tests`.
  Everything else was deprecated-wire-only and died: both
  `stream_pull*` paths, `collect_pull_entries` wrapper,
  `pull_stream_count`, the PullChunk frame writers, both
  `accept_pull_data_connection*` (2 of open finding design-2's 3 spawn
  sites — see Known gaps).
- `crates/blit-daemon/src/service/pull_sync.rs` — metadata_only branch
  + `send_file_header`; relocations; `accept_and_wrap_sinks` back to
  private (borrow-back gone); 1g tombstone comments reworded.
- `crates/blit-daemon/src/service/{mod.rs,core.rs}` — `mod pull`,
  `PullSender`, `PullPayload`, `type PullStream`, the `pull` handler
  and dead imports removed. `ActiveJobKind::Pull` and
  `TRANSFER_KIND_PULL` stay (historical recents rows still carry them).
- `crates/blit-core/src/remote/transfer/operation_spec.rs` —
  `NormalizedTransferOperation.metadata_only`.
- `crates/blit-core/src/remote/pull.rs` — dead `pull()` (183 lines,
  zero callers) deleted; `scan_remote_files`/`open_remote_file`
  reimplemented on PullSync (receives routed through
  `recv_fallback_message`, keeping the audit-h3c chokepoint);
  `RemoteFileStream` rewritten over `ServerPullMessage`.
- `crates/blit-cli/src/transfers/mod.rs` — move+relay rejection
  comment updated: the ported scan deliberately does NOT set
  `require_complete_scan` (copy relay keeps its historical
  send-what's-readable behavior), so the R50-F1 gate stays.
- `crates/blit-core/src/remote/transfer/grpc_fallback.rs` — module-doc
  site list updated (the deleted `pull()` receive site removed).
- Test mocks: 4 `impl Blit` mocks drop `PullStream`/`pull`
  (`pull_sync_with_spec_wire.rs`, `jobs_lifecycle.rs`,
  `remote_remote.rs` ×2).

## Tests

Baseline entering the slice: 1413 / 0 / 2. No test deleted; 2 relocate
(`single_file_filter_tests`, moved with the function they pin — the
only direct tests the entire deprecated path ever had).

New:
- `pull_sync_with_spec_wire.rs` + `CannedFramesServer` (spec-capturing
  frame-script daemon): scan collects bare headers and sends a
  metadata_only+force_grpc spec with the right module/source_path;
  scan survives an old daemon that ignores `metadata_only` and streams
  data/tar shards (the mixed-version pin — headers extracted from
  `file_header` AND `tar_shard_header.files`, bytes discarded); scan
  fails fast on a real (non-fallback) data-plane negotiation;
  open_remote_file yields exact bytes across split `file_data` frames
  and EOFs at `summary` (and its spec advertises no tar support);
  open_remote_file hard-errors on tar-shard frames.
- `remote_remote.rs::remote_to_remote_relay_transfers_nested_tree` —
  real dual daemons: relay of a nested multi-file tree is
  byte-identical (exercises the daemon's metadata_only branch +
  per-file single-file sessions end-to-end).
- `delegated_pull.rs::dst_override_preserves_every_non_capabilities_field`
  extended to prove `metadata_only` survives the delegation override;
  the 1b compat constructors set `metadata_only: true` so the
  old-peer-decode suite covers the newest field.

Existing coverage that pins the port: the relay e2e
(`remote_to_remote_explicit_relay_uses_legacy_cli_byte_path`) passes
unchanged over the new transport; the 13 ue-r2-1b compat tests were
verified independent of the deleted rpc; all 1g multistream/resume/
checksum/mirror/cancellation pins stay green.

## Known gaps

- **design-2 partial resolution**: deleting `service/pull.rs` removed
  2 of the open High finding design-2's 3 orphaned-data-plane spawn
  sites (`pull.rs:186`, `:308`); only `push/control.rs:57` remains.
  Annotated in `.review/findings/design-2-orphaned-daemon-data-planes.md`;
  the w4-1 row now scopes to the surviving site.
- **Relay scan still has no completeness signal by choice**: the
  metadata-only spec could now carry `require_complete_scan = true`,
  which would let the daemon refuse partial scans and eventually lift
  the move+relay CLI rejection. Left unset to preserve copy-relay
  behavior exactly; lifting the gate is a separate owner-facing
  behavior change (candidate for the design queue, noted in the
  `transfers/mod.rs` comment).
- **Old-daemon scan degradation is correct but wasteful** (full bytes
  streamed and discarded), as documented in the proto. Old daemons
  also tar-shard multi-file fallbacks regardless of the client's
  advertised capabilities (pre-existing daemon behavior — the
  capability is honored by push, not by pull_sync's fallback sink);
  the scan handles both shapes, and the single-file session is
  structurally shard-free.
- The metadata-only summary reports enumerated files/bytes (workload
  size), mirroring the deprecated RPC's summary semantics — not "bytes
  moved" (which is zero). Stated in the handler comment.
- `pull_stream_count` died untested (no test anywhere pinned it) —
  nothing to migrate; PullSync's `negotiated_pull_streams`/
  `initial_stream_proposal` (1f/1g) has its own tests.
- Environment note: this is the first slice validated on the owner's
  Windows host, and it surfaced two pre-existing Windows problems,
  each fixed in its own commit preceding the slice:
  1. The gate's clippy leg failed on 15 **pre-existing** violations in
     Windows-only code none of this slice touched (`copy/windows.rs`,
     `win_fs.rs`, `fs_capability/*`, `change_journal/util.rs`, two
     unix-gated test binaries' imports). Mechanical fixes;
     `scripts/windows/run-blit-tests.ps1` historically omitted clippy,
     which is how the debt accumulated invisibly.
  2. The new nested-tree relay e2e exposed a **pre-existing High bug**:
     the Windows daemon's push need-list echoed native separators
     (`nested\mid.txt`), missing the client planner's POSIX manifest
     keys — every nested push to a Windows daemon stalled 30s and
     failed (`.review/findings/win-1-push-needlist-separators.md`).
     Two long-standing push tests (`test_push_nested_directories`,
     `forced_grpc_push_many_files_completes`) failed on Windows for
     the same root cause and pass after the one-line
     `relative_path_to_posix` fix.
- Test-count note: the 1413/0/2 baseline was measured on the previous
  (unix) dev host; on Windows the unix-gated tests compile out, so the
  Windows-host total is not directly comparable. Slice arithmetic:
  +6 new tests (5 relay-session wire tests, 1 nested relay e2e),
  0 deleted, 2 relocated intact.
