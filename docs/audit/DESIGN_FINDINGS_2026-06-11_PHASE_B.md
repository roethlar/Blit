# Design-coherence findings — Phase B (verified)

**Status**: Active (Phase B deliverable of `docs/plan/DESIGN_COHERENCE_REVIEW.md`, D-2026-06-11-1)
**Created**: 2026-06-11
**Method**: 8 dimension reviewers seeded by `DESIGN_MAP_2026-06-11.md` but required to re-derive every mechanism from code; every finding then attacked by 2 adversarial verifiers (mechanism lens + impact lens) with a third-agent tiebreaker on splits. 164 agents, ~7.0M tokens, 1,977 tool calls. HEAD at run time: `7d53107`.
**Outcome**: 76 findings reported → **70 confirmed** (4 high, 40 medium, 26 low after verifier severity corrections), **6 refuted** by the verification layer. ~28 findings were downgraded in severity — the impact lens was the strictest judge. Already-filed items (design-1/2/3) and the queued slice-2 transport work were excluded from re-reporting by prompt and cross-referenced instead.

## How to read this

- **Effective severity** is the most conservative of reviewer + both verifiers.
  The four HIGHs are the items where even adversarial review could not soften
  the impact: the OOM-by-constant network buffer-pool formula
  (`constants-network-pool-ignores-memory`); the live pull path computing full
  tuning then discarding all of it — 1 stream, pool=4 — while push runs up to
  32 streams (`constants-pull-sync-discards-tuning`); the deprecated Pull RPC
  half-migration whose ~500 unreachable daemon lines are *also* the only
  multi-stream pull implementation (`deadcode-pull-rpc-half-migration`); and
  the blanket `#[cfg(unix)]` on remote transfer tests that contain nothing
  unix-specific — the direct mechanism behind untestable Windows parity
  (`tests-cfg-unix-gating-blocks-windows-transfer-coverage`). Notable mediums
  right behind them: the `log::` facade has no backend anywhere (every
  `log::warn`/`error` is discarded) and the workspace-root `tests/` directory
  is never compiled by cargo.
- The **refuted list** is kept deliberately: each entry records why the claim
  died so Phase C and future reviews do not re-find it. Two of the six kills
  were "mechanism textually true, but the conflicting path is unreachable" —
  the same lesson as the Phase A manifest-batch erratum.
- **Coverage notes** record what each dimension checked and found clean; clean
  areas are evidence too.
- Findings overlap across dimensions by design (e.g. the dead contradictory
  retry classifier appears under boundaries, duplication, errors, and deadcode
  with different framings). Phase C's job is to dedup these into single
  slices.

---
## Verified findings by effective severity

Effective severity = the most conservative of the reviewer rating and both
verifier corrections. Original rating shown when downgraded.

### HIGH (4)

#### constants-network-pool-ignores-memory — Network buffer-pool budget is a frozen formula pasted at three sites with zero awareness of available memory

**Principle**: RELIABLE | **Slice**: medium

**Claim**: The buffer pools backing every multi-stream transfer authorize multi-GiB allocations from a hardcoded streams*2+4 formula while the codebase's own memory-aware sizer (used only for local copies) and the pool's own doc comment prescribe checking available RAM.

**Mechanism**: push/client/mod.rs:125-127: pool_size = streams*2+4; buffer_size = chunk_bytes.max(64KiB); memory_budget = buffer_size*pool_size*2 — at the 16-stream/64 MiB tier this authorizes 4.5 GiB and double-buffered sends hold 2 buffers per stream = 2 GiB resident, with no sysinfo consultation. The identical formula is independently pasted in blit-daemon/src/service/pull.rs:686-689 and varied (pool_size literal 4) at pull_sync.rs:636-639. Meanwhile BufferSizer caps local-copy buffers at available_memory/10 (buffer.rs:84) and BufferPool's own usage example documents memory_budget = available_memory/4 (buffer.rs:153) — the riskier regime governs the larger allocations. On a small-RAM host pushing a large tree this is OOM-by-constant, contradicting 'works in every config'.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:125 — pool_size = streams*2+4; memory_budget = buffer_size*pool_size*2 — no available-memory input
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:686 — same formula duplicated verbatim on the daemon legacy-pull side
- /home/michael/dev/Blit/crates/blit-core/src/buffer.rs:84 — BufferSizer: memory_limit = available_memory/10 — the memory-aware path exists but only governs local copies
- /home/michael/dev/Blit/crates/blit-core/src/buffer.rs:153 — BufferPool doc example: 'let memory_budget = available_memory / 4' — the documented intent the call sites ignore

**Proposed fix**: Extract one pool-construction helper (in buffer.rs next to BufferPool) that takes streams+chunk_bytes and clamps the budget against sysinfo available memory (e.g. min(formula, available/4), shrinking buffer_size if needed); replace the three pasted formulas with it.

#### constants-pull-sync-discards-tuning — Live pull path computes full tuning then discards everything except chunk_bytes: 1 stream, pool=4, literal prefetch 8, no socket tuning

**Principle**: FAST | **Slice**: medium

**Claim**: The production pull direction (PullSync RPC, the one the CLI uses) is hardcoded to a single TCP stream with a 4-buffer pool and prefetch 8, while an identical push negotiates up to 16 streams with tuned buffers and prefetch up to 32.

**Mechanism**: pull_sync.rs stream_via_data_plane calls determine_remote_tuning(total_bytes) at line 550 but then: negotiates `stream_count = 1u32` (line 568, comment claims 'multi-stream support lives in pull.rs' — the deprecated Pull RPC); hardcodes `pool_size = 4` (line 637) instead of the streams*2+4 formula push uses; passes literal prefetch `8` to DataPlaneSession::from_stream (line 641) and execute_sink_pipeline (line 651) where tuning.prefetch_count would say 16/32; and tuning.tcp_buffer_size is never applied — the CLI pull client connects with a bare TcpStream::connect (blit-core/src/remote/pull.rs:1710), unlike the push client whose DataPlaneSession::connect applies nodelay/keepalive/buffer sizes (data_plane.rs:99-122). CLI pull routes through pull_sync (pull.rs:707), confirmed live. Push for the same workload: daemon offers desired_streams (up to 16, control.rs:217/233) and the client honors it (push/client/mod.rs:637-653). So big-tree throughput is direction-dependent by authoring-time constants, not hardware.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:568 — let stream_count = 1u32; // Single stream for the resume path (multi-stream support lives in pull.rs)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:637 — let pool_size = 4; — vs push's streams*2+4
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:651 — execute_sink_pipeline(source, vec![sink], planned.payloads, 8, None) — literal prefetch ignores tuning.prefetch_count (16/32)
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1710 — pull client data-plane connect is a bare TcpStream::connect — tuning.tcp_buffer_size / nodelay / keepalive never applied on the pull direction
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:637 — push client honors negotiated stream_count capped by tuning.max_streams, with tuning.prefetch_count and tuning.tcp_buffer_size (line 653)

**Proposed fix**: First slice: honor the tuning the daemon already computed — use tuning.prefetch_count, the shared pool formula, and apply nodelay/keepalive/tcp_buffer_size on the pull client's data-plane socket (mirror DataPlaneSession::connect). Multi-stream pull-sync negotiation is the follow-up slice (the wire field stream_count already exists and the client-side receive loop already supports N invocations).

#### deadcode-pull-rpc-half-migration — Deprecated Pull RPC: client method has zero callers, daemon serves ~500 unreachable TCP lines that are also the only multi-stream pull

**Principle**: FAST | **Slice**: large

**Claim**: RemotePullClient::pull has no callers in any crate or test, the daemon's legacy Pull TCP data plane is unreachable from in-repo code, and that dead plane contains the codebase's only multi-stream pull implementation while the production PullSync path hardcodes stream_count=1.

**Mechanism**: rg for `.pull(` workspace-wide finds only three gRPC-stub calls inside blit-core/src/remote/pull.rs itself: line 305 (inside the deprecated `pull` method at :251), line 491 (scan_remote_files) and line 539 (open_remote_file). The latter two hardcode force_grpc:true (:485, :530), so the daemon's Pull handler always takes the gRPC/non-streaming branches (daemon pull.rs:64 single-file, :85 force_grpc||metadata_only) and never reaches stream_pull_streaming (:208) or the TCP accepts (accept_pull_data_connection :625, accept_pull_data_connection_streaming :841, enumerate_to_channel :764, pull_stream_count :915). The only code that could send force_grpc=false is the deprecated client method, which nothing calls — pull.rs's own test doc at :1855 calls it 'the deprecated `pull` method', and the daemon comment at pull.rs:694-696 calls its server half 'this deprecated-but-exposed Pull RPC path'. Meanwhile the live PullSync handler negotiates stream_count=1 at both of its negotiation sites (pull_sync.rs:567-568 with the comment 'multi-stream support lives in pull.rs', and :707), even though the blit-core client side can receive multiple streams (receive_data_plane_streams_owned, pull.rs:1600-1646). Net: production pull runs one TCP stream; the up-to-16-stream ladder lives only in dead code. Wire-compat caveat: proto/blit.proto:11 still declares `rpc Pull`, so out-of-repo older clients could reach the TCP branches — retiring the RPC is an owner decision.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:251 — pub async fn pull — zero callers workspace-wide (rg '\.pull\(' over crates/ and tests/ matched only :305/:491/:539, all inside this file)
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:485 — scan_remote_files: force_grpc: true — 'Force gRPC to get headers in the control stream'; open_remote_file same at :530
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:694 — 'this deprecated-but-exposed Pull RPC path' — R47-F5 comment inside accept_pull_data_connection; legacy fns at :117/:208/:625/:764/:841/:915 in a 1038-line file
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:567 — 'Single stream for the resume path (multi-stream support lives in pull.rs)'; stream_count = 1 here and at :707 — the only two negotiation sites in the file
- /home/michael/dev/Blit/proto/blit.proto:11 — rpc Pull(PullRequest) returns (stream PullChunk) — still on the wire; retirement needs an owner decision

**Proposed fix**: Two-part: (a) OWNER DECISION — retire the wire-level Pull RPC (or keep it gRPC-only by deleting just the TCP-negotiation branches, which no in-repo client can trigger); (b) port the multi-stream data plane (pull_stream_count ladder + parallel accept) from dead pull.rs into pull_sync's negotiation so the production pull path stops being single-stream. Then delete RemotePullClient::pull and the daemon TCP branches.

#### tests-cfg-unix-gating-blocks-windows-transfer-coverage — Blanket #[cfg(unix)] on remote transfer tests that contain no unix-specific code, so the green Windows CI job never tests the transfer engine's parity-critical paths

**Principle**: RELIABLE | **Slice**: medium

**Claim**: remote_parity.rs, remote_resume.rs, remote_tcp_fallback.rs, remote_checksum_negotiation.rs, remote_remote.rs, remote_pull_mirror.rs, remote_push_single_file.rs, and remote_transfer_edges.rs are entirely #[cfg(unix)]-gated despite using no unix-only APIs, while sibling tests using the identical harness (remote_move.rs, remote_pull_subpath.rs, admin_verbs.rs, blit_utils.rs) run ungated on Windows CI — proving the daemon-spawn harness is Windows-capable and the gating is unnecessary for these files.

**Mechanism**: rg for PermissionsExt|std::os::unix|symlink across blit-cli/tests hits only f2_chroot_containment.rs, remote_regression.rs, remote_push_mirror_safety.rs, and local_move_semantics.rs — the other eight gated files are pure process/fs/CLI tests. The harness already handles Windows binary names (common/mod.rs:94-104, 'blit.exe'/'blit-daemon.exe'). CI runs `cargo test --workspace` on windows-latest (ci.yml test-windows job) and the manual parity runner scripts/windows/run-blit-tests.ps1 also just runs cargo test, so both inherit the same hole: on Windows, mirror purge, push mirror safety, resume (block-level + gRPC fallback variant), TCP/gRPC fallback negotiation, checksum negotiation, and remote-to-remote delegation are compiled out. AGENTS.md §5 says 'Windows parity matters', yet the validation suite structurally cannot observe a Windows regression in any of those paths — a green Windows CI job is a false parity signal.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_parity.rs:8 — #[cfg(unix)] on test_push_tcp_negotiation; body (lines 10-43) is Command + fs only, no unix APIs
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:94 — harness already selects blit.exe/blit-daemon.exe on Windows — Windows support was built, then gated off
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_move.rs:9 — ungated daemon-spawning transfer test (no cfg(unix) anywhere in file) — runs and passes on Windows CI, proving the harness works there
- /home/michael/dev/Blit/.github/workflows/ci.yml:47 — test-windows job runs cargo test --workspace; with the gating it exercises zero mirror/resume/fallback/parity remote tests
- /home/michael/dev/Blit/scripts/windows/run-blit-tests.ps1:48 — the manual Windows parity runner is also just cargo test steps, inheriting the same compiled-out coverage

**Proposed fix**: Remove the file-blanket #[cfg(unix)] from the eight files with no unix APIs; keep gating only on individual assertions/tests that genuinely need chmod/symlink semantics (as orchestrator.rs already does per-test). Run the Windows ps1 runner once to triage any real platform failures into their own findings instead of leaving them invisible.

### MEDIUM (40)

#### async-daemon-handlers-blind-to-disconnect-in-compute-phases — Push/pull/pull_sync daemon spawn closures never race tx.closed() or the cancel token, so compute phases (full-module hashing, purge) run to completion for dead clients — and the code comment claims otherwise

**Principle**: RELIABLE | **Slice**: medium

**Claim**: Only delegated_pull's spawn closure races client-hangup and CancelJob; the push, pull, and pull_sync closures just await their handler, so a client that disconnects during a send-free compute phase leaves the daemon doing unbounded unobservable work that CancelJob explicitly refuses to touch, while an active_jobs comment asserts a tx.closed() drop mechanism that does not exist for these kinds.

**Mechanism**: core.rs:499 (push), :579 (pull), :631 (pull_sync) spawn `handler(...).await` with no select; only delegated_pull (core.rs:741-783) uses resolve_delegated_pull_outcome racing tx.closed() and cancel_token.cancelled(). A disconnect is therefore observed only when the handler next awaits a tx.send — but pull_sync's Phase 3 (pull_sync.rs:111) runs collect_pull_entries_with_checksums first: enumerate + Blake3-hash the entire requested tree inside spawn_blocking+rayon (pull.rs:448-499), which is not abortable and performs zero sends until done; push's mirror purge (control.rs:347) is a second send-free spawn_blocking phase. HTTP/2 keepalive reaping the dead connection (main.rs:137-142) drops the response stream but nothing polls it, so the blocking work continues regardless; ActiveJobKind::supports_cancellation (active_jobs.rs:163) returns false for all three kinds, with the justifying comment at :155-158 — 'a client-side cancel already drops the handler future via tx.closed()' — describing a race that exists only in the delegated_pull spawn site. Repeating connect→checksum-spec→disconnect multiplies concurrent full-disk hash jobs that nothing can cancel.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:579 — pull spawn closure: bare `handler.await`, no tx.closed()/token race (same shape at :499 push, :631 pull_sync)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:768 — delegated_pull's three-way race — the mechanism the other three lack
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:111 — Phase 3 collect-everything-with-checksums before any further send — the unobservable window
- /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:156 — comment claims 'client-side cancel already drops the handler future via tx.closed()' — false for push/pull/pull_sync; supports_cancellation=false at :163

**Proposed fix**: Wrap the three handler awaits in the same select-against-tx.closed()+token helper delegated_pull uses (hoist resolve_delegated_pull_outcome), and fix the active_jobs comment; follow-up (separate slice) to make the checksum collect phase abortable, e.g. check a cancellation flag between rayon batches.

#### async-daemon-push-manifest-blocking-stat-per-entry — Daemon push control loop does two blocking filesystem syscalls (stat + canonicalize ancestor-walk) per manifest entry on the async runtime

**Principle**: FAST | **Slice**: medium

**Claim**: Every FileManifest message in handle_push_stream triggers synchronous std::fs::metadata plus a std::fs::canonicalize retry-loop inline on the tokio runtime thread, so a large push runs millions of blocking syscalls on an executor worker, stalling every other task scheduled there.

**Mechanism**: handle_push_stream (async, spawned at core.rs:499) calls file_requires_upload per entry (control.rs:148), which first runs resolve_contained_path (control.rs:481 → util.rs:107-111) — whose verify_contained does a loop of `std::fs::canonicalize(&probe)` popping components until an existing ancestor is found (path_safety.rs:228-250; for a not-yet-created destination file that is ≥2 canonicalize syscalls) — then `fs::metadata(&full_path)` (control.rs:482, sync std::fs via the `use std::fs` at :17). For a 1M-file push that is ~3M+ blocking syscalls executed on a runtime worker; on NFS/CIFS or cold caches each can take milliseconds, freezing co-scheduled tasks (other RPC handlers, the 10 Hz progress ticker, Subscribe forwarders) for the duration. The daemon's other per-entry hot paths got this right: purge enumeration (admin.rs:68), delete (admin.rs:51), delegated-pull manifest (delegated_pull.rs:549) and pull enumeration (pull.rs:448) are all spawn_blocking.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:482 — sync fs::metadata per manifest entry inside the async control loop
- /home/michael/dev/Blit/crates/blit-core/src/path_safety.rs:231 — verify_contained: std::fs::canonicalize ancestor-walk loop — blocking, called per entry via resolve_contained_path (util.rs:109)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:499 — handle_push_stream runs as a plain tokio::spawn task — all this blocks a shared runtime worker

**Proposed fix**: Batch the requires-upload checks: accumulate manifest entries and stat/contain them in chunked spawn_blocking calls (the need-list batcher already batches the reply side), or cache the canonicalized destination root once and do lexical containment per entry, dropping the per-file canonicalize.

#### async-daemon-push-stream-workers-detach-on-first-error (reviewer: high) — Daemon push data-plane per-stream workers (up to 16) detach when any sibling stream fails — a fourth detach site beyond design-2's three

**Principle**: RELIABLE | **Slice**: small

**Claim**: accept_data_connection_stream spawns up to 16 per-stream receive workers as bare tokio::spawn handles and joins them with `?`-propagation, so the first worker error (or an accept timeout) drops the remaining JoinHandles and detaches live workers that keep writing files into the module after the RPC has already failed.

**Mechanism**: Workers are spawned at data_plane.rs:130 into a plain Vec<JoinHandle>; the join loop at :143-146 does `handle.await.map_err(...)??` — the first Err returns from the function, dropping every remaining handle (detach, not abort). Likewise the accept-timeout arm at :103-110 returns Err after some workers were already spawned. Each detached worker continues running handle_data_plane_stream → receive_push_data_plane → FsTransferSink, writing client bytes to disk with no owner, unreachable by CancelJob (push reports supports_cancellation=false, active_jobs.rs:163-164), until its socket EOFs or the 30s StallGuard fires — and if the client's own detached pipeline (sibling finding) keeps sending, indefinitely. The failed RPC meanwhile drives the client to retry, producing a second writer set against the same destination paths. design-2 (.review/findings/design-2-orphaned-daemon-data-planes.md) names exactly three sites (service/pull.rs:180/:297, push/control.rs:57); this Vec of per-stream workers is a fourth, one level deeper, and needs the same AbortOnDrop/JoinSet-abort treatment or design-2's fix will still leak the inner layer.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:130 — handles.push(tokio::spawn(handle_data_plane_stream(...))) — bare handles in a Vec
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:144 — join loop `handle.await...??` — first error returns, dropping (detaching) remaining live workers
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:103 — accept-timeout early return after idx workers already spawned — same detach
- /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:163 — supports_cancellation() is DelegatedPull-only — orphaned push workers unreachable by CancelJob

**Proposed fix**: Replace the Vec<JoinHandle> with a JoinSet (abort_all on first error / on drop) or wrap each handle in the hoisted AbortOnDrop; extend design-2's regression test to kill one of N streams and assert the siblings terminate.

#### async-daemon-single-file-checksum-blocks-runtime — Single-file pull/pull_sync in checksum mode Blake3-hashes the entire file synchronously on the async runtime thread

**Principle**: FAST | **Slice**: small

**Claim**: collect_pull_entries_with_checksums offloads the directory branch to spawn_blocking+rayon but runs the single-file branch — including a full synchronous read-and-hash of the file — inline in the async fn, pinning a tokio worker for the entire hash of an arbitrarily large file.

**Mechanism**: The `root.is_file()` branch (pull.rs:397-439) calls build_file_header(..., compute_checksums) directly (pull.rs:430); with checksums enabled build_file_header opens the file with std::fs::File::open and loops sync `reader.read(&mut [0u8; 256*1024])` + hasher.update until EOF (pull.rs:513-534) — for a 100 GB file that is the better part of a minute of uninterrupted blocking I/O+CPU on a runtime worker, during which every other future on that worker (other handlers, tickers, event fan-out) is frozen. The directory branch immediately below wraps the identical work in tokio::task::spawn_blocking with rayon par_iter (pull.rs:448-499), so the fix pattern is already in the same function. Reachable from legacy pull (pull.rs:373) and from pull_sync Phase 3 (pull_sync.rs:111) whenever the client requests ComparisonMode::Checksum and the server has checksums enabled. The sync std::fs::metadata pair at pull.rs:418-419 rides along.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:430 — single-file branch calls build_file_header inline in the async fn (no spawn_blocking)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:524 — sync 256 KiB read loop + blake3 hashing of the whole file on the runtime thread
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:448 — directory branch correctly uses spawn_blocking + rayon — pattern exists 18 lines below the bug
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:111 — pull_sync Phase 3 reaches the same single-file branch with compute_checksums

**Proposed fix**: Move the single-file build_file_header call into the same spawn_blocking the directory branch uses (or spawn_blocking just the checksum case); deterministic test: paused-runtime test asserting the async fn yields while hashing a multi-chunk file.

#### async-push-client-pipeline-detach-on-drop (reviewer: high) — Push client data-plane pipeline and response forwarder detach (not abort) when push() errors or is dropped — R32-F2 fix never propagated from pull

**Principle**: RELIABLE | **Slice**: small

**Claim**: The push client holds bare JoinHandles for its spawned data-plane pipeline and response-forwarder tasks, so any early-error return from RemotePushClient::push (or drop of the push future) leaves the pipeline streaming payloads to the daemon with no owner, while blit-app's retry loop can start a second concurrent attempt.

