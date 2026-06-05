# Plan Inventory: Architecture, whitepaper, daemon config, proto
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 4 files, 2866 lines

| File | Lines |
|---|---|
| docs/ARCHITECTURE.md | 502 |
| docs/WHITEPAPER.md | 750 |
| docs/DAEMON_CONFIG.md | 620 |
| proto/blit.proto | 994 |

## Claims (grouped by category)

### principle

#### principle-fastest-most-reliable
**Source**: WHITEPAPER.md:§stated philosophy (top)
**Specificity**: low

"fastest, most reliable, most stable file transfer in any scenario." Adaptive tuning over hardcoded constants. Identical data path for every src↔dst combination.

#### principle-adaptive-over-hardcoded
**Source**: WHITEPAPER.md:§stated philosophy; §8.4
**Specificity**: medium

Adaptive tuning over hardcoded constants. § 8.4 notes hardcoded constants (RECEIVE_CHUNK_SIZE, MAX_PARALLEL_TAR_TASKS, tar shard thresholds, mpsc capacities) that violate this philosophy and "should be in TuningParams."

#### principle-identical-data-path
**Source**: WHITEPAPER.md:§stated philosophy; §2; ARCHITECTURE.md:§Data Flow
**Specificity**: high

"Identical data path for every src↔dst combination." Every transfer (local→local, local→remote push, remote→local pull, remote→remote) routes through the same pipeline; only concrete TransferSource and TransferSink implementations differ.

#### principle-trust-network-not-bind
**Source**: DAEMON_CONFIG.md:§Trust Model and Network Exposure
**Specificity**: high

Security comes from operator network controls + per-transfer auth tokens, not from the bind address. There is no "default safe" mode — operator chooses exposure that fits the network. By default daemon binds 0.0.0.0:9031 because the service's whole purpose is remote reachability.

#### principle-typed-not-bool-soup
**Source**: blit.proto:§TransferOperationSpec preamble
**Specificity**: high

"The wire shape — typed enums and orthogonal fields, not bool soup." Receivers normalize via `NormalizedTransferOperation::from_spec` at the boundary; downstream code never sees raw proto-`Unspecified` or unknown enum values.

#### principle-orthogonal-fields
**Source**: blit.proto:TransferOperationSpec.ignore_existing (lines 478-480); ComparisonMode preamble
**Specificity**: high

`ignore_existing` controls "should we look at this file at all?" while `compare_mode` controls "given that we're looking, do the bytes match?" — orthogonal. CLI rejects `--ignore-existing` combined with `--force` as semantically contradictory.

#### principle-clear-recent-never-touches-perf-history
**Source**: blit.proto:ClearRecent comment (lines 82-89); DAEMON_CONFIG.md (perf_local.jsonl)
**Specificity**: high

ClearRecent deliberately does NOT touch the planner/predictor's historical telemetry (`perf_local.jsonl`) — operator clearing the recents view must never degrade prediction quality.

### invariant

#### invariant-single-pipeline-entry-point
**Source**: ARCHITECTURE.md:§Crate Structure / blit-core; §Data Flow
**Specificity**: high

`execute_sink_pipeline` + `execute_sink_pipeline_streaming` is "the single entry point for every src→dst combination."

#### invariant-module-containment-always-on
**Source**: DAEMON_CONFIG.md:§Path containment
**Specificity**: high

Module containment is always on as of F2. Every daemon read or write resolves the target's deepest existing ancestor through `std::fs::canonicalize` and refuses the operation if the canonical form escapes the module root. A symlink inside the module that points outside it cannot be traversed by daemon operations. No opt-out.

#### invariant-canonicalize-then-operate-toctou
**Source**: DAEMON_CONFIG.md:§Path containment (TOCTOU caveat)
**Specificity**: medium

The check is "canonicalize then operate," so a symlink swapped between the check and the actual filesystem call could in principle be followed. Trust model: authenticated peers + operator-controlled module contents, not adversarial local processes racing the daemon.

#### invariant-data-plane-one-time-tokens
**Source**: DAEMON_CONFIG.md:§Trust Model; blit.proto:DataTransferNegotiation.one_time_token
**Specificity**: high

The TCP data plane uses one-time tokens minted per transfer; a peer that doesn't present the right token is dropped before any bytes flow.

#### invariant-delegation-default-off
**Source**: DAEMON_CONFIG.md:§Outbound delegation; blit.proto:DelegatedPull comment
**Specificity**: high

Default off. A daemon refuses `DelegatedPull` requests unless operator sets `[delegation] allow_delegated_pull = true`. Per-module override can narrow but cannot widen.

#### invariant-delegation-bound-ip
**Source**: DAEMON_CONFIG.md:§Allowlist matching #5
**Specificity**: high

DNS-rebinding mitigation: the validated IP is bound to the outbound connection. The daemon connects to a specific `host = <ip>` URI, never to a re-resolvable hostname. A malicious DNS authority cannot swap addresses between gate check and connect.

#### invariant-delegation-loopback-needs-ip-form
**Source**: DAEMON_CONFIG.md:§Allowlist matching #6
**Specificity**: high

Loopback/link-local/unique-local addresses (127.0.0.0/8, 169.254.0.0/16, 0.0.0.0/8, ::1, fe80::/10, fc00::/7, ::) require IP- or CIDR-form authorization in `allowed_source_hosts`. Hostname alone won't authorize them (SSRF-via-DNS pivot).

#### invariant-delegation-allmatch-resolution
**Source**: DAEMON_CONFIG.md:§Allowlist matching #4
**Specificity**: high

A hostname resolves to its A/AAAA set; every resolved address must match either a CIDR entry or a bare-IP entry. Mixed-result resolution where some addresses are inside and some outside the allowlist is denied.

#### invariant-delegation-invalid-entry-fails-load
**Source**: DAEMON_CONFIG.md:§Allowlist matching #3
**Specificity**: high

CIDR/bare-IP entries parsed once at config load via `ipnet` crate. Invalid entries fail config load loudly — the daemon refuses to start rather than silently treating an unparseable line as "deny everything," which would mask a typo'd permit.

#### invariant-getstate-counters-zero-when-metrics-off
**Source**: blit.proto:DaemonState.counters comment (lines 753-758); GetState comment lines 53-57
**Specificity**: high

Counters read from TransferMetrics atomics — when `--metrics` is off the atomics never incremented, so all fields are zero. `active[]` and `recent[]` are independent of this and remain populated from the always-on ActiveJobs table.

#### invariant-active-and-recent-always-available
**Source**: blit.proto:DaemonState.active comment (lines 744-747); GetState comment
**Specificity**: high

`active[]` and `recent[]` always populate from the always-on ActiveJobs table introduced in milestone B; not gated on `--metrics`. Operator can always ask the daemon what it's doing right now.

#### invariant-cancel-only-delegated-pull
**Source**: blit.proto:CancelJob comment (lines 59-65)
**Specificity**: high

Only delegated remote→remote pulls support cancellation today — push/pull/pull_sync have the CLI in the byte path, so a client-side cancel already drops the handler future and `CancelJob` from another client wouldn't have a meaningful semantic.

#### invariant-cancel-status-semantics
**Source**: blit.proto:CancelJob comment (lines 67-75)
**Specificity**: high

CancelJob status: OK → cancellation token fired; NOT_FOUND → no active transfer matches transfer_id (already completed or never existed); FAILED_PRECONDITION → transfer exists but its kind doesn't honor cancellation today.

#### invariant-clear-recent-wipes-ring-and-jsonl
**Source**: blit.proto:ClearRecent comment (lines 78-89)
**Specificity**: high

ClearRecent wipes the in-memory recent-runs ring AND its persisted backing store (`recents.jsonl`). Does NOT touch `perf_local.jsonl`. Response carries number of entries removed from the in-memory ring.

#### invariant-subscribe-slow-consumer-aborted
**Source**: blit.proto:Subscribe comment (lines 104-107)
**Specificity**: high

