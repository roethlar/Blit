# Code Inventory: blit-tui state machines + pane behavior
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 14 files, 10,876 lines total

| File | Lines |
|------|------:|
| crates/blit-tui/src/f1trigger.rs | 442 |
| crates/blit-tui/src/f1push.rs | 493 |
| crates/blit-tui/src/f3pull.rs | 1282 |
| crates/blit-tui/src/f3del.rs | 494 |
| crates/blit-tui/src/f3du.rs | 292 |
| crates/blit-tui/src/browse.rs | 1918 |
| crates/blit-tui/src/daemons.rs | 918 |
| crates/blit-tui/src/transfer.rs | 483 |
| crates/blit-tui/src/verify.rs | 756 |
| crates/blit-tui/src/state.rs | 1455 |
| crates/blit-tui/src/config.rs | 1461 |
| crates/blit-tui/src/profile.rs | 183 |
| crates/blit-tui/src/help.rs | 562 |
| crates/blit-tui/src/diagnostics.rs | 137 |

## Behaviors (grouped by category)

### state-machine

- **f1trigger-status-editing** — `crates/blit-tui/src/f1trigger.rs:48-72` — `F1TriggerStatus::Editing` carries source+dest strings, `focus` (Source|Dest), `kind` (Copy/Mirror/Move PullKind), an optional validation `error` string, and a `confirming` bool used for destructive push y/N gating (d-65). _(notes: state holds three orthogonal sub-modes inside one variant — editing, error display, confirming — only confirming is exposed via `is_confirming`; `is_confirming` must be checked **before** `is_editing` in the router or the confirm absorbs nothing.)_
- **f1trigger-cycle-kind** — `crates/blit-tui/src/f1trigger.rs:163-175` — `cycle_kind(forward)`: Up cycles Copy→Mirror→Move→Copy; Down reverses. Editing the kind also clears any pending `error` field. _(notes: subtle — Up=forward, but the natural reading "Down=advance one step" wouldn't match; uses tuple-match on (kind,bool).)_
- **f1trigger-peek-no-close** — `crates/blit-tui/src/f1trigger.rs:228-243` — `peek()` returns trimmed `(source, dest, kind)` WITHOUT closing the modal; returns None if either field is blank. Caller must explicitly `close()` after a successful launch or `set_error()` to keep open. _(notes: this is an inversion of the obvious "Enter closes" reflex — dispatcher must remember to call `close()`.)_
- **f1push-status-running-fields** — `crates/blit-tui/src/f1push.rs:51-79` — Running carries `request_id`, label, files, bytes, bytes_per_sec, kind (PullKind), and a `delegated: bool` flag. Done/Error each carry `finished_at`, `kind`, `delegated`. _(notes: `delegated` distinguishes remote→remote vs local→remote for footer verbs only — state machine is identical.)_
- **f1push-begin-noop-while-running** — `crates/blit-tui/src/f1push.rs:132-148` — `begin_inner` returns `None` (no-op) when `is_running()`. Each non-blocked `begin` bumps `request_seq` monotonically. _(notes: dispatcher relies on this to block a second push; tests pin the contract.)_
- **f1push-generation-guarded-replies** — `crates/blit-tui/src/f1push.rs:153-215` — `apply_progress`, `apply_done`, `apply_error` all check `*rid == request_id` and silently drop mismatches. _(notes: same pattern as F3 pull/del/du.)_
- **f1push-terminal-ttl-pair** — `crates/blit-tui/src/f1push.rs:220-257` — `is_terminal()` flags Done|Error; `clear_terminal_if_expired(now, ttl)` collapses to Idle once `elapsed >= ttl`; `terminal_remaining(now, ttl)` returns `Option<Duration>` so the event loop can collapse its sleep budget. _(notes: duplicated pattern across F1Push, F3Pull, F3Del — three almost-identical impls.)_
- **f3pull-status-confirm-precomputes-dest** — `crates/blit-tui/src/f3pull.rs:88-97` — `Confirm` variant stores both the operator-typed string `dest` AND the already-resolved `dest_root: PathBuf`. Resolution happens at the entering-dest→confirm transition, not the confirm→running transition. _(notes: load-bearing — comment notes "must apply identical semantics or a mirror would purge the wrong directory." Different from copy launch path which resolves at running transition.)_
- **f3pull-begin-noop-while-busy** — `crates/blit-tui/src/f3pull.rs:201-235` — `is_busy()` (private) = entering_dest|running|confirming_destructive. `begin_kind` checks busy and silently no-ops; dispatcher checks externally too. _(notes: defensive double-guard.)_
- **f3pull-begin-run-destructive-routing** — `crates/blit-tui/src/f3pull.rs:289-328` — On Enter with a non-empty dest, `begin_run` either launches directly (Copy) or transitions to `Confirm` (Mirror/Move). On empty/whitespace dest, restores the EnteringDest variant (prompt stays open). _(notes: destructive `begin_run` returns `None` for both empty-dest AND confirm-pending — caller can't distinguish.)_
- **f3pull-resolve-dest-shared** — `crates/blit-tui/src/f3pull.rs:380-394` — `resolve_dest` calls `resolve_destination` from `blit_app::transfers::resolution`; preserves Local variant; falls back to raw `PathBuf::from(raw_dest)` on the impossible Remote return. _(notes: critical for not-purging-wrong-dir — applied identically by destructive confirm and copy launch.)_
- **f3pull-confirm-destructive** — `crates/blit-tui/src/f3pull.rs:335-365` — `confirm_destructive` extracts the precomputed `dest_root` from `Confirm`, bumps `request_seq`, transitions to Running, hands back the launch params carrying the kind. `cancel_destructive` returns to Idle. _(notes: destructive cancel goes straight to Idle, NOT back to EnteringDest — the operator must press `p`/`m`/`v` again.)_
- **f3pull-start-pull-direct** — `crates/blit-tui/src/f3pull.rs:278-283` — d-53 batch path: `start_pull(source, raw_dest)` launches without ever entering the prompt; copy-only, blocked if busy or blank dest. _(notes: distinct from `begin_run` — bypasses the EnteringDest state entirely.)_
- **f3del-targets-frozen-at-prompt-open** — `crates/blit-tui/src/f3del.rs:128-144` — `begin` stores `module_endpoint`, `rel_paths`, label, gate_path into `Confirming`; once frozen, cursor/selection changes can't redirect the delete. `begin` no-ops if already confirming/deleting or `rel_paths` is empty. _(notes: this is the "d-30 batch-cancel freezing lesson" applied to delete; load-bearing safety property.)_
- **f3del-confirm-launch** — `crates/blit-tui/src/f3del.rs:157-183` — `confirm()` uses `std::mem::replace` to extract Confirming → Deleting; on non-Confirming, restores the prior state and returns `None`. Bumps `request_seq`. _(notes: mem::replace pattern repeated across F3Pull confirm_destructive too.)_
- **f3del-gate-path-split-behavior** — `crates/blit-tui/src/f3del.rs:39-50,233-244` — `gate_path: Option<String>`: `Some(spec)` for single-row delete (outcome self-hides on cursor move via render-side gate); `None` for batch (no cursor event, so loop-driven TTL via `is_batch_terminal()` + `clear_terminal_if_expired`). _(notes: dual gating — render gate for single, TTL gate for batch — easy to confuse.)_
- **f3del-single-outcome-not-ttl-swept** — `crates/blit-tui/src/f3del.rs:250-267` — `clear_terminal_if_expired` only collapses `gate_path: None` (batch) outcomes; single-row outcomes (`gate_path: Some`) are intentionally left to the renderer's cursor-move path gate. _(notes: pinning test ensures this.)_
- **f3du-cache-hit-no-rpc** — `crates/blit-tui/src/f3du.rs:99-111` — `begin(path)` returns `DuBegin::Cached` if `path` is in `cache` (HashMap), straight to Done, no `request_seq` bump. Cache miss returns `DuBegin::Fetch(id)` and bumps the gen. _(notes: cache key is the canonical cursor spec; cache never invalidates within a session — doc comment notes "stale entries simply re-resolve to the same total." Potential staleness bug if the remote subtree size actually changes.)_
- **f3du-fall-off-by-cursor** — `crates/blit-tui/src/f3du.rs:27-40` — Every non-Idle variant carries the `path` it pertains to; the renderer compares against the current cursor's canonical spec. No timer/TTL. _(notes: simpler than F3Pull's TTL approach — relies on render-side cursor compare.)_
- **transfer-confirming-mirror-and-move-variants** — `crates/blit-tui/src/transfer.rs:46-81` — Two distinct ConfirmingMirror / ConfirmingMove states (no single Confirming-with-kind enum). Each is approached via `begin_confirm_mirror` / `begin_confirm_move`. _(notes: comment notes the symmetry was intentional because banner warnings differ — but this is asymmetric vs F3Pull which uses one `Confirm` variant with a kind field.)_
- **transfer-is-busy-encompasses-confirm** — `crates/blit-tui/src/transfer.rs:162-164` — `is_busy()` = running OR confirming; dispatcher checks this to refuse a second M while the first confirm is still on screen. _(notes: critical for the y/N reflex — without it, a second M would clobber the pending confirm.)_
- **transfer-apply-done-preserves-started-at** — `crates/blit-tui/src/transfer.rs:215-242` — d-8: threads `started_at` from Running into Done; defensive `Instant::now()` fallback if state isn't Running (shouldn't happen given gen guard). _(notes: same shape as verify.rs:294-312.)_
- **transfer-note-validation-error-bumps-gen** — `crates/blit-tui/src/transfer.rs:209-213` — `note_validation_error` bumps `request_id` so a pending Running's reply drops harmlessly when validation fails synchronously. _(notes: subtle — write to Error state without ever calling begin().)_
- **verify-status-running-started-at** — `crates/blit-tui/src/verify.rs:38-56` — Running carries only `started_at`; Done carries `result`, `started_at`, `finished_at`. Error carries only `message`. _(notes: Error has no started_at — Done duration is computable, Error duration is not.)_
- **verify-edits-invalidate-run** — `crates/blit-tui/src/verify.rs:263-279` — Any field edit (insert_char/backspace/clear_focused_field) calls `invalidate_run` which bumps `request_id` AND collapses Running|Done|Error→Idle. _(notes: critical correctness — without this, an in-flight compare's reply would label the now-edited paths with the old comparison. Same applies to `toggle_checksum` / `toggle_one_way`.)_
- **verify-cycle-focus-three-states** — `crates/blit-tui/src/verify.rs:208-214` — `cycle_focus`: None → Source → Destination → None. Editing fields only effective when focus != None. _(notes: tab cycle includes a "not editing" state, distinct from "no field selected.")_
- **verify-can-run-trims-and-blocks-running** — `crates/blit-tui/src/verify.rs:326-330` — `can_run()` = both fields non-empty AFTER trim AND not currently Running. _(notes: whitespace-only inputs don't count; explicit trim.)_
- **browse-view-modules-vs-module** — `crates/blit-tui/src/browse.rs:57-64` — `BrowseView::Modules` (top level) vs `BrowseView::Module { name, path: Vec<String> }` (nested). Empty path = module root. _(notes: path is Vec<String>, not PathBuf — segments are joined with `/` for display.)_
- **browse-descend-no-op-on-hidden-row** — `crates/blit-tui/src/browse.rs:428-460` — `descend()` no-ops if cursor is on a row hidden by the active filter (`!self.row_matches(row)`). d-26 round-2 fix: pre-fix `/zz` + Enter stepped into raw row 0. _(notes: pinned regression — `descend` and `selected_row` both gate on `row_matches`.)_
- **browse-reset-view-state-clears-marks-and-filter** — `crates/blit-tui/src/browse.rs:659-662` — `reset_view_state` is called on descend/ascend/apply_modules/apply_listing; clears BOTH filter AND `marked: HashSet<String>` because marks are keyed to the current row set. _(notes: distinct from `clear_filter_only` (filter-cancel only) — d-49 R2 made cancel_filter NOT clear marks since the row set didn't change.)_
- **browse-module-read-only-captured-on-descend** — `crates/blit-tui/src/browse.rs:432-444` — `module_read_only` is captured when descending FROM modules-list INTO a module; cleared on ascend to modules; dir/file rows don't carry the flag so the flag must be captured at the boundary. _(notes: F3 `D` delete dispatcher gates on this.)_
- **browse-toggle-mark-all-filter-scoped** — `crates/blit-tui/src/browse.rs:195-213` — d-51 `a`: toggles marks on all `visible_indices` rows (filter-aware). If all visible already marked, clears them; else marks all. _(notes: select-all is filter-scoped, not all-rows.)_
- **browse-stale-reply-drop-by-view** — `crates/blit-tui/src/browse.rs:292-353` — `apply_modules` drops the result if current view is not Modules; `apply_listing` drops if `for_module/for_path` don't match the current view. _(notes: two different staleness guards — view-shape vs view-path; complements the `pending_request_id` generation guard.)_
- **daemons-local-row-always-pinned** — `crates/blit-tui/src/daemons.rs:363-406` — `replace_from_discovery` always re-injects the synthetic Local row at index 0. Sorts discovered remotes alphabetically by instance_name. _(notes: Local is a sentinel row — `LOCAL_INSTANCE_NAME` is a hardcoded "local (this machine)" constant.)_
- **daemons-preserve-cursor-by-name** — `crates/blit-tui/src/daemons.rs:363-397` — On rescan, preserves cursor on the same `instance_name` if still present; otherwise falls back to `min(prior_index, rows.len()-1)` — keeps operator near where they were, doesn't teleport to row 0. _(notes: subtle — name-based first, then index-based fallback.)_
- **daemons-detail-survives-rescan** — `crates/blit-tui/src/daemons.rs:265-269,363-406` — `details: HashMap<instance_name, DaemonDetail>` and `request_ids: HashMap<...>` live outside `rows`; `replace_from_discovery` doesn't touch them, so a previously-fetched detail block survives a rescan. _(notes: distinct from F3 browse, where `apply_listing` clears rows.)_
- **daemons-has-live-timestamp-dual-gate** — `crates/blit-tui/src/daemons.rs:238-251` — d-11 R2 fix: `has_live_timestamp` returns true if EITHER discovery is `Live` OR the selected row has a cached `Loaded` detail. Pre-fix, degraded mDNS froze the detail "as of Xs ago" timestamp. _(notes: pinned regression with explicit test.)_
- **state-transfers-merge-snapshot-per-daemon** — `crates/blit-tui/src/state.rs:166-181` — `merge_snapshot(source_daemon, state, fetched_at)` removes only that daemon's active+recent rows then re-inserts; other daemons' rows are untouched. Replaces the old `replace_from_snapshot` which cleared the whole view. _(notes: m2f-3 — additive primitive for fan-out.)_
- **state-transfers-active-keyed-by-composite** — `crates/blit-tui/src/state.rs:108-137,496-498` — `active: HashMap<row_key(source_daemon, transfer_id), ActiveRow>`. `row_key` joins with U+001F (unit separator) — `transfer_id` is only unique within a daemon (`t<ms>-<n>`), so cross-daemon collisions need composite keying. _(notes: U+001F hardcoded magic char.)_
- **state-transfers-id-anchored-cursor** — `crates/blit-tui/src/state.rs:124-137,371-461` — `selected_active_key: Option<String>` anchors on the composite key, not display index. `selected_active_index` derives the visible position; returns None when the row is gone ("falls off"). `select_next/prev/first/last` re-anchor based on derived index. _(notes: pinned regression d-21 R2 — pre-fix the index silently retargeted at a new transfer after the selected one terminated.)_
- **state-transfers-terminal-id-dedup** — `crates/blit-tui/src/state.rs:215-249` — Before dispatching any TransferStarted/Progress/Complete/Error, `apply_event_inner` checks if `(daemon, id)` is in `recent`; if so, drops the event. Closes the a1-2 R3 startup race. _(notes: dedup is per-(daemon, id) so cross-daemon ID collisions don't suppress each other.)_
- **state-transfers-retain-active-daemons** — `crates/blit-tui/src/state.rs:192-200` — m2f-9 R3: `retain_active_daemons(&BTreeSet<String>)` drops active rows whose source_daemon left the watch set; **keeps recent rows** (history); clears cursor if anchored to a pruned row. _(notes: asymmetric pruning — active vs recent.)_
- **profile-confirming-clear** — `crates/blit-tui/src/profile.rs:39-46,100-112` — d-66: `confirming_clear: bool` armed by `begin_clear_confirm` for a y/N before wiping perf history. Permanent action, gated like every other destructive TUI action. _(notes: same shape as transfer ConfirmingMirror/Move but in a separate state struct.)_
- **diagnostics-generation-guarded-dump** — `crates/blit-tui/src/diagnostics.rs:51-75` — `begin_dump` bumps `request_id`, transitions Running; `apply_done`/`apply_error` drop stale generations. Pressing `s` again before the first completes silently supersedes. _(notes: same gen-guard pattern as every other state machine; simplest instance.)_

### confirmation-prompt

- **f1trigger-confirming-push-gate** — `crates/blit-tui/src/f1trigger.rs:103-129` — d-65: destructive PUSH (mirror/move local→remote) opens a y/N confirm via `begin_confirm`; `is_confirming` must be checked BEFORE `is_editing` so the y/N handler absorbs first. `cancel_confirm` returns to plain editing (not Idle). _(notes: confirm cancel returns to editing, NOT to Idle — different from F3Pull's `cancel_destructive`.)_
- **f3pull-destructive-confirm-via-pullstatus-confirm** — `crates/blit-tui/src/f3pull.rs:88-97,335-373` — Single `Confirm` variant for both mirror and move (kind differentiates); `confirm_destructive` launches, `cancel_destructive` → Idle. _(notes: differs from `TransferState` which has separate ConfirmingMirror/ConfirmingMove variants.)_
- **f3del-confirm-only-via-y** — `crates/blit-tui/src/f3del.rs:40-50` — Delete is ALWAYS gated behind Confirming state (no direct-launch path). Targets frozen at prompt-open time. _(notes: no batch-bypass path — every delete confirms.)_
- **transfer-confirm-mirror-vs-move** — `crates/blit-tui/src/transfer.rs:46-81` — Two confirm variants: ConfirmingMirror (delete-extraneous at dest) vs ConfirmingMove (delete source after copy). Different banners. _(notes: asymmetric vs F3Pull single-variant design.)_
- **profile-clear-y-n** — `crates/blit-tui/src/profile.rs:100-112` — `c` to clear perf-history opens a y/N. `[d]`/`[e]` disable/enable don't confirm (reversible). _(notes: only `c` is destructive on F4 profile.)_

### endpoint-parse

- **daemons-endpoint-for-row-local-loopback** — `crates/blit-tui/src/daemons.rs:330-339` — `endpoint_for_row` hardcodes `127.0.0.1:9031` for the Local row. Remote rows take the first `addresses[0]` with the daemon's port. _(notes: `9031` hardcoded; loopback port mismatch (operator-changed daemon port) silently won't connect. Comment acknowledges future config opt-in.)_
- **daemons-remote-endpoints-skip-local-and-unresolvable** — `crates/blit-tui/src/daemons.rs:341-351` — `remote_endpoints()` filters out Local AND any row whose `endpoint_for_row` returns None (no addresses). _(notes: m2f-5 fan-out subscribe set.)_
- **browse-pull-source-endpoint-clone-base** — `crates/blit-tui/src/browse.rs:689-723` — `pull_source_endpoint(view, selected, base)` reuses `base.host` + `base.port` and overwrites the path with the selected row's module+rel_path. Goes through `RemoteEndpoint::display()` for the spec preview — bracketed IPv6 + port-aware. _(notes: d-33 R1 bug: a hand-built host string mangled IPv6; fix routes through the type's own display authority.)_

### path-handling

- **f3pull-resolve-destination-shared** — `crates/blit-tui/src/f3pull.rs:380-437` — `resolve_dest` calls `blit_app::transfers::resolution::resolve_destination` so dir-into-container nests under source basename, file-into-container appends filename, trailing-slash forces container semantics. Applied identically in copy launch AND destructive confirm. _(notes: load-bearing. The `Endpoint::Remote(_)` arm of the match is documented "can't happen" — silently falls back to raw PathBuf::from(raw_dest), no error.)_

### default-value

- **config-keys-defaults** — `crates/blit-tui/src/config.rs:141-148,250-265` — `DEFAULT_QUIT = 'q'`, `DEFAULT_REFRESH = 'r'`, `DEFAULT_PANE = ['1','2','3','4']`, `DEFAULT_MOVE = ['j','k','g','G']`. Multi-char or empty config strings fall back to defaults via `single_char()`. _(notes: chars hardcoded as `const` not config — but reachable.)_
- **config-default-instance-name-local** — `crates/blit-tui/src/daemons.rs:37` — `LOCAL_INSTANCE_NAME = "local (this machine)"`. _(notes: collision-prone if a real daemon advertises this name — caller would need to disambiguate.)_
- **config-live-tick-default-500ms** — `crates/blit-tui/src/config.rs:340-358` — `DEFAULT_INTERVAL_MS = 500`, clamp `[50, 5000]`. Out-of-range silently snapped. _(notes: u64::MAX → 5000; never refuses.)_
- **config-transfer-ttl-defaults** — `crates/blit-tui/src/config.rs:543-606` — Each of `cancel_status_ttl_ms`/`pull_status_ttl_ms`/`delete_status_ttl_ms`/`push_status_ttl_ms` defaults to 5_000ms, clamped `[250, 60_000]`. _(notes: 4 nearly-identical pairs of accessors — duplication.)_
- **config-tui-recent-cap** — `crates/blit-tui/src/state.rs:21` — `TUI_RECENT_CAP = 50` matches daemon's recent ring depth. _(notes: not config-driven.)_
- **transfer-confirm-cancel-default-false** — `crates/blit-tui/src/config.rs:507-516` — `confirm_cancel: false` by default — d-22 one-keystroke cancel behavior. _(notes: opt-in safety, comment notes cancel is reversible.)_

### config-load

- **config-tui-toml-filename** — `crates/blit-tui/src/config.rs:70-71` — `CONFIG_FILENAME = "tui.toml"`. _(notes: shared with reload path; hardcoded.)_
- **config-load-missing-file-silent** — `crates/blit-tui/src/config.rs:688-705` — Missing file → defaults, NO warning. Parse errors → warning via callback + defaults. _(notes: missing-vs-parse-error are intentionally asymmetric.)_
- **config-load-fallback-on-no-config-dir** — `crates/blit-tui/src/config.rs:672-681` — When `blit_core::config::config_dir()` errors, returns `TuiConfig::default()` SILENTLY — no warning. _(notes: subtle — operator never knows their config is being skipped if config_dir() fails.)_
- **config-deny-unknown-fields** — `crates/blit-tui/src/config.rs:73,91,269,282,305,325,374,498` — Every section uses `#[serde(default, deny_unknown_fields)]` — typo'd field names warn instead of silently defaulting. _(notes: comment notes this is intentional to catch typos.)_
- **config-runtime-reload** — `crates/blit-tui/src/config.rs:10-14` — Ctrl+R reload via `main::reload_tui_config` swaps the live config; reload PARSE error keeps the current config (does NOT revert to defaults). _(notes: reload semantics differ from initial load — initial parse error returns defaults, reload error keeps current.)_

### key-dispatch

- **config-keys-collision-policy** — `crates/blit-tui/src/config.rs:187-237` — `resolved()` claims chars in dispatch-precedence order: quit > nav1..4 > refresh > movement (down/up/top/bottom). First-claim wins; later collisions disable that binding (`None`). _(notes: arrow keys/Home/End/Ctrl-R/Esc/Ctrl-C are NOT in the collision set — they're hardcoded failsafes elsewhere. Memory-flagged: "Keymap bindings need a collision policy.")_
- **config-keys-single-char-rejection** — `crates/blit-tui/src/config.rs:241-248` — `single_char(value)`: returns `Some(c)` only if value has exactly one char; multi-char or empty → None (caller warns + falls back). _(notes: doesn't validate against control chars or e.g. multi-codepoint emojis being acceptable.)_
- **help-keymap-source-of-truth** — `crates/blit-tui/src/help.rs:86-153` — `help_lines()` is THE keymap reference; `help_line_count()` shares the same source for the scroll clamp. Renderer + clamp + test all read it. _(notes: keymap is hand-coded — adding a binding to dispatcher requires adding it here too; the d-16 R2 attribution test (help.rs:427-523) catches misplaced entries but not missing ones beyond the explicit grep list.)_
- **help-overlay-absorbs-while-visible** — `crates/blit-tui/src/help.rs:18-81` — `HelpOverlay::visible` boolean controls absorption (router-side); `?` and Esc still pass through to close. `j/k` scroll the overlay when visible. _(notes: scroll keys collide with global movement aliases when overlay is open — input router must check overlay first.)_

### discovery

- **daemons-status-scanning-live-degraded** — `crates/blit-tui/src/daemons.rs:104-119` — `DiscoveryStatus`: Scanning (initial), Live{last_scan_at}, Degraded{message}. Degraded preserves prior rows + adds a banner. _(notes: no "Empty" state — a Live scan with zero remote results still uses Live, just with discovered_count()=0.)_
- **daemons-discovered-count-excludes-local** — `crates/blit-tui/src/daemons.rs:205-207` — Tab strip uses `discovered_count()` (filters out Local) so "0 daemons" really means no remote endpoints. _(notes: e-2 R2 fix — pre-fix counted the always-present Local row.)_

### persistence

- **profile-fetch-status** — `crates/blit-tui/src/profile.rs:18-29` — Idle/Pending/Loaded{fetched_at}/Error{message}. `note_fetch_error` preserves prior report — operator still sees last good snapshot + error banner. _(notes: shape duplicated across daemons.rs (DaemonDetail), browse.rs (BrowseFetchStatus), profile.rs.)_
- **state-clear-recent-empties-only-recent** — `crates/blit-tui/src/state.rs:474-481` — rec-3: TUI-side immediate clear; returns count cleared; only stamps `last_event_at` if rows were actually cleared (won't fabricate footer activity on a no-op). _(notes: active rows untouched.)_

### render-or-display

- **browse-empty-state-message-differentiated** — `crates/blit-tui/src/browse.rs:603-609` — Returns `"(no rows match filter)"` when rows exist + filter is set + matches nothing; otherwise `"(no entries)"`. _(notes: edge case — rows empty AND filter set returns "(no entries)" since "filter can't be the reason" with no input.)_
- **browse-breadcrumb** — `crates/blit-tui/src/browse.rs:505-516` — `"modules"` at top level; `name` at module root; `"name/seg1/seg2"` inside. _(notes: hardcoded strings; path joiner is `/`.)_
- **help-modal-size-bump-history** — `crates/blit-tui/src/help.rs:168-194` — `centered(area, 70, 47)`. The literal `47` (modal height) has been bumped slice-by-slice as the keymap grew (d-26..d-58); the comment lists each bump. _(notes: hardcoded magic number tied to `help_lines()` content — drift between keymap and modal height is possible but not enforced.)_

### format-output

- **f3pull-pullkind-verbs** — `crates/blit-tui/src/f3pull.rs:58-66` — `PullKind::verbs()` returns `(noun, present, past)` tuples — `("pull","pulling","pulled")`, `("mirror","mirroring","mirrored")`, `("move","moving","moved")`. _(notes: footer-only; not used for state transitions.)_
- **transfer-kind-label** — `crates/blit-tui/src/transfer.rs:38-45` — `TransferKind::label()`: "copy"/"mirror"/"move". _(notes: TransferKind in `transfer.rs` is distinct from PullKind in `f3pull.rs` — same shape, separate types.)_

### spawn-task

- **f1push-begin-returns-request-id** — `crates/blit-tui/src/f1push.rs:119-148` — `begin`/`begin_delegated` return `Option<u64>` — the request_id the spawned task must echo back. None signals "blocked." _(notes: caller is responsible for spawning; state machine is pure.)_
- **f3del-confirm-returns-dellaunch** — `crates/blit-tui/src/f3del.rs:77-82,154-183` — `confirm()` returns `Option<DelLaunch { module_endpoint, rel_paths, request_id }>` — the launch params for the spawn helper. _(notes: same pattern as `F3PullState::begin_run` / `confirm_destructive`.)_
- **f3pull-pulllaunch** — `crates/blit-tui/src/f3pull.rs:139-147` — `PullLaunch { source, dest_root, request_id, kind }`. `kind` keys `mirror_mode`, `require_complete_scan`, post-pull purge/source-delete on the spawn helper. _(notes: state machine is decoupled from the actual RPC machinery.)_

### naming

- **f3pull-vs-transfer-pull-kind-vs-transfer-kind** — `crates/blit-tui/src/f3pull.rs:42-47` vs `crates/blit-tui/src/transfer.rs:26-35` — Two parallel enums: `PullKind { Copy, Mirror, Move }` and `TransferKind { Copy, Mirror, Move }`. Same labels, separate types. _(notes: duplication smell — F1 trigger (f1trigger.rs:38) and F1 push (f1push.rs:37) import PullKind; F4 local transfer uses TransferKind. The unification opportunity wasn't taken.)_

### cancellation

- **state-cursor-fall-off-on-terminate** — `crates/blit-tui/src/state.rs:371-401,1046-1069` — When a transfer terminates, `selected_active_index` returns None until operator re-anchors with j/k. Avoids silent retarget. _(notes: pinned regression with explicit scenario tests.)_

## Smells / risks observed

- **Hardcoded 9031 loopback port** — `crates/blit-tui/src/daemons.rs:335` — Local daemon assumed on `127.0.0.1:9031`; if operator changed daemon bind, TUI silently can't reach it. Comment acknowledges "future config opt-in" but no current mitigation.
- **U+001F unit separator hardcoded** — `crates/blit-tui/src/state.rs:496-498` — `row_key` uses `\u{1f}` as composite-key delimiter, with the comment "The unit-separator can't appear in a host or id." Reasonable but undocumented elsewhere in the codebase; a future host validation that allowed control chars would silently corrupt keys.
- **Duplication: terminal TTL machinery across F1Push / F3Pull / F3Del** — `f1push.rs:220-257`, `f3pull.rs:516-567`, `f3del.rs:233-267` — Three almost-identical `is_terminal`/`clear_terminal_if_expired`/`terminal_remaining` implementations. A shared trait or helper struct would reduce maintenance.
- **Duplication: PullKind vs TransferKind** — `f3pull.rs:42-66` and `transfer.rs:26-45` — Two enums with identical variants and only label vocabulary in common; the F1 trigger and F1 push reach into `f3pull::PullKind` while F4 local transfer uses `transfer::TransferKind`. Refactor opportunity.
- **Duplication: BrowseFetchStatus / ProfileFetchStatus / DaemonDetail / DiscoveryStatus** — Four parallel "fetch lifecycle" enums in `browse.rs:95-107`, `profile.rs:18-29`, `daemons.rs:126-145`, `daemons.rs:104-119`. All have Idle/Pending/Loaded{fetched_at}/Error variants with subtle differences.
- **Silent fallback on `config_dir()` failure** — `crates/blit-tui/src/config.rs:672-681` — When `blit_core::config::config_dir()` returns Err, `load()` silently returns defaults without invoking `on_warn`. Operator never finds out their tui.toml was skipped.
- **Help modal height drift risk** — `crates/blit-tui/src/help.rs:168-194` — `centered(area, 70, 47)` literal 47 is hand-maintained alongside `help_lines()` line count; comment lists 12+ historical bumps. No automated check that `centered`'s height ≥ `help_line_count()`. The `scrollbar_absent_when_content_fits` test renders at 52 but doesn't lock 47 to the keymap length.
- **F4 confirm split (ConfirmingMirror vs ConfirmingMove)** — `transfer.rs:46-81` — Asymmetric vs F3Pull's single `Confirm { kind, ... }` variant. Adding a third destructive variant on F4 would need a third state plus accessor.
- **F3Pull destructive `begin_run` returns None for two reasons** — `f3pull.rs:289-328` — Empty dest AND "deferred to confirm" both return None; caller can't distinguish without checking `is_confirming_destructive()` separately.
- **F3Del `confirm` uses mem::replace + restore on miss** — `f3del.rs:158-170` — Same idiom in F3Pull `confirm_destructive` (f3pull.rs:336-348). Defensive pattern but slightly subtle — a panic during the body would leave the state as Idle.
- **F3Du cache is session-immortal** — `crates/blit-tui/src/f3du.rs:60-69` — The doc comment hand-waves "stale entries simply re-resolve to the same total." If the remote subtree grows/shrinks mid-session, cached du never refreshes. No invalidation API.
- **Hardcoded "local (this machine)" instance name** — `crates/blit-tui/src/daemons.rs:37` — Used both as display label and uniqueness key. A real daemon advertising the same string would collide.
- **Help keymap is a hand-edited Vec<Line>** — `crates/blit-tui/src/help.rs:86-153` — Adding a new key requires updating both the dispatcher and `help_lines()`; only the d-16 R2 test grep catches missing-from-help. There's no compile-time check.
- **F2 `selected_active_daemon` API not surfaced as `is_present`** — `state.rs:398-401` — `selected_active_daemon` returns the daemon STRING; callers like CancelJob need both daemon AND id together. The two accessors are called separately — risk of mismatched pair if state mutates between calls.

## Coverage attestation

| File | Lines read | Notes |
|------|-----------:|-------|
| crates/blit-tui/src/f1trigger.rs | 442 | full |
| crates/blit-tui/src/f1push.rs | 493 | full |
| crates/blit-tui/src/f3pull.rs | 1282 | full |
| crates/blit-tui/src/f3del.rs | 494 | full |
| crates/blit-tui/src/f3du.rs | 292 | full |
| crates/blit-tui/src/browse.rs | 1918 | full (2-page read: 1-1560, 1561-1918) |
| crates/blit-tui/src/daemons.rs | 918 | full |
| crates/blit-tui/src/transfer.rs | 483 | full |
| crates/blit-tui/src/verify.rs | 756 | full |
| crates/blit-tui/src/state.rs | 1455 | full |
| crates/blit-tui/src/config.rs | 1461 | full |
| crates/blit-tui/src/profile.rs | 183 | full |
| crates/blit-tui/src/help.rs | 562 | full |
| crates/blit-tui/src/diagnostics.rs | 137 | full |

**Total lines read**: 10,876
**Files NOT read** (with reason): none
