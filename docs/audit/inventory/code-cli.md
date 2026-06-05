# Code Inventory: blit-cli — every verb + flag handler
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 20 files, 4,877 lines of source

## Behaviors (grouped by category)

### flag-handling

- **transferargs-shape** — `crates/blit-cli/src/cli.rs:188-364` — `TransferArgs` defines every flag for Copy/Mirror/Move: comparison (`--checksum`/`--size-only`/`--ignore-times`/`--ignore-existing`/`--force`/`--delete-scope`), reliability (`--resume`/`--retry`/`--wait`), filtering (`--exclude`/`--include`/`--files-from`/`--min-size`/`--max-size`/`--min-age`/`--max-age`), performance (`--force-grpc`/`--relay-via-cli`/`--detach`/`--null`), and hidden (`--workers`/`--trace-data-plane`). _(notes: single struct shared across three verbs even when many combinations are illegal — gates live downstream in run_move/run_transfer)_
- **retry-wait-defaults** — `crates/blit-cli/src/cli.rs:264-272` — `--retry` defaults to 0 (off), `--wait` to 5 seconds; both parse u32/u64; wait passed to `run_with_retries` as `Duration::from_secs`. _(notes: no upper bound on retry or wait; no jitter/backoff visible at CLI layer)_
- **effective-progress** — `crates/blit-cli/src/cli.rs:372-380` — explicit `--progress` wins; with `--json` disables progress; otherwise auto-enable when stdout `is_terminal()`. _(notes: rsync/rclone-style default; piped stdout → silent for scripts)_
- **delete-scope-parse** — `crates/blit-cli/src/cli.rs:247-248, 386-388` — `--delete-scope` parsed as `["subset","all"]` with default `"subset"`, then case-insensitively compared in `delete_scope_all()`. _(notes: stringly typed instead of enum; case-insensitive but value_parser is case-sensitive)_
- **mirror-mode-enum-mapping** — `crates/blit-cli/src/transfers/remote.rs:230-238` — push converts `--delete-scope` → wire `MirrorMode::All` / `FilteredSubset` / `Off`. _(notes: parallel logic in delegated path; if the strings drift between CLI and wire enum naming, silent miscompare)_
- **detach-gate-up-front** — `crates/blit-cli/src/transfers/mod.rs:161-178` — `--detach` rejected with explicit messages for any local endpoint and for `--relay-via-cli`; allowed only for remote→remote delegated. _(notes: gate in CLI saves a round-trip; daemon also refuses on push/pull RPCs per comment)_
- **move-rejects-detach** — `crates/blit-cli/src/transfers/mod.rs:255-269` — `blit move --detach` bails with explanation that source-delete needs CLI to await completion. _(notes: data-loss class)_
- **move-rejects-filters** — `crates/blit-cli/src/transfers/mod.rs:280-296` — `move` rejects ANY of `--exclude/--include/--min-size/--max-size/--min-age/--max-age/--files-from`. _(notes: R49-F1; data-loss class — skipped-then-deleted)_
- **move-rejects-ignore-existing** — `crates/blit-cli/src/transfers/mod.rs:306-314` — `move --ignore-existing` bails; recommends copy-then-rm. _(notes: R51-F1 data-loss class)_
- **move-rejects-null** — `crates/blit-cli/src/transfers/mod.rs:323-331` — `move --null` bails; null sink + source delete = data destruction. _(notes: R52-F1)_
- **move-rejects-force-ignoretimes** — `crates/blit-cli/src/transfers/mod.rs:356-406` — `move --force` and `move --ignore-times` bail because local + local→remote paths don't honor the flags in comparison-mode selection. _(notes: R54-F2; error messages tailor remediation by direction; local→remote has NO safe escape because daemon push compares size+mtime only regardless of --checksum)_
- **move-rejects-relay-with-remote-src** — `crates/blit-cli/src/transfers/mod.rs:579-590` — remote→remote `blit move --relay-via-cli` bails: legacy relay path doesn't carry require_complete_scan signal. _(notes: R50-F1/R51-F2 data-loss closure)_
- **move-rejects-dryrun** — `crates/blit-cli/src/transfers/mod.rs:251-253` — `move --dry-run` bails outright; no semantic for "pretend to delete source". _(notes: terse single-line bail vs the verbose explanations elsewhere)_
- **null-rejects-mirror** — `crates/blit-cli/src/transfers/mod.rs:132-142` — `--null` with `mirror` mode bails: null sink + dest-purge would still delete. _(notes: R54-F1 data-loss class)_
- **null-rejects-remote** — `crates/blit-cli/src/transfers/mod.rs:143-154` — `--null` with any remote endpoint bails: remote push/pull paths silently ignore null. _(notes: null is local-copy-only)_
- **jobs-watch-empty-id** — `crates/blit-cli/src/jobs.rs:177-179` — empty `transfer_id` (after trim) bails before any RPC. _(notes: defensive; clap allows empty string)_

