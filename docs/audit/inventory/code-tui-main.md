# Code Inventory: blit-tui main.rs (event loop + dispatch + render orchestration)
**Generated**: 2026-06-04 by audit workflow
**Coverage**: `crates/blit-tui/src/main.rs` — 10838 lines (read end-to-end via offset windows)

## Behaviors (grouped by category)

### state-machine
- **app-state-struct** — `crates/blit-tui/src/main.rs:163-307` — `AppState` aggregates every pane's substate (dual_pane, daemons, transfers, browse, profile, verify, diagnostics, f1_trigger, f1_push, f3_pull, f3_du, f3_del, f3_batch_pull, cancel_status) plus mpsc senders for each background-task reply channel. Receivers stay in `run_router` local scope.
- **f2-cancel-status-enum** — `crates/blit-tui/src/main.rs:337-415` — `F2CancelStatus` has 7 variants: Idle, Confirming{transfer_id, daemon}, ConfirmingBatch{targets}, ConfirmingClearRecent, Sending{transfer_id, request_id}, BatchInitiated{count, finished_at}, Done{outcome, finished_at}, Error{transfer_id, message, finished_at}. `targets` is frozen at prompt creation (d-30 R2 race fix).
- **reload-banner-ttl** — `crates/blit-tui/src/main.rs:311-328` — `ReloadBanner::TTL = 4s`. Banner color green=ok, red=parse-error.
- **batch-pull-struct** — `crates/blit-tui/src/main.rs:3068-3078` — `BatchPull{remaining, raw_dest, done, total}`; advanced on each successful pull reply. Aborts whole batch on any error or blank-dest re-launch _(note: dest captured ONCE on first commit; subsequent sources reuse it)_.
- **trigger-outcome-enum** — `crates/blit-tui/src/main.rs:3541-3553` — `TriggerOutcome::{Launched, NeedsConfirm, Rejected(String)}` returned by `plan_f1_trigger` / `plan_f1_delegated`.
- **confirmed-cancel-enum** — `crates/blit-tui/src/main.rs:3827-3833` — `ConfirmedCancel::Single{id, daemon}` / `Batch(Vec<(String, String)>)`; clone-out moved outside the `match &app.cancel_status` arm to avoid borrow-lifetime issues.

### key-dispatch
- **router-key-priority-chain** — `crates/blit-tui/src/main.rs:1137-1332` — Keystroke handling priority: 1) help overlay (when visible, absorb everything except `?`/Esc/Ctrl-c, j/k/arrows scroll); 2) `esc_cancels_confirm`; 3) F4 profile clear confirm modal; 4) F4 verify edit handler; 5) F1 trigger destructive confirm; 6) F1 trigger editing modal; 7) F3 filter editing; 8) F3 pull-dest prompt; 9) F3 destructive (mirror/move) confirm; 10) F3 delete confirm; 11) `key_action` → UserAction → dispatch.
- **key-action-fn** — `crates/blit-tui/src/main.rs:5141-5363` — Pure mapping of `KeyEvent` → `Option<UserAction>`. Order: quit predicate → Ctrl+R reload → F-keys 1..4 → keymap.nav digit aliases → keymap.refresh → keymap.movement → static char mapping. Plain-press-only for nav/refresh/movement aliases (Ctrl/Alt fall through). _(note: `c`/`d`/`e`/`s` are F4-only but in the GLOBAL char match — only F4 dispatch acts on them, other panes silently ignore. d-19 added digits 1-4 as F-key aliases for terminals dropping F-keys.)_
- **is-quit-predicate** — `crates/blit-tui/src/main.rs:5766-5776` — Configured quit char only fires on plain press (no Ctrl/Alt) so `quit = "r"` won't steal Ctrl+R. Esc and Ctrl+C are unconditional failsafes _(note: keys-1 R2 hardening)_.
- **keymap-struct** — `crates/blit-tui/src/main.rs:5733-5760` — `KeyMap{quit: KeyCode, refresh: Option<KeyCode>, nav: [Option<KeyCode>; 4], movement: [Option<KeyCode>; 4]}`. Built per-keystroke from hot-reloadable config. `None` slots mean a collision-disabled binding.
- **user-action-enum** — `crates/blit-tui/src/main.rs:4989-5126` — 30+ UserAction variants. Notable mappings: `t`=F1TriggerBegin, `K`=CancelSelectedTransfer, `X`=CancelAllActiveTransfers, `E`=ClearRecent, `p`=F3PullBegin, `P`=F3BatchPullBegin, `m`=F3MirrorBegin, `v`=F3MoveBegin, `u`=F3DuBegin, `D`=F3DeleteBegin, `space`=F3ToggleMark, `a`=F3ToggleMarkAll, `/`=F3FilterBegin, `C`=TransferCopy, `M`=TransferMirror, `V`=TransferMove, `H`=ToggleVerifyChecksum, `O`=ToggleVerifyOneWay, `c`/`d`/`e`=ProfileClear/Disable/Enable, `s`=DiagnosticsDump, `?`=ToggleHelp, `Ctrl+R`=ReloadConfig, `Tab`=DualSwitchPane.