**Mechanism**: MultiStreamSender stores `pipeline_handle: Option<JoinHandle<Result<SinkOutcome>>>` (mod.rs:104) for the task spawned at mod.rs:156 that runs execute_sink_pipeline_streaming over N TCP sinks; spawn_response_task (helpers.rs:243) is likewise a bare tokio::spawn. The push() event loop has many `?`/return exits while `data_plane_sender` is still Some — e.g. a control-stream error returns at mod.rs:707, and every send_payload/queue `?` — at which point MultiStreamSender is dropped, dropping the JoinHandle, which detaches the task: the pipeline keeps preparing and writing all already-queued payloads (tar shards up to MiBs each, prefetch-deep) to the still-open TCP sockets. The workspace's own fix for this exact class, AbortOnDrop (pull.rs:31, R32-F2), is pub(crate) and used throughout pull.rs (:315, :726-732, :765, :1640-1657) but nowhere in push. Consequence: after a retryable control-plane error, blit_app::transfers::retry::run_with_retries (retry.rs:55-77) launches a fresh attempt while the orphaned pipeline of the failed attempt may still be writing the same destination files via the daemon's (also detached, design-2) data-plane workers — two unsynchronized writers per file plus double progress accounting.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:104 — bare `pipeline_handle: Option<JoinHandle<Result<SinkOutcome>>>` — drop detaches, does not abort
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:156 — tokio::spawn of execute_sink_pipeline_streaming owning all N TCP sinks
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:707 — `Some(Err(err)) => return Err(err)` exits push() while data_plane_sender (and its handle) is still live → detach
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/helpers.rs:243 — spawn_response_task: bare tokio::spawn returning bare JoinHandle
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:31 — AbortOnDrop exists (R32-F2) but is pub(crate) to pull.rs's crate and unused by the push client

**Proposed fix**: Hoist AbortOnDrop to blit-core::remote::transfer (same hoist design-2 needs) and wrap pipeline_handle and response_task in it; regression test: drop a push() future mid-transfer and assert the pipeline task stops (no further sink writes).

#### async-push-upload-channel-fallback-wedge (reviewer: high) — Dead push upload channel wedges the daemon control loop silently in gRPC-fallback pushes >262,144 changed files; in TCP mode it is pure drain-and-discard plumbing

**Principle**: RELIABLE | **Slice**: small

**Claim**: handle_push_stream queues every needs-upload FileHeader into a 262,144-capacity channel that no code ever consumes in gRPC-fallback mode, so the daemon's `upload_tx.send().await` blocks forever once the channel fills — a permanent, message-free hang of both daemon handler and client — while in TCP mode the receiver exists only to be locked behind an Arc<AsyncMutex> and drained into the void.

**Mechanism**: control.rs:55 creates `mpsc::channel::<FileHeader>(FILE_UPLOAD_CHANNEL_CAPACITY)` (= 16*1024*16 = 262,144, :31); every manifest entry passing file_requires_upload is sent at :157 *before* the transfer-mode branch. In fallback mode (client --force-grpc, daemon force_grpc_data, or the automatic bind-failure fallback at :181-199) `upload_rx_opt.take()` is never executed (:214/:287 are TCP-only), so the receiver stays alive-but-unread in the local until the function returns: send #262,145 awaits forever. The daemon stops reading the request stream, gRPC flow control backpressures the client's manifest sends, and both sides wedge with no timeout in scope — HTTP/2 keepalive (main.rs:137-142) sees a healthy connection, StallGuard covers TCP data planes only. In TCP mode the consumer (data_plane.rs:89, :164) wraps the receiver in Arc<AsyncMutex>, each of N workers spawns a task (:200-206) whose only body is `while guard.recv().await.is_some() {}` (N-1 of them blocked on the mutex), and the companion `cache` is explicitly voided (:207) — the comment at control.rs:150-156 admits 'Only the gRPC fallback path uses this queue', which is false: the fallback path is the one that never reads it.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:31 — FILE_UPLOAD_CHANNEL_CAPACITY = FILE_LIST_BATCH_MAX_ENTRIES * 16 = 262,144
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:157 — upload_tx.send(file).await inside the manifest loop, before any mode branch — the wedge point
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:181 — bind-failure path flips to fallback where upload_rx_opt is never taken (takes are at :214 and :287, TCP-only)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:200 — per-worker drain task: lock Arc<AsyncMutex<Receiver>> and discard everything; cache voided at :207

**Proposed fix**: Delete the upload_tx/upload_rx + cache plumbing entirely (headers travel on the wire post-Phase-5; nothing consumes them); if any consumer is ever reintroduced it should own the Receiver directly, not share it through a mutex. Regression: force-grpc push with a synthetic >capacity manifest completes instead of hanging.

#### boundaries-mirror-apply-logic-duplicated-daemon-vs-app (reviewer: high) — Mirror manifest enumeration and delete-list application duplicated between blit-app and blit-daemon, with one containment divergence already on record

**Principle**: RELIABLE | **Slice**: medium

**Claim**: enumerate_local_manifest and the mirror delete-list applier exist as independently maintained twins in blit-app and blit-daemon because the daemon cannot depend on blit-app; the pair has already drifted into a security-relevant containment gap once (R58-F3) and currently diverges on performance.

**Mechanism**: blit-daemon depends only on blit-core (Cargo.toml:10), so the delegated-pull path re-implements blit-app logic: delegated_pull.rs:531-537 admits its enumerate_local_manifest is a 'Mirror of blit_app::transfers::remote::enumerate_local_manifest' but sequential ('Parallelize later'), while the blit-app twin (remote.rs:80-99) computes checksums rayon-parallel — identical workloads get different hardware efficiency depending on which side of a delegation runs them (FAST). Worse precedent: apply_delete_list's own doc (delegated_pull.rs:431-445) records that the blit-app twin upgraded to safe_join_contained in R46-F3 while the daemon copy stayed on bare safe_join, leaving a symlink-escape deletion hole until R58-F3 re-synced them — direct in-repo proof this duplication produces security drift, and nothing structural prevents the next fix landing on one side only. The destructive-path logic belongs in the crate both binaries already share.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/delegated_pull.rs:434 — R58-F3 narrative: app twin got safe_join_contained in R46-F3, 'this daemon-side delegated path was still on bare safe_join' — peer-controlled entry could remove an outside file
- /home/michael/dev/Blit/crates/blit-daemon/src/service/delegated_pull.rs:531 — 'Mirror of blit_app::transfers::remote::enumerate_local_manifest ... Sequential walk; Parallelize later'
- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote.rs:80 — the parallel twin: checksums 'computed in parallel via rayon'; delete_listed_paths at :203
- /home/michael/dev/Blit/crates/blit-daemon/Cargo.toml:10 — blit-daemon depends only on blit-core — dependency direction forbids reusing the blit-app copies

**Proposed fix**: Move enumerate_local_manifest (the rayon version) and the delete-list applier into blit-core (next to path_safety, whose helpers they already use) and have both blit-app and blit-daemon call them; keep one containment test suite at the shared site.

#### boundaries-presenter-formatting-fragmented — Byte/throughput formatting fragmented across crates despite blit-app::display existing as the designated shared presenter helper

**Principle**: SIMPLE | **Slice**: small

**Claim**: blit-app::display::format_bytes was created to be 'shared by every presenter', yet blit-cli's jobs verb rolls a decimal-unit format_bps and blit-tui carries multiple private divergent format_bytes copies, so one product prints KiB in `blit ls` and KB in `blit jobs watch`.

**Mechanism**: display.rs:1-2 states the module is 'shared by every presenter (CLI text output, TUI panes, JSON-embedded reason strings)' and format_bytes (display.rs:14-26) uses binary units B..TiB. blit-cli already imports it for ls/df/local transfers, but jobs.rs:471-481 defines its own format_bps with decimal thresholds (1_000_000_000 -> GB) — two unit systems in one binary. blit-tui, which depends on blit-app, re-implements format_bytes privately per screen; the f2.rs copy (f2.rs:555-574) even documents 'd-25: aligned with F4's format_bytes' — copies being synced against each other by review finding instead of importing the library function built for them. Boundary-level duplication only (TUI internals out of scope per Phase 6 rule).

**Evidence**:
- /home/michael/dev/Blit/crates/blit-app/src/display.rs:14 — canonical pub format_bytes, binary units, module doc says shared by every presenter including TUI panes
- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:471 — private format_bps with decimal KB/MB/GB thresholds — different unit system in the same binary
- /home/michael/dev/Blit/crates/blit-tui/src/screens/f2.rs:555 — 'd-25: aligned with F4's format_bytes' — private copy synced against another private copy instead of importing blit_app::display

**Proposed fix**: Add a format_bps (or rate wrapper) to blit_app::display in the binary-unit convention, switch jobs.rs to it, and replace the TUI screen-local copies with imports — one decision: binary units everywhere.

#### boundaries-progress-event-contract-lives-in-consumers (reviewer: high) — ProgressEvent has no owner-defined semantics; the contract lives in consumer-crate folding comments that encode blit-core producer internals

**Principle**: RELIABLE | **Slice**: medium

**Claim**: blit-core's ProgressEvent enum carries no semantic contract, and its producers assign three incompatible meanings to the same variants, so each consumer must hard-code per-direction knowledge of blit-core internals — the structural cause of the already-filed CLI double-count.

**Mechanism**: The enum (progress.rs:6-10) is three undocumented variants. blit-tui's progress_accum.rs is where the actual contract is written down: accumulate_pull_progress's doc (lines 12-21) explains that the TCP path emits the same bytes on BOTH Payload and FileComplete while the gRPC path puts bytes on Payload with FileComplete{bytes:0} — citing pipeline.rs and pull.rs:finalize_active_file by name; accumulate_push_progress (lines 41-50) documents that push reports bytes only on FileComplete and emits no Payload; accumulate_delegated_progress documents a third meaning (Payload carries both deltas, no FileComplete). Three folding functions in a downstream crate encode producer-internal behavior of four blit-core/blit-app emission sites; the CLI, which has only one folding rule, got pulls wrong (design-1, filed — cross-reference, not re-reported). Any new consumer or producer change re-rolls this dice because no type, doc, or test in blit-core states which variant carries bytes.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/progress.rs:6 — ProgressEvent { ManifestBatch, Payload{files,bytes}, FileComplete{path,bytes} } — zero semantic documentation
- /home/michael/dev/Blit/crates/blit-tui/src/progress_accum.rs:12 — consumer doc: 'TCP data-plane path emits BOTH Payload{bytes:N} and FileComplete{bytes:N} for the same file ... direct-gRPC emits FileComplete{bytes:0}' — producer internals documented downstream
- /home/michael/dev/Blit/crates/blit-tui/src/progress_accum.rs:43 — push semantics inverted: 'bytes AND files both come from FileComplete ... push emits no Payload' — third rule for delegated at :69-78

**Proposed fix**: Define the contract in blit-core: normalize all producers to one semantic (bytes ride Payload only; FileComplete carries bytes:0 and counts files), document it on the enum, and add producer-side tests; then collapse the three TUI folding rules and the CLI rule to one shared accumulator (also closes the design-1 class).

#### boundaries-pull-direction-bypasses-socket-policy (reviewer: high) — Entire pull direction bypasses the data-plane socket policy that DataPlaneSession::connect and the push accept path own

**Principle**: FAST | **Slice**: medium

**Claim**: TCP_NODELAY, SO_KEEPALIVE, and the auto-tuner's tcp_buffer_size are applied only on push-direction sockets; the pull/pull-sync data plane on both ends runs on raw kernel defaults, silently discarding the tuner's output.

**Mechanism**: DataPlaneSession::connect (data_plane.rs:92-124) is the socket-policy owner: it sets nodelay (hard error), keepalive (logged best-effort), and applies tuning.tcp_buffer_size to send/recv buffers. The pull client instead calls bare TcpStream::connect in receive_data_plane_stream_inner (pull.rs:1709-1715) — none of those options, ever. Server side mirrors the asymmetry: the push accept path wraps the accepted socket in socket2 and sets nodelay+keepalive (push/data_plane.rs:112-124), while daemon pull.rs and pull_sync.rs accept raw sockets (rg for nodelay/keepalive/socket2 in both files: zero hits). determine_remote_tuning mints tcp_buffer_size 4-8 MiB for large transfers (tuning.rs:31-36), but the only consumer is the push client socket — for every pull, the tuned value is computed and thrown away, and Nagle stays enabled on the path that carries the bytes. Distinct from design-3 (the missing connect timeout at the same call site, already filed): this is the socket-option/ownership half.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1709 — bare TcpStream::connect — no nodelay/keepalive/buffer sizing before handing to the pipeline
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:98 — policy owner: set_tcp_nodelay hard-error, set_keepalive logged, tcp_buffer_size applied at :110-121 — push connect only
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:112 — 'Enable nodelay + keepalive to prevent idle stream timeouts' — the ONLY daemon accept path that sets socket options
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:702 — pull accept hands the raw socket to sinks; rg confirms zero socket2/nodelay/keepalive hits in pull.rs and pull_sync.rs
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:31 — tcp_buffer_size = 8 MiB / 4 MiB minted here; only the push client socket ever applies it

**Proposed fix**: Extract the socket-setup block from DataPlaneSession::connect into one shared configure_data_socket(stream, tcp_buffer_size) helper in blit-core, call it from the pull client connect and from all three daemon accept paths (pull.rs, pull_sync.rs x2), threading tuning.tcp_buffer_size to the accept side.

#### boundaries-push-filter-bypasses-validated-chokepoint (reviewer: high) — Daemon push handler hand-rolls the FilterSpec conversion, skipping blit-core's glob-validating chokepoint

**Principle**: RELIABLE | **Slice**: small

**Claim**: The same wire FilterSpec is converted through blit-core's validated filter_from_spec on the pull-sync path but through an unvalidated hand-rolled copy in the push handler, where it feeds the mirror purge filter.

**Mechanism**: operation_spec.rs:186-208 (filter_from_spec) validates every include/exclude glob individually so 'a malformed pattern from a hostile or buggy peer is a hard error here rather than silently dropped by FileFilter::build_globset later' (R5-F4); pull_sync reaches it via NormalizedTransferOperation::from_spec (pull_sync.rs:58). The push control handler instead maps the wire filter field-by-field itself (push/control.rs:91-105) with no glob validation, and binds the result to purge_filter — the filter that scopes mirror deletion. Per the chokepoint's own rationale, an invalid exclude glob arriving on push is silently dropped downstream, so a mirror purge can delete files the sender's filter was meant to protect, while the identical spec on pull-sync fails plainly. The chokepoint is a private fn (no pub on filter_from_spec), so the daemon could not reuse it without an API change — a pub(crate)-shaped primitive that should be shared.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:91 — hand-rolled FileFilter construction from wire_filter; no glob validation; result assigned to purge_filter at :104
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/operation_spec.rs:186 — private fn filter_from_spec validates each glob: 'hard error here rather than silently dropped by build_globset later (R5-F4)'
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:58 — pull-sync routes through NormalizedTransferOperation::from_spec — the validated path the push handler bypasses

**Proposed fix**: Make filter_from_spec pub in operation_spec.rs and replace the push handler's inline mapping with a call to it, converting the resulting error into Status::invalid_argument as pull_sync already does.

#### boundaries-retry-policy-split-dead-classifier — Retry classification policy split across crates, with a dead contradictory classifier publicly exported from blit-core

**Principle**: maintainability | **Slice**: small

**Claim**: The live retry classifier lives in blit-app while the errors it must classify are minted in blit-core (which can only reference it by comment), and blit-core simultaneously exports a zero-consumer classifier that contradicts the live one on three error kinds.

**Mechanism**: blit_core::errors::categorize_io_error (errors.rs:90-117) marks ConnectionRefused/UnexpectedEof/NotConnected as Fatal (lines 107-113) while the live blit_app::transfers::retry::is_retryable_io_kind (retry.rs:35-46) marks the same three kinds Retryable. errors.rs has zero consumers (rg for categorize_io_error/ErrorCategory/crate::errors across crates/ and tests/ returns only lib.rs:9 `pub mod errors`), yet it is the discoverable, doc-commented module a future contributor would wire up — silently flipping retry semantics. Meanwhile the TODO(audit-h3c-2) blocks at pull.rs:322-329 and 780-788 name `blit_app::transfers::retry::is_retryable` as the contract their future fix must satisfy — cross-crate coupling enforced only by comment, untestable from blit-core because the dependency direction forbids the import. The queued slice-2 chain-preservation work depends on exactly this contract holding.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/errors.rs:107 — WriteZero|UnexpectedEof|...|NotConnected|ConnectionRefused => ErrorCategory::Fatal — 'could go either way - default to fatal'
- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:38 — is_retryable_io_kind lists TimedOut|ConnectionReset|ConnectionAborted|ConnectionRefused|BrokenPipe|UnexpectedEof|NotConnected as retryable — direct contradiction on three kinds
- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — pub mod errors — the dead taxonomy is the publicly exported one
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:325 — TODO names blit_app::transfers::retry::is_retryable across the crate boundary by comment; same at :786

**Proposed fix**: Delete errors.rs's dead taxonomy and move is_retryable/is_retryable_io_kind into blit-core (the crate that mints and converts the errors), re-exporting from blit-app for existing callers; add the classifier-contract test next to the conversion sites so the queued slice-2 chain-preservation work can assert it in-crate.

#### boundaries-stream-count-policy-minted-three-times — Stream-count policy minted in three places: two daemon ladders re-derive what blit-core tuning owns, then clamp against it

**Principle**: SIMPLE | **Slice**: small

**Claim**: The 'how many parallel streams' decision exists as three independently authored byte-threshold ladders — blit-core determine_remote_tuning, daemon pull_stream_count, and daemon desired_streams — that disagree with each other and are reconciled by ad-hoc clamping at the seams.

**Mechanism**: determine_remote_tuning (tuning.rs:14-26) maps the same byte thresholds (32 GiB/8 GiB/2 GiB/512 MiB/128 MiB) to (initial,max) pairs up to (24,32). Daemon pull_stream_count (pull.rs:915-933) re-derives a different ladder over the same thresholds capping at 16, then clamps with `streams.min(tuning_max.max(1))` at :931 — two ladders fighting over one number. Daemon push desired_streams (push/control.rs:499-520) is a third ladder, again capped at 16 but additionally keyed on file_count (200_000/80_000/...), which the other two ignore; the client then clamps the negotiated count by tuning.max_streams (push/client/mod.rs:637-640). Net effect: the effective stream count for a given workload depends on direction and on which ladder ran first, and a tuning change in blit-core does not propagate to either daemon table. Policy minted in the wrong crate, twice.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:915 — pull_stream_count: private 7-tier ladder capping at 16; clamps against core's tuning_max at :931
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:499 — desired_streams: third ladder, byte OR file_count keyed, caps at 16 — file_count keys exist nowhere else
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:14 — determine_remote_tuning: the nominal owner's ladder goes to (24,32) — disagrees with both daemon tables
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:637 — client-side reconciliation: neg.stream_count.max(1).min(tuning.max_streams) — fourth site touching the same decision