### endpoint-parse

- **transfer-endpoint-parse** — `crates/blit-cli/src/transfers/mod.rs:102-106` — `parse_transfer_endpoint` then `resolve_destination` (rsync-style basename resolution) applied before any route selection. _(notes: ordering matters — display string captured BEFORE resolution for verbose log line)_
- **bare-host-routes-to-list-modules** — `crates/blit-cli/src/ls.rs:57-59` — `blit ls host` (RemotePath::Discovery) smart-dispatches to `list_modules_remote` instead of bailing. _(notes: documented dual-entry pattern; same output as `blit list-modules host`)_
- **rm-requires-remote** — `crates/blit-cli/src/rm.rs:14-22` — `blit rm` bails on local path. _(notes: rm has no local form by design)_
- **rm-refuses-empty-relpath** — `crates/blit-cli/src/rm.rs:26-43` — refuses to delete entire module (empty/dot rel_path, also after rebuilding components). _(notes: defense in depth — both `as_os_str().is_empty()` AND post-join string check)_
- **df-requires-remote** — `crates/blit-cli/src/df.rs:8-16` — `blit df` bails on local path. _(notes: matches du/find/rm pattern)_
- **du-requires-remote** — `crates/blit-cli/src/du.rs:9-17` — `blit du` bails on local path. _(notes: parallel to df/find)_
- **find-requires-remote** — `crates/blit-cli/src/find.rs:9-17` — `blit find` bails on local path. _(notes: parallel)_
- **completions-remote-requires-remote** — `crates/blit-cli/src/completions.rs:50-58` — `blit completions remote` rejects local target. _(notes: parallel pattern)_
- **list-modules-uses-RemoteEndpoint-parse** — `crates/blit-cli/src/list_modules.rs:7-9` — uses `RemoteEndpoint::parse` directly (not `parse_endpoint_or_local`). _(notes: divergence — every other verb uses the local-or-remote helper; here a bare local path would yield a less-friendly parse error)_
- **jobs-uses-RemoteEndpoint-parse** — `crates/blit-cli/src/jobs.rs:33-34,46-47,175-176` — `jobs list/cancel/watch` use `RemoteEndpoint::parse` directly. _(notes: same divergence as list-modules; intentional — these verbs are remote-only)_

### confirmation-prompt

- **destructive-prompt-helper** — `crates/blit-cli/src/transfers/mod.rs:87-99` — `confirm_destructive_operation` writes "msg [y/N]: " to stdout, reads stdin, accepts only `y` / `yes` after `trim` + `to_ascii_lowercase`. _(notes: shared by mirror and move; uses stdout (not stderr) for prompt — could clobber piped output, but `--yes` exists)_
- **mirror-prompt** — `crates/blit-cli/src/transfers/mod.rs:181-190` — mirror prompts unless `--yes` OR `--dry-run`; bypassed string is "Mirror will delete extraneous files at destination 'X'. Continue?". _(notes: dry-run does NOT need confirmation; prompt happens BEFORE rejected-flag gates if mirror is set)_
- **move-prompt** — `crates/blit-cli/src/transfers/mod.rs:408-418` — move always prompts unless `--yes`; prompt happens AFTER all reject-gates. _(notes: not gated on dry-run because dry-run bails earlier)_
- **rm-prompt** — `crates/blit-cli/src/rm.rs:48-58` — `rm` prompts "Delete X on Y? [y/N]" unless `--yes`; same accept-vocabulary (y/yes). _(notes: separate impl from `confirm_destructive_operation` — duplicated logic)_

### state-machine

