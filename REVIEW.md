# Review status

See `.review/README.md` for the workflow contract.
See `.review/findings/<id>.md` for per-finding details.

## Legend

- `[ ]` Open — coder may pick up
- `[~]` In progress / pending review — sentinel in `.review/ready/`
- `[x]` Verified — verdict in `.review/results/<id>.verified.json`

## Design-review queue (ratified D-2026-06-11-2, in execution order)

Source: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` (slice specs) +
`docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md` (per-finding evidence).
Coder loop: pick the topmost `[ ]` row. W2.3 requires a `docs/plan/` doc with
**Status: Active** before code.

| ID | Severity | Title | Status | Branch | Commit |
|----|----------|-------|--------|--------|--------|
| w5-1-log-backend | Medium | Install stderr log backend (warn) in all 4 binaries + one prefix convention; today every log::warn/error is discarded | `[x]` | master | `56bda09`+`7145202` |
| w4-2-delete-push-upload-channel | Medium | Delete the 262,144-slot push upload channel (drain-and-discard; wedges gRPC-fallback pushes >262k files) | `[ ]` | — | — |
| w5-2-retry-classifier-consolidation | Medium | Delete dead contradictory blit-core/errors.rs; move is_retryable into blit-core with contract test | `[ ]` | — | — |
| w4-1-abortondrop-family | High | Hoist AbortOnDrop; fix all 5 detach-on-drop sites (incorporates design-2; JoinSet for per-stream workers) | `[ ]` | — | — |
| w9-5-jobs-lifecycle-e2e | Medium | jobs/detach lifecycle e2e tests (Subscribe, watch fallback, cancel exit codes) — net before W4.3 | `[x]` | master | `ad773d8` |
| w4-3-daemon-disconnect-racing | Medium | Daemon handlers race tx.closed()+cancel token (hoist delegated_pull's select); fix false active_jobs comment | `[ ]` | — | — |
| w1-2-data-socket-policy-helper | Medium | Shared configure_data_socket (NODELAY/keepalive/tuned buffers) for pull connect + 3 daemon accepts (coordinates design-3) | `[ ]` | — | — |
| w1-3-tcp-keepalive-honesty | Medium | Configure real TcpKeepalive timing or fix both overselling comments; daemon copy logs failure | `[ ]` | — | — |
| w1-4-accept-token-constants | Low | One shared accept(30s)/token(15s) constant pair replacing 4 local declarations | `[ ]` | — | — |
| w2-1-delete-warmup-machinery | Medium | Delete dead auto_tune warmup branches + analyze_warmup_result (honest static table) | `[x]` | master | `2a8a490` |
| w2-2-stream-ladder-owner | Medium | Single stream-count/chunk owner in determine_remote_tuning (takes file_count); delete 2 daemon ladders + transfer_plan ladder | `[ ]` | — | — |
| w2-3-multistream-pull-plan | High | Multi-stream pull-sync: write plan doc (authorized D-2026-06-11-2), harvest deprecated Pull's pattern, implement | `[ ]` | — | — |
| w2-4-delete-pull-rpc | High | Delete deprecated Pull RPC after w2-3 harvest (owner-decided, wire-breaking OK); port scan_remote_files | `[ ]` | — | — |
| w3-1-memory-aware-buffer-pool | High | BufferPool::for_data_plane(tuning, streams) with available-memory cap; replaces 3 pasted formula sites | `[ ]` | — | — |
| w6-1-progress-event-contract | Medium | Define ProgressEvent semantics in blit-core; normalize producers; shared accumulator (incorporates design-1) | `[ ]` | — | — |
| w6-2-progress-residue-verify | Medium | Verify-then-fix map §1.6 residue: delegated zero progress, daemon counters 0 for 3/4 kinds, no denominators | `[ ]` | — | — |
| w4-4-blocking-work-off-runtime | Medium | spawn_blocking the per-entry stat/canonicalize batch + single-file checksum hashing | `[ ]` | — | — |
| w9-1-ungate-windows-tests | High | Remove blanket #[cfg(unix)] from remote transfer tests with nothing unix-specific | `[x]` | master | `9324559` |
| w9-2-revive-root-tests | Medium | Relocate dead workspace-root tests/ into blit-core/tests (MirrorPlanner coverage); delete connection.rs; fix AGENTS.md §4 | `[x]` | master | `461525d` |
| w9-3-test-harness-builder | Medium | TestContext::builder() consolidating 5 harness clones + 5 cli_bin copies; OnceLock daemon build; fake-server keepalive parity | `[ ]` | — | — |
| w9-4-readonly-enforcement-tests | Medium | Tests for all 3 read-only-module gates (push, purge, delegated pull) — zero coverage today | `[x]` | master | `4d67210` |
| w7-1-mirror-executor-consolidation | Medium | One mirror/purge deletion executor + parallel enumerate_local_manifest in blit-core (R58-F3 class closure) | `[ ]` | — | — |
| w7-2-filter-spec-chokepoint | Medium | filter_from_spec pub; push handler uses validated chokepoint (mirror-purge filter currently unvalidated) | `[ ]` | — | — |
| w7-3-wire-metadata-helpers | Medium | Wire metadata + path helpers into blit-core; one mtime error convention; delete per-crate twins | `[ ]` | — | — |
| w7-4-hash-reader-helper | Medium | checksum::hash_reader owning the 256 KiB loop; daemon build_file_header calls it | `[ ]` | — | — |
| w7-5-presenter-formatting | Medium | format_bps in blit_app::display (binary units); switch jobs.rs + 5 TUI copies | `[ ]` | — | — |
| w7-6-default-port-pub | Low | RemoteEndpoint::DEFAULT_PORT pub; delete 9031 literals | `[ ]` | — | — |
| w8-1-foundation-deadcode-sweep | Medium | Delete tar_stream, delete.rs, copy/parallel+stats, chunked_copy_file, fs_enum leftovers (~800 lines). zero_copy EXCLUDED → w8-1b | `[ ]` | — | — |
| w8-1b-zero-copy-fast-eval | Medium | Evaluate wiring splice/zero_copy into the receive pipeline (owner: FAST potential); outcome = plan doc or deletion | `[x]` | master | `6189d82` |
| w8-2-delete-control-plane-payload | Medium | Delete transfer_payloads_via_control_plane (zero-caller duplicate); sequence with W1.1 chunk_bytes deletion | `[ ]` | — | — |
| w8-3-deadcode-hygiene-sweep | Low | --interval-ms flag, blit-cli unused deps, blit-app stubs, stale #[allow(dead_code)] sweep | `[ ]` | — | — |
| w5-3-daemon-status-helpers | Medium | internal_err({:#}) + io_to_status helpers; sweep ~69 chain-amputating + 116 Status::internal sites | `[ ]` | — | — |
| w5-4-mpsc-sendfail-vocabulary | Medium | One honest mpsc send-failure vocabulary; prefer joining the exited task's real error | `[ ]` | — | — |
| w5-5-logger-trait-cleanup | Low | Logger trait permanently-noop error channel cleanup | `[ ]` | — | — |
| w9-6-test-misc | Low | Harness stderr capture; tuning-tier unit tests | `[ ]` | — | — |
| w10-docs-batch | Medium | Docs batch: AGENTS.md ghost names, WORKFLOW_PHASE_2 re-status, --resume/--retry help scoping (help+manpage+README), comment-truth sweep | `[ ]` | — | — |

## Currently pending review

| ID                | Severity | Title                                       | Status | Branch      | Commit    |
|-------------------|----------|---------------------------------------------|--------|-------------|-----------|
| design-1-cli-pull-byte-double-count | Medium | CLI pull progress double-counts bytes on the TCP data plane (producer reports both Payload and FileComplete with full bytes; CLI fold adds both). From design map §1.6, hand-verified | `[ ]` | — | — |
| design-2-orphaned-daemon-data-planes | High | Daemon data-plane tasks detach (not abort) on control-stream death at 3 spawn sites; orphan unreachable by CancelJob. AbortOnDrop fix exists but never propagated. From design map §1.9, hand-verified | `[ ]` | — | — |
| design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lack timeouts (audit-2 fix never reached the data plane); hangs 60-127s on black-holed ports. From design map §1.1/§1.2, hand-verified | `[ ]` | — | — |
| audit-h1-mirror-relay-incomplete-scan | Data-loss | Reject `mirror --relay-via-cli` for remote→remote (round 2: gate moved before mirror confirm prompt + yes=false regression test) | `[x]` | `master` | `4467faf` |
| audit-h3a-push-receive-stall | Robustness | StallGuard on the daemon push-receive socket (`TRANSFER_STALL_TIMEOUT` hoist) — closes one of three remaining stall-guard gaps from R3 H3; symmetric with audit-1c CLI pull-receive | `[x]` | `master` | `dd51a1c` |
| audit-m28-tui-sot-sweep | Docs | TUI source-of-truth sweep (round 2: audit INDEX + R3 updated to record 2026-06-04 owner ratification of H10b + resolution of L39/M27/M28) | `[x]` | `master` | `15fabbf` |
| audit-l39-m27-env-var-purge | Convention | Owner-directed env-var purge (round 2: bench-script prose + Clap `hide_short_help` doc corrections in 3 sites) | `[x]` | `master` | `ec06a95` |
| audit-h11-f1-confirm-detail-err | Data-loss UI | F1 confirm-detail explicit Local/Remote/Err arms + `debug_assert!` (round 2: re-armed at build-fix HEAD; h11 logic itself was correct in dirty tree, blocked by uncommitted Phase 6 dual-pane modules) | `[x]` | `master` | `1b3cb39` |
| audit-h3b-pull-data-plane-write-stall | Robustness | New `StallGuardWriter<W>` wired inside `DataPlaneSession` (round 2: same build-fix re-arm + tightened `Ok(0)` semantics so a zero-byte poll_write doesn't reset the deadline) | `[x]` | `master` | `1b3cb39` |
| audit-h3c-slice1-grpc-fallback-frame-contract | Robustness | Slice 1 of 2: gRPC fallback chunk cap at 1 MiB (`GRPC_FALLBACK_CHUNK_BYTES`) decoupled from TCP tuning; 3 CLI pull receive sites routed through `recv_fallback_message` (the chokepoint slice 2 will wrap with the dynamic progress watchdog). Round-2 adversarial concerns all addressed. Verified 2026-06-11 (owner accept; review assessment found the cap also fixes the tonic 4 MiB decode-limit failure — see DEVLOG, feeds slice-2 re-scope) | `[x]` | `master` | `bf4cc82` |
| d-62-f1-trigger-error | Feature | Inline validation feedback in the F1 trigger modal (round 2) | `[x]` | `phase5/a1` | `0b47a72` |
| d-63-f1-push-progress | Feature | Live byte/file footer for the F1 push (round 2) | `[x]` | `phase5/a1` | `aba54f8` |
| d-64-f1-push-ttl | Feature | Auto-hide the F1 push outcome footer (round 2) | `[x]` | `phase5/a1` | `2f67e96` |
| d-65-f1-push-mirror-move | Feature | Mirror/move for the F1 push direction (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
| d-66-f4-clear-confirm | Feature | y/N gate on the F4 profile-history clear (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
| d-67-help-clear-confirm | Feature | Flag the F4 clear y/N confirm in the `?` keymap (round 2) | `[x]` | `phase5/a1` | `0f4cd64` |
| d-68-f1-remote-remote-copy | Feature | Remote→remote delegated copy from the F1 trigger (round 4) | `[x]` | `phase5/a1` | `c93bcd6` |
| d-69-f1-delegated-progress | Feature | Live byte/file footer for remote→remote delegated copy | `[x]` | `phase5/a1` | `2f1f5d2` |
| d-70-f1-delegated-mirror | Feature | Remote→remote delegated mirror from the F1 trigger | `[x]` | `phase5/a1` | `0b98666` |
| d-71-f1-delegated-move | Feature | Remote→remote delegated move from the F1 trigger (round 3) | `[x]` | `phase5/a1` | `57ed8e9` |
| m2f-1-f2-source-daemon | Feature | Tag F2 transfer rows with their source daemon (multi-daemon F2 step 1) | `[x]` | `phase5/a1` | `aeac25d` |
| m2f-2-f2-composite-key | Feature | Key F2 transfers by (daemon, transfer_id) (multi-daemon F2 step 2, round 2) | `[x]` | `phase5/a1` | `1aed724` |
| m2f-3-f2-merge-snapshot | Feature | Additive per-daemon snapshot hydration + refresh identity fix (multi-daemon F2 step 3) | `[x]` | `phase5/a1` | `7202418` |
| m2f-4-f2-tagged-events | Feature | Carry the source daemon per F2 stream event (multi-daemon F2 step 4) | `[x]` | `phase5/a1` | `8979ff2` |
| m2f-5-f2-fanout | Feature | F2 watches all discovered daemons via merged Subscribe streams (multi-daemon F2 step 5, round 2) | `[x]` | `phase5/a1` | `49f1fce` |
| m2f-6-f2-daemon-column | Feature | Render the source-daemon column in F2 tables (multi-daemon F2 step 6) | `[x]` | `phase5/a1` | `a5456cc` |
| m2f-7-f2-multi-daemon-cancel | Feature | Single cancel (K) targets the selected row's daemon (multi-daemon F2 step 7) | `[x]` | `phase5/a1` | `bbd0084` |
| m2f-8-f2-batch-cancel | Feature | Batch cancel (X) targets each active row's own daemon (multi-daemon F2 step 8) | `[x]` | `phase5/a1` | `dfdaabd` |
| m2f-9-f2-discovery-refan | Feature | Auto re-fan F2 when the discovered-daemon set changes (multi-daemon F2 step 9, round 3) | `[x]` | `phase5/a1` | `9204a4d` |
| e-8-config-default-remote | Feature | Fall back to `[daemon] default_remote` config when no --remote flag (Milestone E) | `[x]` | `phase5/a1` | `bf56a66` |
| m2f-10-f2-per-daemon-health | Feature | Partial-degrade F2 banner when one daemon's stream drops (multi-daemon F2 step 10) | `[x]` | `phase5/a1` | `365be9a` |
| e-9-theme-f2-row-highlight | Feature | F2 active-row highlight honors `[theme] accent_color`, contrasting fg (Milestone E, round 2) | `[x]` | `phase5/a1` | `7dd3e31` |
| e-10-theme-f3f4-highlight | Feature | F3/F4 selection highlights honor `[theme] accent_color` + contrasting fg (Milestone E) | `[x]` | `phase5/a1` | `895fe06` |
| e-11-theme-f1-highlight | Feature | F1 daemon-list highlight honors `[theme] accent_color` + contrasting fg (Milestone E) | `[x]` | `phase5/a1` | `ab85658` |
| bridge-1-prometheus-scaffold | Feature | Prometheus bridge step 1: GetState→prom-text formatter + print-once CLI (Milestone E, round 2) | `[x]` | `phase5/a1` | `9411754` |
| bridge-2-prometheus-http | Feature | Prometheus bridge step 2: long-running /metrics HTTP server, pull-model scrape (Milestone E, round 3) | `[x]` | `phase5/a1` | `5eb9e61` |
| bridge-3-prometheus-readme | Docs | Prometheus bridge step 3: operator README (usage, scrape config, metric reference) (Milestone E) | `[x]` | `phase5/a1` | `9561fb2` |
| keys-1-config-quit | Feature | Key remapping step 1: configurable quit key via `[keys]` config + KeyMap (Milestone E, round 2) | `[x]` | `phase5/a1` | `19c6b7f` |
| keys-2-config-refresh | Feature | Key remapping step 2: configurable refresh key via `[keys]` config, quit/refresh collision policy (Milestone E, round 2) | `[x]` | `phase5/a1` | `ead1adb` |
| keys-3-config-pane-switch | Feature | Key remapping step 3: configurable pane-switch digit aliases + generalized collision policy (Milestone E) | `[x]` | `phase5/a1` | `43d5842` |
| dark-1-theme-base-colors | Feature | dark/light step 1: configurable `[theme] background`/`foreground` base layer (Milestone E) | `[x]` | `phase5/a1` | `775bbe7` |
| dark-2-theme-mode-preset | Feature | dark/light step 2: `[theme] mode = dark\|light` presets (explicit colors override, incl. invalid→terminal-default) (Milestone E, round 2) | `[x]` | `phase5/a1` | `ce4c50f` |
| keys-4-config-movement | Feature | Key remapping step 4: configurable list-cursor aliases `[keys] move_down/up/top/bottom`, lowest-precedence in the collision policy, arrow/Home/End failsafe (Milestone E) | `[x]` | `phase5/a1` | `f9e3378` |
| rec-1-recent-persistence | Feature | Persist `GetState.recent[]` across daemon restarts via dedicated recents.jsonl (separate from planner's perf_local.jsonl); non-blocking write-through + atomic rewrite, opt-in (recent-persistence step 1) | `[x]` | `phase5/a1` | `7c095b2` |
| rec-2-clear-recent | Feature | `ClearRecent` RPC: wipe recent ring + recents.jsonl, never touching planner's perf_local.jsonl (core safety test); empty request, count response (recent-persistence step 2) | `[x]` | `phase5/a1` | `9c2955e` |
| rec-3-tui-clear-recent | Feature | F2 `E` "clear recent" action: empties local view + fans ClearRecent RPC to watched daemons (fire-and-forget); blit-app client helper; footer hint (recent-persistence step 3, final) | `[x]` | `phase5/a1` | `00d2ba5` |
| audit-3a-mutex-poisoning | Robustness | Recover poisoned ActiveJobs table/recent mutexes via `unwrap_or_else(into_inner)` instead of `expect` panic cascade (audit-3 part 1 of 2) | `[x]` | `phase5/a1` | `198ff31` |
| audit-3b-rng-fallible | Robustness | `generate_token` returns `Result` (RNG failure → Status::Internal) instead of panicking the spawned data-plane task; 6 callers propagate via `?` (audit-3 part 2 of 2) | `[x]` | `phase5/a1` | `eeb7c16` |
| audit-5a-bridge-correctness | Robustness | Prometheus bridge: one-shot scrape timeout (8s, fail-loudly) + `\r` escaping in escape_label (audit-5 part 1 of 2) | `[x]` | `phase5/a1` | `f6d2d2d` |
| audit-1a-delegation-port-zero | Robustness | Reject IANA-reserved source port 0 at the delegation gate before DNS/connect (audit-1 item 5; timeouts deferred to audit-1b + owner decision on idle-timeouts) | `[x]` | `phase5/a1` | `a3147b6` |
| audit-1b-net-timeouts-keepalive | Robustness | Delegation DNS-resolve (10s) + dst→src connect (30s) timeouts via net_timeout::within; daemon HTTP/2 keepalive (30s/20s) reaps vanished subscribers — owner-decided over idle-close (audit-1 items 1/2/4) | `[x]` | `phase5/a1` | `1d88fea` |
| audit-7-cargo-lock | Style | Track Cargo.lock for reproducible builds (4-binary workspace); remove from .gitignore (audit-7 item 10, owner-approved — supersedes the never-add rule for the lockfile only) | `[x]` | `phase5/a1` | `dfaecfe` |
| rec-4-clear-recent-confirm | Feature | F2 `E` clear-recent now asks `clear recent? y/N` first (owner-requested); reuses F2CancelStatus confirm machinery via ConfirmingClearRecent variant | `[x]` | `phase5/a1` | `3673ee1` |
| audit-2a-cli-connect-timeout | Robustness | blit_app::client::connect_with_timeout + swap all 10 admin BlitClient::connect sites incl jobs::query. Round 2: DNS-aware outer timeout (connect_timeout alone didn't bound slow DNS). Round 3: corrected stale connect_timeout docs (audit-2 part 1 of 2) | `[x]` | `phase5/a1` | `179f5fa` |
| audit-2b-remote-connect-timeout | Robustness | Bound remaining connects DNS-aware: RemotePull/PushClient::connect at source (fixes 3 data-path sites) + transfers/remote 2 BlitClient sites + blit-cli completions (audit-2 part 2 of 2) | `[x]` | `phase5/a1` | `40ed2d6` |
| audit-4-windows-handle-leak | Bug | RAII OwnedHandle guard closes the CreateFileW handle on every exit path in capture_snapshot (was leaked on the GetFileInformationByHandle `?`). Windows target cargo check passed with `CARGO_FEATURE_PURE=1`; target clippy blocked by pre-existing Windows warnings; Darwin gates pass | `[x]` | `phase5/a1` | `4e77897` |
| audit-5b1-bridge-listener-write | Robustness | Bridge: SO_REUSEADDR listener (build_listener via TcpSocket) + response write timeout (write_all_within, 10s) (audit-5 items 5/6; part 1 of 2 for the server hardening) | `[x]` | `phase5/a1` | `28e9956` |
| audit-5b2-bridge-server-lifecycle | Robustness | Bridge: graceful ctrl_c shutdown + Semaphore concurrency bound (MAX_CONCURRENT_SCRAPES=64) in serve() (audit-5 items 3/4; part 2 of 2 — completes audit-5) | `[x]` | `phase5/a1` | `05f77ec` |
| audit-6d-path-safety-unicode | Test Gap | path_safety: lock in Unicode-opaque containment boundary (NFC/NFD, bidi U+202E, ZWJ, separator/dot lookalikes) — preserved verbatim, can't smuggle traversal; non-UTF-8 unreachable via &str (audit-6 item 4) | `[x]` | `phase5/a1` | `d75cdcf` |
| audit-11-data-plane-underflow | Bug | send_file_double_buffered: clamp each read to `remaining` before subtracting — an over-returning reader (file grew / lying TransferSource) underflowed remaining (debug panic / release u64::MAX runaway) and could push undeclared bytes; now sends exactly header.size | `[x]` | `phase5/a1` | `6a0feb0` |
| audit-12-buffer-pool-leak | Robustness | BufferPool acquire/try_acquire: defer std::mem::forget(permit) until after the vec! allocation so an alloc panic releases the memory-budget permit by unwind instead of leaking it (permit leak → pool starvation) | `[x]` | `phase5/a1` | `326b3ff` |
| audit-9-cancel-auth | Bug | CancelJob now authorizes the caller against the transfer's originating peer (host/IP-only, port-insensitive; loopback + UDS bypass); cross-tenant cancel → PermissionDenied. New CancelOutcome::Unauthorized | `[x]` | `phase5/a1` | `3c5a398` |
| audit-10-cancel-completion-race | Bug | DelegatedPull select: order the handler branch first in the biased select (via resolve_delegated_pull_outcome helper) so a completion wins over a simultaneous CancelJob — was mis-recording a success as "cancelled via CancelJob" | `[x]` | `phase5/a1` | `3601f1e` |
| audit-8-tui-task-leak | Robustness | TUI Subscribe forwarder races tx.closed() (via forward_step) so it exits on F2 re-fan even for a silent daemon (was leaking conn+Receiver+slot); + outer tokio::time::timeout around jobs::subscribe open. Round 2: also bound the initial GetState snapshot fetch (fetch_snapshot_within → degraded Err) | `[x]` | `phase5/a1` | `2d7b6f7` |
| audit-6f-dns-rebinding-test | Test Gap | delegation_gate DNS-rebinding regression: ScriptedResolver returns IP A then B; assert the gate resolves once, binds A, never consults B (+ converse: denies on the first resolution only) (audit-6 item 6) | `[x]` | `phase5/a1` | `28e0b95` |
| audit-6g-copy-fallback-test | Test Gap | copy_file fast-path→fallback: byte-identical copy on all platforms + macOS clonefile-EEXIST forces the clonefile→fcopyfile hop. Round 2: assert clone_succeeded to pin the hop (not the buffered tail). Buffered-tail needs a production seam (flagged) (audit-6 item 7) | `[x]` | `phase5/a1` | `4c4db89` |
| audit-7e-cleanup | Style | Remove 33 tracked AppleDouble ._* sidecars + 2 empty npm stubs (package.json/lock); gitignore ._*. Rust-only workspace, no build/test impact (audit-7 code-health) | `[x]` | `phase5/a1` | `16a92ce` |
| audit-6c-bridge-http-integration | Test Gap | bridge end-to-end HTTP test: real client → handle_conn over loopback; GET /metrics (unreachable daemon → 200 + blit_daemon_up 0) and GET /favicon → 404 (audit-6 item 3) | `[x]` | `phase5/a1` | `02c7a9c` |
| audit-7b-dead-code | Style | Remove dead compare.rs fns (+orphaned imports), 3 STALE fs_enum allow(dead_code) (fields are live), write-only diagnostics written_at field, empty blit-app progress.rs stub. remote_remote_direct.rs left (live, 285 lines) (audit-7 code-health) | `[x]` | `phase5/a1` | `5a5f735` |
| audit-6a-blit-app-filter-tests | Test Gap | blit-app transfers/filter.rs: 6 tests on build/build_spec (glob/size/age propagation, reference_time capture, malformed-glob + bad-size rejection, Duration→secs). Note: "zero #[cfg(test)]" premise stale — 8 files already tested (audit-6 item 1) | `[x]` | `phase5/a1` | `8820226` |
| audit-7c-docs | Docs | ARCHITECTURE.md: add blit-app/blit-tui/blit-prometheus-bridge crate sections + diagram; complete gRPC surface to all 15 RPCs (verified vs proto). README: fix clone URL your_org→roethlar/Blit. Round 2: bridge as blit-app consumer, full module table (check/scan/display), F4=profile/verify/diagnostics (audit-7 code-health) | `[x]` | `phase5/a1` | `a11845a` |
| audit-6b-tui-render-test | Test Gap | F4 render_into driven through ratatui TestBackend (Profile+Verify+Diagnostics+Transfer): renders default state at 120x40 + tiny 8x3 area, asserts no panic (clamp). f2/f3/help already covered (audit-6 item 2) | `[x]` | `phase5/a1` | `267f093` |
| audit-6e-move-directory-coverage | Test Gap | directory-tree push-move + pull-move integration tests (recursive copy-then-delete-source; all files land + entire source removed). All 4 cardinal directions already covered single-file; this fills the multi-file gap (audit-6 item 5). Round 2: assert recursive remote source-tree removal on pull-move | `[x]` | `phase5/a1` | `6d410ac` |
| audit-1c1-stall-guard | Robustness | StallGuard<R> AsyncRead idle-timeout adapter (no-bytes-for-30s → TimedOut; re-armed per read = idle not total). Owner scope=all pulls. Part 1 of 2; part 2 wires it into the receive pipeline (audit-1c) | `[x]` | `phase5/a1` | `0cfa534` |
| audit-1c2-stall-wiring | Robustness | Wire StallGuard into the receive pipeline: generic-ize execute_receive_pipeline + 6 read helpers over AsyncRead, wrap the socket in pull.rs (unconditional → all pulls). Completes audit-1c (audit-1 item 3) | `[x]` | `phase5/a1` | `906cedf` |
| retry-wait1-classifier-loop | Feature | retryable-error classifier (transient io kinds incl. StallGuard TimedOut; fatal eyre/path/gate not retried) + run_with_retries loop in blit-app. Owner-approved --retry/--wait part 1 of 2; part 2 adds flags+wiring | `[x]` | `phase5/a1` | `e5e59fb` |
| retry-wait2-cli-wiring | Feature | --retry<N>/--wait<SECS> on TransferArgs (default 0/5) + wrap run_transfer/run_move in run_with_retries (resumable retry on transient failures). Completes the retry-wait feature (owner-approved follow-up) | `[x]` | `phase5/a1` | `68b34ac` |
| audit-13-buffer-pool-double-locking | Performance | BufferPool release/return_vec: single-lock cache return via cache_returned_buffer + drop redundant per-release buffer_size zeroing (truncate common path); verified no consumer relies on pre-zeroed pool buffers (Gemini-sourced) | `[x]` | `phase5/a1` | `f9d3f2f` |
| audit-14-resume-copy-redundant-seek | Performance | resume_copy_file: drop the redundant per-iteration src seek (sequential) + track dst_cursor_pos to seek dst only on divergence. Pure syscall reduction; existing byte-exact resume suite covers it (Gemini-sourced) | `[x]` | `phase5/a1` | `b7f8177` |
| audit-15-grpc-missing-connection-timeouts | Robustness | RECOMMEND-DEFER (analysis only, no code): blanket Server::timeout(30s) would kill the 7 streaming RPCs (Subscribe/DelegatedPull/Pull/PullSync/Push/Find/DiskUsage); dead-peer case already covered by audit-1b keepalive. Reviewer to grade decision (Gemini-sourced) | `[x]` | `phase5/a1` | `f0ed9e5` |
| audit-7d1-extract-progress-accum | Refactor | main.rs split part 1: extract 5 pure progress helpers (accumulate_pull/push/delegated_progress, pull_throughput, du_total_from_entries) verbatim into crate::progress_accum; crate-root use keeps call sites + tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `5112705` |
| audit-7d2-extract-display-f3 | Refactor | main.rs split part 2: extract 4 pure F3 state→display mappers (f3_pull_to_display + private confirm_detail, f3_du_to_display, f3_del_to_display) verbatim into crate::display_f3; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `315f923` |
| audit-7d3-extract-display-f1 | Refactor | main.rs split part 3: extract 4 pure F1 state→display mappers (f1_trigger_prompt, f1_push_status + private push_present_verb/push_past_verb) verbatim into crate::display_f1; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `1e50f7d` |
| audit-7d4-extract-display-f2 | Refactor | main.rs split part 4: extract 2 pure F2 cancel mappers (cancel_status_to_display, cancel_status_remaining_ttl) verbatim into crate::display_f2; F2CancelStatus stays in main.rs (event loop mutates it), referenced read-only via crate-root path; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `0ed685a` |
| audit-7d5-extract-config-reload | Refactor | main.rs split part 5: extract Ctrl+R config hot-reload helpers (reload_tui_config I/O wrapper + pure classify_reload) verbatim into crate::config_reload; ReloadBanner stays in main.rs (AppState field), referenced via crate-root path; reload_tui_config re-exported at crate root, classify_reload imported test-locally (sole non-test caller moved with it). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `4e728b5` |
| audit-7d6-extract-tick-budget | Refactor | main.rs split part 6: extract pure sleep-budget math (compute_tick_budget, min_opt — Duration/Option, no AppState) verbatim into crate::tick_budget; crate-root use keeps event-loop call sites + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `a99a136` |
| audit-7d7-extract-del-request | Refactor | main.rs split part 7: extract 3 pure F3-delete request builders (del_wire_path, build_delete_request, is_deletable_remote_path — no async/AppState) verbatim into crate::del_request; crate-root use keeps dispatcher + plan_f1_* + spawn tasks + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `b18e5f9` |
| audit-7d8-extract-exec-plan | Refactor | main.rs split part 8: extract 3 pure transfer-execution builders (f3_pull_options, build_f1_push_execution, build_delegated_execution — no async/AppState/IO) verbatim into crate::exec_plan; remove_local_source stays in main.rs (does IO); crate-root use keeps spawn_* tasks + inline tests unchanged. Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `d47cc24` |
| audit-7d9-extract-theme-color | Refactor | main.rs split part 9: extract 2 pure theme-color mappers (base_theme_style, raw_color_to_ratatui — no AppState) verbatim into crate::theme_color; crate-root use keeps render call sites + inline tests unchanged. Behavior-preserving (audit-7d). NOTE: plan_f1_trigger/plan_f1_delegated inspected + NOT moved (mutate &mut AppState, coupled). Behavior-preserving (audit-7d) | `[x]` | `phase5/a1` | `6ddce2e` |
| bug-mirror-literal-backslash | Bug fix | Mirror failure on POSIX filenames containing literal `\` (e.g. macOS Logic Pro `1\4 Single.pst`): blanket `path.to_string_lossy().replace('\\', "/")` at 11 sites was destructive. New canonical `blit_core::path_posix::relative_path_to_posix` (component-walk) + every Path→wire site routed through it. Round 2: GPT-reopen — `relative_str_to_posix` was dropping trailing `/` in admin completion input (regressed dir-prefix completions); now preserves trailing-separator UX semantic for user input while keeping wire/manifest form clean. 10 regression tests. Owner-verified. | `[x]` | `phase5/a1` | `5a034dd` |
| tui-key-dispatch-press-only-filter | Bug fix | TUI input task dropped Repeat events: `spawn_input_task` matched only `KeyEventKind::Press`, silently dropping autorepeat (and any other non-Press kind). Now accepts Press + Repeat, only filters Release. Plus a `BLIT_TUI_INPUT_TRACE=1` env-gated diagnostic log to `/tmp/blit-tui-input.log` for follow-up if needed. Owner-authorized. | `[x]` | `phase5/a1` | `2e5bcb9` |
| windows-move-tree-hang | Known issue | `test_remote_move_local_to_remote_directory_tree` hangs on Windows CI (14+ consecutive runs). Other 3 `remote_move` tests pass. Suspected: local source-delete `fs::remove_dir_all` blocked by open file handles from push enumeration (POSIX-vs-Windows unlink semantics). Test gated off Windows with `cfg_attr ignore`; root cause needs interactive Windows debugging. Owner-authorized defer. | `[x]` | `phase5/a1` | `2e5bcb9` |

## Open findings

| ID         | Severity | Title                                                    | Branch |
|------------|----------|----------------------------------------------------------|--------|
| B          | Feature  | `GetState` RPC + `ActiveJobs` table + recent ring        | `phase5/getstate` |
| M-Jobs     | Feature  | Daemon-owned transfer lifecycle (`CancelJob`, `detach`)  | `phase5/m-jobs` |
| C          | Feature  | `Subscribe` RPC + byte-level instrumentation             | `phase5/c` |
| A.1        | Feature  | TUI implementation                                       |        |
| D          | Feature  | Verify + diagnostics screens                             |        |
| E          | Feature  | Polish (themes, refresh rates, config)                   |        |
| P0-§2.6    | Feature  | Live remote benchmark capture (hardware-bound)           |        |
| audit-1-daemon-timeouts | Robustness | Network operation timeout gaps in delegation path (DNS, gRPC connect, pull_sync_with_spec, subscribe idle) — items 1/2/4 done (audit-1a/1b); item 3 = audit-1c (design pending) | |
| audit-1c-transfer-stall-timeout | Robustness | DESIGN PENDING APPROVAL: no-bytes-30s idle timeout on the delegated pull via an opt-in AsyncRead StallGuard adapter (delegated-only). See finding for approach + open scope question. Prereq for --retry/--wait | |
| audit-2-cli-timeouts | Robustness | Missing connection timeouts on all CLI/admin-verb gRPC connections (~15 sites) | |
| audit-3-panic-resilience | Robustness | SysRng panic in generate_token + 7 mutex poisoning expects in ActiveJobs | |
| audit-4-windows-handle-leak | Bug | Windows HANDLE leak on GetFileInformationByHandle failure in change journal snapshot | |
| audit-5-bridge-robustness | Robustness | Prometheus bridge: one-shot timeout, \r escaping, graceful shutdown, connection limit, write timeout, SO_REUSEADDR | |
| audit-6-test-gaps | Test Gap | Missing test coverage: blit-app (zero tests), TUI rendering, bridge integration, Unicode paths, DNS rebinding, copy fallback | |
| audit-7-code-health | Style | Monolithic files (11K-line main.rs), dead code, 28 AppleDouble artifacts, stale docs, missing Cargo.lock | |
| audit-8-tui-task-leak | Robustness | TUI subscribe forwarder task leak on reconnect; setup task has no connect/RPC timeout | |
| audit-9-cancel-auth | Bug | CancelJob RPC lacks peer authorization — any client can cancel any transfer | |
| audit-10-cancel-completion-race | Bug | Cancel/completion race in delegated_pull: success recorded as "cancelled" under biased select | |
| audit-11-data-plane-underflow | Bug | Buffer underflow in send_file_double_buffered when reader returns excess bytes | |
| audit-12-buffer-pool-leak | Robustness | Semaphore permit leak on OOM panic in BufferPool acquire/try_acquire | |
| audit-13-buffer-pool-double-locking | Performance | Double-locking and redundant memory zeroing in BufferPool | |
| audit-14-resume-copy-redundant-seek | Performance | Redundant seek system calls in sequential block-level resume | |
| audit-15-grpc-missing-connection-timeouts | Robustness | Missing request/idle timeouts on tonic gRPC server control plane | |

## Verified history

Phase 5 A.0 complete. Per-finding audit trails in
`.review/findings/a0-*.md` and `.review/results/a0-*.verified.json`.
Sub-slices on branch `phase5/blit-app-extract`:

- `b5d2414` Crate scaffold + endpoints
- `4800cfc` df / du / find / list-modules / rm
- `009583c` ls (initial)
- `af436b2` ls — LocalListing enum fix
- `39966df` scan
- `d6ee06a` profile
- `334a684` diagnostics (perf + dump)
- `2626f9b` diagnostics — perf best-effort fix
- `e807f46` check
- `44a4f8c` util.rs split
- `2a37a3e` transfers/local
- `8c4174a` transfers/filter
- `3639159` transfers/resolution
- `65f6031` transfers/resolution — followups (`a0-resolution-fixup`)
- `b2d6c9c` transfers/remote — pull-flow helpers (`a0-remote-helpers`)
- `e25707e` transfers/remote — pull entry-point (`a0-pull-execution`)
- `cb96331` transfers/remote — push entry-point (`a0-push-execution`)
- `1879019` transfers/remote_remote_direct (`a0-delegated-execution`)
- `8860cac` transfers/dispatch (`a0-dispatch`)
- `6eeb214` endpoints — support gates (`a0-endpoints-gates`)
- `29a2026` final cleanup — drop CLI shim re-exports (`a0-final-cleanup`)

Phase 5 B sub-slices on branch `phase5/getstate`:

- `10259ec` ActiveJobs table on `BlitService` (`b-1-active-jobs`)
- `ef46631` Streaming RPCs populate ActiveJobs rows (`b-2-set-endpoint`)
- `eab1a17` TransferRecord ring + outcome capture (`b-3-recent-ring`)
- `b6b6bb2` GetState RPC + DaemonState handler (`b-4-getstate`)
- `5f8ca5b` `blit jobs list <remote>` consumes GetState (`b-5-jobs-list`)

Phase 5 M-Jobs sub-slices on branch `phase5/m-jobs`:

- `1e493c0` Per-row CancellationToken + delegated_pull race (`m-jobs-1-cancel-token`)
- `66df256` CancelJob RPC + `blit jobs cancel` CLI (`m-jobs-2-cancel-rpc`)
- `1221d60` detach field + fire-and-forget CLI path (`m-jobs-3-detach`)
- `09cffbb` `blit jobs watch` polling surface (`m-jobs-6-watch`)

Phase 5 C sub-slices on branch `phase5/c`:

- `234d2c6` Byte counter API in `blit-core` + ActiveJobs row wiring (`c-1a-byte-counter-api`)
- `14eeda7` Delegated-pull data-plane byte reporting, including tar-shard/block paths (`c-1b-byte-counter-wiring`)
- `0ffaac7` Subscribe RPC + DaemonEvent + TransferStarted (`c-2-subscribe-skeleton`)
- `df8249d` SubscribeRequest reserved-tag order aligned with `TUI_DESIGN.md` (`c-2-subscribe-skeleton` round 2)
- `5be5f10` TransferComplete + TransferError terminal events (`c-3-transfer-finished-events`)
- `7d4fd28` Terminal events broadcast after ActiveJobs/GetState drain (`c-3-transfer-finished-events` round 2)
- `69224e0` Periodic TransferProgress events (`c-4-transfer-progress`)
- `5b88f3a` Progress events serialized before terminal events (`c-4-transfer-progress` round 2)
- `6330a7d` SubscribeRequest transfer_id_filter (`c-5a-transfer-id-filter`)
- `7587b46` Per-subscriber forwarder drains broadcast before filtering (`c-5a-transfer-id-filter` round 2)
- `d901656` Forwarder exits on client disconnect (`c-5a-transfer-id-filter` round 3)
- `b71fd6d` Per-job event ring + `replay_recent` (`c-5b-event-ring`)
- `ea7a8d7` `blit jobs watch` consumes Subscribe stream (`c-6-jobs-watch-stream`)
- `f7edcc9` Subscribe-first watch race fix + stable terminal JSON (`c-6-jobs-watch-stream` round 2)
- `7d3ff3f` `blit jobs watch` enables `replay_recent` (`c-7-watch-replay`)

Phase 5 A.1 sub-slices on branch `phase5/a1`:

- `2237521` `blit-tui` crate scaffold + terminal lifecycle (`a1-1-tui-scaffold`)
- `1176331` F2 Transfers pane with live Subscribe stream (`a1-2-f2-transfers`)
- `1fcee97` F1 Daemons pane with Local row, viewport-aware table, and rescan clamp (`a1-3-f1-daemons`)
- `3ac1cb2` F1 detail block populated by `GetState` (`a1-3b-f1-getstate-detail` round 2)
- `2abc71b` F3 Browse pane with modules + directory tree (`a1-4-f3-browse` round 2)
- `d33fedc` F4 Profile pane with read-only perf history + predictor (`a1-5-f4-profile`)
- `72d67ed` F-key screen router with router-owned input task (`a1-6-screen-router` round 2)
- `8719925` AppState unified loop with generation-guarded F2 setup (`a1-6b-state-preservation` round 3)
- `0607c70` F4 profile lifecycle actions preserve mutation-error banners (`d-1-f4-profile-lifecycle` round 2)
- `62eef1e` F4 Verify pane with generation-safe in-flight edits (`d-2-f4-verify` round 2)
- `d26ca9c` F4 diagnostics dump mirrors CLI JSON shape (`d-3-f4-diagnostics` round 2)
- `aba7394` `?` help overlay global from Verify edit mode (`e-1-help-overlay` round 2)
- `470630e` F4 local copy/mirror triggers with mirror confirmation (`d-4-f4-local-transfers` round 2)
- `5b51ee0` Responsive tab-strip counts include F4 local transfers (`e-2-tab-strip-counts` round 2)
- `dc03872` F4 local move trigger with source-delete confirmation (`d-5-f4-local-move`)
- `368dd4b` F4 Verify checksum toggle (`d-6-f4-verify-checksum-toggle`)
- `428cd22` F4 Verify one-way toggle (`d-7-f4-verify-one-way-toggle`)
- `9b01340` F4 Verify/Transfer Done durations (`d-8-f4-elapsed-time`)
- `f011cb3` F4 live elapsed ticker (`d-9-live-tick` round 2)
- `b625ca1` F4 transfer Done throughput (`d-10-transfer-throughput`)
- `fa18813` F1/F3/F4 freshness live tick (`d-11-freshness-tick` round 2)
- `ec8695f` F4 Esc cancels mirror/move confirm (`d-12-esc-cancels-confirm` round 2)
- `8787b5f` F2 footer last-event age (`d-13-f2-freshness-footer`)
- `f1217b7` F2 active-row age column (`d-14-f2-active-row-age`)
- `31541a7` F2 active-row percent complete (`d-15-f2-active-row-progress`)
- `d471f1e` `?` help overlay documents active-pane refresh (`d-16-help-overlay-keymap-sync` round 2)
- `873757b` F4 Verify Done preview lines (`d-17-verify-result-preview`)
- `a4416c8` Ctrl-U clears focused Verify field (`d-18-verify-form-clear`)
- `aac0b22` Digit aliases for F1-F4 pane navigation (`d-19-digit-tab-shortcuts`)
- `2e11732` F2 recent throughput column + layout doc sync (`d-20-f2-recent-throughput` round 2)
- `5e8856f` F2 active-row cursor anchored by transfer_id (`d-21-f2-active-cursor` round 2)
- `852fe10` F2 cancel-selected action + layout doc sync (`d-22-f2-cancel-selected` round 2)
- `94f556a` F2 cancel-status fragment auto-hide TTL (`d-23-cancel-status-auto-clear`)
- `db779dc` Configurable F2 cancel TTL drives sleep budget (`d-24-config-cancel-ttl` round 2)
- `7bc5e57` F2 TiB/TiB/s formatter tiers aligned with F4 (`d-25-f2-tib-tier`)
- `a89dd48` F3 substring filter via `/` (`d-26-f3-filter` round 2)
- `d3ba561` F3 stable sort with deterministic case-variant tiebreak (`d-27-f3-sort` round 2)
- `0fbaad6` F3 no-match filter empty-state message (`d-28-f3-no-matches-msg`)
- `b7b08a9` Opt-in F2 cancel confirmation prompt (`d-29-confirm-cancel`)
- `d778f76` F2 batch cancel freezes confirmed transfer ids (`d-30-batch-cancel` round 2)
- `13459b4` Scrollable help overlay for small terminals (`d-31-help-scroll`)
- `614e58d` Help overlay scrollbar indicator (`d-32-help-scrollbar`)
- `c89d278` TUI config loader warning ordering + Verify defaults (`e-3-config-scaffold` round 2)
- `94a30c2` Configurable tab-strip counts (`e-4-config-tab-strip-counts`)
- `7b5310e` Configurable live-tick interval + source-doc sync (`e-5-config-live-tick-interval` round 2)
- `72b742b` Verify path prefill config + schema doc sync (`e-6-verify-prefill` round 2)
- `12ee960` Configurable tab-strip accent with contrasting active-tab foreground (`e-7-config-theme` round 2)
- `7baf2a4` F3 pull-source spec preview with bracketed IPv6 authority (`d-33-f3-pull-source` round 2)
- `eb1ee45` F3 pull-source preview derived via `RemoteEndpoint` (`d-34-f3-pull-endpoint`)
- `beb5d9e` F3 pull destination prompt + execution with resolved local destinations (`d-35-f3-pull-execute` round 2)
- `27a8005` Ctrl+R hot-reload of tui.toml with parse-error keep-current semantics (`d-36-hot-reload-config`)
- `cd1751a` F3 pull live byte/file footer with pull-receive accumulator semantics (`d-37-f3-pull-progress` round 2)
- `7c523e8` F3 pull Done/Error footer auto-hide TTL (`d-38-f3-pull-ttl`)
- `43258ab` F3 pull progress footer average throughput (`d-39-f3-pull-throughput`)
