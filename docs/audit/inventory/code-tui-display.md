# Code Inventory: blit-tui display mappers + extracted helpers + screens

**Generated**: 2026-06-04 by audit workflow
**Cluster**: State→display mapping, transfer execution builders, sleep-budget math, F3-delete request builders, config hot-reload, theme color translation, progress folding, every renderer (F1/F2/F3/F4) + dual-pane shell.

## Coverage

Files read end-to-end:

| File | Lines |
|------|-------|
| `crates/blit-tui/src/display_f1.rs` | 137 |
| `crates/blit-tui/src/display_f2.rs` | 119 |
| `crates/blit-tui/src/display_f3.rs` | 153 |
| `crates/blit-tui/src/exec_plan.rs` | 108 |
| `crates/blit-tui/src/tick_budget.rs` | 49 |
| `crates/blit-tui/src/del_request.rs` | 74 |
| `crates/blit-tui/src/config_reload.rs` | 57 |
| `crates/blit-tui/src/theme_color.rs` | 54 |
| `crates/blit-tui/src/progress_accum.rs` | 118 |
| `crates/blit-tui/src/screens/mod.rs` | 457 |
| `crates/blit-tui/src/screens/dual_pane.rs` | 273 |
| `crates/blit-tui/src/screens/f1.rs` | 1123 |
| `crates/blit-tui/src/screens/f2.rs` | 928 |
| `crates/blit-tui/src/screens/f3.rs` | 728 |
| `crates/blit-tui/src/screens/f4.rs` | 1002 |

**Total lines read**: 5380.

## Behaviors (grouped by category)

### render-or-display