Slow consumers receive a `gRPC Status::Aborted` on the stream (the daemon's broadcast channel dropped events while they fell behind). TUI consumers handle this by re-subscribing and refreshing snapshot state via `GetState`.

#### invariant-event-field-numbers-stable
**Source**: blit.proto:Subscribe comment (lines 100-102); DaemonEvent payload comment
**Specificity**: high

Field numbers in `DaemonEvent.payload` are stable; new variants append. Field numbers are part of the wire contract.

#### invariant-delegated-pull-spec-override-boundary
**Source**: blit.proto:DelegatedPullRequest.spec comment (lines 612-616)
**Specificity**: high

OVERRIDE BOUNDARY: `spec.client_capabilities` describes the byte recipient's capabilities. In delegation, the byte recipient is the destination daemon, not the CLI. The destination handler mandatorily REPLACES `client_capabilities` with its own `PeerCapabilities` before forwarding the spec to the source — any CLI-supplied value here is non-authoritative.

#### invariant-detach-only-on-delegated-pull
**Source**: blit.proto:DelegatedPullRequest.detach comment (lines 622-642)
**Specificity**: high

`detach` only valid on DelegatedPull (the daemon-to-daemon byte path). Push/pull/pull_sync put the CLI in the byte path — detach is meaningless there because CLI disconnecting drops the bytes. CLI rejects `--detach` for those routes.

#### invariant-spec-version-fail-closed
**Source**: blit.proto:TransferOperationSpec.spec_version comment (lines 426-434)
**Specificity**: high

`spec_version` bumped when wire shape changes in a way receiver must know. Receivers should reject specs with a version they don't understand. Bumped to v2 for `require_complete_scan` so v1 daemons fail closed when receiving a v2 spec instead of silently ignoring the safety-critical field.

#### invariant-receivers-default-capabilities-false
**Source**: blit.proto:PeerCapabilities comment (lines 574-577)
**Specificity**: high

Receivers default missing fields to false (not supported).

#### invariant-require-complete-scan-purge-gate
**Source**: blit.proto:PushHeader.require_complete_scan (lines 178-186); ManifestComplete.scan_complete (lines 213-220)
**Specificity**: high

When `require_complete_scan=true`, the daemon refuses to purge any destination entries if the client's `ManifestComplete.scan_complete` arrives as false — because a partial scan would let absent-due-to-unreadable files be deleted from the destination.

#### invariant-move-requires-complete-scan
**Source**: blit.proto:TransferOperationSpec.require_complete_scan (lines 482-490)
**Specificity**: high

R49-F2: when initiator signals source will be deleted after transfer (`blit move`), origin daemon must refuse if its source-side scan was incomplete (e.g. EACCES). Move never sets `mirror_mode` but carries the same scan-completeness requirement.

#### invariant-mirror-mode-filtered-default
**Source**: blit.proto:MirrorMode enum (lines 545-562); PushHeader.mirror_kind (lines 167-175)
**Specificity**: high

Default `FILTERED_SUBSET` preserves user intent; `ALL` bypasses filter for explicit "destination exactly mirrors filtered source" intent. Pre-fix daemon unconditionally enumerated with `FileFilter::default()` and purged anything not in `expected_rel_files`, so a push of `--include '*.bin' --mirror` deleted destination's non-bin files.

#### invariant-delete-list-server-authoritative
**Source**: blit.proto:ServerPullMessage.delete_list (lines 313-320)
**Specificity**: high

Authoritative deletion list for mirror mode. Server has filtered source manifest and unfiltered client manifest; computes exactly which client paths should be removed (with `MirrorMode::FilteredSubset`, only client files matching source filter are candidates). Replaces prior "client walks dest tree" purge inference, which mis-purged unchanged files and ignored filter scope.

#### invariant-multibyte-big-endian
**Source**: WHITEPAPER.md:§3
**Specificity**: high

The TCP data plane is a tagged stream of records. All multi-byte ints are big-endian.

#### invariant-record-tags
**Source**: WHITEPAPER.md:§3 wire format
**Specificity**: high

Tags: FILE=0x00, TAR_SHARD=0x01, BLOCK=0x02, BLOCK_COMPLETE=0x03, END=0xFF. Token (32 B) prepends the record stream.

#### invariant-file-record-carries-metadata-inline
**Source**: WHITEPAPER.md:§7.1; §3 wire format
**Specificity**: high

Wire format extended so `FILE` records carry `mtime + perms` inline (eliminating the daemon's manifest cache requirement). FILE := 0x00 path_len:u32 path:bytes size:u64 mtime:i64 perms:u32 bytes:size.

#### invariant-block-complete-carries-metadata
**Source**: WHITEPAPER.md:§3 (note paragraph)
**Specificity**: high

`BLOCK_COMPLETE` carries `mtime` and `perms` after `total_size` (commit `a7d659f`). Required so the auto-promote zero-BLOCK path still refreshes destination mtime/perms; otherwise downstream `blit copy` would re-compare every time.

#### invariant-token-prepends-record-stream
**Source**: WHITEPAPER.md:§3 record stream definition
**Specificity**: high

`record stream := token (32 B) (record)* END_TAG`. A 32-byte token precedes the record stream.

### interface

#### iface-grpc-service-blit
**Source**: ARCHITECTURE.md:§Protocol Design; proto/blit.proto:service Blit
**Specificity**: high

Single `Blit` service in `proto/blit.proto` (package `blit.v2`, proto3). All RPCs listed below.

#### iface-rpc-push
**Source**: blit.proto:line 7
**Specificity**: high

`rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse)` — bidirectional stream for "check-then-send" workflow.

#### iface-rpc-pull
**Source**: blit.proto:line 11
**Specificity**: high

`rpc Pull(PullRequest) returns (stream PullChunk)` — streams file/directory from server to client. DEPRECATED: use PullSync for incremental/selective transfers.

#### iface-rpc-pullsync
**Source**: blit.proto:line 15
**Specificity**: high

`rpc PullSync(stream ClientPullMessage) returns (stream ServerPullMessage)` — bidirectional pull with manifest comparison. Client sends local manifest, server compares and sends only needed files.

#### iface-rpc-list
**Source**: blit.proto:line 18
**Specificity**: high

`rpc List(ListRequest) returns (ListResponse)` — lists contents of a remote directory.

#### iface-rpc-purge
**Source**: blit.proto:line 21
**Specificity**: high

`rpc Purge(PurgeRequest) returns (PurgeResponse)` — deletes files/directories on the server for mirror operations.

#### iface-rpc-completepath
**Source**: blit.proto:line 24
**Specificity**: high

`rpc CompletePath(CompletionRequest) returns (CompletionResponse)` — path completion suggestions for a given remote path prefix.

#### iface-rpc-listmodules
**Source**: blit.proto:line 27
**Specificity**: high

`rpc ListModules(ListModulesRequest) returns (ListModulesResponse)` — lists available modules on the server.

#### iface-rpc-find
**Source**: blit.proto:line 30
**Specificity**: high

`rpc Find(FindRequest) returns (stream FindEntry)` — recursively finds files/directories starting at a module path.

#### iface-rpc-diskusage
**Source**: blit.proto:line 33
**Specificity**: high

`rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry)` — summarizes disk usage for a subtree (du-style).

#### iface-rpc-filesystemstats
**Source**: blit.proto:line 36
**Specificity**: high

`rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse)` — module/storage capacity (df-style).

#### iface-rpc-delegatedpull
**Source**: blit.proto:lines 38-48
**Specificity**: high

`rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress)` — destination-side delegated initiator. CLI calls on destination daemon when both endpoints in `blit copy` are remote. Destination validates via delegation gate, opens its own pull against named source, streams progress/results back. Bytes flow source→dst directly; CLI not in byte path.

#### iface-rpc-getstate
**Source**: blit.proto:lines 50-57
**Specificity**: high

`rpc GetState(GetStateRequest) returns (DaemonState)` — daemon-state snapshot for TUI's F1 (Daemons) and F2 (Transfers) panes, plus `blit jobs list <remote>`. Always available regardless of `--metrics`.

#### iface-rpc-canceljob
**Source**: blit.proto:lines 59-76
**Specificity**: high

`rpc CancelJob(CancelJobRequest) returns (CancelJobResponse)` — cancel a daemon-side in-flight transfer by `transfer_id`.

#### iface-rpc-clearrecent
**Source**: blit.proto:lines 78-89
**Specificity**: high

`rpc ClearRecent(ClearRecentRequest) returns (ClearRecentResponse)` — clear daemon's recent-transfers list (wipes ring + recents.jsonl).

#### iface-rpc-subscribe
**Source**: blit.proto:lines 91-107
**Specificity**: high

`rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent)` — server-streaming subscription to live daemon events. Used by TUI's F2 pane and future `blit jobs watch`.

#### iface-data-plane-tcp
**Source**: ARCHITECTURE.md:§Hybrid Data Plane; WHITEPAPER.md:§1; DAEMON_CONFIG.md:§TCP Data Plane
**Specificity**: high

Hybrid: gRPC control plane (manifest, coordination, status, resumable/restartable) + TCP data plane (parallel streams, zero-copy, bulk transfer, throughput path). `DataTransferNegotiation` coordinates handoff.

#### iface-grpc-fallback
**Source**: WHITEPAPER.md:§1; DAEMON_CONFIG.md:§Performance Tuning; blit.proto:TransferOperationSpec.force_grpc
**Specificity**: high

Optional `--force-grpc` / `--force-grpc-data` fallback that pushes file bytes via the gRPC channel. Origin still honors filter/compare/mirror/resume; only byte transport changes.

#### iface-data-transfer-negotiation
**Source**: ARCHITECTURE.md:§Hybrid Data Plane; blit.proto:DataTransferNegotiation
**Specificity**: high

`DataTransferNegotiation` fields: `tcp_port` (1), `one_time_token` (2), `tcp_fallback` (3, true when server couldn't reserve preferred data plane), `stream_count` (4, parallel TCP streams negotiated). Fields 5-10 reserved for RDMA (QP numbers, GID, etc.) — "Phase 3.5".

#### iface-mdns-service
**Source**: DAEMON_CONFIG.md:§mDNS Discovery; ARCHITECTURE.md:§blit-cli scan.rs
**Specificity**: high

Daemon advertises on `_blit._tcp.local.` via mDNS. TXT record includes `version` (daemon version) and `modules` (comma-separated list of exported module names, truncated to 180 chars). Instance name defaults to `blit@<hostname>`.

#### iface-mdns-disable
**Source**: DAEMON_CONFIG.md:§mDNS Discovery; §Configuration Reference
**Specificity**: high

Disable mDNS with `--no-mdns` flag or `no_mdns = true` in `[daemon]` config.

#### iface-remote-url-syntax
**Source**: DAEMON_CONFIG.md:§Remote URL Syntax
**Specificity**: high

URL forms: `host:/module/path` (explicit module + path), `host:port:/module/path` (custom port), `host` (bare host, for list-modules/scan). Remote paths must use forward slashes (`/`), not backslashes.

#### iface-clientpushrequest-oneof
**Source**: blit.proto:lines 130-141
**Specificity**: high

`ClientPushRequest.payload` oneof variants: `PushHeader header=1`, `FileHeader file_manifest=2`, `ManifestComplete manifest_complete=3`, `FileData file_data=4`, `UploadComplete upload_complete=5`, `TarShardHeader tar_shard_header=6`, `TarShardChunk tar_shard_chunk=7`, `TarShardComplete tar_shard_complete=8`.

#### iface-serverpushresponse-oneof
**Source**: blit.proto:lines 143-150
**Specificity**: high

`ServerPushResponse.payload` oneof variants: `Ack ack=1`, `FileList files_to_upload=2` (the "NeedList"), `PushSummary summary=3`, `DataTransferNegotiation negotiation=4`.

#### iface-pushheader-fields
**Source**: blit.proto:lines 152-186
**Specificity**: high

`PushHeader`: `module` (1), `mirror_mode` (2, bool — master switch for purging), `destination_path` (3), `force_grpc` (4), `FilterSpec filter` (5), `MirrorMode mirror_kind` (6), `require_complete_scan` (7, bool).

#### iface-fileheader-fields
**Source**: blit.proto:lines 188-194
**Specificity**: high

`FileHeader`: `relative_path` (1), `size` (2), `mtime_seconds` (3), `permissions` (4), `checksum` (5, Blake3 32-byte hash, empty if not computed).

#### iface-pullsyncack
**Source**: blit.proto:lines 224-227
**Specificity**: high

`PullSyncAck`: `server_checksums_enabled` (1, whether daemon computed checksums for manifest).

#### iface-pushsummary
**Source**: blit.proto:lines 229-235
**Specificity**: high

`PushSummary`: `files_transferred` (1), `bytes_transferred` (2), `bytes_zero_copy` (3, bytes sent via zero-copy kernel paths), `tcp_fallback_used` (4), `entries_deleted` (5, files/dirs removed during mirror purge).

#### iface-pullrequest
**Source**: blit.proto:lines 238-243
**Specificity**: high

`PullRequest`: `module` (1), `path` (2), `force_grpc` (3), `metadata_only` (4).

#### iface-pullchunk-oneof
**Source**: blit.proto:lines 245-253
**Specificity**: high

`PullChunk.payload` oneof: `FileHeader file_header=1`, `FileData file_data=2`, `DataTransferNegotiation negotiation=3`, `PullSummary summary=4`, `ManifestBatch manifest_batch=5`.

#### iface-manifestbatch
**Source**: blit.proto:lines 255-258
**Specificity**: high

`ManifestBatch`: `file_count` (1), `total_bytes` (2).

#### iface-pullsummary
**Source**: blit.proto:lines 260-266
**Specificity**: high

`PullSummary`: `files_transferred` (1), `bytes_transferred` (2), `bytes_zero_copy` (3), `tcp_fallback_used` (4), `entries_deleted` (5).

#### iface-clientpullmessage-oneof
**Source**: blit.proto:lines 273-280
**Specificity**: high

`ClientPullMessage.payload` oneof: `TransferOperationSpec spec=1` (initial request), `FileHeader local_file=2`, `ManifestComplete manifest_done=3`, `BlockHashList block_hashes=4` (for resume mode).

#### iface-blockhashlist
**Source**: blit.proto:lines 282-287
**Specificity**: high

`BlockHashList`: `relative_path` (1), `block_size` (2, last may be smaller), `hashes` (3, Blake3 32-byte hashes in order).

#### iface-blocktransfer
**Source**: blit.proto:lines 289-293
**Specificity**: high

`BlockTransfer`: `relative_path` (1), `offset` (2, byte offset in file), `content` (3, block data).

#### iface-blocktransfercomplete
**Source**: blit.proto:lines 295-298
**Specificity**: high

`BlockTransferComplete`: `relative_path` (1), `total_bytes` (2, final file size for truncation).

#### iface-serverpullmessage-oneof
**Source**: blit.proto:lines 300-331
**Specificity**: high

`ServerPullMessage.payload` oneof: `Ack ack=1` (deprecated, use pull_sync_ack), `FileList files_to_download=2` (NeedList), `DataTransferNegotiation negotiation=3`, `PullSummary summary=4`, `ManifestBatch manifest_batch=5`, `FileHeader file_header=6` (gRPC fallback), `FileData file_data=7` (gRPC fallback), `PullSyncAck pull_sync_ack=8`, `BlockHashRequest block_hash_request=9`, `BlockTransfer block_transfer=10`, `BlockTransferComplete block_complete=11`, `FileList delete_list=12`, `TarShardHeader tar_shard_header=13`, `TarShardChunk tar_shard_chunk=14`, `TarShardComplete tar_shard_complete=15`.

#### iface-blockhashrequest
**Source**: blit.proto:lines 333-337
**Specificity**: high

`BlockHashRequest`: `relative_path` (1), `block_size` (2).

#### iface-listrequest-response
**Source**: blit.proto:lines 340-347
**Specificity**: high

`ListRequest`: `module` (1), `path` (2). `ListResponse`: `repeated FileInfo entries` (1). `FileInfo`: `name` (1), `is_dir` (2), `size` (3), `mtime_seconds` (4).

#### iface-purgerequest-response
**Source**: blit.proto:lines 349-350
**Specificity**: high

`PurgeRequest`: `module` (1), `paths_to_delete` (2). `PurgeResponse`: `files_deleted` (1).

#### iface-completionrequest-response
**Source**: blit.proto:lines 352-358
**Specificity**: high

`CompletionRequest`: `module` (1), `path_prefix` (2), `include_files` (3), `include_directories` (4). `CompletionResponse`: `repeated string completions` (1).

#### iface-listmodules-response
**Source**: blit.proto:lines 360-366
**Specificity**: high

`ListModulesRequest` (empty). `ListModulesResponse`: `repeated ModuleInfo modules`. `ModuleInfo`: `name` (1), `path` (2), `read_only` (3).

#### iface-findrequest
**Source**: blit.proto:lines 368-376
**Specificity**: high

`FindRequest`: `module` (1), `start_path` (2), `pattern` (3), `case_sensitive` (4), `include_files` (5), `include_directories` (6), `max_results` (7).

#### iface-findentry
**Source**: blit.proto:lines 378-383
**Specificity**: high

`FindEntry`: `relative_path` (1), `is_dir` (2), `size` (3), `mtime_seconds` (4).

#### iface-diskusage
**Source**: blit.proto:lines 385-396
**Specificity**: high

`DiskUsageRequest`: `module` (1), `start_path` (2), `max_depth` (3). `DiskUsageEntry`: `relative_path` (1), `byte_total` (2), `file_count` (3), `dir_count` (4).

#### iface-filesystemstats
**Source**: blit.proto:lines 398-407
**Specificity**: high

`FilesystemStatsRequest`: `module` (1). `FilesystemStatsResponse`: `module` (1), `total_bytes` (2), `used_bytes` (3), `free_bytes` (4).

#### iface-transferoperationspec
**Source**: blit.proto:lines 424-491
**Specificity**: high

`TransferOperationSpec` (unified contract): `spec_version` (1), `module` (2), `source_path` (3), `FilterSpec filter` (4), `ComparisonMode compare_mode` (5), `MirrorMode mirror_mode` (6), `ResumeSettings resume` (7), `PeerCapabilities client_capabilities` (8), `force_grpc` (9), `ignore_existing` (10), `require_complete_scan` (11).

#### iface-filterspec
**Source**: blit.proto:lines 495-518
**Specificity**: high

`FilterSpec`: `include` (1, whitelist), `exclude` (2, blacklist), `min_size` (3, optional), `max_size` (4, optional), `min_age_secs` (5, optional), `max_age_secs` (6, optional), `files_from` (7, explicit file list bypassing other rules for inclusion).

#### iface-comparisonmode-enum
**Source**: blit.proto:lines 523-543
**Specificity**: high

`ComparisonMode`: UNSPECIFIED=0 (treat as SIZE_MTIME), SIZE_MTIME=1, CHECKSUM=2 (Blake3 hash equality), SIZE_ONLY=3, IGNORE_TIMES=4, FORCE=6. Number 5 reserved (previously IGNORE_EXISTING, removed in favor of orthogonal field).

#### iface-mirrormode-enum
**Source**: blit.proto:lines 548-562
**Specificity**: high

`MirrorMode`: UNSPECIFIED=0 (treat as OFF), OFF=1, FILTERED_SUBSET=2 (default, mirror only within filter scope), ALL=3 (mirror full target tree against filtered source).

#### iface-resumesettings
**Source**: blit.proto:lines 564-570
**Specificity**: high

`ResumeSettings`: `enabled` (1), `block_size` (2, 0 means origin chooses currently 1 MiB).

#### iface-peercapabilities
**Source**: blit.proto:lines 575-584
**Specificity**: high

`PeerCapabilities`: `supports_resume` (1), `supports_tar_shards` (2), `supports_data_plane_tcp` (3), `supports_filter_spec` (4).

#### iface-delegatedpullrequest
**Source**: blit.proto:lines 591-643
**Specificity**: high

`DelegatedPullRequest`: `dst_module` (1), `dst_destination_path` (2), `RemoteSourceLocator src` (10), `TransferOperationSpec spec` (20), `trace_data_plane` (31), `detach` (32).

#### iface-remotesourcelocator
**Source**: blit.proto:lines 645-658
**Specificity**: high

`RemoteSourceLocator`: `host` (1), `port` (2). Field 10 was `delegated_credential` (removed 2026-05-13, reserved).

#### iface-delegatedpullprogress-oneof
**Source**: blit.proto:lines 660-668
**Specificity**: high

`DelegatedPullProgress.payload` oneof: `DelegatedPullStarted started=1`, `ManifestBatch manifest_batch=2`, `BytesProgress bytes_progress=3`, `DelegatedPullSummary summary=4`, `DelegatedPullError error=5`.

#### iface-delegatedpullstarted
**Source**: blit.proto:lines 670-684
**Specificity**: high

`DelegatedPullStarted`: `source_data_plane_endpoint` (1, "tcp:host:port" or "grpc-fallback"), `stream_count` (2), `transfer_id` (3, daemon-assigned, empty when daemon older than m-jobs-3).

#### iface-bytesprogress
**Source**: blit.proto:lines 686-693
**Specificity**: high

`BytesProgress`: `files_completed` (1), `files_total` (2), `bytes_completed` (3), `bytes_total` (4). Cumulative.

#### iface-delegatedpullsummary
**Source**: blit.proto:lines 695-707
**Specificity**: high

`DelegatedPullSummary`: `files_transferred` (1), `bytes_transferred` (2), `bytes_zero_copy` (3), `tcp_fallback_used` (4), `entries_deleted` (5), `source_peer_observed` (6, dst-observed data-plane peer address).

#### iface-delegatedpullerror-phases
**Source**: blit.proto:lines 709-721
**Specificity**: high

`DelegatedPullError`: `upstream_message` (1), `Phase phase` (2). Phase enum: UNKNOWN=0, DELEGATION_REJECTED=1 (gate denied), CONNECT_SOURCE=2 (dst could not reach src), NEGOTIATE=3 (src refused dst), TRANSFER=4 (mid-transfer pull failure), APPLY=5 (dst-side apply failure: containment, disk, perms).

#### iface-getstaterequest
**Source**: blit.proto:lines 727-733
**Specificity**: high

`GetStateRequest`: `recent_limit` (1, 0=use daemon default of 50, ignored if larger than what daemon tracks).

#### iface-daemonstate-fields
**Source**: blit.proto:lines 735-760
**Specificity**: high

`DaemonState`: `version` (1, CARGO_PKG_VERSION), `uptime_seconds` (2), `repeated ModuleInfo modules` (3), `repeated ActiveTransfer active` (4), `repeated TransferRecord recent` (5, oldest-first, bounded by ring depth default 50), `Counters counters` (6), `delegation_enabled` (7, mirrors mDNS TXT advertisement).

#### iface-transferkind-enum
**Source**: blit.proto:lines 766-772
**Specificity**: high

`TransferKind`: UNSPECIFIED=0, PUSH=1, PULL=2, PULL_SYNC=3, DELEGATED_PULL=4. Top-level (not nested) so milestone C's Subscribe events can share without a cycle.

#### iface-activetransfer
**Source**: blit.proto:lines 774-790
**Specificity**: high

`ActiveTransfer`: `transfer_id` (1), `TransferKind kind` (2), `peer` (3, `<ip>:<port>` or "unknown"), `module` (4), `path` (5, empty until streaming RPCs parse first frame), `start_unix_ms` (6), `bytes_completed` (7, always zero in milestone B), `bytes_total` (8, always zero in milestone B).

#### iface-transferrecord
**Source**: blit.proto:lines 792-812
**Specificity**: high

`TransferRecord`: `transfer_id` (1), `TransferKind kind` (2), `peer` (3), `module` (4), `path` (5), `start_unix_ms` (6), `duration_ms` (7, saturating at zero), `bytes` (8), `files` (9), `ok` (10), `error_message` (11).

#### iface-counters
**Source**: blit.proto:lines 814-820
**Specificity**: high

`Counters`: `push_operations_total` (1), `pull_operations_total` (2), `purge_operations_total` (3), `active_transfers` (4), `transfer_errors_total` (5).

#### iface-canceljob-messages
**Source**: blit.proto:lines 827-837
**Specificity**: high

`CancelJobRequest`: `transfer_id` (1). `CancelJobResponse`: `transfer_id` (1, echoed for confirmation; outcome encoded in gRPC Status not response body).

#### iface-clearrecent-messages
**Source**: blit.proto:lines 845-850
**Specificity**: high

`ClearRecentRequest` (empty). `ClearRecentResponse`: `cleared` (1, number of entries removed from in-memory ring).

#### iface-subscriberequest
**Source**: blit.proto:lines 857-888
**Specificity**: high

`SubscribeRequest`: `event_mask` (1, reserved for future TRANSFERS=1/ERRORS=2/MODULES=4/HEARTBEAT=8; today parsed and ignored), `replay_recent` (2, when true+transfer_id_filter non-empty daemon replays per-job event ring), `transfer_id_filter` (3, daemon-side filtering by transfer_id).

#### iface-daemonevent-oneof
**Source**: blit.proto:lines 895-905
**Specificity**: high

`DaemonEvent.payload` oneof: `TransferStarted transfer_started=1`, `TransferProgress transfer_progress=2`, `TransferComplete transfer_complete=3`, `TransferError transfer_error=4`. Fields 5-6 reserved for ModuleListChanged, DaemonHeartbeat (later C sub-slices).

#### iface-transferstarted
**Source**: blit.proto:lines 914-929
**Specificity**: high

`TransferStarted`: `transfer_id` (1), `TransferKind kind` (2), `peer` (3), `module` (4, empty for streaming RPCs at registration), `path` (5), `start_unix_ms` (6).

#### iface-transferprogress
**Source**: blit.proto:lines 943-959
**Specificity**: high

`TransferProgress`: `transfer_id` (1), `bytes_completed` (2), `bytes_total` (3, 0 until future C sub-slice), `files_completed` (4), `files_total` (5), `throughput_bps` (6, instantaneous over most recent tick; future may replace with EWMA). Daemon fires one per active row per tick (default 10 Hz, see DEFAULT_PROGRESS_TICK_MS).

#### iface-transfercomplete
**Source**: blit.proto:lines 966-982
**Specificity**: high

`TransferComplete`: `transfer_id` (1), `bytes` (2), `files` (3, always zero until future sub-slice), `duration_ms` (4), `tcp_fallback_used` (5, always false in this slice).

#### iface-transfererror
**Source**: blit.proto:lines 987-994
**Specificity**: high

`TransferError`: `transfer_id` (1), `message` (2, "cancelled before outcome recorded" when spawn task didn't reach record_outcome).

#### iface-daemon-toml-daemon
**Source**: DAEMON_CONFIG.md:§[daemon] Section
**Specificity**: high

`[daemon]` section options: `bind` (string, default "0.0.0.0"), `port` (int, default 9031), `motd` (string), `no_mdns` (bool, default false), `mdns_name` (string, default `blit@<hostname>`), `root` (string), `root_read_only` (bool, default false), `no_server_checksums` (bool, default false).

#### iface-daemon-toml-module
**Source**: DAEMON_CONFIG.md:§[[module]] Array
**Specificity**: high

`[[module]]` array-of-tables fields: `name` (required, non-empty, unique), `path` (required, absolute), `read_only` (bool, default false), `comment` (string), `delegation_allowed` (bool, default true; per-module narrowing override).

#### iface-daemon-toml-delegation
**Source**: DAEMON_CONFIG.md:§[delegation] Section
**Specificity**: high

`[delegation]` fields: `allow_delegated_pull` (bool, default false, master switch), `allowed_source_hosts` (array of strings, default `[]`; accepts hostnames IDNA-normalized, CIDR blocks IPv4/IPv6 via ipnet, bare IP literals with optional brackets for IPv6; invalid entries fail config load).

#### iface-daemon-cli-flags
**Source**: DAEMON_CONFIG.md:§Command-Line Options
**Specificity**: high

`blit-daemon` CLI: `--config <PATH>`, `--bind <ADDR>`, `--port <PORT>`, `--root <PATH>`, `--no-mdns`, `--mdns-name <NAME>`, `--force-grpc-data`, `--no-server-checksums`, `-h/--help`.

#### iface-default-config-paths
**Source**: DAEMON_CONFIG.md:§Configuration File
**Specificity**: high

Default config paths: Linux/macOS `/etc/blit/config.toml`; Windows `C:\ProgramData\Blit\config.toml`. Loaded automatically if exists.

#### iface-client-config-paths
**Source**: DAEMON_CONFIG.md:§Client Configuration
**Specificity**: high

`blit` CLI config dir: macOS `~/Library/Application Support/Blit/Blit/`, Linux `$XDG_CONFIG_HOME/Blit/` or `~/.config/Blit/`, Windows `C:\Users\<user>\AppData\Local\Blit\Blit\`, Fallback `~/.config/blit/`. Override with `--config-dir <PATH>`.

#### iface-client-stored-files
**Source**: DAEMON_CONFIG.md:§Client Configuration
**Specificity**: high

Files stored: `settings.json` (performance history toggle), `perf_local.jsonl` (transfer performance records, ~1 MiB cap), `journal_cache.json` (change journal checkpoints — Windows USN, macOS FSEvents, Linux metadata).

#### iface-cli-priority
**Source**: DAEMON_CONFIG.md:§Priority
**Specificity**: high

Configuration priority: CLI flag value (highest) > config file value > built-in default.

#### iface-access-default-module
**Source**: DAEMON_CONFIG.md:§Root-Based Access
**Specificity**: high

Without modules (using `--root`), daemon exports root path as module named `default`. If no modules and no `--root` specified, daemon exports its current working directory as `default` and emits a warning.

#### iface-data-plane-record-formats
**Source**: WHITEPAPER.md:§3 wire format block
**Specificity**: high

Wire records: FILE=0x00, TAR_SHARD=0x01 (count, then N {path_len/path/size/mtime/perms}, then tar_size + tar_bytes), BLOCK=0x02 (path_len, path, offset:u64, len:u32, bytes), BLOCK_COMPLETE=0x03 (path_len, path, total_size:u64, mtime:i64, perms:u32), END=0xFF.

#### iface-transfer-pipeline-wiring
**Source**: ARCHITECTURE.md:§Per-direction wiring
**Specificity**: high

Per-direction Source/Sink: local→local = FsTransferSource/FsTransferSink; local→remote push TCP = FsTransferSource / N×DataPlaneSink; local→remote gRPC fallback = FsTransferSource/GrpcFallbackSink; remote→local pull TCP = daemon's FsTransferSource / N×DataPlaneSink (on daemon); remote→remote = RemoteTransferSource / N×DataPlaneSink.

#### iface-destination-resolution
**Source**: ARCHITECTURE.md:§Destination resolution
**Specificity**: high

`resolve_destination` applies rsync-style trailing-slash semantics uniformly across all directions: Source ends with `/`, `/.`, or is exactly `.` → merge contents into dest; Dest has trailing slash or is an existing local directory → nest under dest; Otherwise → use dest as exact target (rename-style).

### behavior

#### behavior-zero-copy-cascade
**Source**: ARCHITECTURE.md:§Copy Operations; WHITEPAPER.md:§1
**Specificity**: high

Zero-copy cascade: copy_file_range, sendfile, clonefile, block clone. Platform: macOS=clonefile (APFS CoW) / fcopyfile fallback; Linux=copy_file_range (4.5+) / sendfile fallback; Windows=CopyFileExW / ReFS block cloning fallback.

#### behavior-change-journal-platforms
**Source**: ARCHITECTURE.md:§Change Journal
**Specificity**: high

Platform implementations: Windows = USN Change Journal via DeviceIoControl; macOS = FSEvents API; Linux = fallback to mtime comparison.

#### behavior-fs-capability-probing
**Source**: ARCHITECTURE.md:§Filesystem Capability Probing
**Specificity**: high

Detection methods: macOS=statfs→f_fstypename, Linux=statfs→f_type magic number mapping, Windows=GetVolumeInformationW→filesystem name. Detected capabilities (reflink, sparse files, xattrs, sendfile, copy_file_range, block cloning) cached per device ID. Supported FS types: APFS, HFS+, btrfs, XFS, ext2/3/4, ZFS, tmpfs, NFS/CIFS/SMB, NTFS, ReFS.

#### behavior-small-file-batching
**Source**: ARCHITECTURE.md:§Small File Batching; WHITEPAPER.md:§3
**Specificity**: high

Files under a size threshold are batched into tar archives. The planner targets ~8–64 MiB shard size.

#### behavior-tar-shard-targets
**Source**: WHITEPAPER.md:§4
**Specificity**: high

Shard target adapts to total: 4 / 32 / 64 MiB depending on workload size. Count target: 256 / 1024 / 2048 entries depending on file count. Final `chunk_bytes` (network framing) adaptive 16 vs 32 MiB.

#### behavior-build-plan-thresholds
**Source**: WHITEPAPER.md:§4
**Specificity**: high

`build_plan` categorization: size <64 KiB → small (tar shard candidate); <1 MiB → small; <256 MiB → medium (raw bundle); ≥256 MiB → large_files (single TransferTask::Large). use_tar: force_tar → ≥1; otherwise <2 small=false; ≥32 small or avg_small_size ≤128 KiB.

#### behavior-double-buffered-1mib
**Source**: WHITEPAPER.md:§3.1
**Specificity**: high

Send and receive byte-copy loops are intentionally similar: double-buffered, 1 MiB chunks; max two outstanding per session. Without these (e.g. with tokio::io::copy's default 8 KiB buffer), measured throughput dropped from 9.3 → 1 Gbps on push to ZFS.

#### behavior-round-robin-dispatch
**Source**: WHITEPAPER.md:§2.1
**Specificity**: high

`execute_sink_pipeline_streaming` dispatcher pulls from `payload_rx` and round-robins to per-sink channels. "Round-robin is deliberately simple. Adaptive load-balancing is left to the lower layers."

#### behavior-receive-pipeline-unified
**Source**: WHITEPAPER.md:§2.2; §7.1
**Specificity**: high

Daemon's push-receive task and client's pull-receive task both call `execute_receive_pipeline(socket, FsTransferSink, progress)`. ~525 LOC deleted from daemon's bespoke dispatch loop.

#### behavior-mtime-perms-inline
**Source**: WHITEPAPER.md:§7.1; §2.2
**Specificity**: high

Wire format extended so FILE records carry mtime + perms inline, eliminating daemon's manifest cache requirement.

#### behavior-tar-shard-two-phase
**Source**: WHITEPAPER.md:§7.2
**Specificity**: high

Two-phase tar shard extraction: Phase 1 walks in-memory tar serially, buffers (path, contents); Phase 2 writes files to disk in parallel via rayon.

#### behavior-mtime-drop-before-set
**Source**: WHITEPAPER.md:§7.3
**Specificity**: high

mtime preservation fix: when `set_file_mtime` was called while tokio File handle was still open (deferred writes in flight), 5/8 files lost mtime. Fix: drop the handle before the syscall.

#### behavior-pull-sync-gRPC-stream-first
**Source**: WHITEPAPER.md:§7.4
**Specificity**: high

pull_sync deadlock fix: open the gRPC bidi stream FIRST so the daemon starts consuming, then send header and manifest. Manifest send runs as separate spawned task so daemon responses don't block manifest send.

#### behavior-files-to-upload-terminator
**Source**: WHITEPAPER.md:§7.5
**Specificity**: high

Daemon always emits an empty `FilesToUpload` as "no more need-lists" terminator. Client sets `need_lists_done = true` on receipt and gates early finish on that flag.

#### behavior-daemon-push-receive-drain
**Source**: WHITEPAPER.md:§7.6
**Specificity**: high

After receive unification, per-stream handler spawns a drain task that consumes the channel's contents and discards them (since data plane no longer needs them).

#### behavior-perf-history-record
**Source**: ARCHITECTURE.md:§Performance History
**Specificity**: medium

`PerformanceRecord` carries `timestamp`, `bytes_transferred`, `duration_ms`, `file_count`, `strategy_used`.

#### behavior-auto-tune-inputs
**Source**: WHITEPAPER.md:§5
**Specificity**: high

`auto_tune` inputs: total bytes of manifest, number of files, prior `perf_history` records keyed by transfer profile. Output: TuningParams { initial_streams, max_streams, chunk_bytes, prefetch_count, tcp_buffer_size }. Output values smoothed across runs (`perf_predictor.rs`).

#### behavior-pull-sync-flow
**Source**: WHITEPAPER.md:§6
**Specificity**: high

pull_sync flow: (1) Client opens bidi gRPC stream. (2) Client sends `PullSyncHeader` then `LocalFile` per local entry, then `ManifestDone`. (3) Daemon enumerates source, builds server manifest. (4) `compare_manifests(source, target, opts)` produces ManifestDiff. (5) If diff empty: daemon sends Summary, both sides finish. (6) Otherwise: daemon sends FilesToDownload need-list, opens TCP data plane, streams via outbound pipeline.

Note: leading message is now `TransferOperationSpec` (per proto comment lines 269-272); `PullSyncHeader` was removed entirely.

#### behavior-compare-modes
**Source**: WHITEPAPER.md:§6
**Specificity**: high

`compare_file` switches on CompareMode: Default = size+mtime, SizeOnly, IgnoreTimes, Force, Checksum.

#### behavior-block-resume-blake3
**Source**: WHITEPAPER.md:§6
**Specificity**: high

For files marked Modified (size matched but mtime differs, or explicit `--resume`), daemon can request block hashes via gRPC and send only differing blocks via data plane. Blake3-block-hash-based delta protocol, NOT rsync's rolling checksum.

#### behavior-tui-subscribe-merge
**Source**: ARCHITECTURE.md:§blit-tui
**Specificity**: medium

TUI is read-mostly control surface over the daemon: Subscribe's to each discovered daemon's `DaemonEvent` stream and renders live transfer state from `GetState`; can launch transfers / CancelJob / ClearRecent. Daemon discovery is mDNS; multi-daemon F2 merges per-daemon Subscribe streams into one event channel.

#### behavior-prometheus-bridge-pull-model
**Source**: ARCHITECTURE.md:§blit-prometheus-bridge
**Specificity**: high

Standalone Prometheus exporter. Minimal hand-rolled HTTP server serves `GET /metrics`; each scrape triggers a fresh `GetState` against configured daemon (pull model, no background poll) and formats result as Prometheus text. A failed/timed-out scrape still returns `200` with `blit_daemon_up 0` so target registers as up-but-down rather than scrape error.

#### behavior-cli-output-defaults
**Source**: ARCHITECTURE.md:§Admin verbs
**Specificity**: high

All remote commands connect via gRPC to a running daemon. Output defaults to human-readable tables; `--json` emits machine-parsable JSON for scripting.

#### behavior-progress-tick
**Source**: blit.proto:TransferProgress comment (lines 933-942)
**Specificity**: high

Daemon fires one TransferProgress per active row per tick (default 10 Hz, see DEFAULT_PROGRESS_TICK_MS in service/core.rs).

#### behavior-detach-disarms-tx-closed
**Source**: blit.proto:DelegatedPullRequest.detach comment (lines 622-642)
**Specificity**: high

When `detach=false` (historical), destination daemon races transfer against `tx.closed()` so CLI disconnect drops in-flight pull future and data plane tears down (R30-F2). When `detach=true`, the race disarms: destination daemon owns the transfer through completion, failure, or `CancelJob(transfer_id)`. CLI free to exit immediately after `Started` event.

### scope

#### scope-three-transfer-verbs
**Source**: WHITEPAPER.md:§1
**Specificity**: high

Three transfer verbs: `copy`, `mirror`, `move`. Four src/dst combinations: local→local, local→remote, remote→local, remote→remote.

#### scope-cli-verb-list
**Source**: ARCHITECTURE.md:§blit-cli structure
**Specificity**: high

`blit` binary verbs: copy/mirror/move/scan/list/list-modules/ls/find/du/df/rm/completions/profile/diagnostics.

#### scope-workspace-crates
**Source**: WHITEPAPER.md:§top; ARCHITECTURE.md:§Crate Structure
**Specificity**: high

Workspace crates: `blit-core` (library), `blit-cli` (produces `blit` binary), `blit-daemon` (server binary). Plus `blit-app` (shared orchestration library), `blit-tui` (TUI binary), `blit-prometheus-bridge` (exporter binary).

#### scope-blit-app-modules
**Source**: ARCHITECTURE.md:§blit-app
**Specificity**: high

`blit-app` modules: `endpoints`, `transfers` (dispatch, resolution, filter, local, remote, remote_remote_direct), `client` (DNS-aware connect), `admin` (ls/find/du/df/rm/jobs/list_modules), `check`, `scan`, `diagnostics`, `profile`, `display`.

#### scope-binary-name-blit
**Source**: WHITEPAPER.md:§top; ARCHITECTURE.md:§blit-cli
**Specificity**: high

Cargo package is `blit-cli`; produced binary is named `blit` (`[[bin]] name = "blit"`). Admin verbs originally scoped as separate `blit-utils` artifact were merged into this binary during Phase 3.

#### scope-tui-bindings
**Source**: ARCHITECTURE.md:§blit-tui
**Specificity**: high

TUI panes: F1 (trigger/daemons), F2 (transfers), F3 (browse), F4 (profile/verify/diagnostics). Configurable keybindings and theming via `[keys]` / `[theme]` config.

#### scope-test-categories
**Source**: ARCHITECTURE.md:§Testing Strategy
**Specificity**: medium

Test categories: local_transfers, remote_*, remote_transfer_edges, admin_verbs, mirror_planner, perf_predictor, perf_history, fs_capability.

#### scope-cli-config-dir-override
**Source**: DAEMON_CONFIG.md:§Client Configuration
**Specificity**: high

Override CLI config dir with `--config-dir <PATH>`.

### non-goal

#### nongoal-daemon-no-tls
**Source**: DAEMON_CONFIG.md:§Trust Model; §Transport Security
**Specificity**: high

TLS termination, mutual auth, and access-control fronting are explicitly out of scope for the daemon. Operators expected to firewall the daemon port, restrict to trusted network, or front it with WireGuard/Tailscale/SSH-tunnel.

#### nongoal-no-builtin-auth
**Source**: DAEMON_CONFIG.md:§Auth posture
**Specificity**: high

There is no daemon authentication. The trust model is "reachable on a trusted network." The delegation gate is policy, not authentication. This is intentional and not on the roadmap. The `BlitAuth` proto stub and `delegated_credential` passthrough field were removed 2026-05-13 — auth is out of project scope.

#### nongoal-no-default-safe-mode
**Source**: DAEMON_CONFIG.md:§Trust Model
**Specificity**: high

There is no "default safe" mode — you choose the exposure that fits your network. Daemon binds 0.0.0.0:9031 by default.

#### nongoal-no-toctou-race-proof
**Source**: DAEMON_CONFIG.md:§Path containment TOCTOU
**Specificity**: medium

A fully race-proof variant would use `openat` + `O_NOFOLLOW` per-component descent; that is deferred until there's a concrete threat that warrants it.

#### nongoal-no-hostname-wildcards
**Source**: DAEMON_CONFIG.md:§Allowlist matching #2
**Specificity**: high

Hostname matches are exact post-normalization equality. No wildcards in 0.1.0.

#### nongoal-cancel-for-non-delegated
**Source**: blit.proto:CancelJob comment (lines 59-65)
**Specificity**: high

Push/pull/pull_sync do not honor CancelJob — they have CLI in byte path so client-side cancel suffices. Return FAILED_PRECONDITION.

#### nongoal-detach-non-delegated
**Source**: blit.proto:DelegatedPullRequest.detach comment
**Specificity**: high

CLI rejects `--detach` for push/pull/pull_sync routes; daemon-side flag unused on non-delegated kinds.

#### nongoal-hardcoded-not-philosophical
**Source**: WHITEPAPER.md:§8.4
**Specificity**: medium

"Each of these is a reasonable static default; none is adaptive. The project's stated philosophy says they should be." (RECEIVE_CHUNK_SIZE, MAX_PARALLEL_TAR_TASKS, tar shard thresholds, mpsc channel capacities listed as the gap.)

#### nongoal-auto-tune-coverage-gaps
**Source**: WHITEPAPER.md:§5
**Specificity**: high

Currently `auto_tune` does NOT cover manifest-batching parameters or receive-side parallelism; those are hardcoded in places. "This is the top architectural gap."

### shipped

#### shipped-receive-unification
**Source**: WHITEPAPER.md:§7.1
**Specificity**: high

Receive-pipeline unification shipped (commits 1baa981, a232dbd, b64bfd8). Both push-receive and pull-receive call execute_receive_pipeline. ~525 LOC deleted from daemon's bespoke dispatch loop.

#### shipped-block-complete-metadata
**Source**: WHITEPAPER.md:§3 note
**Specificity**: high

BLOCK_COMPLETE carries mtime and perms after total_size (commit a7d659f).

#### shipped-tar-shard-parallel
**Source**: WHITEPAPER.md:§7.2
**Specificity**: high

Tar shard parallel extraction shipped (commit 0bd8bde) — two-phase serial parse + rayon parallel write.

#### shipped-pullsync-deadlock-fix
**Source**: WHITEPAPER.md:§7.4
**Specificity**: high

pull_sync deadlock fix shipped (commit 946bd77) — open gRPC stream first, spawn manifest send concurrently with response loop.

#### shipped-mtime-race-fix
**Source**: WHITEPAPER.md:§7.3
**Specificity**: high

mtime preservation race fix shipped (commit 946bd77) — drop tokio File handle before set_file_mtime syscall.

#### shipped-push-small-file-completion
**Source**: WHITEPAPER.md:§7.5
**Specificity**: high

Push small-file completion race fix shipped (commits 60d152f, 5bb78d9) — daemon always emits empty FilesToUpload terminator, client gates early finish on need_lists_done.

#### shipped-daemon-receive-drain
**Source**: WHITEPAPER.md:§7.6
**Specificity**: high

Daemon push-receive backpressure fix shipped (commit b64bfd8) — spawn drain task to consume orphaned channel after receive unification.

#### shipped-module-containment-always-on
**Source**: DAEMON_CONFIG.md:§Path containment
**Specificity**: high

Module containment is always on as of F2. Previous `use_chroot` and `root_use_chroot` config options were removed.

#### shipped-blitauth-removed
**Source**: blit.proto:lines 110-116, 409; DAEMON_CONFIG.md:§Auth posture
**Specificity**: high

`BlitAuth` service stub removed 2026-05-13. Original design reserved an `Authenticate(token) -> bool` RPC for future token auth layer. Removed entirely from 0.1.0 scope.

#### shipped-delegated-credential-removed
**Source**: blit.proto:lines 654-657; DAEMON_CONFIG.md:§Auth posture
**Specificity**: high

`delegated_credential` passthrough field removed 2026-05-13. Reserved so it can't be reused for unrelated semantics by accident.

#### shipped-mirror-mode-typed
**Source**: blit.proto:lines 167-175; PushHeader.mirror_kind
**Specificity**: high

MirrorMode enum on the wire resolves the prior "client walks dest tree" and unconditional FileFilter::default() purge bugs (R59 #1 F2).

#### shipped-require-complete-scan
**Source**: blit.proto:lines 178-186, 213-220, 482-490
**Specificity**: high

`require_complete_scan` shipped (R59 #1 F1 and R49-F2). Pre-fix the daemon purged destination entries unconditionally after upload, so permission error mid-scan caused silent data loss.

#### shipped-tar-shards-grpc-fallback
**Source**: blit.proto:lines 322-329
**Specificity**: high

Tar-shard payloads on the gRPC server-streaming fallback shipped. The daemon's GrpcServerStreamingSink emits these for batched small files when TCP data plane isn't usable. Without these, gRPC fallback was artificially file-by-file.

#### shipped-getstate
**Source**: blit.proto:lines 50-57, 727-760
**Specificity**: high

GetState RPC shipped (milestone B + later). active[]/recent[] always-on from ActiveJobs table; counters from TransferMetrics atomics.

#### shipped-subscribe-skeleton
**Source**: blit.proto:lines 91-107, 895-905
**Specificity**: high

c-2-subscribe-skeleton lands the wire surface with single event variant (TransferStarted) and daemon-side broadcast channel.

### deferred

#### deferred-rdma-data-plane
**Source**: blit.proto:DataTransferNegotiation reserved 5-10; ARCHITECTURE.md:§Future Directions
**Specificity**: high

RDMA fields (QP numbers, GID, etc.) reserved for Phase 3.5 in DataTransferNegotiation message. "RDMA Support: Reserved fields in protocol for RDMA data plane."

#### deferred-compression
**Source**: ARCHITECTURE.md:§Future Directions
**Specificity**: low

Optional transfer compression — future direction.

#### deferred-end-to-end-encryption
**Source**: ARCHITECTURE.md:§Future Directions
**Specificity**: low

End-to-end encryption for data plane — future direction.

#### deferred-clustering
**Source**: ARCHITECTURE.md:§Future Directions
**Specificity**: low

Multi-daemon coordination ("Clustering") — future direction.

#### deferred-enhanced-incremental-sync
**Source**: ARCHITECTURE.md:§Future Directions
**Specificity**: low

Enhanced change journal integration — future direction.

#### deferred-tls-cert-validation
**Source**: ARCHITECTURE.md:§Planned Enhancements
**Specificity**: low

"TLS certificate validation" listed under planned enhancements. (Conflicts with DAEMON_CONFIG.md non-goal — see Contradictions.)

#### deferred-acl-per-module
**Source**: ARCHITECTURE.md:§Planned Enhancements
**Specificity**: low

"Per-module access control lists" listed under planned enhancements. (Conflicts with DAEMON_CONFIG.md non-goal — see Contradictions.)

#### deferred-audit-logging
**Source**: ARCHITECTURE.md:§Planned Enhancements
**Specificity**: low

"Audit logging" listed under planned enhancements.

#### deferred-adaptive-batched-manifest
**Source**: WHITEPAPER.md:§8.1
**Specificity**: high

Proposed fix for per-file gRPC overhead during push manifest: adaptive batched manifest — `FileManifestBatch` proto variant + Kafka-style opportunistic coalescing driven by in-flight backpressure (no linger_ms). Batch caps live in TuningParams, set by auto_tune::determine_remote_tuning using RTT × file count.

#### deferred-skip-manifest-phase
**Source**: WHITEPAPER.md:§8.1
**Specificity**: low

"More radical alternative ('skip the manifest phase entirely for fresh copies and stream tar shards directly') is mentioned in the same plan but not yet implemented."

#### deferred-mirror-auto-promote-block-hash
**Source**: WHITEPAPER.md:§8.2
**Specificity**: medium

The block-hash resume path exists but isn't triggered for plain `mirror` — only when `--resume` is set or file is newly `Modified`. "Worth examining: when, if ever, should mirror auto-promote a size-match-mtime-mismatch file to block-hash comparison?"

#### deferred-grpc-4gib-limit
**Source**: WHITEPAPER.md:§8.3
**Specificity**: high

PULL gRPC fallback >4 GiB body limit. The gRPC fallback path has an undocumented 4 GB body size cap that errors immediately on any single file ≥4 GiB. The TCP data plane has no such limit. Easy fix (chunk the gRPC frames) but unscheduled.

#### deferred-tuning-hardcoded
**Source**: WHITEPAPER.md:§8.4
**Specificity**: high

Hardcoded constants that should be in TuningParams: RECEIVE_CHUNK_SIZE = 1024*1024 in data_plane.rs; MAX_PARALLEL_TAR_TASKS = 4 in daemon's old TarShardExecutor (now only used by gRPC fallback); tar shard count thresholds (32, 1024, 2048) in transfer_plan.rs; mpsc channel capacities (32 in pull_sync, 32 in push manifest exchange).

#### deferred-fuzz-data-plane
**Source**: WHITEPAPER.md:§9
**Specificity**: medium

Notable test gaps: no fuzz tests for the data plane wire format.

#### deferred-pull-sync-deadlock-test
**Source**: WHITEPAPER.md:§9
**Specificity**: medium

No integration test for the pull_sync deadlock scenario fixed in 946bd77 — would be a 30-line test (mirror to populated dir).

#### deferred-mtime-preservation-test
**Source**: WHITEPAPER.md:§9
**Specificity**: medium

No tests verify mtime preservation end-to-end (bug in 946bd77 passed all existing tests because none checked mtimes).

#### deferred-subscribe-event-variants
**Source**: blit.proto:lines 96-101
**Specificity**: high

Subsequent C sub-slices fan more variants into the DaemonEvent oneof: TransferProgress, TransferComplete, TransferError, ModuleListChanged, DaemonHeartbeat. Add `transfer_id_filter` / `replay_recent` request fields. (TransferProgress/Complete/Error now present; ModuleListChanged + DaemonHeartbeat reserved 5,6 only.)

#### deferred-byte-counters-milestone-b
**Source**: blit.proto:ActiveTransfer/TransferRecord comments
**Specificity**: high

Byte-level progress fields in ActiveTransfer always zero in milestone B — milestone C feeds them from write-loop instrumentation. TransferRecord.bytes/files zero in milestone B.

#### deferred-throughput-bps-ewma
**Source**: blit.proto:TransferProgress comment (lines 942-959)
**Specificity**: medium

`throughput_bps` is instantaneous over the most recent tick; a smoothed EWMA is a follow-up slice once we have an operator pain signal to justify the extra state.

#### deferred-tarshardexecutor-deletion
**Source**: WHITEPAPER.md:§10
**Specificity**: low

"TarShardExecutor in crates/blit-daemon/src/service/push/data_plane.rs is now used only by the gRPC fallback. Worth deleting outright?"

### rejected

#### rejected-pullsyncheader
**Source**: blit.proto:lines 269-272
**Specificity**: high

Previous `PullSyncHeader` bool-soup shape was removed entirely because "we don't carry backward compatibility into the next release." Leading message is now TransferOperationSpec.

#### rejected-comparison-mode-ignore-existing
**Source**: blit.proto:ComparisonMode reserved 5 (lines 525-528)
**Specificity**: high

Reserved: previously COMPARISON_MODE_IGNORE_EXISTING, removed in favor of the orthogonal `ignore_existing` field on TransferOperationSpec. No future variant should reuse number 5.

#### rejected-use-chroot-config
**Source**: DAEMON_CONFIG.md:§Path containment
**Specificity**: high

Previous `use_chroot` and `root_use_chroot` config options were removed (they never enforced anything beyond lexical `safe_join` check, which doesn't follow symlinks). Configs containing those keys silently ignore them (TOML unknown-field tolerance).

#### rejected-blitauth-service
**Source**: blit.proto:lines 110-116; DAEMON_CONFIG.md:§Auth posture
**Specificity**: high

`BlitAuth` service stub removed 2026-05-13. If a real auth scheme is ever needed, design it from scratch rather than retain a misleading stub.

#### rejected-delegated-credential
**Source**: blit.proto:lines 654-657
**Specificity**: high

`bytes delegated_credential` field (forward-compat auth-passthrough hook for the removed BlitAuth service) removed 2026-05-13. Reserved so it can't be reused for unrelated semantics.

#### rejected-pull-rpc-deprecated
**Source**: blit.proto:line 10-11
**Specificity**: high

Pull RPC marked DEPRECATED: "Use PullSync for incremental/selective transfers."

#### rejected-ack-deprecated
**Source**: blit.proto:line 302
**Specificity**: medium

ServerPullMessage.ack=1 marked "(deprecated, use pull_sync_ack)".

#### rejected-client-walks-dest-purge
**Source**: blit.proto:lines 313-320
**Specificity**: high

Server-authoritative delete_list replaces the prior "client walks dest tree" purge inference, which mis-purged unchanged files and ignored filter scope.

#### rejected-uchroot-toml-tolerance
**Source**: DAEMON_CONFIG.md:§Path containment
**Specificity**: high

Configs containing use_chroot or root_use_chroot silently ignore them — TOML unknown-field tolerance — and the runtime behavior is the always-on containment.

### decision

#### decision-spec-version-bump
**Source**: blit.proto:lines 426-434
**Specificity**: high

spec_version history: 1 = original (pre-0.1.0); 2 = added `require_complete_scan` (R49-F2). Bumped so v1 daemons fail closed when receiving a v2 spec instead of silently ignoring the safety-critical field.

#### decision-default-recent-50
**Source**: blit.proto:lines 728-733
**Specificity**: high

GetStateRequest.recent_limit: 0 = use daemon default (50). Ignored if larger than what daemon is tracking.

#### decision-progress-default-10hz
**Source**: blit.proto:lines 933-942
**Specificity**: high

Default progress tick 10 Hz (DEFAULT_PROGRESS_TICK_MS in service/core.rs).

#### decision-mdns-txt-modules-truncated
**Source**: DAEMON_CONFIG.md:§mDNS Discovery
**Specificity**: high

mDNS TXT record includes `modules` — comma-separated list of exported module names, truncated to 180 chars.

#### decision-default-port-9031
**Source**: DAEMON_CONFIG.md:§Quick Start; §Configuration Reference
**Specificity**: high

Default daemon port: 9031. Default bind: 0.0.0.0.

#### decision-default-instance-name
**Source**: DAEMON_CONFIG.md:§mDNS Discovery
**Specificity**: high

mDNS instance name defaults to `blit@<hostname>`.

#### decision-error-handling-eyre
**Source**: ARCHITECTURE.md:§Error Handling
**Specificity**: medium

Blit uses `eyre` for error handling with rich context. Error types propagate through the stack with full context for debugging.

#### decision-cli-flag-override
**Source**: DAEMON_CONFIG.md:§Priority
**Specificity**: high

CLI flag value (highest priority) > config file value > built-in default.

#### decision-perf-cap-1mib
**Source**: DAEMON_CONFIG.md:§Client Configuration
**Specificity**: high

`perf_local.jsonl` transfer performance records have ~1 MiB cap.

#### decision-job-event-ring-cap
**Source**: blit.proto:lines 866-876
**Specificity**: medium

Daemon replays the per-job event ring for the targeted transfer up to JOB_EVENT_RING_CAP recent events before forwarding live broadcast events (when `replay_recent=true` AND `transfer_id_filter` non-empty).

#### decision-cli-rejects-ignore-existing-plus-force
**Source**: blit.proto:lines 477-480
**Specificity**: high

CLI rejects `--ignore-existing` combined with `--force` since those are semantically contradictory.

#### decision-cli-rejects-detach-non-delegated
**Source**: blit.proto:lines 622-642
**Specificity**: high

CLI rejects `--detach` for push/pull/pull_sync routes; daemon-side flag therefore unused on non-delegated kinds.

#### decision-mtime-error-swallowed
**Source**: WHITEPAPER.md:§7.3 (Watch for)
**Specificity**: medium

The `let _ = filetime::set_file_mtime(...)` swallows errors. "Deliberate (cross-fs / permission cases) but masks bugs."

#### decision-delegation-empty-allowlist-any-host
**Source**: DAEMON_CONFIG.md:§[delegation] Section; §Configuration block
**Specificity**: high

Empty `allowed_source_hosts` + master switch true means "any host." Only honor that posture on a fully trusted LAN.

#### decision-hostname-normalization
**Source**: DAEMON_CONFIG.md:§Allowlist matching #1
**Specificity**: high

Hostname normalization: comparison is case-insensitive after trimming a trailing dot and applying IDNA punycode. `Server-A.LAN.` and `server-a.lan` both match `server-a.lan`.

## Contradictions

### contradiction-tls-and-acl-non-goal-vs-planned
**Source A**: ARCHITECTURE.md:§Planned Enhancements lists "TLS certificate validation" and "Per-module access control lists" as planned.
**Source B**: DAEMON_CONFIG.md:§Trust Model explicitly says "TLS termination, mutual auth, and access-control fronting are explicitly out of scope for the daemon" and §Auth posture says "This is intentional and not on the roadmap."

ARCHITECTURE.md treats TLS validation and per-module ACLs as on-roadmap; DAEMON_CONFIG.md treats them as deliberately out-of-scope. The DAEMON_CONFIG.md position is the more recent / decision-bearing one (it cites the 2026-05-13 BlitAuth removal) — ARCHITECTURE.md's "Planned Enhancements" subsection is stale.

### contradiction-auth-tokens-vs-no-auth
**Source A**: ARCHITECTURE.md:§Security Considerations Current State says "Authentication: Token-based (placeholder in proto)."
**Source B**: DAEMON_CONFIG.md:§Auth posture says "There is no daemon authentication … The `BlitAuth` proto stub and the `delegated_credential` passthrough field were removed 2026-05-13."

ARCHITECTURE.md references the removed token-auth placeholder; DAEMON_CONFIG.md and proto comment confirm it was deleted. ARCHITECTURE.md text is stale.

### contradiction-grpc-tls-not-enforced
**Source A**: ARCHITECTURE.md:§Security Considerations Current State says "Transport: gRPC with optional TLS (not enforced)."
**Source B**: DAEMON_CONFIG.md:§Transport Security says "The daemon does not implement built-in TLS." (Secure via SSH tunnel / VPN / reverse proxy.)

ARCHITECTURE.md implies optional TLS exists in the daemon; DAEMON_CONFIG.md says no built-in TLS at all. Stale architecture phrasing.

### contradiction-pullsync-leading-message
**Source A**: WHITEPAPER.md:§6 narrates flow as "Client sends `PullSyncHeader` then a `LocalFile` per local entry, then `ManifestDone`."
**Source B**: blit.proto:lines 269-275 says "The leading message is now `TransferOperationSpec` … The previous `PullSyncHeader` bool-soup shape was removed entirely."

Whitepaper still references PullSyncHeader in its narrative; the proto has replaced it with TransferOperationSpec. Whitepaper text is stale.

### contradiction-getstate-counters-always-zero-or-not
**Source A**: blit.proto:lines 53-57 (GetState comment) and lines 753-757 (DaemonState.counters comment) both say counters read from atomics; when `--metrics` off the atomics never incremented so all fields are zero.
**Source B**: Same passages frame `Counters` as a present-but-zero snapshot. No within-cluster contradiction; flagged here only because the project memory note (feedback_getstate_counters_zero.md) treats present-but-zero as a known footgun. Within the cluster, the docs are internally consistent.

(No contradiction; included for awareness.)

### contradiction-rdma-phase-3.5-vs-future-directions
**Source A**: blit.proto:DataTransferNegotiation comment "reserved 5 to 10; // RDMA fields (QP numbers, GID, etc.) for Phase 3.5".
**Source B**: ARCHITECTURE.md:§Future Directions lists "RDMA Support: Reserved fields in protocol for RDMA data plane" as a future direction with no phase number.

Minor — proto names "Phase 3.5" but ARCHITECTURE.md just lists it as future without phase scoping. Not a substantive contradiction.

### contradiction-blit-scan-command-form
**Source A**: DAEMON_CONFIG.md:§mDNS Discovery says "discover it with `blit scan` or `blit scan`" — duplicated identical command, no alternative form. Likely an editing slip.

A single-doc minor anomaly; flagged as a smell.

## Coverage attestation

| File | Lines | Notes |
|---|---|---|
| docs/ARCHITECTURE.md | 502 | Read end-to-end in one Read call. |
| docs/WHITEPAPER.md | 750 | Read end-to-end in one Read call. |
| docs/DAEMON_CONFIG.md | 620 | Read end-to-end in one Read call. |
| proto/blit.proto | 994 | Read end-to-end in one Read call. |
**Total lines**: 2866