### confirmation-prompt
- **f1-trigger-destructive-confirm** — `crates/blit-tui/src/main.rs:4405-4440` — F1 push mirror/move confirm: `y`/`Y` re-calls `plan_f1_trigger(confirmed=true)`; `n`/`N`/`Esc` calls `f1_trigger.cancel_confirm()`. Modal swallows everything else; `?`/Ctrl-c/F-keys bubble.
- **f1-trigger-editing-modal** — `crates/blit-tui/src/main.rs:4494-4558` — Editing modal: Tab=toggle focus, Up/Down cycle kind (copy→mirror→move), Backspace, chars insert, Enter commits (calls `plan_f1_trigger`), Esc cancels.
- **f3-pull-dest-prompt** — `crates/blit-tui/src/main.rs:4623-4704` — F3 pull destination editor: chars insert, Backspace, Enter commits (auto-appends `/` for batch dest if missing — d-53 R2), Esc cancels (also clears batch).
- **f3-delete-confirm** — `crates/blit-tui/src/main.rs:4713-4747` — Modal: `y`/`Y` calls `f3_del.confirm()` → spawns Purge; `n`/`N`/`Esc` cancels; all other keys swallowed.
- **f3-destructive-confirm** — `crates/blit-tui/src/main.rs:4756-4792` — Mirror/move post-prompt confirm: `y`/`Y` calls `f3_pull.confirm_destructive()` → spawns pull; `n`/`N`/`Esc` cancels.
- **profile-clear-confirm** — `crates/blit-tui/src/main.rs:4448-4481` — F4 destructive history clear: `y`/`Y` runs `apply_profile_clear()`; `n`/`N`/`Esc` cancels. Re-fetches profile only on Ok (avoids hiding the Error banner).
- **verify-edit-handler** — `crates/blit-tui/src/main.rs:4313-4398` — F4 Verify form edit: Esc=clear focus, Tab=cycle focus, Enter=spawn verify run if can_run, Backspace+chars edit field. Ctrl-U clears focused field. Editing invalidates any pending mirror-confirm prompt _(d-4 R2)_.
- **esc-cancels-confirm-predicate** — `crates/blit-tui/src/main.rs:4009-4015` — Bare Esc (no Ctrl/Alt) while `transfer.is_confirming()` OR `cancel_status.is_confirming()` returns true. Runs BEFORE verify-edit handler and `key_action` (where bare Esc would map to Quit).
- **f3-filter-edit** — `crates/blit-tui/src/main.rs:4566-4611` — F3 `/` filter editor: chars push, Backspace pop, Enter commit, Esc cancel.
- **f2-cancel-confirm-clear-recent** — `crates/blit-tui/src/main.rs:1788-1810` — `E` arms `ConfirmingClearRecent`; on `y`, clears recent list + fans `ClearRecent` RPC to each watched daemon. Ignored mid-cycle.