**Proposed fix**: Move the stream-count decision into determine_remote_tuning (optionally taking file_count as an input so push's file-count signal survives) and have both daemon sites call it, deleting the two private ladders; keep only the negotiation clamp.

#### boundaries-wire-path-metadata-helpers-duplicated — Wire-form path and metadata helpers duplicated across blit-core, blit-app, and blit-daemon despite the path_posix mandate

**Principle**: maintainability | **Slice**: small

**Claim**: path_posix.rs declares that every Path-to-wire conversion must route through relative_path_to_posix, yet a byte-identical normalize_for_request exists in two crates plus inline join('/') copies, and the FileHeader metadata helpers (permissions_mode, mtime-seconds) are re-implemented per crate with divergent signatures.

**Mechanism**: path_posix.rs:4-6: 'Every place blit converts a local Path ... must route through relative_path_to_posix'. Bypasses: normalize_for_request in blit-core (pull.rs:1840-1849) and a byte-identical private twin in blit-app (transfers/remote.rs:638-647), both doing iter().join('/') with their own empty-path convention ('.') that differs from the canonical helper's (''); plus two more inline rel.iter()...join('/') conversions in blit-app remote.rs:108-112 and :141-145. The metadata family is likewise duplicated: blit-core helpers.rs:59-83 (unix_seconds -> i64, defaults 0 on error; permissions_mode) vs blit-daemon util.rs:127-151 (metadata_mtime_seconds -> Option<i64>; identical permissions_mode body) — same wire fields, divergent error behavior, both private/pub(crate) so neither can be reused. Because the duplicates feed FileHeader construction on opposite ends of one protocol, drift here changes wire semantics silently — the exact failure mode path_posix.rs documents having already happened with replace('\\', "/").

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/path_posix.rs:6 — mandate: every conversion must route through relative_path_to_posix
- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1840 — normalize_for_request: iter().join("/"), empty -> "." — bypasses the canonical helper
- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote.rs:638 — byte-identical normalize_for_request twin; inline join copies at :108-112 and :141-145
- /home/michael/dev/Blit/crates/blit-daemon/src/service/util.rs:127 — metadata_mtime_seconds -> Option<i64> + permissions_mode — duplicates blit-core helpers.rs:59-83 (i64, default 0) with divergent error shape

**Proposed fix**: Add a pub normalize_for_request (request-path convention) and pub permissions_mode/mtime_seconds beside relative_path_to_posix in blit-core, delete the per-crate copies, and decide once whether metadata errors yield 0 or None.

#### constants-dead-warmup-adaptive-path (reviewer: high) — The advertised adaptive tuning is dead code; every remote tuning decision is a frozen byte-count ladder

**Principle**: SIMPLE | **Slice**: small

**Claim**: auto_tune's bandwidth-adaptive branches are unreachable in production, so the 'adapt at runtime' promise is implemented as static authoring-time tables keyed only on total_bytes.

**Mechanism**: determine_remote_tuning (tuning.rs:13) is the only production caller of determine_tuning and always passes warmup_result=None, so every `if let Some(gbps)` branch in auto_tune/mod.rs:45-67 is dead; tuning.rs:27-28 then overwrites the initial_streams/max_streams that determine_tuning returned, leaving only chunk_bytes flowing through — itself a frozen 16/32/64 MiB ladder keyed on total_bytes (tuning.rs:5-11). analyze_warmup_result (auto_tune/mod.rs:26-32), a third chunk heuristic keyed on Gbps, has zero callers anywhere (rg over all crates: only its own tests). Result: a 1 GbE link and a 100 GbE link transferring the same tree get identical chunk/stream/buffer choices, and the publicly exported warmup API is a trap inviting the next contributor to believe adaptation exists.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:13 — let mut tuning = determine_tuning(default_chunk_bytes, None); — warmup parameter hardwired to None at the only production call site
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:27 — tuning.initial_streams / max_streams immediately overwritten, discarding determine_tuning's stream logic
- /home/michael/dev/Blit/crates/blit-core/src/auto_tune/mod.rs:26 — analyze_warmup_result — zero callers outside its own module/tests
- /home/michael/dev/Blit/crates/blit-core/src/auto_tune/mod.rs:45 — if let Some(gbps) = warmup_gbps — all bandwidth-adaptive branches dead given the always-None caller

**Proposed fix**: Delete analyze_warmup_result and the warmup_result parameter/branches of determine_tuning, collapsing determine_remote_tuning into one honest size-keyed table (or, the larger follow-up, wire a real warmup probe). Deletion is the immediate slice; it removes the false-advertising surface and the contradictory max_streams=8 inside determine_tuning.

#### constants-three-disagreeing-stream-ladders — Three frozen stream-count ladders for the same decision; client's 24/32 tier is unreachable dead headroom

**Principle**: SIMPLE | **Slice**: medium

**Claim**: Stream parallelism is decided by three independently-authored byte-tier ladders (client tuning, daemon push negotiation, legacy daemon pull) that disagree, and the client's largest tiers can never be exercised.

**Mechanism**: tuning.rs:14-26 returns (initial,max) up to (24,32). But the daemon's push negotiation computes its own ladder desired_streams (control.rs:499-519, max 16, keyed on bytes OR file count), and the client takes neg.stream_count.max(1).min(tuning.max_streams) (push/client/mod.rs:637-640) — so the effective stream count is always the daemon's ≤16 and the client's 24/32 tiers are dead configuration. The legacy Pull RPC has a third near-duplicate ladder pull_stream_count (pull.rs:915-933, same byte tiers as desired_streams but no file-count criterion, capped by tuning.max_streams). The live pull-sync path uses none of them (constant 1). Three authorities, none measuring anything at runtime, agreeing only where literals coincide.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:14 — ladder (4,8)..(24,32) keyed on total_bytes
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:499 — desired_streams: second ladder, max 16, bytes OR file_count keyed — this one actually governs push
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:638 — stream_target = neg.stream_count.max(1).min(tuning.max_streams) — daemon's ≤16 always wins; tuning's 24/32 tiers unreachable
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:915 — pull_stream_count: third ladder, legacy Pull RPC only

**Proposed fix**: Pick one owner (the daemon negotiation is the natural one since it sees the manifest) and make the other sites consume it: delete pull_stream_count with the legacy path or alias it to desired_streams, and either remove tuning.rs's unreachable 24/32 tiers or make them the single shared ladder both ends reference.

#### deadcode-core-abandoned-foundation-modules — Four whole blit-core modules plus copy/fs_enum leftovers (~1,000 lines) have zero callers: tar_stream, zero_copy, delete, copy/parallel+stats, chunked_copy_file, fs_enum helpers

**Principle**: maintainability | **Slice**: medium

**Claim**: tar_stream.rs (414 lines), zero_copy.rs (219), delete.rs (93), copy/parallel.rs+stats.rs (68), chunked_copy_file, and four fs_enum helpers are all pub-exported with zero callers outside their own files/re-exports, each superseded by a live implementation elsewhere.

**Mechanism**: Per-symbol rg across crates/ and tests/ this session: (1) tar_stream/TarConfig/TarEvent — only lib.rs:23; its header cites a parent file streaming_batch.rs that no longer exists; live tar shards go through remote/transfer/payload.rs + tar_safety. (2) zero_copy::/splice_from_socket_to_file/ZeroCopyResult/AsRawFileDescriptor — only lib.rs:27; the receive pipeline never adopted splice. (3) crate::delete/DeletePlan/compute_delete_plan/generate_delete_plan — only lib.rs:7; mirror deletion actually flows through MirrorPlanner (orchestrator/fast_path.rs:104). (4) parallel_copy_files/CopyStats/chunked_copy_file — only the definitions and copy/mod.rs:11-14 re-exports; the live local fast path is copy_paths_blocking/copy_file (orchestrator.rs:355, :1263) and the live parallel path is execute_sink_pipeline (transfer/pipeline.rs:24). (5) categorize_files (fs_enum.rs:498), enumerate_symlinks+SymlinkEntry (:20, :439, :475), enumerate_directory_deref_filtered (:521) — zero hits outside fs_enum.rs; the size split lives in transfer_plan.rs now. All compile-checked dead only because they are pub. zero_copy is the one needing an owner decision: delete, or revive as a FAST-principle feature.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/tar_stream.rs:2 — 'Pulled from streaming_batch.rs' — cites a deleted file; only workspace reference is lib.rs:23 pub mod
- /home/michael/dev/Blit/crates/blit-core/src/delete.rs:9 — DeletePlan — rg for DeletePlan/compute_delete_plan/generate_delete_plan outside this file returns nothing; MirrorPlanner at orchestrator/fast_path.rs:104 is the live mirror-delete path
- /home/michael/dev/Blit/crates/blit-core/src/copy/mod.rs:11 — re-exports chunked_copy_file (:11), parallel_copy_files (:13), CopyStats (:14) — rg shows the re-exports and definitions are the only hits
- /home/michael/dev/Blit/crates/blit-core/src/fs_enum.rs:498 — categorize_files; enumerate_symlinks at :439/:475, SymlinkEntry at :20, enumerate_directory_deref_filtered at :521 — zero external callers
- /home/michael/dev/Blit/crates/blit-core/src/zero_copy.rs:1 — 'Zero-copy primitives for high-performance I/O' — 219 lines, only lib.rs:27 references it; delete-vs-revive is an owner decision (FAST)

**Proposed fix**: One deletion slice removing tar_stream.rs, delete.rs, copy/parallel.rs, copy/stats.rs, chunked_copy_file, and the four fs_enum helpers plus their lib.rs/copy/mod.rs exports (pure dead weight, no wire impact); a separate owner question for zero_copy.rs: delete now or file a plan to wire splice into the receive pipeline.

#### deadcode-core-control-plane-payload-duplicate — transfer_payloads_via_control_plane is a self-admitted zero-caller duplicate of the gRPC fallback sink, kept pub and maintained defensively

**Principle**: maintainability | **Slice**: small

**Claim**: payload.rs's transfer_payloads_via_control_plane has no callers — its own comment says so — yet stays pub, re-exported, and carries an actively-maintained audit-h3c chunk clamp for hypothetical future callers, duplicating the live GrpcFallbackSink path.

**Mechanism**: rg for transfer_payloads_via_control_plane returns exactly three hits: the definition (payload.rs:234), the re-export (transfer/mod.rs:20), and a doc reference in grpc_fallback.rs:46. The function's own comment at payload.rs:247-249 states 'No live caller today (grep returns zero matches), but the function is pub and re-exported, so any future caller would silently bypass the cap without this line' — i.e. the audit stratum is paying ongoing maintenance cost (the clamp_fallback_chunk_size call at :251) on a dead function. It emits the same FileManifest/FileData/TarShard loop the live GrpcFallbackSink owns. Every future transport-policy change (e.g. the queued slice-2 chunk work) must be applied here too or the dead copy drifts; deleting it removes a whole replication site.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/payload.rs:247 — 'No live caller today (grep returns zero matches), but the function is pub and re-exported' — self-documented dead code with maintained clamp at :251
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/mod.rs:20 — re-export of transfer_payloads_via_control_plane — the only thing keeping it reachable

**Proposed fix**: Delete the function and its re-export; the grpc_fallback.rs:46 doc reference becomes the tombstone. Coordinate ordering with the queued slice-2 chunk_bytes deletion so neither slice has to patch the dead copy.

#### deadcode-daemon-push-upload-channel — Daemon push handler builds a 262,144-slot manifest channel whose only consumer is a drain-and-discard task, with a comment that misstates who uses it

**Principle**: SIMPLE | **Slice**: medium

**Claim**: The push control loop clones every uploadable FileHeader into a 262,144-capacity mpsc channel whose receiver is handed to the TCP data plane solely so a spawned task can drain and discard it; the comment claiming 'Only the gRPC fallback path uses this queue' is false — the fallback takes the files_to_upload Vec instead.

**Mechanism**: control.rs:55 creates (upload_tx, upload_rx) with FILE_UPLOAD_CHANNEL_CAPACITY = FILE_LIST_BATCH_MAX_ENTRIES*16 = 262,144 (control.rs:31). Every file passing file_requires_upload gets file.clone() sent into it (control.rs:157) on the hot per-manifest-entry path. The receiver is taken at control.rs:214-215 and :287 and passed only into accept_data_connection_stream (TCP path); inside data_plane.rs:200-207 a task is spawned that does `while guard.recv().await.is_some() {}` — drain and discard — and :207 voids the companion cache param ('headers come off the wire; cache no longer needed'). The gRPC fallback at control.rs:275 calls execute_grpc_fallback with files_to_upload.clone(), not the channel, directly contradicting the comment at control.rs:151-154. Net cost: one FileHeader clone per uploaded file plus a spawned drain task per data plane, purely to feed a Phase-5 leftover; net risk: the false comment misleads the next editor about liveness.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:55 — channel created with FILE_UPLOAD_CHANNEL_CAPACITY (=16*1024*16, control.rs:31); per-file clone sent at :157
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:151 — 'Only the gRPC fallback path uses this queue' — false: fallback at :275 takes files_to_upload.clone(), and upload_rx goes only to accept_data_connection_stream (:222, :294)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:200 — drain task: `while guard.recv().await.is_some() {}`; `let _ = cache;` at :207 — receiver exists only to be emptied

**Proposed fix**: Delete upload_tx/upload_rx, FILE_UPLOAD_CHANNEL_CAPACITY, the drain task, and the now-unused `files`/`cache` parameters of handle_data_plane_stream/accept_data_connection_stream; remove the per-file clone at control.rs:157. Internal-only — no wire change.

#### drift-agents-md-ghost-identifiers — AGENTS.md §6 tells every agent to 'match existing names' transfer_engine and PLAN_OPTIONS — neither exists

**Principle**: maintainability | **Slice**: small

**Claim**: The always-loaded agent contract cites transfer_engine (no such module) and PLAN_OPTIONS (no such constant; the real symbol is the PascalCase struct PlanOptions) as canonical existing names, and §4 lists 'transfer engine' in the blit-core project map; additionally the active design map wrongly asserts TransferOrchestrator does not exist.

**Mechanism**: rg 'transfer_engine' over crates/ returns zero matches (blit-core's actual modules are transfer_plan.rs and remote/transfer/; verified against ls of crates/blit-core/src). rg 'PLAN_OPTIONS' over the whole repo matches only AGENTS.md:113 itself; the real symbol is `pub struct PlanOptions` at transfer_plan.rs:25 — cited in AGENTS.md as a SHOUT_CASE constants example, so it is wrong in both name and case category. The third example, TransferOrchestrator, DOES exist (orchestrator.rs:116, re-exported at orchestrator/mod.rs:8, used by blit-app and tests/integration/*) — which means DESIGN_MAP_2026-06-11.md:105-106 ('name modules (transfer_engine, TransferOrchestrator) that do not exist in the tree') is itself half-refuted and needs an erratum before Phase C consumes it.

**Evidence**:
- /home/michael/dev/Blit/AGENTS.md:112 — §6: 'match existing names (`transfer_engine`, `TransferOrchestrator`, `PLAN_OPTIONS`)' — first and third are ghosts
- /home/michael/dev/Blit/AGENTS.md:75 — §4 project map: 'crates/blit-core/ — core library (enumeration, planner, transfer engine, orchestrator)' — no transfer-engine module exists
- /home/michael/dev/Blit/crates/blit-core/src/transfer_plan.rs:25 — pub struct PlanOptions — the real symbol PLAN_OPTIONS garbles
- /home/michael/dev/Blit/crates/blit-core/src/orchestrator/orchestrator.rs:116 — pub struct TransferOrchestrator — exists, refuting half of the design-map/graveyard claim
- /home/michael/dev/Blit/docs/audit/DESIGN_MAP_2026-06-11.md:106 — Map asserts '(transfer_engine, TransferOrchestrator) ... do not exist in the tree' — TransferOrchestrator does exist; map needs a second erratum

**Proposed fix**: Edit AGENTS.md §6 to cite real symbols (e.g. `TransferOrchestrator`, `PlanOptions`, an actual SHOUT_CASE constant like `DEFAULT_PAYLOAD_PREFETCH`) and §4 to name real blit-core areas (enumeration, mirror_planner, remote::transfer, orchestrator); append an erratum to DESIGN_MAP_2026-06-11.md headline correcting the TransferOrchestrator claim.

#### drift-pipeline-filestream-ghost-variant-comment — pipeline.rs doc comment names PreparedPayload::FileStream and FsTransferSink::write_payload(FileStream {…}) — neither exists

**Principle**: maintainability | **Slice**: small

**Claim**: The execute_receive_pipeline doc comment documents file data flowing through a PreparedPayload::FileStream variant and a write_payload(FileStream {…}) call, but the enum has no such variant and the code calls write_file_stream directly.

**Mechanism**: PreparedPayload (payload.rs:78-106) has exactly four variants: File, TarShard, FileBlock, FileBlockComplete. payload.rs:73-76 explicitly documents the design decision that streaming file bytes 'are NOT a payload variant — they go through TransferSink::write_file_stream directly'. The function body confirms it: the DATA_PLANE_RECORD_FILE arm calls sink.write_file_stream(&header, &mut reader) at pipeline.rs:230, never constructing any payload. So the doc comment at pipeline.rs:190 ('producing [`PreparedPayload::FileStream`] ... events') and :196 ('hits disk through `FsTransferSink::write_payload(FileStream { … })`') describes the abandoned UNIFIED_RECEIVE_PIPELINE.md Phase 1 design, not the shipped one. The same ghost propagated into docs/audit/2026-05-04_roadmap_audit.md:146, which marks the variant SHIPPING with fabricated evidence ('payload.rs exposes FileStream').

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/pipeline.rs:190 — doc comment: 'producing [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] / [`PreparedPayload::FileBlock`] events'
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/pipeline.rs:196 — doc comment: 'file data hits disk through `FsTransferSink::write_payload(FileStream { … })`' — actual call is sink.write_file_stream at line 230
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/payload.rs:73 — authoritative counter-statement: streaming bytes 'are NOT a payload variant — they go through TransferSink::write_file_stream directly'
- /home/michael/dev/Blit/docs/audit/2026-05-04_roadmap_audit.md:146 — claims 'SHIPPING | payload.rs exposes FileStream' — false; propagation of the same ghost into an audit doc

**Proposed fix**: Rewrite pipeline.rs:184-199 to describe the real flow (FILE records → write_file_stream with a Take-limited borrowed reader; TarShard/FileBlock/FileBlockComplete → write_payload), and add a one-line correction note to the 2026-05-04 roadmap audit row.

#### drift-resume-retry-help-overstates-push-coverage — --resume/--retry help, manpage, and retry.rs doc state the resumability premise unconditionally, but push has no block resume and silently ignores --resume

**Principle**: RELIABLE | **Slice**: small

**Claim**: User-facing help ('Resume interrupted transfers using block-level comparison'; 'Because transfers are resumable, each retry continues rather than restarts'), the manpage, and the retry.rs module doc claim resumability for all transfers, but the push direction never produces resume payloads and the CLI never even passes args.resume into the push path.

**Mechanism**: In crates/blit-cli/src/transfers/remote.rs, `resume: args.resume` appears exactly once (line 382), inside PullSyncOptions in the pull execution; run_remote_push_transfer_inner (same file, :203+) never reads args.resume — so `blit push --resume` is accepted and silently does nothing block-level. On the library side, the push source can only produce File/TarShard payloads: push/client/mod.rs:266-269 documents 'Resume payloads originate on the receive side; the outbound prune path never sees them', and payload.rs prepare_payload bails on FileBlock from a filesystem source (:61-63). Consequently a retried push re-sends any partially-transferred file from byte 0 (manifest diff only skips completed files), while cli.rs:268-271 tells the user 'Because transfers are resumable, each retry continues rather than restarts' and retry.rs:6-10 states the same premise as the loop's justification.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/src/cli.rs:265 — --resume help: 'Resume interrupted transfers using block-level comparison' — no direction qualifier
- /home/michael/dev/Blit/crates/blit-cli/src/cli.rs:270 — --retry help: 'Because transfers are resumable, each retry continues rather than restarts' — unconditional
- /home/michael/dev/Blit/crates/blit-cli/src/transfers/remote.rs:382 — only consumer of args.resume — PullSyncOptions (pull path); push path never reads it
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:266 — 'Resume payloads originate on the receive side; the outbound prune path never sees them' — push cannot block-resume
- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:6 — module doc: 'blit transfers are **resumable** — ... a retry continues rather than restarts' — unconditional premise
- /home/michael/dev/Blit/docs/cli/blit.1.md:110 — manpage --resume: 'Enable block-level resumption for interrupted transfers' — same unqualified claim

**Proposed fix**: Scope the claims: amend cli.rs --resume/--retry help, blit.1.md, and retry.rs's module doc to state block resume applies to local/pull (and delegated specs), and that push retries resume at whole-file granularity; optionally emit a warning when --resume is passed to a push (per project rule, help + manpage + README change in the same slice).

#### drift-set-keepalive-comments-oversell-liveness — Both set_keepalive(true) sites carry comments claiming they prevent idle-stream timeouts, but OS-default keepalive timing (~2h on Linux) makes them inert for that purpose

**Principle**: RELIABLE | **Slice**: small

**Claim**: Comments at the two TCP data-plane keepalive sites claim the calls keep idle connections alive / prevent idle stream timeouts during long transfers, but set_keepalive(true) without a TcpKeepalive config enables SO_KEEPALIVE at kernel-default timing (Linux tcp_keepalive_time = 7200s), so no probe fires within any realistic transfer window.

**Mechanism**: socket2's Socket::set_keepalive(bool) only toggles SO_KEEPALIVE; per-socket timing requires set_tcp_keepalive(&TcpKeepalive). rg across all crates shows no TcpKeepalive or set_tcp_keepalive anywhere — only the two bare set_keepalive(true) calls (core data_plane.rs:106, daemon push/data_plane.rs:120). With Linux defaults the first probe is sent after 2 hours of idle, far beyond the 30s/15s accept/token windows and the 30s StallGuard, so the documented purpose ('Keep idle connections alive during long transfers on other streams', 'prevent idle stream timeouts during long transfers') is not achieved — at best dead peers are reaped hours later. The two copies also diverge on failure handling: the core comment (data_plane.rs:104-105) insists failures must be surfaced 'so a misconfigured run isn't silent' and logs a warning, while the daemon copy silently swallows the same failure with `let _ =` at push/data_plane.rs:120. Future authors reading either comment will assume liveness coverage that does not exist. (Distinct from queued slice-2, which adds HTTP/2 keepalive on client gRPC channels, not these raw TCP sockets.)

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:101 — comment 'Keep idle connections alive during long transfers on other streams' above set_keepalive(true) at :106 — no timing configured
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:112 — comment 'Enable nodelay + keepalive to prevent idle stream timeouts during long transfers' above `let _ = s2.set_keepalive(true);` at :120 — error silently swallowed, contradicting the core copy's stated rationale

**Proposed fix**: Either configure explicit TcpKeepalive timing (e.g. with_time/with_interval in tens of seconds) at both sites, or rewrite both comments to state the actual behavior (OS-default SO_KEEPALIVE, dead-peer reaping after hours, not a stall/liveness mechanism); in the same slice make the daemon copy log its failure like the core copy.

#### drift-workflow-phase2-shipped-ghost-machinery (reviewer: high) — WORKFLOW_PHASE_2.md is marked Shipped and certifies streaming-planner machinery with zero trace in code

**Principle**: maintainability | **Slice**: small

**Claim**: A Shipped plan doc certifies, with checkmarks, deliverables (TransferFacade::stream_local_plan, PlannerEvent, heartbeat loop and 10s stall guard in drive_planner_events, transfer_engine streaming tests) that do not exist anywhere in the workspace.

**Mechanism**: rg for stream_local_plan, PlannerEvent, drive_planner_events, TransferFacade, and transfer_engine over crates/ and tests/ returns zero matches; rg -i heartbeat in crates/ matches only the daemon's unrelated DaemonHeartbeat event (service/core.rs:280). The shipped architecture is the opposite of the certified one: orchestrator.rs collects ALL scan headers before planning ('// 2. Collect all headers' + while let Some(h) = header_rx.recv().await at orchestrator.rs:542-546, no timeout, no incremental flush, no stall detector — the only stall guard in the repo is the remote TCP StallGuard in remote/transfer/stall_guard.rs). Fast-path routing (orchestrator/fast_path.rs) and tests/integration/local_transfers.rs DO exist, so the doc is partially true, which makes the false rows more credible. STATE.md queue item 6 (lines 49-50) already concedes the streaming planner is 'owner-ratified, not yet built (H10b)' — directly contradicting this doc's Shipped status, violating the AGENTS.md §1 rule that the losing doc must be fixed.

**Evidence**:
- /home/michael/dev/Blit/docs/plan/WORKFLOW_PHASE_2.md:5 — **Status**: Shipped (was: In progress (streaming planner + fast-path routing in place))
- /home/michael/dev/Blit/docs/plan/WORKFLOW_PHASE_2.md:11 — Success criterion: 'Planner flushes batches incrementally; stall detector aborts with clear messaging after 10 s of inactivity' — neither exists
- /home/michael/dev/Blit/docs/plan/WORKFLOW_PHASE_2.md:29 — Rows 29-31: '✅ TransferFacade::stream_local_plan emitting PlannerEvent', '✅ Heartbeat loop in drive_planner_events', '✅ Stall guard in drive_planner_events; Windows+Linux verified' — all three symbols have zero grep matches in crates/
- /home/michael/dev/Blit/docs/plan/WORKFLOW_PHASE_2.md:58 — 2.4.1 deliverable: 'transfer_engine streaming tests passing on Windows/Linux' — no transfer_engine module exists
- /home/michael/dev/Blit/crates/blit-core/src/orchestrator/orchestrator.rs:544 — while let Some(h) = header_rx.recv().await { all_headers.push(h) } — collect-everything-then-plan, no streaming, no heartbeat, no timeout
- /home/michael/dev/Blit/docs/STATE.md:49 — Queue item 6: 'greenfield_plan_v6.md §1.1 streaming planner + 1 s heartbeat + 10 s stall detector — owner-ratified, not yet built (H10b)' — contradicts the Shipped header

**Proposed fix**: Re-status WORKFLOW_PHASE_2.md to Historical (or 'Partially shipped') with a dated erratum: replace the three ✅ deliverable cells at lines 29-31 and the line-58 cell with what actually shipped (synchronous collect-then-plan orchestrator, fast_path.rs routing, remote-only 30s StallGuard), strike the 10s-stall success criterion at line 11, and cross-link STATE.md queue item 6 as the live home of the unbuilt work.

#### duplication-buffer-pool-sizing-formula (reviewer: high) — BufferPool sizing formula re-derived at four sites; live pull-sync path hardcodes pool=4/prefetch=8 and discards computed tuning

**Principle**: FAST | **Slice**: medium

**Claim**: The buffer_size/pool_size/memory_budget construction formula for data-plane BufferPools is independently written four times, and the two copies on the live pull-sync path have already diverged: they hardcode pool_size=4 and a literal prefetch of 8 while the push path derives both from TuningParams.

**Mechanism**: push/client/mod.rs:125-128 computes pool_size = streams*2+4, buffer_size = chunk_bytes.max(64*1024), budget = buffer_size*pool_size*2; daemon pull.rs:685-688 is a verbatim copy. The two pull_sync copies (pull_sync.rs:636-639 and 758-761) replace the formula with pool_size = 4 and pass bare literal 8 as payload_prefetch to DataPlaneSession::from_stream (pull_sync.rs:641 and 765). pull_sync.rs calls determine_remote_tuning at lines 500/550/687 — which computes initial_streams/max_streams, tcp_buffer_size (4-8 MiB), and prefetch_count (16/32) — but grep shows the file never reads any of those fields; only tuning.chunk_bytes is used. So the auto-tuner's output is computed and then thrown away on the primary pull path: large pulls run with a 4-buffer pool and prefetch 8 regardless of transfer size, while an identical push adapts. The 64 KiB floor literal is also re-stated in every copy plus data_plane.rs:66 and :584 with no shared constant.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/mod.rs:125 — pool_size = streams * 2 + 4; buffer_size = chunk_bytes.max(64 * 1024); memory_budget = buffer_size * pool_size * 2
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:685 — verbatim copy of the same three-line formula
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:637 — diverged copy: let pool_size = 4; literal prefetch 8 passed at line 641
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:758 — second diverged copy in the same file (resume path), literal prefetch 8 at line 765
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:30 — determine_remote_tuning sets tcp_buffer_size and prefetch_count (16/32) that pull_sync never reads (grep: no prefetch_count/tcp_buffer_size/initial_streams references in pull_sync.rs)

**Proposed fix**: Add a constructor (e.g. BufferPool::for_data_plane(tuning: &TuningParams, streams: usize)) in blit-core that owns the formula and the 64 KiB floor; make all four sites call it and make pull_sync consume tuning.prefetch_count instead of literal 8.

#### duplication-byte-total-tuning-ladders (reviewer: high) — Four independent byte-total tuning ladders (chunk size / stream count) that already disagree between push and pull

**Principle**: FAST | **Slice**: medium

**Claim**: The 'total bytes -> chunk size / stream count' decision is implemented in four places with different breakpoints and outputs, so push and pull of the same tree run different parallelism and chunk sizes, and one of the four implementations is dead code advertising adaptation that never runs.

**Mechanism**: tuning.rs:5-28 maps total_bytes to chunk 16/32/64 MiB (binary 512 MiB / 8 GiB breaks) and streams (4,8)..(24,32). pull.rs:915-933 (pull_stream_count) re-implements the stream ladder over the same byte breakpoints with different outputs — 16/12/10/8/4/2/1, min'd with tuning_max — so at 32 GiB push tunes initial_streams=24 while pull negotiates 16, and at 128 MiB push uses 6 vs pull 4. transfer_plan.rs:223-228 is a third chunk ladder with a decimal 1_000_000_000 threshold, a large-file-dominance criterion, and no 64 MiB tier — it disagrees with tuning.rs and only loses when chunk_bytes_override is set (line 229). chunked.rs:35-39 is a fourth: file_size > 1_073_741_824 -> fixed 16 MiB bypassing BufferSizer. Meanwhile auto_tune/mod.rs:26-32 (analyze_warmup_result) and the warmup branches of determine_tuning (mod.rs:39-67) are unreachable: the only production caller is tuning.rs:13 with warmup_result=None, and rg shows analyze_warmup_result has zero non-test callers — its initial_streams=2/max_streams=8 outputs are immediately overwritten at tuning.rs:27-28.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:14 — stream ladder (24,32)/(16,24)/(12,16)/(8,12)/(6,10)/(4,8) at 32G/8G/2G/512M/128M breaks
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:916 — pull_stream_count: same breakpoints, different outputs 16/12/10/8/4/2/1
- /home/michael/dev/Blit/crates/blit-core/src/transfer_plan.rs:223 — third chunk ladder: decimal 1e9 threshold, 50% large-bytes criterion, no 64 MiB tier
- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/chunked.rs:35 — fourth ladder: >1 GiB -> fixed 16 MiB, bypassing BufferSizer
- /home/michael/dev/Blit/crates/blit-core/src/auto_tune/mod.rs:26 — analyze_warmup_result: zero callers (rg verified); determine_tuning only ever called with warmup=None (tuning.rs:13)

**Proposed fix**: Make remote/tuning.rs the single owner of the bytes->parallelism/chunk decision: have pull_stream_count call into it (or delete it and ship tuning.initial_streams over the negotiation), delete the dead auto_tune warmup branches and analyze_warmup_result, and have transfer_plan take chunk_bytes as a required input instead of embedding its own ladder.

#### duplication-cli-test-daemon-harness — Daemon test harness cloned into three integration-test files despite an existing shared TestContext, and fake tonic servers diverge from production config

**Principle**: maintainability | **Slice**: medium

**Claim**: blit-cli has a shared test harness (tests/common/mod.rs TestContext) used by 11 test files, but remote_pull_mirror.rs, remote_tcp_fallback.rs, and remote_checksum_negotiation.rs each carry a private verbatim clone of its config structs, port picker, and daemon bring-up; separately, all in-repo fake tonic servers are bare Server::builder() while production sets HTTP/2 keepalive.

**Mechanism**: common/mod.rs:12-42 defines DaemonConfig/DaemonSection/ModuleSection and pick_unused_port; remote_pull_mirror.rs:12-42 and remote_tcp_fallback.rs:12-41 re-declare them byte-for-byte (remote_checksum_negotiation.rs:20/43 likewise, grep-verified), and each then repeats TestContext::new()'s body inline — blitd.toml serialization, current_exe-relative binary discovery, the `cargo build -p blit-daemon` step (remote_pull_mirror.rs:47-110+, remote_tcp_fallback.rs:47-118) — so a harness fix (e.g. readiness wait, new config field) only reaches the files that use common. The fake gRPC servers are a second axis: remote_remote.rs duplicates a whole thread+runtime+Server::builder bring-up twice in one file (528-548 vs 566-586, near-identical), and those plus pull_sync_with_spec_wire.rs:201 build bare Server::builder() with no http2_keepalive_interval/timeout, whereas production main.rs:137-139 makes keepalive load-bearing (owner decision 2026-05-23) — so wire tests exercise a server config that differs from production in exactly that field.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:12 — shared DaemonConfig/TestContext exists and is used via `mod common` by 11 test files
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_pull_mirror.rs:12 — verbatim re-declaration of DaemonConfig/DaemonSection/ModuleSection + pick_unused_port (36-42), bring-up body inlined at 47-110
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_tcp_fallback.rs:12 — same verbatim clone, bring-up inlined at 47-118
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:540 — bare Server::builder() fake server, duplicated again at 578; production main.rs:138-139 adds keepalive these tests lack

**Proposed fix**: Port the three holdout files onto common::TestContext (extending it for their extra knobs like --no-server-checksums and dual daemons), and add a shared spawn_fake_server helper that mirrors production Server::builder settings (keepalive) so wire tests exercise the deployed config.

#### duplication-file-hash-read-loop — The 256 KiB file-hashing read loop is copy-pasted four times in checksum.rs and reimplemented a fifth time in the daemon

**Principle**: maintainability | **Slice**: small

**Claim**: The read-loop-into-hasher pattern with a 256 KiB buffer exists as four near-identical copies inside checksum.rs and a fifth independent Blake3 implementation in the daemon's build_file_header, which uses a 256 KiB stack array instead of calling the checksum module.

**Mechanism**: checksum.rs declares `vec![0u8; 256 * 1024]` and the identical read/update loop at lines 148 (Blake3), 160 (XxHash3), 174 (Md5), and 194 (partial hash). blit-daemon/src/service/pull.rs:521-534 re-implements Blake3 whole-file hashing from scratch — blake3::Hasher::new + BufReader + `let mut buf = [0u8; 256 * 1024]` stack array — instead of calling checksum::hash_file (it wants the already-open File, which hash_file's path-based signature cannot accept). Five sites re-state the 256 KiB buffer decision; a change in checksum strategy (e.g. mmap, rayon-chunked Blake3) would not reach the daemon's pull checksum path, silently desynchronizing client/daemon checksums' performance characteristics, and the daemon copy puts a 256 KiB frame on the stack of an async-adjacent task.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/checksum.rs:148 — first of four identical 256 KiB read loops (also 160, 174, 194)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:524 — fifth copy: stack array [0u8; 256 * 1024] + hand-rolled Blake3 loop in build_file_header instead of checksum::hash_file

**Proposed fix**: Add a hash_reader(reader: impl Read, ty: ChecksumType) helper in checksum.rs that owns the buffer size and loop; hash_file and the daemon's build_file_header both call it (daemon passes its already-open File).

#### duplication-mirror-purge-executors (reviewer: high) — Four mirror/purge deletion executors, including a near-verbatim cross-crate twin; only two clear Windows read-only, so the same mirror command succeeds or fails by direction

**Principle**: RELIABLE | **Slice**: medium

**Claim**: Mirror-deletion is implemented four times; blit-app's delete_listed_paths and blit-daemon's apply_delete_list are near byte-identical clones, and the four copies have already diverged on Windows read-only handling and on directory-count accounting.

**Mechanism**: orchestrator.rs:973-997 (local mirror) and daemon admin.rs:195-201 (push-side purge) call win_fs::clear_readonly_recursive before remove_file under cfg(windows). delegated_pull.rs:445-502 (apply_delete_list) and blit-app transfers/remote.rs:203-264 (delete_listed_paths) are the same algorithm — canonical_dest_root + safe_join_contained + remove_file + deepest-first parent pruning — written twice across crates, and neither clears read-only, so a read-only destination file deletes fine under local/push mirror but hard-fails the transfer under pull/delegated mirror on Windows. The clones have also drifted internally: delete_listed_paths counts dirs_deleted on successful remove_dir (remote.rs:258-261) while apply_delete_list discards the result entirely (`let _ = tokio::fs::remove_dir` at delegated_pull.rs:498) and returns only files_deleted; delegated_pull.rs:493-494 even cites the blit-app function by name in a comment as its behavioral reference — duplication coupled by comment instead of by code.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/orchestrator/orchestrator.rs:974 — cfg(windows) clear_readonly_recursive before remove_file (and again at 996 for dirs)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/admin.rs:197 — second readonly-clearing copy in the daemon purge loop
- /home/michael/dev/Blit/crates/blit-daemon/src/service/delegated_pull.rs:445 — apply_delete_list: clone of delete_listed_paths, no readonly clearing, `let _ = remove_dir` at 498
- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote.rs:203 — delete_listed_paths: the original of the clone pair, no readonly clearing, counts dirs at 258-261

**Proposed fix**: Extract one purge executor into blit-core (taking dest_root + relative paths, doing containment, readonly clearing, and dir pruning with consistent stats) and have all four call sites use it; blit-daemon already depends on blit-core so the cross-crate clone disappears.

#### duplication-progress-folding-rules — ProgressEvent folding semantics live in two consumers (TUI's three per-direction rules vs CLI's one generic rule) with no contract on the enum

**Principle**: RELIABLE | **Slice**: medium

**Claim**: The knowledge of what Payload.bytes vs FileComplete.bytes mean per transfer direction is duplicated between blit-tui/progress_accum.rs (three documented direction-specific folders) and blit-cli/transfers/remote.rs (one generic folder), with the contract written only in TUI doc comments — so each new consumer re-derives the folklore, and the CLI already derived it wrong.

**Mechanism**: progress_accum.rs:21-36 (pull: bytes from Payload only — its doc at lines 12-20 explicitly names the pipeline.rs double-emit trap), :50-65 (push: bytes+files from FileComplete only), :75-88 (delegated: both from Payload) encode three incompatible producer semantics. blit-cli/transfers/remote.rs:64-76 uses a single rule for all directions that adds bytes from BOTH Payload and FileComplete — the exact combination progress_accum.rs:12-16 documents as double-counting on the TCP pull receive path (the resulting CLI bug is filed as design-1-cli-pull-byte-double-count; this finding is about the structure that produced it). The semantics are documented nowhere on ProgressEvent itself (blit-core/src/remote/transfer/progress.rs), only in TUI-crate doc comments, so fixing design-1 inside the CLI would create a third independent copy of the folding rules rather than a shared owner.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-tui/src/progress_accum.rs:12 — doc comment documenting the Payload+FileComplete double-emit and why bytes must come from Payload only on pull
- /home/michael/dev/Blit/crates/blit-tui/src/progress_accum.rs:50 — push folder: bytes from FileComplete only — opposite rule, same enum
- /home/michael/dev/Blit/crates/blit-cli/src/transfers/remote.rs:68 — CLI single rule adds bytes from Payload (68-71) AND FileComplete (73-75) for all directions

**Proposed fix**: Move the three folding rules out of blit-tui into a shared module (blit-app, or next to ProgressEvent in blit-core) keyed by direction, document the per-direction contract on the enum, and make the CLI monitor consume the shared pull/push/delegated folders — this is the structural half of the design-1 fix.

#### duplication-wire-metadata-helpers — FileHeader metadata helpers (permissions_mode, mtime-seconds) duplicated across blit-core and blit-daemon with one already-diverged signature

**Principle**: maintainability | **Slice**: small

**Claim**: The helpers that capture wire metadata for FileHeader are copy-pasted between the push client and the daemon: permissions_mode is byte-identical in both crates, and the mtime helper has already diverged in shape (i64-with-0-default vs Option<i64>).

**Mechanism**: blit-core/src/remote/push/client/helpers.rs:72-83 and blit-daemon/src/service/util.rs:140-151 contain byte-identical permissions_mode bodies (cfg(unix) mode bits, cfg(not(unix)) returns 0). The sibling mtime helpers have drifted: helpers.rs:59-70 (unix_seconds) returns i64 with `Err(_) => 0` when modified() is unavailable, while util.rs:130-138 (metadata_mtime_seconds) returns Option<i64> and leaves the default to each caller (pull.rs:539 does .unwrap_or(0)). Both are the producer half of the wire metadata contract (proto FileHeader), so a fix to either side — e.g. representing Windows read-only in permissions, or changing pre-epoch handling — must be discovered and re-applied in the other crate by hand; nothing ties them together, and blit-daemon already depends on blit-core so the duplication is purely historical.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/push/client/helpers.rs:72 — permissions_mode — identical body to the daemon copy
- /home/michael/dev/Blit/crates/blit-daemon/src/service/util.rs:140 — permissions_mode — byte-identical twin
- /home/michael/dev/Blit/crates/blit-daemon/src/service/util.rs:130 — metadata_mtime_seconds returns Option<i64>; helpers.rs:59-69 unix_seconds returns i64 defaulting to 0 — diverged shape for the same wire field

**Proposed fix**: Move both helpers into blit-core (e.g. next to path_posix or a remote::wire_meta module) as the single producer of FileHeader metadata fields; daemon util.rs and push helpers.rs re-export/call them.

#### errors-daemon-eyre-to-status-chain-amputation — Daemon eyre→Status formatting is line-dependent: ~12 {err:#} sites preserve the cause chain, ~69 {err} sites amputate it

**Principle**: RELIABLE | **Slice**: medium

**Claim**: Whether the root cause of a daemon-side failure crosses the wire depends on which line failed: most Status::*(format!(...)) sites format the eyre Report with {err}/{e} (outermost message only) while a minority use {err:#} (full chain), with both styles coexisting in the same files.

**Mechanism**: eyre::Report's Display prints only the top message; the alternate {:#} prints 'top: cause1: cause2…'. In pull_sync.rs the two pipeline failure paths diverge: line 507 'planning gRPC payloads: {err}' (chain dropped) vs line 653 'pull sync data plane pipeline: {err:#}' (chain kept). util.rs does the same within one file: line 61 'path not allowed: {}: {e}' vs lines 110/124 'path containment: {e:#}'. delegated_pull.rs:374 sends 'delegated pull: {err}' as the wire upstream_message for the entire Transfer phase — the mid-transfer root cause (e.g. an io error three layers down) is amputated before it reaches the CLI. Counting with rg: 12 Status constructions format with :# vs ~69 with plain {var}. This is daemon-side eyre→Status, not the queued slice-2 client-side Status→eyre work.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:507 — 'planning gRPC payloads: {err}' — chain dropped
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:653 — 'pull sync data plane pipeline: {err:#}' — chain kept; same file, same kind of failure
- /home/michael/dev/Blit/crates/blit-daemon/src/service/util.rs:61 — {e} at 61 vs {e:#} at 110/124 in one file
- /home/michael/dev/Blit/crates/blit-daemon/src/service/delegated_pull.rs:374 — Transfer-phase wire message formats with {err}, amputating the upstream chain before it crosses back to the CLI

**Proposed fix**: One daemon-boundary helper (e.g. internal_err(context, &Report) using {:#}) and a mechanical sweep of the ~69 plain-format sites; pairs naturally with the io_to_status helper from errors-daemon-status-internal-collapse.

#### errors-dead-classifier-contradicts-live (reviewer: high) — Dead blit_core::errors module is publicly exported and contradicts the live retry classifier on three error kinds

**Principle**: maintainability | **Slice**: small

**Claim**: blit-core/src/errors.rs (ErrorCategory/TransferError/categorize_io_error) has zero importers anywhere in the workspace yet is publicly exported, and its retryability table directly contradicts the live classifier in blit-app on ConnectionRefused, UnexpectedEof, and NotConnected.

**Mechanism**: errors.rs:108-113 classifies WriteZero/UnexpectedEof/AddrInUse/AddrNotAvailable/NotConnected/ConnectionRefused as Fatal ('could go either way - default to fatal'), while the live classifier retry.rs:35-46 marks ConnectionRefused/UnexpectedEof/NotConnected Retryable; errors.rs:94-98 additionally marks Interrupted/WouldBlock retryable, which retry.rs treats as fatal. I verified zero consumers: rg for categorize_io_error|ErrorCategory|blit_core::errors|crate::errors across all crates matches only errors.rs itself — every other `TransferError` hit is the unrelated proto-generated message (e.g. blit-cli/src/jobs.rs:112 uses blit_core::generated::TransferError), a name collision that makes the trap worse: the doc-commented, lib.rs:9-exported core module looks like the designated owner of retry policy, and any future contributor wiring it up flips retry semantics for three error kinds and shadows the wire type name.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/errors.rs:108 — ConnectionRefused/UnexpectedEof/NotConnected → ErrorCategory::Fatal (lines 108-113)
- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:41 — same three kinds → retryable in is_retryable_io_kind (lines 38-45)
- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — `pub mod errors;` — dead module publicly exported
- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:112 — blit_core::generated::TransferError — the proto message that name-collides with the dead errors::TransferError struct

**Proposed fix**: Delete crates/blit-core/src/errors.rs and the lib.rs:9 export (call out the 4 removed tests in the finding doc's Known gaps); if a shared classifier home is wanted later, the queued slice-2 retry-classifier work is the place to design it.

#### errors-log-facade-has-no-backend (reviewer: high) — All log::warn/log::error failure reporting is silently discarded — no log backend exists in any binary

**Principle**: RELIABLE | **Slice**: small

**Claim**: Roughly 20 error/degradation reports in blit-core go through the `log` facade, but no binary in the workspace installs a logger, so every one of them is dropped at runtime.

**Mechanism**: blit-core depends on `log = "0.4"` (crates/blit-core/Cargo.toml:9) and calls log::warn!/log::error! at ~20 sites, but rg over every Cargo.toml and all source for env_logger / tracing-subscriber / log::set_logger / set_boxed_logger returns zero hits — the `log` crate's default logger is a no-op, so the messages are formatted-and-discarded. The damage is concrete: best-effort mtime/permission application failures during receive (sink.rs:416, 428, 519, 611, 620, 722, 730; tar_safety.rs:242, 251) do NOT propagate as errors — the warn is their ONLY surface, and the comment at sink.rs:413-414 explicitly claims 'Surface via log::warn! so the failure is visible' — it is not. Security-degradation warnings ('escape protection unavailable', sink.rs:196, 471, 579) and socket-option failures (data_plane.rs:107, 116, 119 — the fix POST_REVIEW_FIXES §1.1 claimed was 'logged') are likewise invisible. A user who passes preserve_times and gets wrong mtimes sees nothing at all.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/sink.rs:413 — comment: 'Surface via log::warn! so the failure is visible without making it a hard transfer error' — followed by log::warn! at 416; no binary installs a backend so it is invisible
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/sink.rs:196 — log::warn! 'R46-F3 escape protection unavailable' — security degradation warning, silently dropped
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:107 — log::warn! on SO_KEEPALIVE failure — the 'logged' half of the core-vs-daemon divergence is actually a no-op
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/tar_safety.rs:242 — set-mtime failure on extracted tar entries reported only via log::warn — sole surface for the failure

**Proposed fix**: Install a stderr-printing log backend (env_logger or a 10-line custom logger matching the chosen stderr prefix convention) in the main() of blit-cli, blit-daemon, blit-tui, and blit-prometheus-bridge; default level warn.

#### errors-mpsc-sendfail-fixed-strings — 30+ mpsc send failures map to ad-hoc fixed strings that name the symptom, never the cause

**Principle**: RELIABLE | **Slice**: medium

**Claim**: Every channel-send failure in the transfer paths discards the SendError and substitutes a per-site fixed string ('data plane died', 'failed to send ack', 'gRPC channel closed'), so the user-visible error for 'receiver task exited' never says why it exited.

**Mechanism**: A tokio mpsc send fails only when the receiver was dropped — i.e. the consuming task (response stream / data-plane writer) already exited, usually because IT hit the real error or the client disconnected. The daemon converts this to Status::internal with 11 distinct strings in pull_sync.rs (378 'failed to send ack', 393, 403, 413, 481, 582, 712, 782, 963, 1021, 1040) and 8 in pull.rs (178, 289, 315/337 'data plane died', 593, 619, 950, 965); blit-core sink.rs uses eyre!("gRPC channel closed") at 985, 1016, 1038, 1048, 1056, 1080 and 'gRPC pull stream closed' at 1156-1227. Because outcome_from_status (core.rs:1330) records exactly this string, `blit jobs` history shows 'failed to send ack' for what was actually a client disconnect or a prior pipeline error — three different vocabularies for the same condition, none stating the cause, and the real error in the exited task races against (and often loses to) the send failure.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:315 — map_err(|_| Status::internal("data plane died")) — SendError discarded, cause unstated
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:378 — 'failed to send ack' — same underlying condition, different vocabulary
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/sink.rs:985 — eyre!("gRPC channel closed") — 11 sites in this file across two strings
- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:1330 — outcome_from_status stores the fixed string as the job's recorded failure message

**Proposed fix**: One shared helper per side (daemon Status, core eyre) with a single honest text like 'response channel closed (peer disconnected or pipeline failed): <what was being sent>'; where the receiver task handle is available, prefer joining it and surfacing its error over the send failure.

#### tests-dead-workspace-root-test-suite (reviewer: high) — Workspace-root tests/ directory is never compiled or run by cargo

**Principle**: RELIABLE | **Slice**: medium

**Claim**: The six test files under /tests (mirror_planner_tests.rs with 16 tests, enumeration_tests.rs, checksum_partial.rs, connection.rs, integration/local_transfers.rs, integration/predictor_streaming.rs) are dead: cargo test --workspace never builds them, so the coverage they appear to provide is false.

**Mechanism**: Cargo only compiles a tests/ directory that belongs to a package. The root Cargo.toml is a virtual workspace ([workspace] only, no [package] section, lines 1-10), and no member crate declares a [[test]] target pointing at the root tests/ (verified: rg '[[test]]' over all crates/*/Cargo.toml returns nothing). So `cargo test --workspace` — the validation suite in AGENTS.md §5 — never compiles these files; they get no compile check and no execution. They are not trivially stale (mirror_planner_tests.rs imports blit_core::mirror_planner which exists at lib.rs:16; local_transfers.rs imports blit_core::orchestrator::TransferOrchestrator which exists at lib.rs:17), so they would mostly revive if relocated. mirror_planner.rs itself has zero #[test] (rg -c '#[test]' returns no match for it), meaning MirrorPlanner's semantic tests exist ONLY in this dead directory. connection.rs additionally requires an externally running blitd on :50051 and can never pass in CI as written.

**Evidence**:
- /home/michael/dev/Blit/Cargo.toml:1 — [workspace] with members list only; no [package] section anywhere in the file, so the root tests/ dir is not a cargo target
- /home/michael/dev/Blit/tests/mirror_planner_tests.rs:13 — first of 16 #[test] fns covering MirrorPlanner skip/copy decisions — never compiled
- /home/michael/dev/Blit/tests/connection.rs:5 — test_server_connection requires a separately-running blitd on localhost:50051; comment admits it
- /home/michael/dev/Blit/tests/integration/local_transfers.rs:2 — imports blit_core::orchestrator::TransferOrchestrator — dead end-to-end local-transfer tests
- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:16 — mirror_planner and orchestrator modules exist, so the dead tests reference live APIs and would mostly revive if moved

**Proposed fix**: Move mirror_planner_tests.rs, enumeration_tests.rs, checksum_partial.rs, integration/local_transfers.rs, and integration/predictor_streaming.rs into crates/blit-core/tests/ (fixing whatever no longer compiles); delete connection.rs (it can never run unattended). Update AGENTS.md §4, which still describes tests/ as 'workspace-level integration tests'.

#### tests-five-daemon-harness-clones — Five copy-pasted daemon-spawn harnesses and five cli_bin re-implementations, already drifted from each other

**Principle**: maintainability | **Slice**: medium

**Claim**: The daemon-spawn harness (config struct + port pick + cargo build + spawn + readiness poll) is implemented five times across blit-cli/tests, and cli_bin()/run_with_timeout are re-implemented in four-five more files, with real behavioral drift already present between copies.

**Mechanism**: tests/common/mod.rs::TestContext (lines 56-169) is the nominal shared harness, but remote_remote.rs (DualDaemonContext::spawn_daemon, line 119), remote_pull_mirror.rs (spawn_daemon, line 247), remote_checksum_negotiation.rs (spawn_daemon_harness, line 96), and remote_tcp_fallback.rs (inline clone, cargo build at line 104) each re-implement it because TestContext cannot express their one extra knob (delegation config, second daemon, extra daemon args like --no-server-checksums / --force-grpc-data). Drift is already observable: common's DaemonConfig lacks the delegation/delegation_allowed fields remote_remote.rs added (remote_remote.rs:17-42); remote_remote.rs::build_daemon (line 417) dropped the --target triple handling the other four copies carry (common/mod.rs:123-127), so it builds into the wrong directory under cross-target test runs; stderr policy differs (common pipes it, the others null it). cli_bin() is additionally pasted into single_file_copy.rs:32, local_move_semantics.rs:38, diagnostics_dump.rs:14, cli_arg_safety_gates.rs:40, and remote_pull_mirror.rs:329. Any daemon config-schema or harness fix must now land in 5 places, and the next one will miss at least one (the --target drift proves it already happened).

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:56 — TestContext::new — the nominal shared harness
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:417 — build_daemon clone WITHOUT the --target triple handling common/mod.rs:123-127 has — concrete drift
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_checksum_negotiation.rs:96 — spawn_daemon_harness clone existing only to pass extra_daemon_args
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_tcp_fallback.rs:104 — fifth full harness clone (map said four), differing only by the --force-grpc-data daemon arg
- /home/michael/dev/Blit/crates/blit-cli/tests/single_file_copy.rs:32 — one of five pasted cli_bin() copies

**Proposed fix**: Grow tests/common/mod.rs into a builder: TestContext::builder().extra_daemon_args(..).delegation(..).read_only(..) plus a second_daemon() helper; export cli_bin()/run_with_timeout from common and delete every clone. Pure test refactor, no production code.

#### tests-jobs-lifecycle-no-e2e — No integration test invokes blit jobs (list/watch/cancel) or --detach against a real daemon

**Principle**: RELIABLE | **Slice**: medium

**Claim**: The entire detached-job lifecycle — Subscribe stream, watch loop with GetState fallback reconciliation, CancelJob exit-code contract, --detach output — is never executed end-to-end by cargo test; coverage stops at formatting/exit-code unit tests.

**Mechanism**: rg for 'jobs|Subscribe|cancel' across blit-cli/tests hits only remote_remote.rs, and those hits (e.g. :649, :717) are the fake server's unimplemented trait-method stubs, not jobs-verb tests. No test passes 'jobs' or '--detach' to the CLI binary (rg 'detach' over tests: zero hits). What exists: blit-cli/src/jobs.rs:795+ unit tests (pure formatting/exit-code mapping) and blit-daemon/src/active_jobs.rs unit tests (29, in-process registry). The wire path — jobs watch opening Subscribe, the stream-error fallback to one final GetState (jobs.rs:348-362), cancel_exit_code's 0/1 contract against a real daemon's CancelJob — runs in zero tests. Given the already-filed design-2 (orphaned daemon data planes, cancellation reaching one of four transfer kinds), the team is actively changing cancellation behavior with no harness to detect regressions in the user-facing verbs.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:795 — mod tests — formatting and exit-code unit tests only; no e2e anywhere
- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:60 — cancel_exit_code maps CancelJobOutcome to the exit-code contract — contract never exercised against a live daemon
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:649 — the only 'Subscribe' matches in the test tree are fake-server trait stubs, not jobs tests

**Proposed fix**: Add one integration file using the shared harness: start a daemon, run a push with --detach, assert the job id line; run `blit jobs list`/`jobs watch --timeout-secs 10` and assert terminal state; run `blit jobs cancel <id>` on a fake/slow job and assert exit codes 0 (cancelled) and 1 (not found). Keep payloads tiny so it stays deterministic.

#### tests-per-test-cargo-build-subprocess — Every daemon-backed integration test shells out to `cargo build -p blit-daemon`, serializing the parallel test suite on the cargo lock

**Principle**: FAST | **Slice**: small

**Claim**: TestContext::new() and all four clone harnesses run a `cargo build` subprocess per test; with 69 TestContext::new() call sites plus the clones, a full `cargo test --workspace` run spawns ~75 nested cargo invocations that all contend for the target-directory flock.

**Mechanism**: common/mod.rs:111-134 spawns `cargo build -p blit-daemon --bin blit-daemon` inside TestContext::new(), which is called once per test (counted: 69 uses across 11 files, e.g. admin_verbs.rs 15, blit_utils.rs 22). Tests within one binary run on parallel threads, so concurrent nested cargo processes serialize on the build-dir lock; even no-op rebuilds cost several hundred ms each plus lock wait. There is no Once/OnceLock dedup anywhere in common/mod.rs (verified by rg). The build exists for a real reason (cargo test -p blit-cli does not build blit-daemon's bin; comment at remote_checksum_negotiation.rs:91-93 cites R16-F1 ordering), but per-test invocation is the wrong granularity — once per process is sufficient.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:111 — Command::new("cargo") inside TestContext::new(), executed once per test
- /home/michael/dev/Blit/crates/blit-cli/tests/admin_verbs.rs:1 — 15 TestContext::new() call sites in this file alone (rg -c), each paying a nested cargo build
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_checksum_negotiation.rs:91 — comment documents why the build exists (R16-F1: no cross-test ordering dependency) — the fix must preserve this property per process, not per test

**Proposed fix**: Wrap the build step in a `static BUILD: OnceLock<PathBuf>` (or std::sync::Once) in tests/common so each test binary builds the daemon at most once; clones disappear with the harness consolidation finding.

#### tests-readonly-module-enforcement-untested — All three read-only-module enforcement gates (push, purge, delegated pull) have zero test coverage

**Principle**: RELIABLE | **Slice**: small

**Claim**: The daemon refuses writes to read-only modules in three places, but no test in the workspace ever configures a module with read_only: true, so a regression that drops any of these permission_denied gates would pass the full validation suite.

**Mechanism**: Production gates: push control stream rejects read-only modules (push/control.rs:77-82), purge_inner rejects mirror deletion on read-only modules (core.rs:138-143), delegated_pull rejects read-only destinations (delegated_pull.rs:231). rg 'read_only: true' across all crates hits only blit-tui label-formatting tests; every integration harness hardcodes read_only: false (common/mod.rs:78, remote_remote.rs:133, remote_tcp_fallback.rs:72), and push/control.rs has no #[test]/mod tests at all (verified by rg). The harness struct cannot even express a read-only module, so the gap is structural, not accidental. This is the exact 'mirror deletion on readonly' blind spot: a daemon-side mirror push to a read-only module deletes destination files if the control.rs:77 check regresses, and nothing would catch it.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:77 — if config.read_only { permission_denied } — file contains zero tests
- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:138 — purge_inner read-only rejection — untested
- /home/michael/dev/Blit/crates/blit-daemon/src/service/delegated_pull.rs:231 — delegated pull read-only rejection — untested
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:78 — harness hardcodes read_only: false; no test config anywhere sets true

**Proposed fix**: Add a read_only knob to the consolidated harness and three small integration tests: push to read-only module fails with a plain read-only error and leaves the module untouched; mirror push (purge path) likewise; delegated pull to a read-only destination rejected. Assert on the error text to also lock in failure-message quality.

### LOW (26)

#### boundaries-private-default-port-literal-duplication (reviewer: medium) — RemoteEndpoint::DEFAULT_PORT is private, forcing consumers to re-state 9031 as magic literals

**Principle**: maintainability | **Slice**: small

**Claim**: The canonical daemon port constant is non-pub inside impl RemoteEndpoint, so blit-cli and blit-tui hardcode 9031 in behavior-bearing positions.

**Mechanism**: endpoint.rs:25 declares `const DEFAULT_PORT: u16 = 9031;` with no pub, inside the impl. blit-cli/scan.rs:63 branches on `if service.port == 9031` to decide whether the printed endpoint omits the port — display logic keyed on a literal that must track the private constant. blit-tui/daemons.rs:335 builds the local fallback via RemoteEndpoint::parse("127.0.0.1:9031"). A port-default change in blit-core compiles cleanly while scan output and the TUI's local-daemon row silently point at the wrong port — exactly the pub(crate)-primitive-that-should-be-shared pattern.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/endpoint.rs:25 — const DEFAULT_PORT: u16 = 9031; — not pub
- /home/michael/dev/Blit/crates/blit-cli/src/scan.rs:63 — if service.port == 9031 — display behavior keyed on the duplicated literal
- /home/michael/dev/Blit/crates/blit-tui/src/daemons.rs:335 — RemoteEndpoint::parse("127.0.0.1:9031") — string-literal re-statement of the default

**Proposed fix**: Make DEFAULT_PORT a pub associated const on RemoteEndpoint and replace the scan.rs comparison and the TUI fallback string with references to it.

#### boundaries-unified-source-depends-on-push-v1-helpers (reviewer: medium) — Unified FsTransferSource reaches backwards into the push-v1 helpers module it was meant to subsume

**Principle**: maintainability | **Slice**: small

**Claim**: The pipeline-unification abstraction (TransferSource) depends on crate::remote::push::client::helpers for its core scan and availability operations, inverting the intended layering.

**Mechanism**: FsTransferSource::scan (source.rs:69) imports and delegates to crate::remote::push::client::helpers::spawn_manifest_task, and check_availability (source.rs:86-88) delegates to helpers::filter_readable_headers. The unification layer (transfer/source.rs, per the PIPELINE_UNIFICATION plan it cites) was built to replace the push-v1 monolith, but its generic source is implemented in terms of push-private helpers — so the v1 module can never be deleted without breaking the abstraction that supersedes it, and any push-specific behavior change in those helpers silently changes every pipeline consumer (local mirror, daemon receive). This is the documented unfinished 'step 3b/4' seam frozen into a dependency edge.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/source.rs:69 — use crate::remote::push::client::helpers::spawn_manifest_task inside TransferSource::scan
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/source.rs:86 — check_availability delegates to crate::remote::push::client::helpers::filter_readable_headers

**Proposed fix**: Move spawn_manifest_task and filter_readable_headers from push/client/helpers.rs into remote/transfer (source.rs or a manifest module) and have the push client import them from there, restoring the dependency direction unification intended.

#### constants-accept-token-timeout-quadruplication (reviewer: medium) — The 30s/15s data-plane accept+token timeout policy is independently declared four times, twice in one file under aliased types

**Principle**: maintainability | **Slice**: small

**Claim**: One policy decision (how long the daemon waits for a data-plane peer to connect and authenticate) exists as four separate const pairs, guaranteeing the next adjustment misses at least one site.

**Mechanism**: pull_sync.rs:597-598 (PULL_SYNC_ACCEPT_TIMEOUT/PULL_SYNC_TOKEN_TIMEOUT, via `use std::time::Duration as StdDuration` inside the function), pull_sync.rs:719-720 (ACCEPT_TIMEOUT/TOKEN_TIMEOUT via the alias `StdDuration2` — same file, second declaration whose comment even points back at the first), pull.rs:698-699 (PULL_ACCEPT_TIMEOUT/PULL_TOKEN_TIMEOUT, comment: 'the same timeouts the push / pull-sync paths use'), and push/data_plane.rs:71/78 (DATA_PLANE_ACCEPT_TIMEOUT/DATA_PLANE_TOKEN_TIMEOUT). All four pairs are 30s/15s today only because R46-F7/R47-F5 manually landed the same fix in each file — the pasted comments are the audit trail of the propagation cost. These are function-local consts, so no site can reference another.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:597 — first 30s/15s pair (StdDuration alias)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:719 — second 30s/15s pair in the same file (StdDuration2 alias), comment references the first pair it cannot name
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:698 — third pair, comment: 'the same timeouts the push / pull-sync paths use (R46-F7)'
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:71 — fourth pair (module-level DATA_PLANE_ACCEPT_TIMEOUT/DATA_PLANE_TOKEN_TIMEOUT)

**Proposed fix**: Promote push/data_plane.rs's module-level DATA_PLANE_ACCEPT_TIMEOUT/DATA_PLANE_TOKEN_TIMEOUT to a shared daemon (or blit-core transfer) module and delete the three function-local re-declarations. Distinct from the queued slice-2 client-channel work, which covers gRPC client connect timeouts, not these daemon TCP accept/token waits.

#### constants-coupled-64mib-block-caps (reviewer: medium) — Resume-block wire cap is a parallel 64 MiB literal coupled to crate::copy::MAX_BLOCK_SIZE only by comment

**Principle**: RELIABLE | **Slice**: small

**Claim**: The receive-side wire cap for resume blocks (MAX_WIRE_BLOCK_BYTES) re-types the sender-side MAX_BLOCK_SIZE as an independent literal, so raising the block-size ceiling in one file makes receivers reject every oversized block from the other.

**Mechanism**: pipeline.rs:339 declares `const MAX_WIRE_BLOCK_BYTES: usize = 64 * 1024 * 1024` with the doc comment 'Aligns with crate::copy::MAX_BLOCK_SIZE' but no symbol reference; the receive loop bails on any DATA_PLANE_RECORD_BLOCK whose len exceeds it (pipeline.rs:256-261). The sender-side clamp is the separate constant MAX_BLOCK_SIZE=64 MiB (resume.rs:19), enforced at pull_sync.rs:84 (daemon clamps requested block_size) and pull.rs:1164 (client validation). Bumping MAX_BLOCK_SIZE without editing pipeline.rs makes a new-build sender emit blocks an unchanged (or older) receiver rejects mid-transfer with a bail. Precision note vs the design map: tuning.rs's 64 MiB max chunk coincides numerically but does NOT flow into block frames (file streams are length-prefixed by file_size, not chunk-framed), so the genuine coupling is resume.rs:19 ↔ pipeline.rs:339.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/pipeline.rs:339 — MAX_WIRE_BLOCK_BYTES = 64 MiB literal; comment 'Aligns with crate::copy::MAX_BLOCK_SIZE' instead of referencing it
- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/resume.rs:19 — pub const MAX_BLOCK_SIZE: usize = 64 * 1024 * 1024 — the sender-side twin
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:84 — daemon clamps client block_size to MAX_BLOCK_SIZE — sender bound declared in a different file than the receiver bound
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/pipeline.rs:256 — receive loop bails when len > MAX_WIRE_BLOCK_BYTES

**Proposed fix**: Replace the literal with `const MAX_WIRE_BLOCK_BYTES: usize = crate::copy::MAX_BLOCK_SIZE;` (one-line change; tar shard caps already demonstrate the pattern via tar_safety::MAX_TAR_SHARD_BYTES at pipeline.rs:335-337, the F8 fix).

#### constants-grpc-fallback-untar-frozen-4 (reviewer: medium) — gRPC-fallback untar concurrency frozen at MAX_PARALLEL_TAR_TASKS=4 regardless of core count

**Principle**: FAST | **Slice**: small

**Claim**: The daemon's gRPC-fallback push receiver caps concurrent tar-shard extraction at a hardcoded 4 spawn_blocking tasks, never consulting available_parallelism.

**Mechanism**: receive_fallback_data builds TarShardExecutor::new(MAX_PARALLEL_TAR_TASKS) (push/data_plane.rs:338) with the constant 4 declared at line 28; the semaphore at line 648 gates spawn_blocking untar jobs (line 695). Nothing keys this to core count, while local copy work uses rayon's core-count default (copy/parallel.rs:30) and CopyOptions defaults workers to num_cpus (orchestrator/options.rs:99) — the adaptation pattern exists in the same workspace. Note (correcting the design map's framing): this cap governs only the gRPC fallback receive; the TCP push path untars inline inside FsTransferSink with parallelism = N accepted streams (data_plane.rs:210-214 comment), so the impact is the fallback path's overlap of network receive with extraction on big-core servers, not a 32-stream bottleneck.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:28 — const MAX_PARALLEL_TAR_TASKS: usize = 4;
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:338 — TarShardExecutor::new(MAX_PARALLEL_TAR_TASKS) inside receive_fallback_data (gRPC fallback only)
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:648 — Semaphore::new(max_parallel) gating spawn_blocking untar tasks
- /home/michael/dev/Blit/crates/blit-core/src/orchestrator/options.rs:99 — workers: num_cpus::get().max(1) — the core-count-adaptive pattern already used elsewhere

**Proposed fix**: Derive the cap from std::thread::available_parallelism (e.g. clamp(2, cores/2, 8)) when constructing TarShardExecutor; the TAR_BUFFER_SIZE/TAR_BUFFER_POOL_SIZE pair can stay until the planned post-0.1.0 sink unification absorbs this executor.

#### constants-receive-chunk-1mib-asymmetry (reviewer: medium) — Receive side frozen at 1 MiB fresh allocations per file while send side uses 16-64 MiB pooled tuned buffers; the doc comment is false

**Principle**: FAST | **Slice**: medium

**Claim**: RECEIVE_CHUNK_SIZE pins every wire receive to 1 MiB double-buffers freshly allocated per file, outside the tuning system, and its comment incorrectly claims it matches the send side's pool size.

**Mechanism**: data_plane.rs:546 declares RECEIVE_CHUNK_SIZE=1 MiB with the comment 'Matches what the send side's buffer pool uses for chunk_bytes' — but the send side's pool buffer_size IS tuning.chunk_bytes (16-64 MiB; pool built at push/client/mod.rs:126, buffers acquired in send_file_double_buffered at data_plane.rs:285-286). FsTransferSink passes the constant at sink.rs:355/381/901, and receive_stream_double_buffered allocates two fresh `vec![0u8; cap]` per call — i.e. per streamed file (data_plane.rs:586-587), never pooled. So receive chunk size never participates in tuning (a 64 MiB-chunk send is consumed in 1 MiB slices) and many-file transfers pay 2 MiB of allocation churn per file. The constant's justification (vs tokio's 8 KiB) defends the floor, not the ceiling.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:546 — RECEIVE_CHUNK_SIZE = 1 MiB; comment claims it 'matches' the send-side pool — send side is 16-64 MiB tuned
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:586 — two fresh vec![0u8; cap] allocated per receive_stream_double_buffered call (per file), no pool
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/sink.rs:355 — FsTransferSink hardcodes RECEIVE_CHUNK_SIZE into the unified receive path used by both push-receive and pull-receive

**Proposed fix**: Thread a buffer size (and ideally a shared BufferPool) into FsTransferSink/receive_stream_double_buffered from the same tuning that sized the send side; at minimum fix the false comment so the next reader doesn't assume symmetry.

#### constants-three-size-taxonomies (reviewer: medium) — Three small/medium/large taxonomies with different boundaries, one of them dead-but-exported

**Principle**: maintainability | **Slice**: small

**Claim**: A file's size class depends on which layer asks: the planner says medium below 256 MiB, BufferSizer's scaling treats >100 MB as large, and a third classifier with 1 MB/100 MB boundaries is dead code still publicly exported.

**Mechanism**: transfer_plan.rs:77-108 bins files at <64 KiB / <1 MiB (small), <256 MiB (medium, decimal-mixed `256 * 1_048_576`), ≥256 MiB (large). buffer.rs:61-66 uses small_limit=10 MB / medium_limit=100 MB for buffer scaling. fs_enum.rs:498-516 categorize_files uses SMALL_LIMIT=1_048_576 / MEDIUM_LIMIT=104_857_600 — and rg across all crates/tests shows zero callers, yet it remains a pub fn (a trap and a third boundary set to keep 'consistent'). A 150 MB file is medium to the planner but takes BufferSizer's large-file scaling branch and categorize_files' large bucket; strategy boundaries shift between layers of one transfer, so performance behavior can't be reasoned about with one vocabulary.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/transfer_plan.rs:88 — medium boundary e.size < 256 * 1_048_576; small at <1_048_576 with 64 KiB sub-bin (line 78-83)
- /home/michael/dev/Blit/crates/blit-core/src/buffer.rs:61 — small_limit = 10*MB, medium_limit = 100*MB — second taxonomy
- /home/michael/dev/Blit/crates/blit-core/src/fs_enum.rs:499 — SMALL_LIMIT=1 MB / MEDIUM_LIMIT=100 MB — third taxonomy; categorize_files has zero callers workspace-wide

**Proposed fix**: Delete categorize_files (dead, zero callers). Then name the two surviving boundary sets as documented module constants and add a comment cross-referencing why the planner's task-strategy boundaries legitimately differ from buffer-sizing boundaries (or unify them if they don't).

#### deadcode-app-stub-module-and-perf-query (reviewer: medium) — blit-app ships an empty remote_remote_direct stub from an A.0 move that never happened, plus a zero-consumer perf::query/PerfReport API

**Principle**: maintainability | **Slice**: small

**Claim**: blit-app/src/transfers/remote_remote_direct.rs contains only a two-line comment claiming the code 'Moved from blit-cli ... in a later A.0 commit' that never landed (the live relay is still in blit-cli), and diagnostics::perf::query()/PerfReport were built for a TUI consumer that chose a different module and have zero callers.

**Mechanism**: (1) The entire blit-app stub file is the comment '//! Transfer shape: remote_remote_direct. Moved from blit-cli/src/transfers/remote_remote_direct.rs in a later A.0 commit.' — no items — yet it is published as `pub mod remote_remote_direct` (transfers/mod.rs:17) and referenced in dispatch.rs doc links (:17). The live implementation remains in blit-cli: transfers/mod.rs:46 imports run_remote_to_remote_direct and :641 calls run_remote_to_remote_direct_deferred. So a public module advertises a migration that did not happen; an agent following the doc link finds nothing. (2) perf.rs:54 query() bundling PerfReport (:18) was written for 'the TUI F4 diagnostics screen' (comment at :49-53), but rg for perf::query|PerfReport shows only the definitions: blit-cli's diagnostics verb uses the granular fns (blit-cli/src/diagnostics.rs:12-20 calls perf::set_enabled), and the TUI F4 pane consumes blit_app::profile::ProfileReport instead (blit-tui/src/screens/f4.rs:34). Also in this crate: the stale #[allow(dead_code)] on WatchSnapshot (admin/jobs.rs:165) suppresses lints on an enum that is consumed cross-crate (blit-cli/src/jobs.rs:3, :220-245).

**Evidence**:
- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote_remote_direct.rs:1 — entire file is a two-line 'Moved from ...' comment; exported at transfers/mod.rs:17; live impl still at blit-cli/src/transfers/mod.rs:46 and :641
- /home/michael/dev/Blit/crates/blit-app/src/diagnostics/perf.rs:54 — pub fn query(limit) -> Result<PerfReport> — rg shows zero consumers; CLI uses granular fns, TUI F4 uses blit_app::profile::ProfileReport (f4.rs:34)
- /home/michael/dev/Blit/crates/blit-app/src/admin/jobs.rs:165 — #[allow(dead_code)] on WatchSnapshot despite live consumers at blit-cli/src/jobs.rs:3/:220/:232/:245 — stale suppression

**Proposed fix**: Owner decision on direction: either finish the A.0 move (relocate blit-cli's remote_remote_direct body into the blit-app stub) or delete the stub and its dispatch.rs doc link. Independently: delete perf::query/PerfReport (TUI F4 will be reworked in Phase 6 anyway) and drop the WatchSnapshot allow.

#### deadcode-cli-interval-ms-flag (reviewer: medium) — blit jobs watch --interval-ms is parsed and documented but never read; its --help text describes behavior that no longer exists

**Principle**: RELIABLE | **Slice**: small

**Claim**: The --interval-ms flag's help text tells users it 'controls the GetState polling cadence', but since the Subscribe streaming RPC replaced polling the value is never read anywhere — users are actively misinformed by --help.

**Mechanism**: cli.rs:133-138 declares `#[arg(long, default_value_t = 1000)] pub interval_ms: u64` with doc-comment help text: 'A future milestone-C Subscribe RPC will replace polling ... until then this flag controls the GetState polling cadence.' That future already arrived: jobs.rs:171-173 states 'args.interval_ms is preserved on the CLI for backward compatibility but has no effect under the streaming model — Subscribe pushes; no polling cadence to configure.' rg for interval_ms across crates/blit-cli/src confirms those are the only two occurrences — no code reads the value. So clap accepts and silently ignores it while --help promises an effect: a plain-failure/honest-text violation. Supersessor: the Subscribe stream (jobs.rs watch loop) with --timeout-secs as the only live knob.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/src/cli.rs:138 — pub interval_ms: u64 with help text claiming it 'controls the GetState polling cadence' (doc comment at :133-137)
- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:171 — 'preserved on the CLI for backward compatibility but has no effect under the streaming model' — the only other interval_ms occurrence in the crate

**Proposed fix**: Either remove the flag (owner call on CLI backward compatibility) or keep it hidden/deprecated with corrected help text; per the repo's docs rule, the same slice must update --help, the manpage, and README together.

#### deadcode-cli-unused-deps (reviewer: medium) — blit-cli declares walkdir, rayon, and sysinfo it never uses, and tonic as a regular dependency used only by one test

**Principle**: FAST | **Slice**: small

**Claim**: Three blit-utils-era dependencies (walkdir 2.5, rayon 1.12, sysinfo 0.38) remain in blit-cli's [dependencies] with zero uses in src/ or tests/, and tonic 0.14 sits in [dependencies] though only tests/remote_remote.rs uses it.

**Mechanism**: Cargo.toml [dependencies] lists tonic, walkdir, rayon, sysinfo. rg for walkdir|rayon|sysinfo across crates/blit-cli/src and tests hits only the English word 'walkdir' in a comment (tests/local_move_semantics.rs:137). rg for 'use tonic|tonic::' over src/ returns nothing; the only tonic consumer is the fake daemon in tests/remote_remote.rs (:536, :574, :598). The admin logic that needed walkdir/rayon/sysinfo moved to blit-app during the blit-utils absorption (blit-app declares and uses them itself). Effect: needless compile/link work on every CLI build (sysinfo in particular) and a Cargo.toml that falsely advertises the crate does transport and filesystem-walking work. tonic belongs in [dev-dependencies].

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/Cargo.toml:18 — tonic = "0.14" in [dependencies]; walkdir/rayon/sysinfo at lines 27, 28, 31 — rg over src/ and tests/ finds zero code uses of the latter three
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:536 — use tonic::transport::Server — the only tonic consumer in the crate; justifies dev-dependency, not dependency

**Proposed fix**: Remove walkdir, rayon, sysinfo from blit-cli's Cargo.toml and move tonic to [dev-dependencies]; verify with the standard validation suite ([state: skip]-class mechanical change candidate, but Cargo.toml touch under crates/** still trips the docs gate — note in commit).

#### deadcode-core-errors-contradictory-classifier (reviewer: high) — blit-core errors.rs is a dead retry classifier that contradicts the live one and stays publicly exported

**Principle**: RELIABLE | **Slice**: small

**Claim**: The 154-line blit_core::errors module (TransferError/ErrorCategory/categorize_io_error/TransferResult) has zero consumers anywhere in the workspace, yet is pub-exported and classifies ConnectionRefused/UnexpectedEof/NotConnected as Fatal while the live classifier in blit-app marks all three Retryable.

**Mechanism**: rg for TransferError|ErrorCategory|categorize_io_error|TransferResult across crates/ and tests/ shows every hit outside errors.rs is the protobuf-generated TransferError message (e.g. blit-cli/src/jobs.rs:112 uses blit_core::generated::TransferError). The module is reachable only via lib.rs:9 `pub mod errors;`. Its categorize_io_error explicitly defaults NotConnected/ConnectionRefused/UnexpectedEof to Fatal ('could go either way - default to fatal'), while the production policy in blit-app/src/transfers/retry.rs:35-46 (is_retryable_io_kind) lists ConnectionRefused, UnexpectedEof, and NotConnected as retryable. Any future contributor who discovers the doc-commented core module and wires it up silently flips retry semantics for three error kinds. Supersessor: blit_app::transfers::retry (live, wired to --retry/--wait).

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/errors.rs:90 — categorize_io_error: NotConnected/ConnectionRefused/UnexpectedEof => ErrorCategory::Fatal in the 'could go either way - default to fatal' arm (read lines 85-125 this session)
- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:35 — is_retryable_io_kind matches ConnectionRefused | UnexpectedEof | NotConnected as retryable — direct contradiction
- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — pub mod errors; — only non-test reference to the module in the workspace (rg verified)

**Proposed fix**: Delete crates/blit-core/src/errors.rs and the lib.rs:9 export outright (zero callers, no wire impact). If a shared classifier is wanted later, it should be the slice-2 tonic-Status-aware extension of blit-app's live table, not a revival of this one.

#### deadcode-core-warmup-machinery (reviewer: medium) — auto_tune warmup machinery is dead: analyze_warmup_result has zero callers and every warmup branch of determine_tuning is unreachable

**Principle**: SIMPLE | **Slice**: small

**Claim**: The module that promises 'warmup probes and heuristics' performs no runtime adaptation: analyze_warmup_result is never called, determine_tuning's only production caller always passes warmup_result=None, and the few values its None-branch does produce are immediately overwritten by the caller's static table.

**Mechanism**: rg for analyze_warmup_result hits only its definition (auto_tune/mod.rs:26) plus nothing else. rg for determine_tuning shows the sole production caller is remote/tuning.rs:13, which passes None — so warmup_gbps is always None, making the bandwidth-keyed initial_streams (mod.rs:45-55) and tcp_buffer_size/prefetch ladders (mod.rs:57-67) unreachable outside tests (mod.rs:85 passes Some in a test). determine_remote_tuning then overwrites initial_streams/max_streams from its own byte ladder (tuning.rs:27-28) and tcp_buffer_size/prefetch_count (tuning.rs:30-36), so even the None-branch outputs (initial_streams=2, max_streams=8, buffers None) never survive. All remote sizing is the static byte-keyed table in tuning.rs; the module doc (mod.rs:3 'Provides warmup probes') advertises adaptivity the system does not have — a SIMPLE-principle lie in the code. Cross-ref: the queued slice-2 'adaptive windows' transport work is about TCP/HTTP2 windows on channels, not this warmup path; no overlap.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/auto_tune/mod.rs:26 — pub fn analyze_warmup_result — definition is the only rg hit in the workspace
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:13 — let mut tuning = determine_tuning(default_chunk_bytes, None); — sole production caller, warmup always None
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:27 — tuning.initial_streams / max_streams overwritten at :27-28; tcp_buffer_size/prefetch_count overwritten at :30-36 — the None-branch outputs never survive

**Proposed fix**: Delete analyze_warmup_result and the warmup_result parameter/branches of determine_tuning (collapsing it into determine_remote_tuning's static table), or rename/document the module honestly as a static table. Reviving real warmup-probe adaptation would be a separate owner-approved plan; the dead branches as written would be superseded by it anyway.

#### deadcode-daemon-allow-deadcode-masking (reviewer: medium) — Daemon #[allow(dead_code)] annotations hide genuinely dead items (ModuleOptOut, resolve_contained_wire, acquire_buffer, ActiveJobs::cancel/as_str) behind false comments

**Principle**: maintainability | **Slice**: small

**Claim**: Five daemon items are production-dead but invisible to the clippy -D warnings gate because of #[allow(dead_code)], and two of them carry doc comments asserting liveness that is false.

**Mechanism**: Re-derived each: (1) GateDenial::ModuleOptOut (delegation_gate.rs:98) — doc says 'Populated from the handler when ModuleConfig::delegation_allowed is false', but rg shows it is never constructed anywhere; the handler does the opt-out check inline at delegated_pull.rs:221-229 with its own message string, so the variant and its Display arm (:127) are dead divergent failure text. (2) resolve_contained_wire (util.rs:122) — self-described 'Reserved for future call sites', zero callers. (3) TarShardExecutor::acquire_buffer (push/data_plane.rs:658) — comment claims 'Currently only used by the gRPC fallback path' but rg for acquire_buffer in the workspace returns only the definition; the pool's acquire side pools nothing. (4) ActiveJobs::cancel (active_jobs.rs:460, under allow at :459) — only test callers (:1701 etc.); production uses cancel_authorized (:484). (5) ActiveJobKind::as_str (:134) — test-only callers. Because each carries allow(dead_code), the repo's mandatory `cargo clippy --workspace --all-targets -- -D warnings` gate (AGENTS.md §5) can never flag them, and the false comments will misdirect the next reader.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/delegation_gate.rs:98 — ModuleOptOut variant — never constructed (rg: only definition :98 and Display arm :127); live check is inline at delegated_pull.rs:221-229 with different wording
- /home/michael/dev/Blit/crates/blit-daemon/src/service/util.rs:122 — resolve_contained_wire — 'Reserved for future call sites', #[allow(dead_code)], zero callers
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:658 — acquire_buffer — comment 'Currently only used by the gRPC fallback path' is false; rg finds no call site
- /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:460 — ActiveJobs::cancel under #[allow(dead_code)] — callers are tests only (:1701, :1823, :1840...); cancel_authorized (:484) is the live API

**Proposed fix**: Sweep: delete ModuleOptOut+its Display arm (or actually construct it from the handler so denial text has one owner), delete resolve_contained_wire and acquire_buffer, move cancel/as_str under #[cfg(test)] or delete, and drop the stale allow(dead_code) on now-live items (e.g. spawn_progress_ticker, WatchSnapshot in blit-app) so the lint gate regains authority.

#### drift-manpage-verbose-claims-heartbeat-messages (reviewer: medium) — Manpage says --verbose emits 'planner heartbeat messages' — no heartbeat exists anywhere in the workspace

**Principle**: maintainability | **Slice**: small

**Claim**: docs/cli/blit.1.md documents --verbose as emitting planner heartbeat messages, but no code path can produce them; verbose actually emits journal/fast-path and predictor diagnostics.

**Mechanism**: rg -i heartbeat over crates/ matches only the daemon's DaemonHeartbeat event-type comments (service/core.rs:280) — nothing in the local planner/orchestrator, and no heartbeat string is ever printed. What options.verbose actually gates in orchestrator.rs is journal fast-path messaging (e.g. 'Filesystem journal fast-path: source/destination unchanged; skipping planner.' at :254-256) and predictor updates (update_predictor calls at :284/:341/:378). So half the manpage sentence describes the never-built Phase 2 heartbeat scheduler (same ghost as WORKFLOW_PHASE_2.md rows 2.1.2-2.1.3); a user running --verbose and seeing no heartbeats cannot distinguish a doc lie from a broken install.

**Evidence**:
- /home/michael/dev/Blit/docs/cli/blit.1.md:116 — '--verbose  Emit planner heartbeat messages and fast-path decisions to stderr.' — heartbeat half is a ghost
- /home/michael/dev/Blit/crates/blit-core/src/orchestrator/orchestrator.rs:254 — what verbose actually emits: journal fast-path / predictor diagnostics; no heartbeat emission exists in any crate

**Proposed fix**: Rewrite the blit.1.md --verbose entry to describe the real output (fast-path/journal decisions, predictor predicted-vs-actual lines); cross-check the same sentence isn't repeated in README or --help text in the same slice.

#### drift-pull-sync-reuse-comment-over-redefined-constants (reviewer: medium) — pull_sync.rs resume-path comment cites PULL_SYNC_ACCEPT_TIMEOUT/PULL_SYNC_TOKEN_TIMEOUT as 'defined above' while redefining new constants two lines later

**Principle**: maintainability | **Slice**: small

**Claim**: A comment in the pull-sync resume path implies the accept/token timeout constants from the streaming path are being reused, but the code immediately declares a second, independent ACCEPT_TIMEOUT/TOKEN_TIMEOUT pair in the same file.

**Mechanism**: pull_sync.rs:714-717 reads 'Same rationale as the streaming pull-sync path (PULL_SYNC_ACCEPT_TIMEOUT / PULL_SYNC_TOKEN_TIMEOUT defined above)', then lines 719-720 declare `const ACCEPT_TIMEOUT: StdDuration2 = StdDuration2::from_secs(30); const TOKEN_TIMEOUT: ... from_secs(15);`. The named constants at :597-598 are function-local consts inside a different function, so they cannot be referenced here — the comment papers over a duplication the language forced. Anyone retuning 'the' pull-sync timeout per the comment will change :597-598 and silently leave the resume path at the old values (the file already has two naming schemes for one policy; the daemon repo-wide has four — push/data_plane.rs:71/78 and pull.rs:698-699 are the others, per code read this session for the first three).

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:714 — comment: 'Same rationale as the streaming pull-sync path (PULL_SYNC_ACCEPT_TIMEOUT / PULL_SYNC_TOKEN_TIMEOUT defined above)'
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:719 — const ACCEPT_TIMEOUT / TOKEN_TIMEOUT redefined (30s/15s) instead of reusing
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:597 — first pair: const PULL_SYNC_ACCEPT_TIMEOUT / PULL_SYNC_TOKEN_TIMEOUT, function-local in a different function

**Proposed fix**: Hoist one module-level (or daemon-shared) ACCEPT/TOKEN timeout pair and reference it from both pull_sync paths so the comment becomes true; at minimum rewrite the comment to say the values are intentionally duplicated and must be retuned in lockstep with :597-598.

#### duplication-accept-token-timeout-quadruple (reviewer: medium) — The 30s/15s data-plane accept/token timeout pair is declared four times under four names, twice in one file under a comment claiming reuse

**Principle**: maintainability | **Slice**: small

**Claim**: One policy decision (accept 30s, token-read 15s on daemon data-plane listeners) exists as four independent const declarations, and pull_sync.rs's second copy sits under a comment saying the constants are 'defined above' while actually redefining them.

**Mechanism**: push/data_plane.rs:71/78 (DATA_PLANE_ACCEPT_TIMEOUT/DATA_PLANE_TOKEN_TIMEOUT), pull.rs:698-699 (PULL_*), pull_sync.rs:597-598 (PULL_SYNC_*), and pull_sync.rs:719-720 (ACCEPT_TIMEOUT/TOKEN_TIMEOUT) all declare Duration::from_secs(30)/from_secs(15) locally. The comment at pull_sync.rs:714-717 reads 'Same rationale as the streaming pull-sync path (PULL_SYNC_ACCEPT_TIMEOUT / PULL_SYNC_TOKEN_TIMEOUT defined above)' but the code below it declares a fresh anonymous pair (with its own `use std::time::Duration as StdDuration2` alias) — documentation has already drifted from code within a single file. The surrounding accept+token-verify blocks (pull_sync.rs:600-633 vs 721-755 vs pull.rs:702-734) are themselves copy-pasted loops, so any retune or error-text fix must land four times.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:719 — second const pair in the same file, under the 'defined above' comment at 715-716
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:597 — first pair: PULL_SYNC_ACCEPT_TIMEOUT/PULL_SYNC_TOKEN_TIMEOUT
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:698 — third pair: PULL_ACCEPT_TIMEOUT/PULL_TOKEN_TIMEOUT
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:71 — fourth pair: DATA_PLANE_ACCEPT_TIMEOUT (=30s), TOKEN at 78 (=15s)

**Proposed fix**: One shared accept_data_plane_socket(listener, token) helper in the daemon service module (or blit-core) owning the two constants and the accept+token-verify loop; the four sites collapse to calls.

#### duplication-fs-enum-cfg-twins (reviewer: medium) — enumerate_directory_filtered and enumerate_symlinks exist as byte-identical cfg(windows)/cfg(not(windows)) pairs

**Principle**: SIMPLE | **Slice**: small

**Claim**: fs_enum.rs carries two functions duplicated verbatim under opposite cfg gates — a vestige of a removed Windows-native enumerator — so the platform split is pure dead weight inviting one-sided edits.

**Mechanism**: fs_enum.rs:423-459 (cfg(not(windows))) and fs_enum.rs:461-495 (cfg(windows)) define enumerate_directory_filtered and enumerate_symlinks with byte-identical bodies (both build FileEnumerator::new(filter.clone_without_cache()) and run the same filter_map over EnumeratedEntry). The comment at line 420 ('All Windows-specific code removed.') records that the platform-specific implementation is gone, but the cfg split survived it. Because the compiler only ever checks one arm per platform, an edit to the non-Windows copy (the one CI compiles) silently leaves the Windows copy stale — the exact drift this codebase's Windows-parity rule (AGENTS.md §5) cannot catch since the ps1 test script is manual.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/fs_enum.rs:420 — comment: 'All Windows-specific code removed.' directly above the surviving split
- /home/michael/dev/Blit/crates/blit-core/src/fs_enum.rs:423 — cfg(not(windows)) enumerate_directory_filtered — body identical to cfg(windows) copy at 461-472
- /home/michael/dev/Blit/crates/blit-core/src/fs_enum.rs:475 — cfg(windows) enumerate_symlinks — identical to 438-459

**Proposed fix**: Delete the cfg gates and keep one copy of each function; behavior is provably identical since the bodies are.

#### duplication-retry-classifier-dead-twin (reviewer: medium) — Dead, publicly exported retry classifier in blit-core contradicts the live one in blit-app on three error kinds

**Principle**: RELIABLE | **Slice**: small

**Claim**: blit-core/src/errors.rs ships a doc-commented, publicly exported categorize_io_error with zero consumers that classifies ConnectionRefused/UnexpectedEof/NotConnected as Fatal, while the live classifier blit-app/transfers/retry.rs classifies the same three kinds as retryable — a discoverable trap for the next contributor.

**Mechanism**: errors.rs:90-118 marks TimedOut/Interrupted/ConnectionReset/ConnectionAborted/BrokenPipe/WouldBlock retryable and explicitly sends UnexpectedEof/NotConnected/ConnectionRefused to Fatal (lines 108-113, 'default to fatal to avoid infinite loops'). retry.rs:35-46 (is_retryable_io_kind) includes ConnectionRefused, UnexpectedEof, and NotConnected as retryable and omits Interrupted/WouldBlock. rg for categorize_io_error/crate::errors/blit_core::errors across all crates returns zero hits outside errors.rs itself, yet lib.rs:9 exports `pub mod errors`. Anyone importing the visible blit-core classifier (the 'obvious' home for retry policy) gets the opposite decision from production on exactly the transient-connection kinds --retry exists for.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/errors.rs:108 — WriteZero|UnexpectedEof|...|NotConnected|ConnectionRefused => ErrorCategory::Fatal
- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:41 — ConnectionRefused, UnexpectedEof, NotConnected listed retryable in is_retryable_io_kind
- /home/michael/dev/Blit/crates/blit-core/src/lib.rs:9 — pub mod errors — dead module publicly exported; rg found zero external consumers

**Proposed fix**: Delete errors.rs (or reduce it to whatever the slice-2 error-chain work actually needs) so exactly one io::ErrorKind classification table exists; if blit-core must own the table for layering reasons, move retry.rs's table down and re-export it, never both.

#### duplication-win-extended-prefix-mount-match (reviewer: medium) — Windows \\?\ prefix-strip plus longest-mount-match duplicated in blit-app and blit-daemon with different tie-break rules, both missing the UNC form

**Principle**: RELIABLE | **Slice**: small

**Claim**: The 'strip \\?\ from canonicalize output, then find the longest matching sysinfo mount point' logic is written twice — diagnostics dump.rs and daemon admin.rs — with identical prefix handling (both miss \\?\UNC\) but different longest-match selection, so df/du and diagnostics can attribute the same path to different disks.

**Mechanism**: dump.rs:167-175 (strip_windows_extended_prefix) and admin.rs:620-628 (inline block) both strip only the literal r"\\?\" prefix; fs::canonicalize on a UNC destination yields \\?\UNC\server\share which neither strips, so mount matching silently fails for UNC paths in both places. The surrounding selection loops differ: dump.rs:151-159 picks the longest mount by OsStr byte length with strict '>', admin.rs:632-643 picks by component count with '>='. For nested mount points (e.g. C:\ vs C:\mnt\data) byte-length and component-count can pick different winners on ties or multi-component mounts, so blit's diagnostics and the daemon's FilesystemStats RPC can disagree about free space for the same path — and any future fix (e.g. adding UNC handling) must be found and landed twice.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-app/src/diagnostics/dump.rs:170 — strip_prefix(r"\\?\") only; longest match by os_str().len() with '>' at 154-158
- /home/michael/dev/Blit/crates/blit-daemon/src/service/admin.rs:623 — identical strip inline; longest match by components().count() with '>=' at 636-641

**Proposed fix**: One helper in blit-core (strip extended prefix incl. \\?\UNC\, plus the longest-mount-match selection) consumed by both diagnostics and the daemon admin verbs.

#### duplication-windows-copyfile-twins (reviewer: medium) — Two CopyFileExW wrappers (one dead and strictly worse) plus two windows_copyfile call sites with divergent metadata/thread-local handling

**Principle**: RELIABLE | **Slice**: small

**Claim**: The Windows copy entry point is duplicated at two levels: fs_capability/windows.rs has a second, inferior CopyFileExW wrapper reachable only through a dead trait method, and the two live call sites of windows_copyfile handle the block-clone thread-local flag inconsistently.

**Mechanism**: copy/windows.rs:331-369 (windows_copyfile) attempts ReFS block clone first (line 340), applies the adaptive COPY_FILE_NO_BUFFERING flag (349-351), and falls back to fs::copy on failure (367). fs_capability/windows.rs:230-251 (try_copyfileex) is a bare CopyFileExW with COPYFILE_FLAGS(0) — no clone, no buffering heuristic, no fallback — and rg shows its only route, FilesystemCapability::fast_copy, has zero callers outside the trait impls, so the 'capability abstraction' silently offers a worse copy. At the call-site level, file_copy/mod.rs:43-53 consumes windows::take_last_block_clone_success() and skips preserve_metadata when the clone preserved it; file_copy/chunked.rs:23-29 calls windows_copyfile then preserve_metadata unconditionally and never consumes the thread-local flag, leaving it set for the next caller on that thread.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/fs_capability/windows.rs:230 — try_copyfileex: COPYFILE_FLAGS(0), no clone/no-buffering/fallback; fast_copy has zero callers (rg verified)
- /home/michael/dev/Blit/crates/blit-core/src/copy/windows.rs:340 — windows_copyfile: block clone attempt + adaptive NO_BUFFERING (349) + fs::copy fallback (367)
- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/mod.rs:45 — consumes take_last_block_clone_success() to skip redundant preserve_metadata
- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/chunked.rs:25 — second call site: preserve_metadata unconditional, thread-local flag never consumed

**Proposed fix**: Delete fast_copy from the FilesystemCapability trait (or implement it as a call into windows_copyfile), and wrap windows_copyfile + clone-flag + preserve_metadata into one helper so both call sites share the metadata decision.

#### errors-daemon-status-internal-collapse (reviewer: medium) — Daemon collapses 116 of ~199 Status constructions to Status::internal, erasing gRPC code semantics for io errors

**Principle**: RELIABLE | **Slice**: medium

**Claim**: More than half of all daemon error returns use Status::internal regardless of cause, so a missing source file or permission failure on pull crosses the wire as code Internal, making daemon-originated causes undifferentiable to any code()-branching client.

**Mechanism**: Tallying Status:: constructors across crates/blit-daemon/src gives 116 internal vs 40 invalid_argument, 13 permission_denied, 7 not_found, etc. (~199 total). In the pull handler, plain file io errors — open/stat/read on the requested path — are wrapped as Status::internal(format!("open {}: {}", ...)) at pull.rs:516, 519, 528, 546, 579, 601, 608, so io::ErrorKind::NotFound/PermissionDenied become Internal on the wire. Client code that branches on status.code() to choose wording or remediation (blit-app/src/transfers/remote.rs:709-751 for Unimplemented/Unavailable; blit-app/src/admin/jobs.rs:83-96 for NotFound/FailedPrecondition) can therefore only ever distinguish transport-level or explicitly-coded conditions; every daemon-side io failure lands in the generic fallback arm, and any future code-based retry classification (queued slice-2) is structurally blind to daemon causes.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:516 — Status::internal(format!("open {}: {}", abs_path.display(), err)) — io NotFound/PermissionDenied collapsed to Internal; same at 546, 579, 601
- /home/michael/dev/Blit/crates/blit-app/src/admin/jobs.rs:83 — client branches on Code::NotFound / Code::FailedPrecondition — pattern that daemon io-error collapse defeats
- /home/michael/dev/Blit/crates/blit-app/src/transfers/remote.rs:709 — code-conditional wording (Unimplemented/Unavailable) — only transport-level codes are ever distinguishable

**Proposed fix**: Add one daemon-boundary helper io_to_status(context, io::Error) mapping NotFound→not_found, PermissionDenied→permission_denied, else internal, and convert the pull/push/pull_sync handler io sites to it.

#### errors-logger-trait-permanently-noop (reviewer: medium) — The Logger trait error channel is permanently NoopLogger in production and TextLogger has zero consumers

**Principle**: maintainability | **Slice**: medium

**Claim**: blit-core's second error-reporting layer — the Logger trait with error()/copy_done() callbacks and a file-writing TextLogger — is dead: every production instantiation is NoopLogger, and TextLogger is constructed nowhere outside its own file.

**Mechanism**: rg over the workspace shows the only non-test instantiations of the Logger trait are NoopLogger at local_worker.rs:30 and sink.rs:507 (orchestrator.rs:1146/1262 are in test modules), and TextLogger's only mentions are its own definition (logger.rs:18-51). The logger.error(...) calls in the copy engine (file_copy/mod.rs:220, chunked.rs:64) therefore never report anywhere; the errors do also propagate via Err(e), so this is redundancy rather than loss — but it is a third coexisting failure-text mechanism (alongside eyre and the backend-less log facade) that threads a &dyn Logger parameter through copy_file/parallel/chunked signatures while contributing nothing, and it invites a future caller to rely on TextLogger for an rsync-style --log feature that is not actually wired to anything.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/local_worker.rs:30 — let logger = NoopLogger; — production path hardcodes the no-op
- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/sink.rs:507 — second production NoopLogger hardcode
- /home/michael/dev/Blit/crates/blit-core/src/logger.rs:18 — TextLogger defined here; zero constructors anywhere else in the workspace
- /home/michael/dev/Blit/crates/blit-core/src/copy/file_copy/mod.rs:220 — logger.error("copy", src, …) — always no-op; the error also propagates via Err(e), confirming redundancy

**Proposed fix**: Either delete the Logger trait/TextLogger and the threaded parameters, or wire TextLogger to a real CLI --log flag — decide once; deleting is the SIMPLE-aligned default given errors already propagate.

#### errors-stderr-prefix-babel (reviewer: medium) — Nine different stderr prefixes (plus unprefixed lines) across the binaries — no greppable failure convention

**Principle**: RELIABLE | **Slice**: medium

**Claim**: Failure and warning lines on stderr use at least nine distinct prefixes ('blit:', '[push]', '[pull]', '[pull-data-plane]', '[blitd]', '[warn]'/'[info]', '[failed]'/'[stream-error]'/'[stream-end]', 'blit-prometheus-bridge:', and bare unprefixed text), so neither a user nor a wrapper script can identify blit errors by any single pattern.

**Mechanism**: Re-derived by rg over eprintln! sites: retry announces failures as 'blit: transfer failed…' (retry.rs:70); push-side skips print '[push] skipping…' (helpers.rs:184); pull prints '[pull] …' (pull.rs:428); the daemon mixes '[warn]'/'[info]' (main.rs:35, 91) with '[blitd]' (active_jobs.rs:832) and '[pull-data-plane]' (service/pull.rs:739); blit jobs watch emits '[failed]', '[stream-error]', '[stream-end]' (jobs.rs:336, 351, 360); the bridge uses its crate name (server.rs:224); and other failure lines have no prefix at all ('Cannot cancel transfer…' jobs.rs:601, 'logger write error' logger.rs:38). One binary (blitd) alone uses four conventions. This is the visible face of the smeared error ownership and directly contradicts 'failures are plain'.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-app/src/transfers/retry.rs:70 — 'blit: transfer failed, retrying…'
- /home/michael/dev/Blit/crates/blit-daemon/src/main.rs:35 — '[warn] {warning}' — daemon also uses [info] (91), [blitd] (active_jobs.rs:832), [pull-data-plane] (service/pull.rs:739)
- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:336 — '[failed] transfer …' plus [stream-error] (351) and [stream-end] (360) in the same command
- /home/michael/dev/Blit/crates/blit-prometheus-bridge/src/server.rs:224 — 'blit-prometheus-bridge: scrape failed: {err:#}' — fourth convention family

**Proposed fix**: Pick one convention (suggest '<binary>: ' for human lines, matching the bridge and retry.rs styles) and mechanically converge the eprintln sites; combine with the log-backend slice so log::warn output follows the same convention.

#### tests-fake-server-config-skew (reviewer: medium) — All in-process tonic test servers omit the production HTTP/2 keepalive config, so wire tests exercise a non-production server

**Principle**: RELIABLE | **Slice**: small

**Claim**: The three in-process gRPC servers used by tests are bare Server::builder() while the production daemon sets http2_keepalive_interval(30s)/timeout(20s), so the only wire-level client test harness can never catch a keepalive-interaction regression — exactly the axis the queued slice-2 transport work (client keepalive, adaptive windows, decode limits) is about to change.

**Mechanism**: Production: blit-daemon/src/main.rs:136-139 builds the server with http2_keepalive_interval(Some(30s)) and http2_keepalive_timeout(Some(20s)) per the 2026-05-23 owner decision quoted in the comment block above it. Tests: remote_remote.rs:540 and :578 (fake unimplemented/rejecting servers) and blit-core/tests/pull_sync_with_spec_wire.rs:201 (the SpyServer the PullSync spec wire-contract test runs the real client against) all call bare Server::builder() with no keepalive. When slice-2 lands client-side keepalive/window/decode settings, the one test that validates client wire behavior (pull_sync_with_spec_wire) will validate it against a server shaped differently from every production daemon, and a server/client keepalive mismatch (e.g. server GOAWAY on too-frequent client PINGs) would be invisible to cargo test. Note: this skew matters only for the in-process fakes — the spawned-daemon harnesses run the real production server config.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-daemon/src/main.rs:137 — production keepalive: http2_keepalive_interval(30s) / http2_keepalive_timeout(20s)
- /home/michael/dev/Blit/crates/blit-core/tests/pull_sync_with_spec_wire.rs:201 — bare Server::builder() — the wire-contract spy server lacks production keepalive
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_remote.rs:540 — bare Server::builder() for the fake unimplemented destination (second copy at :578)

**Proposed fix**: Extract a single 'production-shaped server builder' (a small pub fn in blit-core or a daemon-exported helper returning the configured Server::builder()) and use it in main.rs and all three test servers, so test/prod server config cannot drift. Do this as part of, or immediately before, the queued slice-2 transport slice.

#### tests-harness-stderr-blackhole (reviewer: medium) — TestContext pipes daemon stderr 'for debugging' but never reads it: opaque startup failures now, write-blocking hazard later

**Principle**: RELIABLE | **Slice**: small

**Claim**: The shared harness captures the daemon's stderr into a pipe that no code ever drains or prints, so when the daemon fails to start the test panics with only 'daemon failed to listen on {port}' while the real cause (config parse error, bind failure) sits unread in the pipe; the same unread pipe is a latent deadlock once a chatty daemon fills the 64 KiB pipe buffer.

**Mechanism**: common/mod.rs:145 sets .stderr(Stdio::piped()) with the comment 'Capture stderr for debugging', but no call site reads daemon.child stderr — not the readiness failure path (assert!(ready, ...) at :158 discards it) and not ChildGuard::drop (:205-211, kill+wait only). Result (a): any daemon startup failure surfaces as a bare timeout assertion with zero diagnostic text, the exact 'failures are plain' violation in a place built to debug failures. Result (b): the daemon writes per-connection/per-stream lines to stderr unconditionally (eprintln at push/data_plane.rs:125, :152, :191; pull.rs:661,:717; plus startup module lines main.rs:43-53); a test daemon that serves enough operations to exceed the OS pipe buffer blocks forever inside eprintln!, which then manifests as an unrelated run_with_timeout panic in the CLI under test. The four clone harnesses 'fixed' this by using Stdio::null() (e.g. remote_checksum_negotiation.rs:180), losing diagnostics entirely — both policies are wrong in different directions.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:145 — .stderr(Stdio::piped()) // Capture stderr for debugging — never read anywhere
- /home/michael/dev/Blit/crates/blit-cli/tests/common/mod.rs:158 — assert!(ready, "daemon failed to listen on {port}") — discards the captured stderr that explains why
- /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:125 — unconditional per-connection eprintln — the writer side of the unread pipe

**Proposed fix**: In the harness, spawn a thread that drains daemon stderr into an Arc<Mutex<String>>; include the buffer in the readiness-failure panic message and optionally dump it from ChildGuard::drop when thread::panicking(). Apply once in the consolidated harness.

#### tests-tuning-tiers-never-exercised (reviewer: medium) — determine_remote_tuning's size-tier table has no unit tests and every integration payload (max 3 MiB) stays in the smallest tier

**Principle**: FAST | **Slice**: small

**Claim**: The byte-count tier table that decides chunk size, stream counts, TCP buffer size, and prefetch for every remote transfer is pinned by zero tests: tuning.rs has no #[test], and the largest payload in any integration test is 3 MiB — far below the first 128 MiB tier boundary — so every tier above the floor (and the 64 KiB chunk-floor interplay) is dead air in the validation suite.

**Mechanism**: crates/blit-core/src/remote/tuning.rs:4-38 defines the tiers (chunk 16/32/64 MiB at 512 MiB / 8 GiB; streams 4/8 up to 24/32 at 32 GiB; tcp_buffer_size and prefetch_count set only at >=512 MiB) and contains no test module (rg '#[test]|mod tests' over the file: zero hits). Integration payload survey: largest files created by any compiled test are 3 MiB (remote_resume.rs:19) and 2 MiB (remote_remote.rs:186, remote_regression.rs:170) — every integration run therefore takes the smallest branch of every tier expression, and a transposed boundary or swapped tier value (a one-character FAST regression affecting all large transfers) would pass cargo test --workspace, clippy, and fmt. The daemon-side pull_stream_count table (service/pull.rs:915-925) is equally unpinned. Cheapest closure is unit tests, not giant fixtures: the functions are pure u64 -> params.

**Evidence**:
- /home/michael/dev/Blit/crates/blit-core/src/remote/tuning.rs:4 — determine_remote_tuning tier table — no #[test] anywhere in the file
- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:915 — pull_stream_count second tier table, also unpinned by tests
- /home/michael/dev/Blit/crates/blit-cli/tests/remote_resume.rs:19 — 3 MiB server file — the largest payload in the entire integration suite, below every tier boundary

**Proposed fix**: Add boundary unit tests for determine_remote_tuning and pull_stream_count (each threshold, one byte below/at it), pinning chunk_bytes, initial/max_streams, tcp_buffer_size, prefetch_count per tier. Coordinate with the queued slice-2/chunk-ownership work so the tests pin the post-consolidation table, not the disagreeing duals.

## Refuted findings (verification kills — recorded so they are not re-found)

- **boundaries-planner-owns-transport-chunk-heuristic** — Transport-agnostic planner computes its own TCP chunk-size heuristic that disagrees with remote/tuning.rs
  - kill reason: The dual heuristics exist textually (transfer_plan.rs:223-229 vs tuning.rs:5-11), but the claimed live conflict is unreachable. The chunk_bytes==0 fallback at push/client/mod.rs:961 is dead: all three stream_fallback_from_queue callers (mod.rs:495/595/746) pass tuning.chunk_bytes, which is always 16/32/64 MiB (tuning.rs:5-11; determine_tuning passes the default through at auto_tune/mod.rs:39-42), and ensure_remote_tuning (mod.rs:231-234) sets plan_options.chunk_bytes_override before every push-client plan call, so build_plan's override at transfer_plan.rs:229 would return the tuning value even if the branch fired. Every other production path either sets the override (pull.rs:142-143/262-263, pull_sync.rs:501-502/551-552) or never reads planned.chunk_bytes (orchestrator local-mirror via plan_local_mirror, diff_planner.rs:102 — zero chunk_bytes reads in orchestrator.rs; pipeline.rs callers are under #[cfg(test)] at pipeline.rs:418). The planner's internal heuristic is dead code with no consumer, so the claim that chunk size depends on call path is false; the residue is a dead-code cleanup, not a SIMPLE violation with behavioral effect.
- **errors-status-strip-code-and-empty-render** — 14 eyre!(status.message()) sites drop the gRPC code and render a code-only Status as a completely empty error
  - kill reason: Mechanism verified (all strip sites exist: e.g. /home/michael/dev/Blit/crates/blit-app/src/admin/rm.rs:30, blit-core/src/remote/pull.rs:307/332/493/507/541, push/client/helpers.rs:48-49; private format_status at pull.rs:116-122; code-preserving outlier at admin/jobs.rs:91-95), but the finding duplicates two already-tracked items. The admin/CLI half is audit finding R3-H12 (AUDIT_REPORT_2026-06-04_R2.md:333-348, kept HIGH at _R3.md:168), whose written remediation is exactly the proposed fix — a shared `status_to_eyre(rpc_name, status)` helper with code preservation used everywhere. The blit-core half is the queued slice-2 "tonic Status → eyre" preservation on the DO-NOT-RE-REPORT list (docs/STATE.md:34), with TODO(audit-h3c-2) at pull.rs:322-329 pinning those exact map_err sites for a chain-preserving conversion that inherently restores the code and fixes the empty-message blank render. The novel residue (empty-message fallback detail, completions.rs:84 site) belongs as annotation to R3-H12/slice-2, not a separate medium finding.
- **errors-wire-transfer-error-code-drop-empty-render** — Job failure messages cross the wire code-less and possibly empty, and both renderers print them unguarded
  - kill reason: The headline blank-render impact is unreachable: all four production callers of build_transfer_finished_event pass Some(non-empty) on failure (core.rs:533/591/653 via outcome_from_status at core.rs:1330, which forwards status.message() on every Err; core.rs:801-818 uses hard-coded markers), so the unwrap_or("") at core.rs:336 fires only in tests (core.rs:1790-2037). No Status in blit-daemon is built with an empty message — every construction in util.rs, pull_sync.rs, pull.rs, and push/*.rs uses a literal or format!, and the one raw propagation (push/control.rs:62) yields descriptive tonic transport statuses. The proto's documented empty case (blit.proto:986-993) is the no-outcome drop path, where active_jobs.rs:1065 substitutes the non-empty "cancelled before outcome recorded" marker into recents and the event send at core.rs:541 is never reached, so nothing blank is ever rendered by jobs.rs:336 or blit-tui main.rs:5732-5735 (the latter is the subscribe-stream error path, not the TransferError render). The residual code-drop adds little to self-describing format! messages and overlaps the queued slice-2 tonic-Status error-chain work, which I was instructed to cross-reference rather than re-report.
- **constants-two-live-chunk-ladders** — Two live, disagreeing chunk-size ladders answer 'what chunk for N bytes' differently depending on call path
  - kill reason: The cited lines exist, but the liveness mechanism fails. All three callers of stream_fallback_from_queue (push/client/mod.rs:495-502, 595-602, 746-753) pass tuning.chunk_bytes from ensure_remote_tuning, and determine_remote_tuning always yields 16/32/64 MiB (tuning.rs:5-11; auto_tune/mod.rs:39-42 keeps the default when warmup is None), so the chunk_bytes==0 branch at mod.rs:961-962 is unreachable; even if reached, ensure_remote_tuning sets plan_options.chunk_bytes_override (mod.rs:233) before plan_transfer_payloads runs (mod.rs:955), making planned.chunk_bytes equal the tuning value via transfer_plan.rs:229. The only production path where the planner ladder actually computes (local mirror, orchestrator.rs:442-445 leaves override None) never reads planned.chunk_bytes — the sole production consumer is the dead mod.rs:962 branch, and daemon pull paths all override (pull.rs:141-143, 261-263; pull_sync.rs:500-502, 550-552). So there is one live ladder plus a dead duplicate, not two live disagreeing ladders selected by call path; chunked.rs:35-39 is real but is a local-copy buffer rule, not a competing answer on the remote chunk question.
- **drift-shipped-docs-cite-gitignored-absent-benchmark-logs** — Shipped gate docs (WORKFLOW_PHASE_2.5, WORKFLOW_PHASE_3) cite specific logs/ artifacts as acceptance evidence, but logs/ is gitignored and absent
  - kill reason: The dead-link mechanism is real (WORKFLOW_PHASE_2.5.md:30,65-91 and WORKFLOW_PHASE_3.md:71 cite logs/ paths; .gitignore:34 ignores logs/; the directory is absent), but the finding's impact claim is false: the load-bearing parity numbers are committed inline next to every dead path (WORKFLOW_PHASE_2.5.md:64-91, e.g. line 65 "3.85 s vs rsync 6.61 s") and duplicated with the full NO-GO-to-GO trail in DEVLOG.md:259-272, so the Shipped verdicts ARE verifiable from the repo. Machine-local raw logs plus tracked summaries was the doc's explicit design (WORKFLOW_PHASE_2.5.md:18 "Summaries go into DEVLOG.md and this workflow document"), and the proposed fix's main action — move summary numbers to a tracked location — is already done. Both docs are historical (WORKFLOW_PHASE_3.md:3-8); the residual "label citations machine-local" is a cosmetic annotation below finding threshold.
- **drift-psa-architecture-map-ghost-modules** — PROJECT_STATE_ASSESSMENT.md architecture diagram lists transfer_engine and transfer_facade as blit-core modules — neither exists
  - kill reason: The finding's central claim — that transfer_engine and transfer_facade "never existed" and are therefore ghosts rather than staleness — is contradicted by git history. crates/blit-core/src/transfer_engine.rs and transfer_facade.rs were added in initial commit 1503d02 (2025-10-17) and deleted in d20446c (2026-04-22, "refactor: delete dead code from old transfer engine"); the PSA is dated 2026-04-07 (docs/plan/PROJECT_STATE_ASSESSMENT.md:10), so the diagram at lines 113-115 was accurate when written. The superseded banner at lines 3-8 already states the doc "predates pipeline-unification (Phase 4.7)" — the exact refactor that removed these modules — so the existing guard covers this drift and the proposed banner amendment ("modules that were never built") would itself be false.

## Coverage notes per dimension (what was checked and found clean)

### boundaries

Method: read the design map Parts 1.1-1.5, 2 (all crate sections), then re-derived every reported mechanism from code reads in this session — every finding's mechanism rests on lines I personally read (file:line in evidence). Excluded per instructions and cross-referenced instead of re-reported: design-1 (CLI pull byte double-count — my boundaries-progress-event finding is the structural contract gap, not the bug), design-2, design-3 (my socket-policy finding covers the option/ownership half of the same call sites, not the missing connect timeout), and all queued slice-2 transport work (channel-builder triplication, client keepalive, max_decoding_message_size, adaptive windows, tonic-Status chain preservation, inert sink chunk_bytes — none re-reported; findings 1 and 3 are adjacent but cover the classifier-ownership and dual-heuristic halves the queue does not). Checked and found clean: workspace dependency directions are acyclic and sensible (blit-daemon -> blit-core only; blit-cli/blit-tui -> blit-app -> blit-core; verified all four Cargo.tomls) — the violations are duplications forced by those directions, not actual cycles; blit-cli's transport boundary is clean (no channel construction in src, admin verbs delegate to blit_app::admin — verified jobs.rs/scan.rs reads plus the map's rg results); the path_safety chokepoint is honored on both destructive delete twins today (safe_join_contained verified at delegated_pull.rs:449 and blit-app remote.rs:58/223 — the R58-F3 fix landed on both sides; my finding is about the duplication that caused the original divergence); blit-core helpers.rs:52-57 and daemon util.rs:153-158 both correctly delegate normalize_relative_path to path_posix (the mandate bypass is at other sites); the daemon push accept path and DataPlaneSession::connect agree on socket policy with each other (the asymmetry is pull-side only); blit-tui builds no gRPC channels (daemons.rs feeds RemoteEndpoint into the shared blit-app path — light-pass verification only, per the TUI rule). Not covered: blit-prometheus-bridge (map reports coherent local policy; I did not independently read it), deep TUI internals (Phase 6 rule), proto/blit.proto message-level boundary questions, change_journal/fs_capability/win_fs platform modules, and Part 1.6-1.9 of the map beyond the progress section (cancellation/platform dimensions presumably owned by sibling agents). The map's 1.4 section claims about errors.rs being dead were independently re-verified by rg this session (zero consumers confirmed).

### duplication

Verified from code this session (not map trust): every finding's mechanism was re-read at the cited lines. Checked and found CLEAN (good single-owner patterns, valuable for Phase C): TRANSFER_STALL_TIMEOUT is declared once in stall_guard.rs and imported everywhere else (rg showed only use-sites in blit-core pull/data_plane and blit-daemon push/data_plane — the one liveness constant that did consolidate correctly); mDNS is genuinely owned by blit-core/src/mdns.rs (ServiceDaemon::new appears only there); relative_path_to_posix has a single definition in path_posix.rs with both the push client (helpers.rs:52) and daemon (util.rs:153) delegating to it — the normalize_relative_path 'twins' are thin wrappers, not duplication. MAX_TAR_SHARD_BYTES is single-sourced in tar_safety.rs and referenced (not re-typed) by pipeline.rs per the map; I did not re-verify every wire cap. Deliberately NOT reported (cross-referenced instead): the triplicated gRPC channel builder, client keepalive absence, tonic decode limit, and the Status->eyre stringification family (all inside queued slice-2 transport work, STATE.md Queue item 2); unbounded data-plane connects (design-3); the CLI pull byte double-count bug itself (design-1 — I filed only the structural folding-rule duplication around it); orphaned daemon data planes / AbortOnDrop-vs-bare-JoinHandle (design-2 territory). Dropped as low severity: tar-shard 1 MiB reservation duplicated twice; TUI byte-formatter ladder duplicated in f2.rs/f4.rs plus blit-app/display.rs (display-only; TUI light-pass rule); throughput smoothing triplication (per-layer cadences arguably legitimate); mpsc send-error fixed-string family (likely reshaped by queued error-chain work); the '1 MiB' five-constant family (its real risk is the decode-limit invariant, which is queued). Not covered in depth: blit-prometheus-bridge (map reports it self-consistent; spot-checked only), blit-tui internals beyond progress_accum (Phase 6 rule), the double-buffered send/receive loop twins in data_plane.rs (read but judged a FAST/design question for the adaptive-streams landing rather than a consolidation slice), and Windows casefold-key divergence (already tracked as h-paths-2 in docs/audit/findings/inconsistency-paths.md per the map; not re-filed).

### errors

Checked and re-derived from code this session: blit-core/src/errors.rs (full read), blit-app/src/transfers/retry.rs (full read), blit-core/src/remote/pull.rs 75-135 (PullSyncError/format_status), push/client/helpers.rs (map_status, [push] prefix), all daemon service files via targeted rg + reads (Status constructor tally: 116 internal / ~199 total; {err} vs {err:#} counts 12 vs ~69), util.rs (full read), delegated_pull.rs 190-260 and 360-375, core.rs outcome_from_status and TransferError event construction, blit-cli/src/jobs.rs watch render paths, blit-app/src/admin/{rm,jobs}.rs, blit-app/src/transfers/remote.rs 698-760, proto/blit.proto error messages, sink.rs log::warn and mpsc sites, logger.rs (full read), workspace-wide rg for log backends (none exist). Found clean: run_with_retries loop logic and its tests (correct budget/classification semantics); format_status itself (the one well-designed converter — finding is that it is private); admin/jobs.rs cancel_job code branching (good pattern, includes code+message); delegated_pull's R37-F1 negotiation-phase preservation via PullSyncError downcast (works as documented); color_eyre installed in blit-cli main; prometheus bridge stderr usage is internally consistent; copy-engine errors propagate correctly despite the noop Logger (verified Err(e) returned at file_copy/mod.rs:215-223). Not covered per instructions: deep TUI internals (only the main.rs:5734 boundary site, kept as light-pass evidence); the queued slice-2 territory — connect-site chain stripping (pull.rs:245, push/client/mod.rs:313), the TODO(audit-h3c-2) retry no-op at pull.rs:780-788, Status→eyre chain preservation, and retry-classifier extension to tonic codes — cross-referenced in findings 4 and 3 rather than re-reported; the three already-filed design-1/2/3 findings. Not exhaustively audited: blit-app TUI-facing transfer progress text, Windows-only win_fs error paths, and the bug-report-style empty-message reachability of every individual tonic transport error variant (mechanism shown via the in-repo format_status guard plus core.rs unwrap_or(\"\") instead).

### constants

Checked and re-derived from code this session: remote/tuning.rs + auto_tune/mod.rs (full read), transfer_plan.rs size bins and chunk heuristic, buffer.rs (BufferSizer + BufferPool semantics), copy/file_copy/{resume.rs,chunked.rs}, copy/parallel.rs, fs_enum.rs categorize_files (+ workspace-wide caller search), remote/transfer/{data_plane.rs send+receive paths, pipeline.rs wire caps and receive loop, grpc_fallback.rs, payload.rs}, remote/push/client/mod.rs (pool formula, negotiation, fallback chunk selection), remote/pull.rs (client pull path, bare connect at 1710), blit-daemon service/{pull_sync.rs, pull.rs, push/control.rs, push/data_plane.rs}, proto/blit.proto RPC surface. Clean areas (valuable for Phase C): (1) derive_local_plan_tuning (auto_tune/mod.rs:116-166) is a genuine closed loop — perf-history-driven, clamped, run_kind-filtered (R56-F1); the only real runtime adaptation, and it is sound. (2) Local copy parallelism adapts to hardware: rayon par_iter (copy/parallel.rs:30) and workers=num_cpus (orchestrator/options.rs:99). (3) BufferSizer itself (buffer.rs:27-89) is properly memory-aware with a sane sysinfo fallback. (4) Wire structural caps are fine as shape constants: MAX_WIRE_PATH_LEN 64 KiB, MAX_WIRE_TAR_SHARD_FILES 1Mi, and MAX_WIRE_TAR_SHARD_BYTES correctly single-sourced from tar_safety::MAX_TAR_SHARD_BYTES (pipeline.rs:325-337, the F8 pattern the 64 MiB block cap should copy). (5) DEFAULT_BLOCK_SIZE=1 MiB with client-sends-0-means-default is a reasonable protocol default. Map corrections made: push max streams is 16 (daemon desired_streams caps the negotiation; client min()s it), not 24/32 as the map's headline-4 framing implies; tuning's 64 MiB chunk does NOT flow into MAX_WIRE_BLOCK_BYTES-checked frames (only resume blocks are length-checked there), so the 'tuning bump bricks transfers' coupling is really resume.rs MAX_BLOCK_SIZE ↔ pipeline.rs literal; MAX_PARALLEL_TAR_TASKS=4 governs only the gRPC-fallback receive, not the TCP path (which untars inline per stream). Deliberately not re-reported (queued/filed): 4 MiB tonic decode default / max_decoding_message_size absence, GrpcFallback/GrpcServerStreaming sink chunk clamp inertness and GRPC_FALLBACK_CHUNK_BYTES (queued slice-2 inert-chunk_bytes deletion + decode-size work), client channel keepalive/connect timeouts (slice-2), design-1/2/3. Not covered: blit-prometheus-bridge (map reports it clean of tuning constants; not independently verified), blit-tui internals (light-pass rule; its constants are layout-shape, plus the format_bytes duplication which belongs to another dimension), Windows-specific code paths, and scripts/.

### async

Re-derived every reported mechanism from code this session; map claims were treated as leads only. CHECKED AND CLEAN: (1) Client pull path drop-cancellation — AbortOnDrop correctly wraps every internal spawn in pull.rs (:315/:384 data-plane receiver, :726-732 manifest send task, :953, :1640-1657 worker vec); the push side is the gap (filed). (2) mDNS — discover() is a synchronous flume recv_timeout loop (mdns.rs:195-219) but all async consumers wrap it in spawn_blocking (blit-app/src/scan.rs:22; TUI routes through scan::discover at main.rs:4886); daemon advertise runs at sync startup; the Drop-side blocking recv_timeout(1s) (mdns.rs:88-95) only executes after serve() returns (main.rs:144) so it is moot today — noted, not filed. (3) Local engine lifts — blocking orchestrator via spawn_blocking (blit-app/src/transfers/local.rs:44), rayon manifest enumeration inside spawn_blocking (blit-app/src/transfers/remote.rs:93), daemon delegated_pull WalkDir manifest in spawn_blocking (delegated_pull.rs:549), purge/delete in spawn_blocking (admin.rs:51/:68). (4) pipeline.rs streaming executor — traced dispatcher/worker shutdown: dead worker → dispatcher send fails → returns → senders dropped → remaining workers finish(); no deadlock; bounded per-sink channels give backpressure. (5) TarShardExecutor semaphore (push/data_plane.rs:677-704) — bounded at 4, permits released with tasks, extraction in spawn_blocking. (6) BufferPool::acquire semaphore — permit-on-unwind handled (buffer.rs:239-256, audit-12). (7) select! sites — delegated_pull three-way race is biased handler-first (audit-10, core.rs:762-768); push client loop's select arms are cancel-safe mpsc recv (mod.rs:436-855); TUI discovery select (main.rs:4872) clean. (8) StallGuard covers both TCP directions (transfer/data_plane.rs:31/:68 send; daemon push receive :841; daemon pull/pull_sync via DataPlaneSession::from_stream). (9) recents writer does small sync atomic writes on the runtime (active_jobs.rs:831 → recents_store.rs:74-92) — judged low, not filed. NOT COVERED / EXCLUDED PER INSTRUCTIONS: gRPC-stream liveness on healthy-TCP wedged peers (client pull.rs:1232 RemoteFileStream, push response forwarder recv as a *liveness* issue, daemon control.rs:62 / pull_sync recvs, daemon tx.send flow-control stalls) — all in the queued slice-2 keepalive + cadence-watchdog scope (STATE.md Queue 2, grpc_fallback.rs:50-71); design-1/2/3 sites not re-reported (design-2 cross-referenced from the new fourth detach site); blit-tui internals light pass only (single-owner spawn_blocking input task and discovery task looked clean); daemon graceful-shutdown absence (main.rs:137-145) observed but left to the lifecycle/design dimension. Could not exercise anything at runtime (read-only constraint); all mechanisms are static-analysis derived.

### deadcode

Scope: verified every entry in the design map's Part 2 Dead/abandoned lists for blit-core, blit-daemon, blit-cli, and blit-app, re-deriving each from code (rg caller searches + file reads) rather than trusting the map. VERIFIED-DEAD and reported: blit-core errors.rs, tar_stream.rs, zero_copy.rs, delete.rs, copy/parallel.rs+stats.rs, chunked_copy_file, fs_enum categorize_files/enumerate_symlinks/SymlinkEntry/enumerate_directory_deref_filtered, auto_tune warmup machinery, transfer_payloads_via_control_plane, RemotePullClient::pull + daemon legacy Pull TCP plane (with the pull_sync single-stream FAST consequence and the proto wire-compat owner decision), daemon push upload channel + drain task, daemon dead items behind allow(dead_code) (ModuleOptOut, resolve_contained_wire, acquire_buffer, ActiveJobs::cancel/as_str), CLI --interval-ms, CLI unused deps, blit-app empty remote_remote_direct stub + perf::query/PerfReport + stale WatchSnapshot allow. CHECKED AND FOUND LIVE (clean — valuable for Phase C): manifest.rs (consumed by daemon pull_sync); copy_file/copy_paths_blocking/resume_copy_file/mmap_copy_file (live local fast path via orchestrator.rs:355/:1263, local_worker.rs); scan_remote_files and open_remote_file (live, but force_grpc=true only); build_spec_from_options (live in blit-app, blit-tui, daemon); pull_sync client multi-stream receive machinery (pull.rs:1600-1646 — capable but never fed >1 stream by the daemon, folded into the Pull finding); WatchSnapshot, spawn_progress_ticker, active_jobs snapshot/recent/transfer_id/bytes_counter (live — only their allow annotations are stale); cancel_authorized (live at core.rs); push/data_plane.rs+push/payload.rs re-export shims (alive as indirection; judged low severity, dropped); blit-cli endpoints.rs wrapper module, DeferredPullState/DeferredDelegatedState aliases, rm.rs re-export (alive A.0 shims, low, dropped); tests/blit_utils.rs (runs and partially unique — overlap is a test-hygiene issue, judged low for this dimension); ls.rs defensive unreachable Discovery arm (intentional, low, dropped); buffer.rs BufferPool stats counters (vestigial but low, dropped). NOT RE-REPORTED per instructions: design-1/2/3 findings and all queued slice-2 transport items — notably I confirmed blit-app client::CONNECT_TIMEOUT has zero external consumers but folded that into the queued shared-channel-builder work instead of filing it. blit-tui: light pass only — confirmed it consumes RemotePullClient::build_spec_from_options and blit_app::profile (no TUI-internal dead-code findings filed per the Phase-6 rule). Not covered: blit-prometheus-bridge (map reports no dead list for it; did not independently sweep), Windows-only win_fs paths (cannot exercise; caller search only), and git-history dating of modules (read-only session, relied on map dates only for narrative, not for any claim).

### tests

Checked and found clean: (1) Local mirror deletion semantics — orchestrator.rs has a substantial cfg(test) suite (lines 1300+, 1971+) including mirror_still_deletes_truly_unrelated_destination_dirs (:1616), mirror_refuses_when_source_scan_incomplete (:1448), local_mirror_all_scope_deletes_through_filter (:1753), compare-mode and ignore-existing tests; diff_planner.rs has 14 unit tests covering plan_local_mirror; local_move_semantics.rs covers move-vs-mirror regression (R46-F1). (2) Force-gRPC paths ARE integration-tested (contrary to the dimension brief's suspicion): remote_parity.rs test_pull_grpc_fallback/:92 and test_push_grpc_fallback/:133, remote_tcp_fallback.rs --force-grpc-data/:137 and --force-grpc/:168, remote_resume.rs gRPC-fallback resume/:91 — but ALL unix-gated (folded into the cfg(unix) finding). (3) Remote pull-mirror purge semantics well covered on unix (remote_pull_mirror.rs:46,:342,:404 cover purge, FilteredSubset preservation, delete-scope-all). (4) Push mirror safety F1/F2 (incomplete-scan refusal, filtered enumeration) covered in remote_push_mirror_safety.rs. (5) Daemon unit-test density is good: active_jobs.rs 29, delegation_gate.rs 31, core.rs 25, pull_sync.rs 6, push/data_plane.rs 13. (6) Checksum negotiation ack flow covered (remote_checksum_negotiation.rs). Notable non-finding per instructions: multi-stream pull is NOT a test gap — the live PullSync path hard-codes stream_count=1 (pull_sync.rs:568, :707) and the multi-stream machinery lives only in the deprecated Pull client (blit-core pull.rs:251/1600) which has zero callers outside its own file (rg verified) — that is a Phase B dead-code question, already adjacent to the map's 'deprecated pull client path' dead-weight list. Map claim correction: 'all remote integration tests are cfg(unix)' is overstated — remote_move.rs, remote_pull_subpath.rs, admin_verbs.rs, and blit_utils.rs spawn daemons ungated and run on Windows CI; my cfg(unix) finding is re-derived around the files that are gated without unix APIs. Not covered: blit-tui test quality (light-pass rule; it has many in-crate tests, e.g. main.rs:8736+ purge-safety assertions, left to Phase 6), blit-prometheus-bridge tests, the .ps1 journal/USN scripts' coverage, and proto-level compatibility tests (no buf/breaking-change gate exists, but I could not size that without speculating). I did not run any cargo commands (read-only constraint), so test counts are from source inspection, not execution.

### drift

Verified-clean areas (Phase C input): (1) docs/ARCHITECTURE.md blit-core module table (lines 44-64) — every listed module (remote::transfer::{pipeline,source,sink,payload}, orchestrator, mirror_planner, enumeration, copy, checksum, change_journal, tar_stream, auto_tune, perf_predictor, perf_history, fs_capability) exists in crates/blit-core/src; its blit-cli structure section matches the actual files; no ghosts found. (2) AGENTS.md/CLAUDE.md infrastructure claims all check out: .claude/commands/ has all six slash commands, scripts/agent/{context,check-docs,precompact,catchup}.sh and scripts/windows/run-blit-tests.ps1 exist, .agents/ layout (state.md/decisions.md pointer stubs, repo-map.json, skills/{catchup,handoff}) matches §0/§3. (3) payload.rs:67-76 doc comment is the accurate counter-model for the FileStream design and is correct. (4) Plan-doc status hygiene is mostly good: UNIFIED_RECEIVE_PIPELINE.md, LOCAL_TRANSFER_HEURISTICS.md, PIPELINE_UNIFICATION.md, BENCHMARK_10GBE_PLAN.md all correctly marked Historical. (5) README.md, docs/API.md ghost-scans clean (no transfer_engine/TransferFacade/FileStream/heartbeat claims). (6) STATE.md itself is internally consistent and current. Partially refuted map claims corrected in findings: TransferOrchestrator exists (drift-agents-md finding includes the map erratum); WORKFLOW_PHASE_2's fast-path-routing row and tests/integration/local_transfers.rs are real, so only the streaming-planner rows are ghosts. Not covered / deliberately skipped: grpc_fallback.rs:68-71 'handled by HTTP/2 keepalive + cancel-on-disconnect' coverage claim (substantively owned by queued slice-2 watchdog re-scope; only the comment side would be drift and it will be rewritten by that slice); blit-tui internals (light-pass rule — only a perf_history identifier scan done); docs/WHITEPAPER.md, docs/DAEMON_CONFIG.md, docs/perf/remote_remote_benchmarks.md not audited beyond identifier grep; greenfield_plan_v6.md:193 mentions transfer_engine in its Phase 0 port directive (Active plan, intent-language — noted but not filed; fix it alongside the AGENTS.md slice if desired); docs/audit/findings/drift-phases.md (2026-06-04) already documents the WORKFLOW_PHASE_2 streaming-planner gap — my finding 1 confirms it independently and adds the still-unfixed Shipped header + manpage propagation.