- **jobs-watch-stream-first** — `crates/blit-cli/src/jobs.rs:194-211` — subscribe FIRST, then GetState — closes the race where transfer drains between snapshot and subscribe registration. _(notes: documented "c-6 round 2"; relies on broadcast.replay_recent=true)_
- **jobs-watch-snapshot-branches** — `crates/blit-cli/src/jobs.rs:219-268` — three branches off initial GetState: Finished → emit + return (ok/err), NotFound → emit + return 2, Active → emit + cache snapshot + fall through to stream loop. _(notes: caches `ActiveSnapshot` for merging terminal events into Finished JSON shape — c-6 schema parity)_
- **jobs-watch-event-loop** — `crates/blit-cli/src/jobs.rs:270-367` — stream-message loop: TransferProgress → progress line/json; TransferComplete → terminal + exit 0; TransferError → terminal + exit 1; TransferStarted/None → ignored; Err → reconcile via GetState; Ok(None) → reconcile. _(notes: payload `None` is a future-proof catch)_
- **jobs-watch-reconcile-fallback** — `crates/blit-cli/src/jobs.rs:373-413` — on stream error/end, do one more GetState; Active outcome → exit 3 (gave-up-watching) without polling further. _(notes: hardcoded "exit 3 = timeout-equivalent" rather than retry/poll)_
- **jobs-watch-deadline** — `crates/blit-cli/src/jobs.rs:270-306` — wraps each `stream.message()` in `tokio::time::timeout(remaining, ...)`; exit code 3 + `state: "timeout"` JSON when fired. _(notes: deadline=0 means wait forever)_

### timeout-or-retry

- **run-with-retries-driver** — `crates/blit-cli/src/main.rs:46-63` — `run_with_retries(args.retry, wait, |_n| run_transfer(...))` for Copy/Mirror/Move from `blit_app::transfers::retry`. _(notes: ignores attempt number; resumes per docs because transfers are resumable)_
- **jobs-watch-poll-deprecated** — `crates/blit-cli/src/cli.rs:118-124` — `--interval-ms` (default 1000) preserved on CLI but has no effect under streaming Subscribe; doc-commented. _(notes: dead flag; smells)_

### render-or-display

- **collapse-slashes-display** — `crates/blit-cli/src/transfers/mod.rs:54-72` — `display_endpoint` collapses `//+` to `/` in local-path display only; remote endpoints use `format_remote_endpoint`. _(notes: display-only; actual path handling unchanged — rsync trailing-slash semantics still apply)_
- **starting-banner-stderr** — `crates/blit-cli/src/transfers/mod.rs:194-203` — "starting copy SRC -> DST" goes to stderr; suppressed under `--json`; verbose also prints resolution-changed hint. _(notes: stdout reserved for summary/json)_
- **move-banner** — `crates/blit-cli/src/transfers/mod.rs:420-428` — same pattern for move. _(notes: banner repeated; duplicated structure)_
- **throughput-noise-suppression** — `crates/blit-cli/src/transfers/local.rs:205-291` — throughput/workers line suppressed when bytes < 1 MiB AND copied_files ≤ 1 AND not verbose. _(notes: hardcoded `THROUGHPUT_LINE_MIN_BYTES = 1024 * 1024`)_
- **outcome-branches** — `crates/blit-cli/src/transfers/local.rs:234-257` — local summary distinguishes JournalSkip / UpToDate / SourceEmpty / Transferred; first three short-circuit print. _(notes: previously all collapsed into one print masking bugs)_
- **scan-port-elision** — `crates/blit-cli/src/scan.rs:63-67` — endpoint formatting drops `:9031` (default port) → `host://`, otherwise `host:port://`. _(notes: hardcoded 9031 magic number)_
- **scan-module-truncation** — `crates/blit-cli/src/scan.rs:85-99` — three branches when `module_count > modules.len()` (truncated TXT vs names-not-advertised vs full list). _(notes: handles compact-daemon variant)_
- **jobs-human-recent-newest-first** — `crates/blit-cli/src/jobs.rs:723-740` — recent[] is wire oldest-first; human output iterates `.rev()` so newest is on top. _(notes: silent transformation between wire and human; JSON path preserves wire order)_
- **age_ms_since-uses-systemtime** — `crates/blit-cli/src/jobs.rs:785-792` — age computation uses `SystemTime::now()` minus `start_unix_ms`; `saturating_sub` to avoid negative. _(notes: clock-skew sensitive)_
- **format_ms / format_uptime** — `crates/blit-cli/src/jobs.rs:764-783` — ms < 1000 → "Xms", else "X.Ys"; uptime > 0 hours → "Xh Ym", else "Xm Ys" / "Xs". _(notes: tested; no microseconds support)_
- **format_bps** — `crates/blit-cli/src/jobs.rs:471-481` — decimal scaling (1000/1000_000/...) not binary (1024). _(notes: inconsistent with format_bytes elsewhere which uses binary — see code-core-io for `format_bytes`)_