- **f1-trigger-prompt-bridge** — `crates/blit-tui/src/display_f1.rs:11-58` — Maps `F1TriggerState::Editing` into a `TriggerPrompt` struct with source/dest/focus/mode/destructive/error/confirm_detail. Spells "copy" (matching the launcher vocabulary) rather than the kind's own "pull" verb for `PullKind::Copy`. _(notes: literal verbs `"copy"`/`"mirror"`/`"move"` hardcoded; confirm_detail strings hardcoded.)_
- **f1-push-status-bridge** — `crates/blit-tui/src/display_f1.rs:64-112` — Maps `F1PushStatus` Running/Done/Error into renderer-facing `PushStatusDisplay`; uses `push_present_verb` / `push_past_verb` for the verb tense based on `(delegated, kind)`. _(notes: a remote→remote delegated **mirror** falls through to `(_, Mirror) => "mirroring"` even when delegated — special-case only triggers for delegated Copy ("delegating"); see push-verb-table.)_
- **push-verb-table** — `crates/blit-tui/src/display_f1.rs:121-137` — Static verb tables: present participle `delegating/mirroring/moving/pushing`; past tense `delegated/mirrored/moved/pushed`. _(notes: delegated mirror & move don't get a "delegated_*" label — only Copy gets the "delegating" rename when delegated.)_
- **f2-cancel-status-to-display** — `crates/blit-tui/src/display_f2.rs:16-82` — Big match from internal `F2CancelStatus` → `screens::f2::F2CancelDisplay`. Done/Error/BatchInitiated variants auto-hide once `now - finished_at >= ttl`. _(notes: state itself stays — renderer just returns `Hidden`. Sending has no deadline.)_
- **f2-cancel-remaining-ttl** — `crates/blit-tui/src/display_f2.rs:98-119` — Returns `Some(ttl - elapsed)` for Done/Error/BatchInitiated, `None` for Idle/Sending/Confirming. Drives the event loop's sleep budget. _(notes: BatchInitiated added (d-30) — must wake on the same TTL boundary.)_
- **f3-pull-to-display** — `crates/blit-tui/src/display_f3.rs:13-64` — Maps `F3PullStatus` → `F3PullDisplay` with verbs pulled from `PullKind::verbs()` and a hardcoded confirm-detail string. _(notes: Confirm/EnteringDest pass `kind.verbs().0` imperative; Running passes `.1` present-participle; Done passes `.2` past-tense — verb-triple defined upstream in `f3pull`.)_
- **f3-pull-confirm-detail** — `crates/blit-tui/src/display_f3.rs:69-76` — Hardcoded strings: "deletes extraneous" (Mirror) vs "deletes the remote source" (Move) vs "" (Copy). _(notes: same string literals duplicated in `display_f1.rs:46-54` for push direction — see hardcoded-string-duplication.)_
- **f3-du-display-path-gate** — `crates/blit-tui/src/display_f3.rs:85-106` — Renders Running/Done/Error only when the du's path equals the cursor's current path; otherwise Hidden. Avoids showing stale subtree totals on a different row. _(notes: gate is `current_path == Some(path)` exact compare.)_
- **f3-del-display-path-gate** — `crates/blit-tui/src/display_f3.rs:113-153` — Confirming/Deleting always visible; Done/Error gated on `gate_path` matching `current_path`. Batch deletes carry `gate_path = None` → always shown until next action. _(notes: single-row gate identical to du; batch uses `None` sentinel.)_
- **render-tab-strip** — `crates/blit-tui/src/screens/mod.rs:67-141` — Tab strip with four width regimes (full tabs + full counts / full tabs + short counts / short tabs + short counts / short tabs only). Reload banner overrides the right-edge counts when present. _(notes: width-fallback chain `area.width >= full_tab + full_counts`; banner has higher right-edge priority than counts.)_
- **contrasting-fg** — `crates/blit-tui/src/screens/mod.rs:200-210` — Classifies accent backgrounds as dark (Black/Red/Green/Blue/Magenta/DarkGray → White fg) vs light (everything else → Black fg). _(notes: Yellow + Cyan classed "light" → black text; unrecognized variants fall back to Black fg. Used by F1/F2/F3/F4/dual_pane highlights.)_
- **render-pane-dual** — `crates/blit-tui/src/screens/dual_pane.rs:14-45` — Two stacked panes (left/right 50/50) + 1-line help + 1-line actions. Pane border switches to accent on active. _(notes: layout has 3 fixed sections.)_
- **render-pane-empty-fork** — `crates/blit-tui/src/screens/dual_pane.rs:59-84` — When `entries().is_empty()`, swap to a single-line paragraph with status-derived suffix; for Error returns early (early-return inside the if-empty block). _(notes: Error case uses a slightly different layout (no inner split), see also pane-render-error-return.)_
- **format-mtime-dual-pane** — `crates/blit-tui/src/screens/dual_pane.rs:223-227` — Returns raw `mtime_seconds` as a string (or empty if None). _(notes: no YMD formatting unlike F3's `format_mtime`; explicitly raw — different from F3.)_
- **f1-render-into** — `crates/blit-tui/src/screens/f1.rs:60-93` — Layout: header (1) / table (Min 5) / detail (5) / footer (1). Footer priority: trigger modal > push status > discovery status. _(notes: detail height is fixed at 5 — see f1-detail-fixed-height.)_
- **f1-render-push** — `crates/blit-tui/src/screens/f1.rs:124-169` — Live push footer: yellow `verb → label...` with optional `(N files · X · X/s)` once bytes flow; green Done; red Error. _(notes: rate suppressed at `bytes_per_sec == 0` (warmup); first byte triggers count display.)_
- **f1-render-trigger** — `crates/blit-tui/src/screens/f1.rs:174-241` — Trigger entry prompt with focused-field cursor `_`, destructive-flag red mode tag, validation error replacing hint. Destructive confirm replaces the entry prompt with `mode src → dst? detail y/N`. _(notes: cursor character is `_`; arrow keys hint shown when no error; mode color = Red destructive / Green copy.)_
- **f1-render-table** — `crates/blit-tui/src/screens/f1.rs:255-293` — 6 fixed-width columns (name 20 / addr 18 / port 6 / version 8 / modules 10 / delegation 10); themed accent highlight + contrasting fg. _(notes: viewport auto-scroll via TableState.)_
- **f1-render-detail-local** — `crates/blit-tui/src/screens/f1.rs:531-606` — Local-row detail body has 4 distinct branches (Loaded shows live counters, Pending shows "fetching", Error shows "no local daemon detected", None shows "GetState not yet attempted"). _(notes: hard-codes `127.0.0.1:9031` in spinner + error lines.)_
- **f1-render-detail-remote** — `crates/blit-tui/src/screens/f1.rs:394-495` — Remote detail body falls back to mDNS-only lines while GetState is Pending/Error/None; Loaded shows counters + modules-with-capacities + uptime. _(notes: modules annotation falls back to mDNS-advertised names when GetState's modules is empty.)_
- **f1-format-bytes-modules** — `crates/blit-tui/src/screens/f1.rs:614-626` — IEC byte formatter (B/KiB/MiB/GiB/TiB) duplicated for module-capacity annotation. _(notes: identical tiers to F2/F4 per existing per-screen convention; see duplicated-format-bytes.)_
- **f1-format-uptime** — `crates/blit-tui/src/screens/f1.rs:628-642` — Uptime formatter: "{d}d {h}h {m}m" / "{h}h {m}m" / "{m}m {s}s" / "{s}s". _(notes: drops smallest unit at next tier — "1d 1h 1m" not "1d 1h 1m 1s".)_
- **f1-format-address** — `crates/blit-tui/src/screens/f1.rs:648-654` — Renders first address; appends "+N" for multi-addr daemons; "(no addr)" for empty. _(notes: trades precision for column width — header summary only.)_
- **f1-format-since** — `crates/blit-tui/src/screens/f1.rs:671-681` — "Xs ago" / "Xm ago" / "Xh ago" rolling-up at 60s / 3600s. _(notes: duplicated verbatim in F2 (`f2.rs:417`), F3 (`f3.rs:632`), F4 (`f4.rs:713`).)_
- **f2-render-into** — `crates/blit-tui/src/screens/f2.rs:152-189` — Layout: header (1) / active table (Min 5) / recent table (Min 5) / footer (1). Captures `now_unix_ms` once per frame for age calculations. _(notes: SystemTime call fallback to 0 on failure.)_
- **f2-render-active-table** — `crates/blit-tui/src/screens/f2.rs:205-264` — 8 columns: daemon/transfer_id/kind/peer/module-or-path/bytes-with-percent/throughput/age. Accent highlight uses `contrasting_fg`. _(notes: m2f-6 added daemon column; bytes column = 18 chars; throughput column = 12; age column = 10.)_
- **f2-format-bytes-progress** — `crates/blit-tui/src/screens/f2.rs:593-606` — When `total > 0`, append " · NN%"; clamps to 100 on counter drift; "0 B" when `total == 0`. _(notes: u128 multiply guards overflow; clamp on `completed >= total`.)_
- **f2-format-age** — `crates/blit-tui/src/screens/f2.rs:461-477` — Returns "-" if any input is zero or now < start; otherwise "Nms" < 1s, "Ns" < 60s, "Nm" < 3600s, else "Nh". _(notes: garbage-tolerant — explicit "-" on clock skew rather than panic.)_
- **f2-format-recent-throughput** — `crates/blit-tui/src/screens/f2.rs:514-540` — Returns "-" for failed/zero-bytes/zero-duration; else "X.X TiB/s"/"GiB/s"/"MiB/s"/"KiB/s"/"N B/s". _(notes: tier list now matches F4 (post-d-25); pre-d-25 ceiling was GiB/s.)_
- **f2-format-bytes** — `crates/blit-tui/src/screens/f2.rs:562-574` — Same IEC tier set as F1/F4 (B/KiB/MiB/GiB/TiB). _(notes: explicitly duplicated per "per-screen convention" comment; see duplicated-format-bytes.)_
- **f2-format-ms** — `crates/blit-tui/src/screens/f2.rs:608-614` — "Nms" or "X.Xs" — sub-second vs second tier. _(notes: simpler than F4's `format_elapsed`.)_
- **f2-module-path** — `crates/blit-tui/src/screens/f2.rs:546-553` — `(empty, empty) → "/"`, `(empty, path) → path`, `(mod, empty) → mod`, `(mod, path) → "mod/path"`. _(notes: special-case for both-empty using "/" sentinel.)_
- **f3-render-into** — `crates/blit-tui/src/screens/f3.rs:167-199` — Layout: header (1) / table (Min 5) / stats (6) / footer (1). _(notes: stats grew 4→5→6 lines for Pull (d-33) + Subtree (d-41).)_
- **f3-render-table** — `crates/blit-tui/src/screens/f3.rs:214-268` — Uses `visible_indices()` for filter-aware rows; remaps the selected model-cursor through `visible_selected_position()` into the visible ordinal. _(notes: filter aware; TableState index addresses visible-row ordinal not model.)_
- **f3-render-stats** — `crates/blit-tui/src/screens/f3.rs:270-347` — Stats block shows Selected + View + Pull (optional pull_spec) + Subtree (du display). On `selected_row=None`, shows differentiated empty-state message from `empty_state_message()`. _(notes: file size column shows "—" for non-File kinds; du renders 4-state — Hidden/Running/Done/Error.)_
- **f3-render-footer** — `crates/blit-tui/src/screens/f3.rs:349-543` — Footer composes status + filter + marked-count + pull display + del display + batch-pull progress + tail key hints. _(notes: 7-fragment composition; filter renders even when empty if `is_editing_filter` is true.)_
- **f3-row-to-table-row** — `crates/blit-tui/src/screens/f3.rs:545-575` — Marked rows get a leading "◉ " marker + bold name; unmarked get a leading "  " 2-space padding for alignment. _(notes: padding ensures cursor highlight column-width is constant whether marked or not.)_
- **f3-format-mtime** — `crates/blit-tui/src/screens/f3.rs:598-614` — Renders epoch-seconds as YYYY-MM-DD via a hand-rolled Gregorian calc (no chrono dependency). _(notes: chrono "isn't a workspace dep here" per comment; Howard Hinnant algorithm in `days_to_ymd`.)_
- **f3-days-to-ymd** — `crates/blit-tui/src/screens/f3.rs:616-630` — Hinnant's days_from_civil inverse, adapted for i64 day count. _(notes: epoch base 719_468 days; era 146_097 days; produces (year, month, day) tuple.)_
- **f3-format-bytes** — `crates/blit-tui/src/screens/f3.rs:586-596` — **NO TiB tier** — caps at GiB. _(notes: contradicts F1/F2/F4 which include TiB; potential reformat skew on huge listings, see format-bytes-tier-mismatch.)_
- **f4-format-elapsed** — `crates/blit-tui/src/screens/f4.rs:48-67` — Compact duration: "Nms" / "N.Ns" / "Nm Ns" / "Nh Nm". Tenths-of-a-second precision under a minute. _(notes: drops tenths above 60s, drops seconds above 60m.)_
- **f4-format-rate** — `crates/blit-tui/src/screens/f4.rs:113-141` — Returns `Option<String>` — None for zero bytes, zero duration, or rate < 1 B/s. Tier ladder matches F2's `format_recent_throughput`. _(notes: optionality lets caller suppress rate; underflow safety via `< 1.0`.)_
- **f4-render-into** — `crates/blit-tui/src/screens/f4.rs:195-233` — 7-section vertical layout: header (1) / records (4) / predictor (Min 2) / verify (9) / diagnostics (3) / transfer (3) / footer (1). _(notes: only Min(2) is flexible; rest is fixed → totals ~23 rows minimum.)_
- **f4-render-transfer** — `crates/blit-tui/src/screens/f4.rs:235-291` — Local-transfer block. ConfirmingMirror/Move render bold-red confirm banners; Done appends throughput via format_rate when meaningful. _(notes: idle copy renders "press C/M/V" instructions in dark-gray.)_
- **f4-render-records-summary** — `crates/blit-tui/src/screens/f4.rs:338-362` — While `is_confirming_clear()` is true, replaces the records summary with a red `clear ALL local performance history? ...` banner (d-66). _(notes: confirm prompt usurps the block, mirroring Local-transfer confirms.)_
- **f4-render-verify** — `crates/blit-tui/src/screens/f4.rs:386-473` — 4 line block + d-17 preview lines on Done; mode hint goes Magenta if either non-default (checksum or one-way) is on. _(notes: focused field shows "value▏" caret; unfocused empty field shows "(empty)" placeholder.)_
- **f4-field-line** — `crates/blit-tui/src/screens/f4.rs:514-547` — Focused fields use accent bg + `contrasting_fg`; unfocused empty fields render "(empty)" in DarkGray. _(notes: caret character is "▏" U+258F not "_" used by F1 trigger.)_
- **f4-verify-preview-lines** — `crates/blit-tui/src/screens/f4.rs:481-512` — Categorized preview: differing → missing-on-dest → missing-on-src → errors, capped at `max`. _(notes: errors render in red; others in yellow.)_
- **f4-render-predictor** — `crates/blit-tui/src/screens/f4.rs:364-384` — Per-mode (copy/mirror) summary lines via `extend_predictor_block`. _(notes: 1 line per missing mode, 3 lines per mode with coefficients (header + planner + transfer); total = 4 lines for a half-populated predictor.)_
- **f4-format-bytes** — `crates/blit-tui/src/screens/f4.rs:699-711` — Same B/KiB/MiB/GiB/TiB ladder as F1/F2. _(notes: see duplicated-format-bytes.)_

### state-machine

- **f1-trigger-status-fork** — `crates/blit-tui/src/display_f1.rs:14-58` — Three-state machine: Idle (returns None) vs Editing vs (implicit subset of Editing with `confirming=true`). _(notes: confirming is a bool inside Editing, not a separate variant — see also f1-trigger-prompt-bridge.)_
- **f1-push-lifecycle** — `crates/blit-tui/src/display_f1.rs:67-112` — Idle (None) / Running / Done / Error mapped to PushStatusDisplay. _(notes: kind + delegated flags drive verb tense; delegated mirror still labels "mirroring" (no "delegating-mirror" verb).)_
- **f2-cancel-status-fork** — `crates/blit-tui/src/display_f2.rs:20-82` — 9 states: Idle / Confirming / ConfirmingBatch / ConfirmingClearRecent / BatchInitiated / Sending / Done × 3 outcomes / Error. _(notes: rec-4 added ConfirmingClearRecent.)_
- **f3-pull-status-fork** — `crates/blit-tui/src/display_f3.rs:16-64` — Idle / EnteringDest / Confirm / Running / Done / Error with kind-driven verb triples. _(notes: Confirm only reached for Mirror/Move; Copy bypasses it.)_
- **f3-du-status-fork** — `crates/blit-tui/src/display_f3.rs:92-105` — Path-gated 4-state machine. _(notes: gate uses `current_path == Some(path)` exact compare.)_
- **f3-del-status-fork** — `crates/blit-tui/src/display_f3.rs:128-153` — 5-state machine with `gate_path: Option<String>` discriminator for batch-vs-single semantics. _(notes: batch (`None`) always shows; single shows only while cursor on path.)_

### default-value

- **base-theme-style-default** — `crates/blit-tui/src/theme_color.rs:13-28` — Returns `None` when BOTH bg and fg are unset (caller skips painting). _(notes: only one of bg/fg set → builds partial style.)_
- **f2-cancel-status-ttl** — `crates/blit-tui/src/screens/f2.rs:48-53` — Operator-tunable via `[transfer] cancel_status_ttl_ms`; default 5000ms; clamped [250, 60000]. _(notes: clamp applied at access-time per comment.)_
- **f3-render-into-stats-height** — `crates/blit-tui/src/screens/f3.rs:190` — Stats block fixed at `Constraint::Length(6)` to fit Selected + View + Pull + Subtree. _(notes: d-33 grew 4→5, d-41 grew 5→6.)_
- **f4-render-into-verify-height** — `crates/blit-tui/src/screens/f4.rs:220` — Verify block fixed at `Constraint::Length(9)` for source/dest/mode/status + up to 3 preview lines + space. _(notes: d-17 added preview lines, hence height.)_
- **dual-pane-action-bg** — `crates/blit-tui/src/screens/dual_pane.rs:183-206` — Action labels use accent bg + contrasting fg always (not gated on focus). _(notes: matches active-tab visual treatment.)_

### timeout-or-retry

- **cancel-status-auto-hide** — `crates/blit-tui/src/display_f2.rs:33-80` — Done/Error/BatchInitiated re-render as `Hidden` once `now - finished_at >= ttl`. _(notes: state object itself NOT mutated by renderer; the loop must wake on the deadline boundary — see cancel-remaining-ttl.)_
- **compute-tick-budget** — `crates/blit-tui/src/tick_budget.rs:18-29` — Picks the shorter of live_tick_interval vs cancel_remaining; returns None if neither is needed. _(notes: 4-way match on (needs_live_tick, cancel_remaining).)_
- **min-opt-deadlines** — `crates/blit-tui/src/tick_budget.rs:40-49` — Generic `Option<Duration>` minimum. _(notes: 3-way match — defensively short on `(None, Some(b))` → returns b directly.)_
- **pull-throughput-warmup** — `crates/blit-tui/src/progress_accum.rs:98-104` — Returns 0 bytes/sec for `elapsed_secs < 1.0` to suppress meaningless initial spikes. _(notes: simple cumulative average post-warmup, NOT instantaneous rate.)_

### config-load

- **reload-tui-config** — `crates/blit-tui/src/config_reload.rs:19-26` — Wraps `config::load` with a `&mut warning` capture closure; calls `classify_reload` for the keep-vs-adopt decision. _(notes: I/O isolated from decision so latter is unit-testable.)_
- **classify-reload-keep-on-error** — `crates/blit-tui/src/config_reload.rs:33-57` — On `Some(warning)`, keeps current config (NOT the defaults the loader would have returned); message prefixed `"reload failed: ... — kept previous"`. On success, adopts loaded. _(notes: critical safety property — silent wipe was a concern.)_

### format-output

- **format-counts-full** — `crates/blit-tui/src/screens/mod.rs:212-217` — `"3 daemons · 1 active · 47 recent · ? help"` format. _(notes: middle dot separator U+00B7.)_
- **format-counts-short** — `crates/blit-tui/src/screens/mod.rs:219-224` — `"3d · 1a · 47r"` format. _(notes: drops "? help" hint per test assertion.)_
- **dual-pane-format-size** — `crates/blit-tui/src/screens/dual_pane.rs:208-221` — Option<u64> → empty string for None; B/KiB/MiB/GiB tiers (NO TiB tier). _(notes: different tier set from F1/F2/F4; ".1" not ".2" decimal precision unlike F1/F2/F4; uses base-1024 boundary `<1024 *...`.)_
- **format-bytes-pull-frag** — `crates/blit-tui/src/screens/f1.rs:142-148` — Running frag pattern `"verb → label... (N file(s) · X{rate})"`. _(notes: brace fragments composed in renderer not in display_f1.)_

### path-handling

- **del-wire-path** — `crates/blit-tui/src/del_request.rs:13-15` — Forward-slash joins regardless of client OS via `rel_path_to_string`. _(notes: explicit OS-independent wire conversion.)_
- **is-deletable-remote-path** — `crates/blit-tui/src/del_request.rs:65-74` — Rejects module roots (empty rel-path or "."), Discovery endpoints, and anything where `extract_module_and_path` errors. _(notes: mirrors `blit rm` guard; safety check against accidental whole-module nuke.)_
- **f3-du-current-path-canonical-spec** — `crates/blit-tui/src/display_f3.rs:91` — Uses `current_path: Option<&str>` (canonical path spec) for the equality check. _(notes: exact string equality — not path canonicalization; assumes upstream normalization.)_

### safety-check

- **f1-push-mirror-require-complete-scan** — `crates/blit-tui/src/exec_plan.rs:61-76` — Local→remote push mirror sets `require_complete_scan: mirror` so a partial local enumeration cannot drive the destination purge. Matches CLI's `require_complete_scan: mirror_mode`. _(notes: critical guard; Copy/Move push leave it OFF — see comment about retryable under-copies.)_
- **f3-pull-move-require-complete-scan** — `crates/blit-tui/src/exec_plan.rs:28-35` — Pull Move sets `require_complete_scan: true` so daemon refuses partial source scan before deleting remote source. _(notes: mirrors CLI's move guard `run_remote_pull_transfer_deferred(.., true)`.)_
- **delegated-pull-no-require-complete-scan** — `crates/blit-tui/src/exec_plan.rs:79-108` — Delegated transfers: daemons enumerate not the client, so client-side complete-scan guard doesn't apply. Move rejected upstream. _(notes: explicit divergence from F3-pull-move path; flagged as deliberate.)_
- **build-delete-request-filters** — `crates/blit-tui/src/del_request.rs:31-57` — Filters endpoints via `is_deletable_remote_path` BEFORE building the request; returns `None` when nothing deletable. _(notes: caller-safe — never builds an empty Purge.)_
- **f1-trigger-confirm-detail-source-classify** — `crates/blit-tui/src/display_f1.rs:46-54` — Classifies the source endpoint via `parse_transfer_endpoint` to decide whether move deletes local or remote source. _(notes: only `Endpoint::Remote(_)` says "deletes the remote source"; everything else (Local/Discovery/Err) defaults to "deletes the local source" — see also smell about Err treatment.)_

### endpoint-parse

- **f1-confirm-detail-parse-fallback** — `crates/blit-tui/src/display_f1.rs:46-54` — On `parse_transfer_endpoint(source)` returning Err, falls through to "deletes the local source". _(notes: This deviates from project memory item "Endpoint parse Err must reject" — here Err is silently classified as local source. Reviewer flagged d-61/d-68 ×3 for this exact pattern elsewhere. Audit-relevant.)_

### data-plane

- **accumulate-pull-progress** — `crates/blit-tui/src/progress_accum.rs:21-36` — Bytes from `Payload` only; files from `FileComplete` only. _(notes: avoids double-count on TCP path which emits both. Direct-gRPC path emits `FileComplete{bytes:0}` so bytes-from-Payload is correct on both paths.)_
- **accumulate-push-progress** — `crates/blit-tui/src/progress_accum.rs:50-65` — Bytes AND files from `FileComplete`; `Payload`/`ManifestBatch` defensively ignored. _(notes: push send path emits NO Payload — Payload here would report 0; defensive ignore matters if emitter changes.)_
- **accumulate-delegated-progress** — `crates/blit-tui/src/progress_accum.rs:75-88` — Both fields from `Payload` only (cumulative deltas via `report_bytes_progress`); FileComplete/ManifestBatch ignored. _(notes: three different paths, three different fold rules — see also accumulator-divergence.)_
- **du-total-from-entries** — `crates/blit-tui/src/progress_accum.rs:109-118` — Keeps entry with largest byte count; F3 du reports deepest measurement. _(notes: "best" = max bytes; ties prefer existing acc.)_

### spawn-task

- **build-f1-push-execution** — `crates/blit-tui/src/exec_plan.rs:50-76` — Constructs `PushExecution` with mirror_mode/mirror_kind/require_complete_scan derived from `kind == Mirror`. _(notes: `force_grpc: false`, `trace_data_plane: false` hardcoded; `remote_label` derived from `remote.display()`.)_
- **build-delegated-execution** — `crates/blit-tui/src/exec_plan.rs:91-108` — Constructs `DelegatedPullExecution` with `detach: false` always. _(notes: detached/F2-visible delegation is "a follow-up" — TODO marker in doc comment.)_
- **f1-push-options-from-kind** — `crates/blit-tui/src/exec_plan.rs:28-35` — `PullSyncOptions { mirror_mode, require_complete_scan }` from a single PullKind discriminant. _(notes: NB the function lives in the F3 pull options path but is reused by F1 delegated builder. Name slightly misleading.)_

### naming

- **f1-push-options-naming** — `crates/blit-tui/src/exec_plan.rs:28` — `f3_pull_options` reused for F1 delegated push by `build_delegated_execution`. _(notes: name implies F3-only; comment explains it builds `PullSyncOptions` used by delegated paths.)_
- **f1-format-bytes-naming** — `crates/blit-tui/src/screens/f1.rs:614` — `format_bytes` in F1 is for the module-capacity line, not transfer bytes; same identifier as F2/F4/F3 helpers. _(notes: per-screen convention per comment.)_
- **del-wire-path-naming** — `crates/blit-tui/src/del_request.rs:13` — Thin wrapper over `rel_path_to_string` with a renamed boundary. _(notes: explicitly named "boundary" — comment justifies the rename.)_

## Smells / risks observed

### Hardcoded strings
- **confirm-detail-strings-duplicated** — `display_f1.rs:47-53` vs `display_f3.rs:71-75`. Strings "deletes extraneous", "deletes the remote source", "deletes the local source" hardcoded in two places (F1 push direction inverts move's victim — Remote source if `Endpoint::Remote`; local source otherwise; vs F3 pull direction where move always deletes the remote source). Drift risk: if a string changes, both must update. Plus the F1 push path adds "deletes extraneous at dest" which is subtly different from F3's "deletes extraneous".
- **127.0.0.1:9031** — `screens/f1.rs:574, 585` — Hardcoded loopback address+port appears in Local-row error text ("(127.0.0.1:9031 — {message})") and spinner ("fetching GetState from 127.0.0.1:9031..."). Should be sourced from a constant; daemon port mismatch would mislead the operator.
- **Verb tables** — `display_f1.rs:122-137` — String literals `delegating/mirroring/moving/pushing` + past-tense duplicated; sister tables in `f3pull::PullKind::verbs()` upstream.
- **Tab strip counts format** — `screens/mod.rs:213, 220` — "{} daemons · {} active · {} recent · ? help" and "{}d · {}a · {}r" literals; if order changes both must update.
- **Cancel status verbs** — `screens/f2.rs:333-401` — "cancel ... y/N" / "cancelling ..." / "cancelled ..." / "cancel: id ... not found" / "cancel unsupported for ..." / "cancel ... failed" all spelled inline in render_footer.
- **Discovery footer key hints** — `screens/f1.rs:328-333` — "q/Esc quit · r refresh · ↑↓ select" hardcoded; sister key-hint blocks in F2/F3/F4 with different shapes.

### Format-bytes tier mismatch (contradiction)
- **format-bytes-tier-mismatch** — `screens/f3.rs:586-596` lacks the TiB tier present in `screens/f1.rs:614-626`, `screens/f2.rs:562-574`, `screens/f4.rs:699-711`, and the F2 tests (d-25 explicitly added TiB to F2 to align with F4). F3 still caps at GiB and would render a 2 TiB subtree total as "2048.00 GiB". The d-25 follow-on apparently never landed on F3. **Active contradiction within the cluster.**
- **dual-pane-format-size-mismatch** — `screens/dual_pane.rs:208-221` uses ".1" decimal precision AND lacks TiB tier — different from BOTH F1/F2/F4 (`.2` + TiB) and F3 (`.2` + no TiB). Three different byte formatters in one cluster.

### Duplicated patterns
- **duplicated-format-bytes** — Five separate `format_bytes` implementations: `f1.rs:614`, `f2.rs:562`, `f3.rs:586`, `f4.rs:699`, `dual_pane.rs:208`. Comment in `f1.rs:613` says "duplicated per the existing per-screen convention", but the convention is the smell.
- **duplicated-format-since** — Identical "Xs ago / Xm ago / Xh ago" helper in `f1.rs:671`, `f2.rs:417`, `f3.rs:632`, `f4.rs:713`. All four match line-by-line.
- **duplicated-row-highlight-style** — `Style::default().fg(contrasting_fg(accent)).bg(accent).add_modifier(Modifier::BOLD)` repeated in `f1.rs:286`, `f2.rs:257`, `f3.rs:257`, `dual_pane.rs:127`, `f4.rs:538`, `mod.rs:159-161`. Could be a helper.
- **duplicated-confirm-detail** — As above (`display_f1.rs:47-53` vs `display_f3.rs:71-75`).

### Endpoint parse Err treatment
- **f1-trigger-parse-err-falls-through** — `display_f1.rs:46-54` — On `parse_transfer_endpoint(source)` returning Err, code silently classifies as "deletes the local source". Per project memory: "Endpoint parse Err must reject". This is a confirmation-prompt detail (not a transfer dispatch), but the operator-facing copy lies about WHICH source gets deleted when the parser already gave up. Worth flagging — even if benign for the confirm prompt, the pattern was a recurring bug elsewhere (reviewer reopened d-61, d-68 ×3 for it).

### Push verb table omits delegated mirror/move
- **push-verb-table-missing-delegated** — `display_f1.rs:121-137` — `(true, Copy) → "delegating"` but `(true, Mirror)` and `(true, Move)` fall through to "mirroring" / "moving" with no delegated annotation. Comment claims this is intentional (the destructive dest-purge is salient and the label shows the remote dest, so context is clear). Audit-flag because the asymmetry is non-obvious from the table shape — a future Mirror delegated will read identically to a local mirror push.

### State-vs-renderer mutation surface
- **renderer-reads-finished-at-without-mutation** — `display_f2.rs:33, 49, 73, 113` — F2 cancel display flips to Hidden post-TTL but does NOT clear the underlying `F2CancelStatus`. The state object retains the Done/Error variant indefinitely until next cancel; if `cancel_remaining_ttl` is wired right, the loop wakes once then never again. **Reviewer comment lines 47-50 explicitly says "state itself stays — we don't mutate from renderer".** Potential subtle bug if the event loop ever skips the wakeup.

### Path-gate string equality (no canonicalization)
- **f3-du-path-gate-string-eq** — `display_f3.rs:91` — `current_path == Some(path)` is byte-equal compare. If upstream produces two equivalent specs that differ in casing or trailing slash, gating fails silently. Comment says `current_path` is "the cursor's canonical spec" — relies on caller normalization. No defense against caller bug.

### TODO/follow-up markers (not formal TODO comments, but explicit follow-ups in comments)
- **detach-follow-up** — `exec_plan.rs:106` — `detach: false` always; "detached/F2-visible delegation is a follow-up". Implicit TODO.
- **a1-3b-getstate-detail** — `screens/f1.rs:341` — Comment says "A follow-up slice (a1-3b-f1-getstate-detail) will populate the modules cell from a loopback `GetState` query when a local daemon is running." Active TODO marker.
- **relay-fallback-suggestable** — `exec_plan.rs:103-104` — "The TUI doesn't surface a `--relay-via-cli` toggle yet, so don't suggest it in transport-error hints." TODO posture.
- **GetState modules empty fallback** — `screens/f1.rs:427-431` — "Some daemons might return empty modules in GetState (e.g., perms)" — defensive workaround; suggests a wire-level inconsistency upstream worth tracking.

### Dead-ish defaults
- **min-opt-bias** — `tick_budget.rs:46-48` — `(None, b) => b` returns the `b` whether `b` is Some or None. Functionally correct but stylistically inconsistent with `(Some(x), None) => Some(x)`. Could be written as `(None, b) | (_, b@None) => b` for symmetry. Minor.

### Cross-cluster observations
- **identical-cancel-status-cycle-bound** — `display_f2.rs:33, 49, 73` — Three nearly-identical branches differ only in which payload they unpack. Could be folded into a single helper that returns either Hidden or a "render variants" value, removing 12 lines of structural dup.
- **f3-confirm-detail-vs-f1-confirm-detail** — Same logical question ("what gets deleted?") answered with different strings depending on push vs pull direction AND move source-side classify. Refactor candidate.

### cfg(...)/platform-specific code
- None observed in this cluster; all behavior is OS-independent (verified across all 5380 lines).

### References to deprecated/removed features
- None observed (no blit-utils, BlitAuth, AI telemetry references in any of the audited files).