### spawn-task
- **spawn-input-task** — `crates/blit-tui/src/main.rs:4916-4981` — Single-owner spawn_blocking task wrapping crossterm poll/read. Accepts Press + Repeat, drops Release. Optional `BLIT_TUI_INPUT_TRACE=1` env writes JSONL to `/tmp/blit-tui-input.log` _(note: hardcoded path)_.
- **spawn-discovery-task** — `crates/blit-tui/src/main.rs:4850-4889` — mDNS loop on `tokio::time::interval` cadence (`F1_DISCOVERY_INTERVAL = 5s`). `set_missed_tick_behavior(Skip)`. Refresh-trigger channel breaks out of interval wait; resets ticker after manual scan.
- **spawn-f2-setup-task** — `crates/blit-tui/src/main.rs:2447-2493` — Fans out Subscribe + initial GetState to each watched daemon. Iterates sequentially (no concurrency). A daemon failing to subscribe is skipped; all-failed returns `Failed`. Sets `any_subscribed` flag. Bounded by `SUBSCRIBE_OPEN_TIMEOUT` (30s) and `SNAPSHOT_FETCH_TIMEOUT` (30s).
- **spawn-detail-fetch** — `crates/blit-tui/src/main.rs:2683-2718` — GetState + per-module `df` capacity fan-out (sequential). `df` failure for any module omits its capacity but doesn't fail the request.
- **kick-browse-fetch** — `crates/blit-tui/src/main.rs:2838-2872` — Per-view fetcher: `list_modules` for Modules view, `ls::list_remote(name, path.join("/"))` for Module view. Generation-stamped reply via `is_current_request`.
- **spawn-dual-pane-fetch** — `crates/blit-tui/src/main.rs:2959-3011` — Local listing via `spawn_blocking`; remote modules via `list_modules::query`; remote listing via `ls::list_remote`. Replies tagged with `(pane_id, request_id)`.
- **spawn-profile-fetch** — `crates/blit-tui/src/main.rs:3028-3045` — Spawn_blocking wrap of sync `profile::query(0)` (no limit).
- **spawn-f3-pull** — `crates/blit-tui/src/main.rs:3238-3354` — Forks a progress forwarder + main pull task. After receive: Copy→done; Mirror→`apply_pull_mirror_purge(force=true)`; Move→extracts `(module, rel_path)`, builds wire path via `del_wire_path`, calls `delete_remote_path`. Progress forwarder closes BEFORE destructive phase (drop progress → await forwarder → destructive).
- **spawn-f1-push** — `crates/blit-tui/src/main.rs:3361-3433` — Same shape: forwarder + push task. Move=delete local source after push lands. Mirror sets `mirror_mode=true` + `require_complete_scan=true` (the safety gate per `feedback-port-cli-safety-guards`).
- **spawn-f1-delegated-pull** — `crates/blit-tui/src/main.rs:3461-3538` — Remote→remote delegated transfer (destination daemon pulls from source daemon, CLI not in byte path). Move=delete remote source after copy.
- **spawn-f3-del** — `crates/blit-tui/src/main.rs:3118-3130` — Wraps `run_f3_del` which calls `rm::extract_module_and_path` then `rm::purge`.
- **spawn-f3-du** — `crates/blit-tui/src/main.rs:3149-3156` — Wraps `run_f3_du_total` with `F3_DU_MAX_DEPTH = 1` (compile-time guarded: `0` is unbounded in daemon).
- **spawn-cancel-transfer** — `crates/blit-tui/src/main.rs:3899-3917` — Thin wrap of `jobs::cancel(endpoint, transfer_id)`.
- **spawn-cancels-for-targets** — `crates/blit-tui/src/main.rs:3876-3892` — One CancelJob per (daemon, id); skips targets whose daemon identity won't parse.
- **spawn-clear-recent** — `crates/blit-tui/src/main.rs:3926-3930` — Fire-and-forget `jobs::clear_recent(endpoint)`; result intentionally dropped _(note: per-daemon outcome not surfaced; cleared locally first for responsiveness, transient unreachable daemon's stale rows may reappear on next snapshot)_.
- **spawn-diagnostics-dump** — `crates/blit-tui/src/main.rs:4211-4232` — Spawn_blocking wrap of `run_diagnostics_dump`.
- **spawn-local-transfer** — `crates/blit-tui/src/main.rs:4076-4101` — Calls `blit_app::transfers::local::run` with `mirror` flag and `perf_history` read fresh at call time via `blit_core::perf_history::perf_history_enabled().unwrap_or(true)`.
- **spawn-local-move** — `crates/blit-tui/src/main.rs:4121-4196` — Local move = copy + R47-F4 unreadable-paths gate + source delete. `perform_local_move` is the async core.
- **spawn-verify-run** — `crates/blit-tui/src/main.rs:4804-4837` — Spawn_blocking wrap of `compare_trees(src, dst, use_checksum, one_way, FileFilter::default())`.

### rpc-handler
- **open-subscribe-stream** — `crates/blit-tui/src/main.rs:5626-5653` — Awaits `jobs::subscribe(endpoint, "", false)` inline under outer `SUBSCRIBE_OPEN_TIMEOUT` (30s). Sends Connected event immediately on success. Forwarder runs in detached task via `forward_subscribe_stream`.
- **forward-step** — `crates/blit-tui/src/main.rs:5676-5685` — `tokio::select! { biased; _ = tx.closed() => Closed, msg = next_msg => Message(msg) }`. Audit-8 fix: a silent daemon won't keep forwarder alive after merged_rx is dropped.
- **forward-subscribe-stream** — `crates/blit-tui/src/main.rs:5691-5726` — Loop over `forward_step`. On Ok(None) sends "stream ended"; on Err sends "stream: {status.message()}".
- **fetch-snapshot-within** — `crates/blit-tui/src/main.rs:2429-2445` — Outer-timeout wrapper for `jobs::query`. Stalled GetState degrades to `Err` snapshot rather than hanging setup task (audit-8 R2).
- **refan-f2-setup** — `crates/blit-tui/src/main.rs:2298-2332` — `r` refresh path. Reconciles `transfers.retain_active_daemons(&watched_ids)` BEFORE dropping receiver. Clears `f2_degraded_daemons`. Empty watch set → NoRemote status, no respawn.
- **reset-f2-for-resubscribe** — `crates/blit-tui/src/main.rs:2786-2808` — d-47/d-48: F1 Enter on a daemon row repoints F2. Clears transfers, repoints `parsed_remote`/label, drops `cancel_status` to Idle (d-48 R2: stale confirm against old daemon could fire CancelJob with wrong ids).
- **cancel-endpoint** — `crates/blit-tui/src/main.rs:3840-3842` — Parses daemon identity (`host_port_display`) back to `RemoteEndpoint` via `RemoteEndpoint::parse`. None on malformed identity.

### timeout-or-retry
- **subscribe-open-timeout** — `crates/blit-tui/src/main.rs:5624` — `SUBSCRIBE_OPEN_TIMEOUT: Duration = 30s` (audit-8 outer-timeout for connect + RPC).
- **snapshot-fetch-timeout** — `crates/blit-tui/src/main.rs:2416` — `SNAPSHOT_FETCH_TIMEOUT: Duration = 30s` for initial GetState fetch.
- **f1-discovery-interval** — `crates/blit-tui/src/main.rs:2500` — 5s cadence between mDNS rescans.
- **f1-discovery-scan-timeout** — `crates/blit-tui/src/main.rs:2505` — 1500ms per-scan timeout.
- **event-poll-interval** — `crates/blit-tui/src/main.rs:444` — 50ms crossterm poll cadence.
- **f1-detail-buffer** — `crates/blit-tui/src/main.rs:2733` — 8-slot bounded channel for detail-fetch replies.
- **tui-event-buffer** — `crates/blit-tui/src/main.rs:2512` — 256-slot merged F2Event channel.
- **live-tick-clamping** — `crates/blit-tui/src/main.rs:1081` — `tui_config.live_tick.interval_ms_clamped()` to [50, 5000] ms.
- **cancel-status-ttl-config** — `crates/blit-tui/src/main.rs:1083` — Reads `cancel_status_ttl_ms_clamped()` each frame.
- **pull-status-ttl-config** — `crates/blit-tui/src/main.rs:894` — Reads `pull_status_ttl_ms_clamped()` each frame.
- **delete-status-ttl-config** — `crates/blit-tui/src/main.rs:901` — Reads `delete_status_ttl_ms_clamped()` each frame.
- **push-status-ttl-config** — `crates/blit-tui/src/main.rs:905` — Reads `push_status_ttl_ms_clamped()` each frame.
- **tick-budget-collapse** — `crates/blit-tui/src/main.rs:1107-1118` — `compute_tick_budget(needs_live_tick, live_tick_interval, min_opt(cancel_remaining, pull_remaining, push_remaining))` — short TTLs override long live-tick interval.

### endpoint-parse
- **parse-launch-remote** — `crates/blit-tui/src/main.rs:671-684` — CLI `--remote` first; else `[daemon] default_remote` from tui.toml. Bad value surfaces parse_error_message → F3 banner + F2 Degraded.
- **resolve-launch-remote-fn** — `crates/blit-tui/src/main.rs:637-647` — Explicit `--remote` (even empty) wins; blank/whitespace config treated as unset; otherwise trims and uses config value.
- **f1-trigger-classify** — `crates/blit-tui/src/main.rs:3567-3726` — `plan_f1_trigger`: parses src + dst via `parse_transfer_endpoint`. Remote src + Remote dst (supported) = delegated; Remote src + bare-host discovery dst = falls through to pull (treated as relative local path); Remote src + Err dst = Rejected; Remote src + Local dst = pull family; Local src + Remote dst = push family. Each branch calls `resolve_destination(src, dest, &src_endpoint, dst_endpoint)` before launch (d-71 R2 / d-72) so non-trailing-slash source basename appends correctly. _(note: this is the `feedback-endpoint-parse-err` rule — Err MUST reject, never fall through.)_

### path-handling
- **del-wire-path** — `crates/blit-tui/src/main.rs:3334-3340` — Uses `blit_app::admin::rm::extract_module_and_path` + `del_wire_path` to produce forward-slash wire path (NOT platform `PathBuf` to_string_lossy). Test pins this for Windows path assembly _(d-45 R2 reviewer reopen)_.
- **resolve-destination-push** — `crates/blit-tui/src/main.rs:3694-3705` — Push branch resolves dest via `resolve_destination` before launch — required to honor rsync trailing-slash rule and prevent data loss on push-move.
- **resolve-destination-delegated** — `crates/blit-tui/src/main.rs:3614-3618` — Delegated branch resolves dest before delegating — d-71 R2 reviewer reopen fix.
- **prepare-local-transfer** — `crates/blit-tui/src/main.rs:4041-4062` — F4 local transfer validation: parses both endpoints, rejects any Remote endpoint, resolves dest via `resolve_destination`. Mirrors CLI `blit copy` semantics.
- **f3-mirror-purge-force-flag** — `crates/blit-tui/src/main.rs:3316` — `apply_pull_mirror_purge(&outcome, true)` — force=true. _(note: hardcoded literal)_.

### safety-check
- **f3-move-deletable-gate** — `crates/blit-tui/src/main.rs:1993` — F3 `v` move gates source through `is_deletable_remote_path` to refuse module roots — daemon rejects empty/root purge paths, so without this the copy would succeed and source-delete fail (data loss surface) _(d-57 R2 reviewer reopen)_.
- **f3-move-readonly-gate** — `crates/blit-tui/src/main.rs:1968-1992` — F3 `v` also gated on `current_module_read_only()` — read-only source can't be moved out of.
- **f3-mirror-no-readonly-gate** — `crates/blit-tui/src/main.rs:1946-1958` — F3 `m` mirror is NOT gated on read-only (mirror reads from remote, deletes at LOCAL dest, so source read-only is irrelevant) _(possible subtle distinction — easy to mis-symmetrize)_.
- **plan-f1-move-deletable-gate** — `crates/blit-tui/src/main.rs:3648-3650` — F1-trigger move on remote source must refuse module root (same `is_deletable_remote_path` gate).
- **plan-f1-delegated-move-gate** — `crates/blit-tui/src/main.rs:3749-3751` — Delegated move source must NOT be a module root.
- **f1-push-mirror-require-complete-scan** — implicit via `build_f1_push_execution` — d-65 R2 test pins it. Mirror push enables `require_complete_scan` so partial enumeration can't make valid remote files look extraneous and get purged.
- **f3-du-max-depth-static-assert** — `crates/blit-tui/src/main.rs:3176-3179` — `assert!(F3_DU_MAX_DEPTH >= 1)` compile-time guard. 0 maps to unbounded in daemon.
- **local-move-incomplete-scan-refuse** — `crates/blit-tui/src/main.rs:4156-4180` — R47-F4: if `summary.unreadable_paths` non-empty, refuse source delete; quotes first 3 paths in error.
- **f3-delete-module-root-build-gate** — `crates/blit-tui/src/main.rs:2074-2079` — `build_delete_request` filters module roots (single delete returns None; batch's set drops roots).

### config-load
- **tui-config-load** — `crates/blit-tui/src/main.rs:467-468` — `config::load(|msg| config_warnings.push(msg))` BEFORE entering alternate screen (e-3 R2 — otherwise eprintln corrupts UI or gets swallowed).
- **theme-validation-warnings** — `crates/blit-tui/src/main.rs:475-605` — Validates `accent_color`, `background`, `foreground`, `mode`, `keys.quit`, `keys.refresh`, `keys.pane_fN`, `keys.move_*` — each invalid value buffers a warning + falls back to default.
- **keymap-collision-policy** — `crates/blit-tui/src/main.rs:550-604` — Precedence: quit > nav > refresh > movement. A binding colliding with a higher-precedence key is disabled (slot becomes None). Each collision flagged with a specific warning.
- **reload-tui-config** — `crates/blit-tui/src/main.rs:1291-1295` — `Ctrl+R` swaps in `reload_tui_config()` result. Parse error keeps current config + red banner. Recomputed accent, base_style, banner every frame so reload takes effect live.

### persistence
- **diagnostics-dump-path** — `crates/blit-tui/src/main.rs:4258` — `dir.join(format!("diagnostics-{now_ms}.json"))` where `dir = perf_history::config_dir()`. Pretty-printed JSON.
- **diagnostics-snapshot-shape** — `crates/blit-tui/src/main.rs:4290-4305` — JSON includes `blit_version`, `invocation` (process argv), `source`, `destination` (against RESOLVED dest), `rsync_resolution` block (source_is_contents, destination_is_container, pre_resolve_destination, resolved_destination, resolution_changed), `same_device`.
- **input-trace-log** — `crates/blit-tui/src/main.rs:4917-4921` — `BLIT_TUI_INPUT_TRACE=1` env opens `/tmp/blit-tui-input.log` in append mode _(note: hardcoded path, no rotation, may grow indefinitely)_.

### render-or-display
- **terminal-draw** — `crates/blit-tui/src/main.rs:939-1062` — Per-frame draw: optional base-style background block, tab strip (unless Dual screen which uses full area), per-screen render_into. Help overlay renders on top of body_area.
- **dual-pane-fullscreen** — `crates/blit-tui/src/main.rs:951-953` — Dual screen uses `frame.area()` directly, no tab strip — only screen that hides the tab strip.
- **tabstrip-counts** — `crates/blit-tui/src/main.rs:959-965` — `TabStripCounts{daemons: discovered_count(), active_transfers: transfers.active_count + transfer.count_active, recent_transfers: transfers.recent_count + transfer.count_recent}`. Folds F4 local transfers into the daemon-stream counts.
- **f3-cursor-spec** — `crates/blit-tui/src/main.rs:986-989` — Derives `f3_pull_spec` via `pull_source_endpoint(view, selected_row, base).map(|e| e.display())` — canonical display used as both du target and renderer gate.
- **f3-label-fallback** — `crates/blit-tui/src/main.rs:993-997` — F3 header uses `browse_target.host_port_display()` falling back to `remote_label` (launch label when no target).
- **f2-status-from-health** — `crates/blit-tui/src/main.rs:5437-5455` — Banner from `degraded` set vs `watched_total`: empty→Live, some→partial Degraded with named daemons, all→full Degraded.

### data-plane
- **apply-f2-event** — `crates/blit-tui/src/main.rs:5457-5504` — Returns false ONLY on `None` (all senders closed); Error keeps receiver alive (other daemons still feed). Connected/Event call `mark_daemon_healthy`. Error adds daemon to `f2_degraded_daemons` and re-derives banner.
- **mark-daemon-healthy** — `crates/blit-tui/src/main.rs:5516-5524` — Recovered (was in degraded set) → re-derive banner. Otherwise only lift Connecting→Live; must NOT clobber Degraded (snapshot failure).
- **drain-startup-events** — `crates/blit-tui/src/main.rs:5526-5570` — try_recv loop. Connected/first-Event only lifts Connecting→Live (not Degraded). Error overwrites status.
- **f1-push-progress-accumulator** — implicit (uses `accumulate_push_progress`) — push counts bytes from `FileComplete` (NOT `Payload`), opposite of pull.
- **delegated-progress-accumulator** — implicit (uses `accumulate_delegated_progress`) — takes both files and bytes from `Payload` (delegated path's cumulative deltas).
- **pull-progress-accumulator** — implicit (uses `accumulate_pull_progress`) — bytes from `Payload`, files from `FileComplete` (data-plane path) — avoids double-count.

### discovery
- **f2-watched-endpoints** — `crates/blit-tui/src/main.rs:5388-5404` — Launch `parsed_remote` first, then `daemons.remote_endpoints()`. Deduped by `host_port_display()` (host plus `:port` when non-default). _(known edge: hostname-launched + IP-discovered daemon has two identities, watched twice — m2f-6 follow-up)_.
- **f2-watched-identities** — `crates/blit-tui/src/main.rs:5411-5416` — `BTreeSet<String>` of identities; comparison detects watch-set churn.
- **handle-discovery-watch-change** — `crates/blit-tui/src/main.rs:2345-2360` — Compares `before` to current identities. If pending, sets `transfers_refan_after_setup` (deferred); else calls `refan_f2_setup`.
- **apply-deferred-refan** — `crates/blit-tui/src/main.rs:2367-2377` — Runs deferred re-fan after pending setup completes. m2f-9 R2.
- **dual-pane-places** — `crates/blit-tui/src/main.rs:2928-2957` — Places list: cwd, $HOME, MAIN_SEPARATOR root, parsed_remote, browse_target, daemons.remote_endpoints. Deduped by display string.

### error-propagation
- **f3-pull-error-format** — `crates/blit-tui/src/main.rs:3338-3346` — Move-delete failure surfaces as `"received but failed to delete remote source: {err:#}"` or `"received but cannot resolve remote source to delete: {err:#}"` _(deliberate dual-arm: the copy succeeded but the operator must know source wasn't removed)_.
- **f1-push-move-error-format** — `crates/blit-tui/src/main.rs:3422-3425` — `"pushed but failed to delete local source: {err:#}"`.
- **delegated-move-error-format** — `crates/blit-tui/src/main.rs:3521-3530` — `"delegated but failed to delete remote source: {err:#}"` / `"delegated but cannot resolve remote source to delete: {err:#}"`.
- **diagnostics-task-panic-error** — `crates/blit-tui/src/main.rs:4223` — `"diagnostics task panicked: {join_err}"`.
- **profile-task-panic-error** — `crates/blit-tui/src/main.rs:3036` — `"profile read task panicked: {join_err}"`.
- **verify-task-panic-error** — `crates/blit-tui/src/main.rs:4828` — `"verify task panicked: {join_err}"`.
- **f2-degraded-status-no-daemon-name** — `crates/blit-tui/src/main.rs:5481-5497` — Error branch deliberately discards per-daemon error text in banner (could be many daemons); banner names the daemon identity + count instead.

### cancellation
- **panic-hook** — `crates/blit-tui/src/main.rs:2620-2626` — `take_hook()` + `set_hook` chain: `restore_terminal()` then call original.
- **tui-guard-drop** — `crates/blit-tui/src/main.rs:2583-2588` — Idempotent restore via `TUI_ACTIVE: AtomicBool`.
- **take-active-for-restore** — `crates/blit-tui/src/main.rs:2595-2597` — Pure swap-to-false predicate; parameterized so tests use local `AtomicBool` instead of process-global.
- **f2-setup-generation-gate** — `crates/blit-tui/src/main.rs:1398-1453` — Reply arm drops stale gen; on land, clears `transfers_setup_pending`, applies snapshots additively, drains startup events, runs deferred re-fan.
- **cancel-request-generation-guard** — `crates/blit-tui/src/main.rs:1517-1521` — Compares current `Sending.request_id` to reply; mismatched (older cancel superseded) is dropped.
- **f3-del-stale-reply-no-refresh** — `crates/blit-tui/src/main.rs:1633-1656` — Only refresh F3 listing when THIS delete actually applied (apply_done returns true) — d-45 R2.
- **batch-pull-abort-on-error** — `crates/blit-tui/src/main.rs:1562-1571` — `f3_batch_pull = None` on any applied error to prevent silent batch continuation past failures.

### default-value
- **default-screen** — `crates/blit-tui/src/main.rs:105-106` — `ScreenArg::Dual` is the CLI default (Phase 6 dual-pane shell).
- **f3-du-max-depth** — `crates/blit-tui/src/main.rs:3171` — `F3_DU_MAX_DEPTH: u32 = 1`.
- **subscribe-open-timeout** — `crates/blit-tui/src/main.rs:5624` — 30s.
- **snapshot-fetch-timeout** — `crates/blit-tui/src/main.rs:2416` — 30s.
- **f1-discovery-interval** — `crates/blit-tui/src/main.rs:2500` — 5s.
- **f1-discovery-scan-timeout** — `crates/blit-tui/src/main.rs:2505` — 1500ms.
- **default-perf-history** — `crates/blit-tui/src/main.rs:4084` — `perf_history_enabled().unwrap_or(true)` — fallback enables history on config-read error.
- **default-accent** — `crates/blit-tui/src/main.rs:914` — `ratatui::style::Color::Cyan` when no theme accent configured.
- **profile-query-no-limit** — `crates/blit-tui/src/main.rs:3032` — `profile::query(0)` — `0` documented as "no limit" matching CLI.

### naming
- **screen-vs-screenarg-distinction** — `crates/blit-tui/src/main.rs:109-144` — `ScreenArg` (CLI value-enum, lowercase) vs `Screen` (PascalCase enum used throughout). `From` impl maps them.
- **kind-name-divergence-f3** — `crates/blit-tui/src/main.rs:5256, 5310-5319` — `s` for "snapshot" (diagnostics dump) and `u` for "usage" (du) deliberately diverge from `d`/`d` in TUI_DESIGN to avoid collisions with `d`=ProfileDisable. Comments document the divergence.
- **mnemonic-collision-resolutions** — `crates/blit-tui/src/main.rs:5266-5340` — Case-distinct mappings: `C`/`c`, `M`/`m`, `D`/`d`, `V`/`v`, `E`/`e`, `P`/`p`, `H`/`h`. Each commented.

### format-output
- **f1-cancel-error-no-active-rows** — `crates/blit-tui/src/main.rs:5446` — `"all {} daemon stream(s) down"`.
- **partial-degraded** — `crates/blit-tui/src/main.rs:5449-5454` — `"{}/{total} daemon streams down: {}"` joined by `, `.
- **failed-no-daemons** — `crates/blit-tui/src/main.rs:2487` — `"no daemons discovered yet"`.
- **failed-no-reachable** — `crates/blit-tui/src/main.rs:2489` — `"no reachable daemons"`.
- **f3-pull-spec-via-display** — `crates/blit-tui/src/main.rs:986-989` — Uses `endpoint.display()` for the round-tripped cursor spec (bracketed IPv6, port-aware).

### flag-handling
- **args-struct** — `crates/blit-tui/src/main.rs:94-107` — `Args { remote: Option<String>, screen: ScreenArg }`. Long `--remote`, long `--screen`.

## Smells / risks observed

- **Hardcoded path: `/tmp/blit-tui-input.log`** (line 4921) — input trace log; no rotation; may bloat. Not platform-portable (Windows `/tmp` doesn't exist).
- **Sequential subscribe fan-out** — `spawn_f2_setup_task` (line 2452) iterates daemons sequentially; with N daemons and 30s timeout each, worst case = 30N seconds before any draw of a healthy daemon's data. _(Possible refactor: `futures::join_all` or `FuturesUnordered`.)_
- **Sequential per-module `df` fan-out** — `spawn_detail_fetch` (line 2699) iterates modules sequentially. For a daemon with many modules this is slow.
- **Hardcoded subscribe filter** — `jobs::subscribe(endpoint, "", false)` (line 5630). Empty-string filter + false detach; no obvious place to surface filter to operator.
- **`mark_daemon_healthy` doesn't lift Degraded for non-degraded daemons** — line 5521. If `transfers_status` is Degraded(snapshot) and stream-Connected arrives, it stays Degraded. Documented as intentional, but subtle: a daemon whose snapshot failed will stay banner-Degraded forever even if everything else works.
- **`spawn_clear_recent` discards result** — line 3927: `_ = blit_app::admin::jobs::clear_recent(&endpoint).await`. No per-daemon outcome surfaced (intentional per `feedback-port-cli-safety-guards`? but mass-fan with all failures would be invisible).
- **`f2_watched_endpoints` known edge** — line 5384-5387: hostname-launched + IP-discovered daemon yields two identities → watched twice, rows appear under both labels. Documented as m2f-6 follow-up.
- **Module-root hardcoded message** — `"cannot move a module root"` (line 3649, 3750) — duplicated message in two functions.
- **`build_f1_push_execution`/`build_delegated_execution`/`f3_pull_options` live in `exec_plan` module** — main.rs uses them but their definitions aren't here. Reading their pin tests on lines 8702-9026 reveals the contracts; the implementations need a sibling-module audit.
- **`receive but failed to delete remote source` vs `received but failed to delete remote source`** — line 3338 says "received"; line 3522 says "delegated". These near-identical error messages could be a maintenance trap. **Actual contradiction**: line 3338 reads `"received but failed to delete remote source"` (past tense of receive) — clear. Line 3522 says `"delegated but failed to delete remote source"` — uses "delegated" as past-tense verb. Not technically wrong but inconsistent phrasing.
- **Two parallel cancel paths** — `spawn_cancel_transfer` is called directly for single cancels (line 1770, 1840) and via `spawn_cancels_for_targets` for batch — could be unified.
- **Empty test daemon for `merge_snapshot`** — line 10063 calls `merge_snapshot("", DaemonState::default(), now)` with empty daemon string; production code passes `host_port_display` — empty would not be a valid identity in steady state.
- **`F2CancelStatus::Sending` never carries `daemon` field** — Single Confirming has `daemon` but Sending doesn't. After `Confirming → Sending` transition, the daemon field is consumed; reply is matched via `request_id` only. _(Possible subtle bug if multiple daemons could plausibly send the same transfer_id, but identities scope it.)_
- **`drain_startup_events` runs synchronously on the main loop** — line 1430-1434 calls it during setup-reply arm. Could block briefly if many buffered events. No explicit bound.
- **`tokio::time::interval` MissedTickBehavior::Skip** — line 4860: discovery interval may skip ticks under load. Documented; just noting.
- **`merge_snapshot` accepts an empty daemon string in tests** — production caller passes `host_port_display`; test path stresses with `""`. If empty ever escapes to production, the `row_key` would clash across daemons.
- **`needs_live_tick` doesn't cover F3 pull Running** — line 3956-3994. It DOES cover terminal fragments (line 3967) but `f3_pull.is_running()` isn't gated here. If the pull never sends progress (silent transfer), the elapsed-time display would freeze. _(May rely on progress events as the wake source instead.)_
- **`f3_batch_pull` blank-dest handling on advance** — line 3812-3814: if `start_pull` returns None due to blank dest, batch is silently dropped. Edge: an operator pressing Enter with empty dest captures blank into `batch.raw_dest`; queued sources can't start.
- **`apply_f2_event` Error arm uses `app.f2_degraded_daemons.insert(daemon)` then re-derives banner from `f2_watched_endpoints(app).len()`** — recomputes the watched count every event; not cached.
- **Compile-time `assert!` on F3_DU_MAX_DEPTH** — line 3176: a good safety pattern; should be replicated for other "0 means unbounded" footguns elsewhere.
- **`pull_source_endpoint` for a top-level module row resolves to `Module { rel_path: "" }`** — line 1990-1992 comments document this; the move guard relies on `is_deletable_remote_path` to refuse it. The top-level module's read-only flag is NOT tracked by `current_module_read_only()`, so the gate also covers that case incidentally.
- **`spawn_input_task` accepts both Press and Repeat** — line 4943: documented choice. Could double-fire on autorepeat-prone terminals if Repeat is reported as Press, but appears intentional.

## Coverage attestation

| File | Lines read | Notes |
|------|-----------|-------|
| crates/blit-tui/src/main.rs | 10838 (full) | Read end-to-end via offset windows 1-1500, 1500-3000, 3000-4500, 4500-5800, 5800-7300, 7300-8800, 8800-9890, 9890-10838 |

**Total lines read**: 10838
**Files NOT read** (with reason): None — entire file covered.