### format-output

- **transferargs-summary-paths** — `crates/blit-cli/src/transfers/local.rs:46-71, 207-316` — local summary JSON includes operation/source/destination/files_transferred/files_examined/total_bytes/deleted_files/deleted_dirs/duration_ms/dry_run/outcome. Text summary varies by outcome and verbose. _(notes: `outcome` string mapping {transferred, journal_skip, up_to_date, source_empty} parallel to text-branch labels)_
- **push-summary-json** — `crates/blit-cli/src/transfers/remote.rs:474-488` — push JSON has operation/destination/files_requested/files_transferred/bytes_transferred/bytes_zero_copy/entries_deleted/tcp_fallback/first_payload_ms. _(notes: parallel shape to pull-json but distinct fields — destination as plain string not endpoint shape)_
- **pull-summary-json** — `crates/blit-cli/src/transfers/remote.rs:447-472` — pull JSON adds mirror_purge nested object {files_deleted, dirs_deleted}; R46-F6 requires same-document not appended-text. _(notes: pull summary uses report.summary.files_transferred with fallback to top-level field — defensive against older daemons)_
- **delegated-summary-json** — `crates/blit-cli/src/transfers/remote_remote_direct.rs:235-253` — delegated json: operation=delegated_pull, source/destination/files_transferred/.../source_peer_observed. _(notes: distinct "operation" string from push/pull — schemas not unified)_
- **detach-json** — `crates/blit-cli/src/transfers/remote_remote_direct.rs:224-233` — detached prints `{"outcome":"detached","transfer_id":...}`. _(notes: minimal envelope; doesn't include `dst_host_hint` so JSON consumers must derive it)_
- **detach-human-hint** — `crates/blit-cli/src/transfers/remote_remote_direct.rs:218-222` — human output shows `cancel: blit jobs cancel HOST ID` and `status: blit jobs list HOST` lines. _(notes: HOST derived via `host_port_display` to handle IPv6/port — pre-fix comment notes that string-splitting `args.destination` broke for those cases)_
- **jobs-watch-progress-json** — `crates/blit-cli/src/jobs.rs:483-497` — progress per event: state="progress" + bytes_completed/bytes_total/files_completed/files_total/throughput_bps. _(notes: NDJSON one-per-line via println!)_
- **jobs-watch-snapshot-json** — `crates/blit-cli/src/jobs.rs:506-541` — three states: active/finished/not_found with full TransferRecord shape on finished. _(notes: c-6 stream→finished synthesis ensures schema parity)_
- **jobs-watch-timeout-json** — `crates/blit-cli/src/jobs.rs:548-558` — special `state="timeout"` line emitted before exit 3. _(notes: stream consumers rely on a terminal state row)_
- **jobs-cancel-json** — `crates/blit-cli/src/jobs.rs:560-584` — three outcomes: cancelled/not_found/unsupported with transfer_id (and message for unsupported). _(notes: pretty-printed)_
- **jobs-list-json** — `crates/blit-cli/src/jobs.rs:606-674` — assembles full DaemonState JSON: version/uptime/delegation/modules/active/recent/counters; counters nested-optional. _(notes: `counters` reflects whether daemon emitted them — see project memory "GetState Counters" hazard)_
- **rm-json** — `crates/blit-cli/src/rm.rs:62-76` — inline struct serialized as path/host/port/entries_deleted. _(notes: nested struct definition inside if-block; duplicates fields available from RemoteEndpoint)_
- **diagnostics-dump-json** — `crates/blit-cli/src/diagnostics.rs:176-197` — single big JSON: blit_version/invocation/source/destination/rsync_resolution/same_device. _(notes: invocation captures `std::env::args()` so secrets in argv land in the dump)_
- **profile-json** — `crates/blit-cli/src/profile.rs:8-22` — emits enabled/records/predictor_path/predictor with nested copy/mirror coefficient blocks. _(notes: predictor null when no coefficients learned yet)_
- **find-json** — `crates/blit-cli/src/find.rs:41-48` — collects entries into Vec, then pretty-prints; non-streaming. _(notes: large result sets buffered fully before output)_
- **du-json** — `crates/blit-cli/src/du.rs:22-29` — same Vec-collect pattern. _(notes: same buffering caveat)_
- **df-json** — `crates/blit-cli/src/df.rs:20-21` — single object from `df::query`. _(notes: thinnest verb)_

### data-plane

- **progress-monitor-spawn** — `crates/blit-cli/src/transfers/remote.rs:33-164` — async task receives `ProgressEvent` (ManifestBatch / Payload / FileComplete), keeps rolling 1-sec ticker, emits human or NDJSON-to-stderr. _(notes: NDJSON event names: file_complete, progress, manifest, final; manual JSON formatting with escape-only-for-`"`/`\\`)_
- **progress-monitor-final-line-gate** — `crates/blit-cli/src/transfers/remote.rs:134-160` — `suppress_final_line=true` (deferred/move) skips the post-transfer "final" line so a move source-delete failure doesn't show success. _(notes: R53-F1; per-file lines still emit during transfer for liveness)_
- **progress-lifecycle-ordering** — `crates/blit-cli/src/transfers/remote.rs:393-417` — pull lifecycle: PullSync RPC → drop progress handle + await monitor → apply mirror-purge → print. _(notes: round-2 fix; round-1 had purge during live monitor causing stale ticks)_
- **push-progress-no-postlude** — `crates/blit-cli/src/transfers/remote.rs:252-262` — push has no caller-side destructive step; monitor lifetime matches the RPC. _(notes: asymmetry vs pull's separate purge step is intentional — daemon handles purge for push)_
- **delegated-detach-flow** — `crates/blit-cli/src/transfers/remote_remote_direct.rs:146-189` — detach path drops progress monitor early, calls `run_delegated_pull_until_started`, prints detach hint with `host_port_display`-derived host (preserves port + IPv6 brackets). _(notes: synthesizes zero-summary outcome to satisfy callers)_
- **delegated-started-callback** — `crates/blit-cli/src/transfers/remote_remote_direct.rs:194-204` — verbose-human prints "[delegation] destination pulling from PEER (N stream(s))" via closure callback. _(notes: stopgap until M-C `AppProgressEvent` reshape)_

### safety-check

- **require-complete-scan-pushed-on-mirror** — `crates/blit-cli/src/transfers/remote.rs:240-250` — push sets `require_complete_scan: mirror_mode` so mirror won't proceed on partial source enumeration. _(notes: copy/move have separate require_complete_scan thread)_
- **require-complete-scan-pull-deferred** — `crates/blit-cli/src/transfers/remote.rs:386-387` — pull pulls `require_complete_scan` from caller arg; move callers set true, copy/mirror set false. _(notes: move is the data-loss-critical caller)_
- **deferred-output-for-move** — `crates/blit-cli/src/transfers/local.rs:30-38, 73-136, 446-503` — move calls `_deferred` variant; success summary printed only after source-delete succeeds. _(notes: R49-F3 / R51-F4 / R53-F1 — pattern repeats across local/remote/delegated three paths)_
- **move-pre-delete-unreadable-gate** — `crates/blit-cli/src/transfers/mod.rs:455-480` — local-local move refuses source delete when `summary.unreadable_paths` non-empty; prints first 5. _(notes: R47-F4; mirror's R46-F2 gate covers mirror=true only — move uses mirror=false so needs its own gate)_
- **check-exists-gate** — `crates/blit-cli/src/check.rs:24-29` — bails when src or dst doesn't exist. _(notes: bypasses spawn_blocking; pre-flight)_

### spawn-task

- **check-spawn-blocking** — `crates/blit-cli/src/check.rs:47-51` — `compare_trees` lifted into `spawn_blocking`; result awaited; panic context "check task panicked". _(notes: only verb that uses spawn_blocking explicitly)_
- **progress-monitor-task** — `crates/blit-cli/src/transfers/remote.rs:45-161` — `tokio::spawn`ed select-loop with biased channel + 1s ticker. _(notes: drop handle to signal end; task awaited after monitor handle drop)_

### persistence

- **perf-history-cache-refresh** — `crates/blit-cli/src/diagnostics.rs:32-39` — after enable/disable/clear, attempts `perf::read_enabled()`; on Err keeps loaded value. _(notes: comment says malformed settings.json shouldn't block the verb)_
- **perf-enable/disable** — `crates/blit-cli/src/diagnostics.rs:12-30` — `--enable` and `--disable` (conflicts_with each other), and `--clear` (independent). _(notes: clap precedence enforced via conflicts_with at struct level)_
- **appcontext-perf-default-enabled** — `crates/blit-cli/src/context.rs:9-22` — on settings read failure, default is `perf_history_enabled = true`. _(notes: opt-out posture for perf history; user-facing warn message)_

### config-load

- **config-dir-override** — `crates/blit-cli/src/main.rs:39-41` — `--config-dir` global flag sets `blit_core::config::set_config_dir` before AppContext loads. _(notes: must precede AppContext::load — ordering matters)_

### default-value

- **scan-default-wait** — `crates/blit-cli/src/cli.rs:392-395` — `blit scan --wait` defaults to 2 seconds for mDNS. _(notes: hardcoded magic time)_
- **jobs-list-recent-zero** — `crates/blit-cli/src/cli.rs:102-106` — `--recent-limit 0` means daemon default (50). _(notes: 0=sentinel, value 50 commented but not enforced CLI-side)_
- **jobs-watch-defaults** — `crates/blit-cli/src/cli.rs:119-129` — `--interval-ms` 1000, `--timeout-secs` 0 (forever). _(notes: interval is dead; timeout-0-means-forever is a UX choice that can hang scripts)_
- **profile-default-limit** — `crates/blit-cli/src/cli.rs:530-535` — `--limit` defaults to 50; diagnostics perf same. _(notes: matches jobs-list semantics for `recent-limit` value but not for sentinel)_

### discovery

- **scan-mdns-discover** — `crates/blit-cli/src/scan.rs:26-105` — calls `blit_app::scan::discover(Duration)`; renders modules + delegation_enabled hint when present. _(notes: text output strips trailing-dot hostname; default-port elision via hardcoded 9031)_

### naming

- **clap-shell-completion-test-coverage** — `crates/blit-cli/src/cli.rs:638-670` — explicit list of verbs tested in bash output: copy/mirror/move/scan/list/find/completions. _(notes: hardcoded subset; new verbs would need test update — `ls`, `du`, `df`, `rm`, `jobs`, `check`, `diagnostics`, `profile`, `list-modules` are NOT in the verified list)_
- **completions-shell-bin-name** — `crates/blit-cli/src/completions.rs:42-47` — hardcoded `bin_name = "blit"` even though crate is `blit-cli`. _(notes: matches `[[bin]] name = "blit"` in Cargo.toml — change one without the other and completions break)_

### error-propagation

- **check-exit-codes** — `crates/blit-cli/src/check.rs:59-68` — errors → 2; differing/missing → 1 (one_way ignores missing_on_src); else 0. _(notes: documented 0/1/2 contract; `check` is one of two verbs that `main.rs` propagates ExitCode directly)_
- **jobs-cancel-exit-codes** — `crates/blit-cli/src/jobs.rs:60-66` — Cancelled→0 / NotFound→1 / Unsupported→2; mapping pulled out as sync helper for unit tests. _(notes: §6.5 of TUI_DESIGN doc)_
- **jobs-watch-exit-codes** — `crates/blit-cli/src/jobs.rs:139-168, 226-243, 286-301, 329, 338, 387-400, 409-410` — finished-ok→0 / finished-fail→1 / not-found→2 / timeout→3 / stream-loss-active→3. _(notes: 4 exit codes; the "stream-loss-active = 3" overload mixes "we gave up" with "deadline fired")_
- **other-verbs-default-zero** — `crates/blit-cli/src/main.rs:73-89` — Copy/Mirror/Move/Scan/Ls/Du/Df/Rm/Find/Completions/Profile/Diagnostics return Ok(()) and main returns SUCCESS. _(notes: only check + jobs cancel/watch have semantic exit codes)_

### rpc-handler

- **completions-remote-rpc** — `crates/blit-cli/src/completions.rs:73-85` — `connect_with_timeout` → `complete_path(CompletionRequest{module, path_prefix, include_files, include_directories})`; `status.message().to_string()` becomes the eyre error. _(notes: tonic Status loses code/details going to eyre)_
- **find-streaming** — `crates/blit-cli/src/find.rs:42-61` — `find::stream` with callback for each FindEntry; human path emits header row + per-entry lines. _(notes: streaming on the wire but JSON path collects into Vec — JSON not actually streaming)_
- **du-streaming** — `crates/blit-cli/src/du.rs:23-43` — `du::stream` same callback pattern. _(notes: same JSON-buffer caveat)_
- **df-query** — `crates/blit-cli/src/df.rs:18` — `df::query` returns single FsStats. _(notes: no streaming because it's one-shot)_
- **list-modules-query** — `crates/blit-cli/src/list_modules.rs:18` — `list_modules::query` returns Vec. _(notes: small payload)_
- **rm-purge-RPC** — `crates/blit-cli/src/rm.rs:60` — `rm::purge` takes a Vec but `blit rm` always sends a single-path Vec. _(notes: API is plural, caller is singular — possible misuse if caller drifts)_

## Smells / risks observed

- **--interval-ms is dead flag** — `crates/blit-cli/src/cli.rs:119-124` — preserved for backward compat per comment but does nothing under the streaming Subscribe model. UX hazard: users adjust it and observe no behavior change.

- **format_bps vs format_bytes unit mismatch** — `crates/blit-cli/src/jobs.rs:471-481` uses **decimal** (1000/1_000_000), whereas `blit_app::display::format_bytes` (used in `local.rs`, `df.rs`, `ls.rs`) uses **binary** (1024). Two distinct meanings for `MB`/`GB` in the same tool's output, depending on which verb you ran.

- **hardcoded mDNS default port `9031`** — `crates/blit-cli/src/scan.rs:63` — magic literal; if daemon default ever changes, scan elides the wrong port silently. Should be a constant in a shared place.

- **--delete-scope is stringly typed** — `crates/blit-cli/src/cli.rs:247-248, 386-388` — `value_parser` is case-sensitive but `delete_scope_all()` is case-insensitive: `blit copy --delete-scope ALL` would be rejected by clap, but `blit copy --delete-scope all` accepted; meanwhile internal code accepts `All` too if it ever bypassed clap. Should be an enum.

- **TransferArgs is a god-struct** — `crates/blit-cli/src/cli.rs:188-364` — 30+ flags shared across Copy/Mirror/Move where many combinations are illegal. The reject-gates in `transfers/mod.rs` cover the obvious data-loss cases, but new flags risk silently being accepted across verbs that shouldn't expose them.

- **invocation field in diagnostics dump leaks argv** — `crates/blit-cli/src/diagnostics.rs:178` — `std::env::args().collect()` ends up in the JSON. If a user paste-passes a password via `--config-dir` (or future secret flag), it lands in the dump. No redaction.

- **--null + local-source push silently safe** — `crates/blit-cli/src/transfers/mod.rs:143-154` rejects --null with remote endpoints citing "silently ignored", but the wording in the bail message could mislead users about whether local→local is safe — needs careful UX read.

- **Rm purge sends single-path Vec** — `crates/blit-cli/src/rm.rs:60` — the underlying RPC takes a Vec but every caller is single-path; if the RPC adds batching semantics the CLI is locked into single behavior.

- **list-modules uses RemoteEndpoint::parse directly** — `crates/blit-cli/src/list_modules.rs:7-9` — diverges from `parse_endpoint_or_local` pattern used everywhere else (du/df/find/ls/rm/completions). A `blit list-modules /local/path` shows a different error message than `blit du /local/path`.

- **Duplicated confirm-prompt logic** — `crates/blit-cli/src/transfers/mod.rs:87-99` defines `confirm_destructive_operation`; `crates/blit-cli/src/rm.rs:48-58` reimplements it inline with the same y/yes vocabulary. Risk of divergence (e.g. one accepts "Y" vs both accepting "y").

- **Confirm prompt writes to stdout, not stderr** — `crates/blit-cli/src/transfers/mod.rs:93-94` and `crates/blit-cli/src/rm.rs:49` — `print!` to stdout could interleave with piped output of an automation chain. Most CLIs write prompts to /dev/tty or stderr.

- **jobs-list `counters` may publish present-but-zero** — `crates/blit-cli/src/jobs.rs:643-651` JSON includes counters when `state.counters.is_some()` — per project memory `feedback_getstate_counters_zero`, daemon emits Some even with metrics-off, producing false zeros to JSON consumers.

- **Hardcoded `THROUGHPUT_LINE_MIN_BYTES = 1024 * 1024`** — `crates/blit-cli/src/transfers/local.rs:205` — magic constant for "is the throughput line meaningful"; not configurable.

- **Bin name `blit` hardcoded twice** — `crates/blit-cli/src/completions.rs:44` AND `Cargo.toml:13`. Renaming requires both. clap's `Cli::command().get_name()` could be used as the single source of truth.

- **No TODO/FIXME/HACK markers and no cfg(windows)/cfg(unix) divergence** in any blit-cli/src file — clean from those classes, but suggests platform-specific quirks (if any) live in blit-core/blit-app dependencies.

- **No references to deprecated features** (blit-utils, BlitAuth, AI telemetry) in CLI surface — consistent with the rewritten state.

- **`--retry`/`--wait` have no upper bound** — `crates/blit-cli/src/cli.rs:264-272` — `--retry 4294967295 --wait 18446744073709551615` would be accepted by clap. Practically a script footgun.

- **JSON consumers see two distinct schemas for the same outcome** — push reports `files_requested.len()` (Vec length); pull reports `files_transferred` from optional `summary` (with fallback to top-level `report.files_transferred`). The defensive double-read in `print_pull_json:464` suggests historical schema drift.

## Coverage attestation

| File | Lines read | Notes |
| --- | --- | --- |
| `crates/blit-cli/src/main.rs` | 90 | Verb dispatch + ExitCode propagation |
| `crates/blit-cli/src/cli.rs` | 696 | All clap structs + clap-surface tests |
| `crates/blit-cli/src/jobs.rs` | 923 | List/Cancel/Watch + JSON + format helpers + tests |
| `crates/blit-cli/src/transfers/mod.rs` | 827 | `run_transfer`/`run_move` + all flag-reject gates + tests |
| `crates/blit-cli/src/transfers/remote.rs` | 577 | Push/Pull orchestration + progress monitor + deferred-output |
| `crates/blit-cli/src/transfers/local.rs` | 351 | Local transfer wrapper + summary printers |
| `crates/blit-cli/src/transfers/remote_remote_direct.rs` | 285 | Delegated pull + detach flow |
| `crates/blit-cli/src/transfers/endpoints.rs` | 27 | Clap-arg adapters for the support-gate helpers |
| `crates/blit-cli/src/diagnostics.rs` | 286 | `perf` + `dump` subverbs |
| `crates/blit-cli/src/check.rs` | 120 | `blit check` + exit-code policy |
| `crates/blit-cli/src/scan.rs` | 106 | mDNS discovery + JSON/text |
| `crates/blit-cli/src/ls.rs` | 100 | Local + remote listing + smart-dispatch |
| `crates/blit-cli/src/completions.rs` | 96 | Shell + remote completion |
| `crates/blit-cli/src/rm.rs` | 92 | Remote rm + prompt |
| `crates/blit-cli/src/profile.rs` | 92 | Performance predictor display |
| `crates/blit-cli/src/find.rs` | 65 | Remote find streaming |
| `crates/blit-cli/src/du.rs` | 46 | Remote du streaming |
| `crates/blit-cli/src/df.rs` | 42 | Remote df one-shot |
| `crates/blit-cli/src/list_modules.rs` | 33 | List modules shared helper |
| `crates/blit-cli/src/context.rs` | 23 | AppContext (perf cache) |
| `crates/blit-cli/Cargo.toml` | 43 | Bin-name override, deps |

**Total lines read**: 4,920 (4,877 .rs + 43 Cargo.toml)
**Files NOT read** (with reason): (none in scope)
