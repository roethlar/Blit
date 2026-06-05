# Plan Inventory: TUI design + rework
**Generated**: 2026-06-04 by audit workflow
**Coverage**: 2 files, 1529 lines total

| File | Lines |
|---|---|
| docs/plan/TUI_DESIGN.md | 1118 |
| docs/plan/TUI_REWORK.md | 411 |

## Claims (grouped by category)

### principle

#### principle-reuse-unified-pipeline
**Source**: TUI_DESIGN.md §4 Design principles (1)
**Specificity**: high

"Reuse the unified pipeline. The TUI's 'trigger a transfer' action instantiates a normal `RemotePushClient` / `RemotePullClient` / `DelegatedPullClient` and hands off to the exact code path the CLI uses. No alternate transfer code, no shadow planner."

#### principle-no-tui-specific-rpcs
**Source**: TUI_DESIGN.md §4 Design principles (2)
**Specificity**: high

"No TUI-specific RPCs. Every wire surface the TUI introduces (`Subscribe`, `GetState`) is generally useful — a future GUI / web client / Prometheus bridge / health-check probe all consume the same shape."

#### principle-progressive-enhancement
**Source**: TUI_DESIGN.md §4 Design principles (3)
**Specificity**: medium

"Progressive enhancement. A useful TUI exists at milestone A (discovery + browse + trigger) with NO new wire pieces. Each subsequent milestone adds visible value without breaking what came before."

#### principle-cli-parity-floor
**Source**: TUI_DESIGN.md §4 Design principles (4), TUI_REWORK.md §3 Product Principles (8)
**Specificity**: high

"CLI parity is the floor. Anything the CLI can do, the TUI can do. The CLI continues to be the scripting surface; the TUI is the interactive surface. Neither shrinks." (DESIGN). "CLI parity remains the floor. Anything available through `blit copy`, `mirror`, `move`, `rm`, `check`, `profile`, and diagnostics has a TUI home." (REWORK)

#### principle-metrics-vs-getstate-split
**Source**: TUI_DESIGN.md §4 Design principles (5)
**Specificity**: high

"`--metrics` is the operator signal, `GetState` is the programmatic one. `--metrics` stderr summaries are useful for `journalctl -u blit-daemon` watching; the TUI uses `GetState` for structured per-daemon state. The TUI does NOT scrape stderr."

#### principle-two-locations-visible
**Source**: TUI_REWORK.md §3 Product Principles (1)
**Specificity**: high

"Two locations visible. The UI optimizes for transfer work, so it shows source and destination side by side."

#### principle-visible-actions-first
**Source**: TUI_REWORK.md §3 Product Principles (2)
**Specificity**: high

"Visible actions first. Copy, mirror, move, delete, verify, and batch operations appear in an action bar or focused menu. Letter keys are optional accelerators, not the product model."

#### principle-common-controls
**Source**: TUI_REWORK.md §3 Product Principles (3)
**Specificity**: high

"Common controls carry the workflow. Arrow keys move, `Enter` opens or activates the focused control, `Space` marks/toggles, `Tab` changes focus, `Esc` backs out, `/` searches, and `?` opens help."

#### principle-browse-and-type-are-peers
**Source**: TUI_REWORK.md §3 Product Principles (4)
**Specificity**: high

"Browse and type are peers. Every path-bearing control has a visible editable path field, but no normal flow requires typing a path that is not on screen."

#### principle-destructive-work-is-reviewed
**Source**: TUI_REWORK.md §3 Product Principles (5)
**Specificity**: high

"Destructive work is reviewed. Mirror, move, delete, and batch operations show a review/confirm surface before execution."

#### principle-batch-is-first-class
**Source**: TUI_REWORK.md §3 Product Principles (6)
**Specificity**: high

"Batch is first-class. Multi-daemon fan-out is represented as a single batch draft with per-destination rows, not a sequence of wizard screens."

#### principle-tui-does-not-own-scheduling
**Source**: TUI_REWORK.md §3 Product Principles (7), §8.3
**Specificity**: high

"The TUI does not own transfer scheduling. The TUI submits transfer intent. `blit-app` / core orchestration chooses serial vs parallel execution." Also §8.3: "Serial vs parallel fan-out is an orchestrator/app concern, not a TUI preference knob."

#### principle-pick-not-type
**Source**: TUI_REWORK.md §1 Decision
**Specificity**: high

"Transfers are launched from visible action buttons, not from hidden memorized letter commands. Dialogs are used only for confirmation, focused editing, help, and batch review. Normal transfer flow must not depend on modal path pickers."

#### principle-active-source-inactive-dest
**Source**: TUI_REWORK.md §1, §4 Pane Behavior
**Specificity**: high

"The active pane supplies the source selection. The inactive pane supplies the default destination." Also: "The active pane is the source context. The inactive pane is the destination context for directional actions. If rows are marked, the marked set is the source selection. If nothing is marked, the highlighted row is the source selection."

#### principle-replacement-principle
**Source**: TUI_REWORK.md §2 Problem
**Specificity**: medium

"Make both sides of the transfer visible, make available actions visible, and let users either navigate or type without forcing either style."

#### principle-tui-is-grpc-client
**Source**: TUI_DESIGN.md §1 Purpose
**Specificity**: high

"The TUI is a **gRPC client** of one or more daemons. It is not a daemon itself. Every byte of wire shape the TUI consumes will also be consumable by a future GUI / web client on the same protocol — this design intentionally avoids TUI-specific RPCs."

### invariant

#### invariant-four-screen-architecture
**Source**: TUI_DESIGN.md §12 Structural commitments
**Specificity**: high

"Four-screen architecture (F1 / F2 / F3 / F4)." (Note: superseded by dual-pane model per TUI_REWORK; see contradictions.)

#### invariant-three-new-rpcs
**Source**: TUI_DESIGN.md §12 Structural commitments
**Specificity**: high

"Three new RPCs (`GetState`, `Subscribe`, `CancelJob`) plus the `detach: bool` field on `DelegatedPullRequest`, the `transfer_id` field on `DelegatedPullStarted`, and the `transfer_id_filter` field on `SubscribeRequest` — names and message fields are the contract. M-Jobs lands `GetState` / `CancelJob` / `detach`; C lands `Subscribe` and `transfer_id_filter`."

#### invariant-always-on-activejobs-table
**Source**: TUI_DESIGN.md §12 Structural commitments, §6.4
**Specificity**: high

"Always-on `ActiveJobs` table on `BlitService` (decoupled from `--metrics`). Ships in Milestone B; M-Jobs and C extend rows."

#### invariant-detach-delegated-only
**Source**: TUI_DESIGN.md §12, §6.5, §11
**Specificity**: high

"Daemon-owned transfer lifecycle for remote→remote (delegated) transfers when `detach=true`; client-owned (today's behavior) for everything else and for `detach=false` (CLI default)."

#### invariant-foundation-first-milestone-order
**Source**: TUI_DESIGN.md §8, §10, §12
**Specificity**: high

"Foundation-first milestone order: A.0 → B → M-Jobs → C → A.1 → D → E."

#### invariant-separate-blit-app-and-blit-tui
**Source**: TUI_DESIGN.md §12, §7.1
**Specificity**: high

"Separate `blit-app` library crate and separate `blit-tui` binary crate."

#### invariant-no-shadow-transfer-code
**Source**: TUI_DESIGN.md §12 Structural commitments
**Specificity**: high

"Reuse the unified pipeline; no shadow transfer code."

#### invariant-pane-state-fields
**Source**: TUI_REWORK.md §4 Pane Behavior
**Specificity**: high

"Each pane owns: A `Location`: local path, remote daemon discovery row, remote module, or remote subpath. An editable path bar. A row list. A cursor. A marked set. Filter/search state. Sort and display preferences."

#### invariant-metrics-does-not-gate-observability
**Source**: TUI_DESIGN.md §6.4
**Specificity**: high

"`GetState` and `Subscribe` always work; an operator running the daemon expects to be able to ask it what it's doing without a separate opt-in. When `--metrics` is off the `GetState.counters` block returns zero (because the underlying atomics weren't incremented), but `GetState.active[]` / `GetState.recent[]` and the Subscribe event stream remain fully populated."

#### invariant-each-milestone-shippable
**Source**: TUI_DESIGN.md §8 (each independently shippable), TUI_REWORK.md §9
**Specificity**: medium

"Each milestone ships as a reviewed slice and leaves the TUI usable." (REWORK). DESIGN §8: "Milestones (each independently shippable)".

#### invariant-state-types-separation
**Source**: TUI_REWORK.md §8.1
**Specificity**: high

"browser state, selection state, transfer intent, and execution state must not stay tangled in one main-loop structure."

#### invariant-local-mode-first-class
**Source**: TUI_DESIGN.md §10 Decisions taken
**Specificity**: high

"Local-only TUI mode is first-class. `blit-tui` works without any daemon on the network. 'Local' appears in the F1 daemon list as a sentinel row so the F3 / F2 flows treat it symmetrically with remote daemons. Driving a `blit copy` between two local paths must work from inside the TUI."

#### invariant-rework-no-wire-change-required
**Source**: TUI_REWORK.md §11 Compatibility And Non-Goals
**Specificity**: high

"No daemon wire change is required for the shell itself. No transfer hot-path change is required by the UI rework. The rework must not slow copy/mirror/move data paths. CLI verbs are untouched."

### interface

#### iface-subscribe-rpc
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"`Subscribe` — server-streaming live events. `rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);` `SubscribeRequest` has `event_mask` (uint32, bitfield with TRANSFERS=1, ERRORS=2, MODULES=4, HEARTBEAT=8; 0 = all), `replay_recent` (bool — replay in-memory recent-event ring on connect), and `transfer_id_filter` (string — empty = all events; set to a transfer_id to receive only that job's events plus a per-job replay buffer)."

#### iface-daemon-event-oneof
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"`DaemonEvent` oneof payload: `TransferStarted transfer_started = 1`, `TransferProgress transfer_progress = 2`, `TransferComplete transfer_complete = 3`, `TransferError transfer_error = 4`, `ModuleListChanged module_list_changed = 5`, `DaemonHeartbeat heartbeat = 6`."

#### iface-transfer-started-message
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"`TransferStarted { string transfer_id; Kind kind (PUSH=0, PULL=1, PULL_SYNC=2, DELEGATED_PULL=3, PURGE=4); string peer; string module; string path; uint64 start_unix_ms; }`"

#### iface-transfer-progress-message
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"`TransferProgress { string transfer_id; uint64 bytes_completed; uint64 bytes_total (0 when not yet known); uint64 files_completed; uint64 files_total; uint64 throughput_bps (1-second EWMA); }`"

#### iface-transfer-complete-message
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"`TransferComplete { string transfer_id; uint64 bytes; uint64 files; uint64 duration_ms; bool tcp_fallback_used; }`"

#### iface-transfer-error-message
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"`TransferError { string transfer_id; string message; }`"

#### iface-getstate-rpc
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"`GetState` — daemon snapshot. `rpc GetState(GetStateRequest) returns (DaemonState);` `GetStateRequest { uint32 recent_limit (0 = use daemon default 50); }`"

#### iface-daemon-state-message
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"`DaemonState { string version; uint64 uptime_seconds; repeated ModuleInfo modules (existing message reuse); repeated ActiveTransfer active; repeated TransferRecord recent; Counters counters; bool delegation_enabled (mirrors mDNS TXT); }`"

#### iface-active-transfer-message
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"`ActiveTransfer { string transfer_id; TransferStarted.Kind kind; string peer; string module; string path; uint64 start_unix_ms; uint64 bytes_completed; uint64 bytes_total; }`"

#### iface-transfer-record-message
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"`TransferRecord { string transfer_id; TransferStarted.Kind kind; string peer; string module; string path; uint64 start_unix_ms; uint64 duration_ms; uint64 bytes; uint64 files; bool ok; string error_message (empty when ok=true); }`"

#### iface-counters-message
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"`Counters { uint64 push_operations_total; uint64 pull_operations_total; uint64 purge_operations_total; uint64 active_transfers; uint64 transfer_errors_total; }`"

#### iface-canceljob-rpc
**Source**: TUI_DESIGN.md §6.5
**Specificity**: high

"`CancelJob` RPC: `rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);` `CancelJobRequest { string transfer_id; }` `CancelJobResponse { bool cancelled (true if job existed and cancellation was initiated, false if not found); string reason (human-readable; empty when cancelled=true); }`"

#### iface-detach-field-location
**Source**: TUI_DESIGN.md §6.5 (1)
**Specificity**: high

"`detach: bool` field on `DelegatedPullRequest`. Field lives on the delegated-pull request itself, **not** on `TransferOperationSpec`. Reason: the spec is shared with PullSync, where the bit can never mean anything; putting it on the universal spec would create a footgun where a `pull_sync` client could set `detach=true` and confuse the daemon. Local-endpoint paths (push / pull_sync) don't get the field at all."

#### iface-existing-rpcs-unchanged
**Source**: TUI_DESIGN.md §6.1
**Specificity**: high

"What we already use (no changes): `ListModules`, `List`, `Find`, `DiskUsage`, `FilesystemStats`, `Push`, `PullSync`, `DelegatedPull`, `Purge`. The TUI is a normal client of all of these."

#### iface-app-progress-event-channel
**Source**: TUI_DESIGN.md §7.4, §10
**Specificity**: high

"`blit-app` exposes a single event channel pattern. Every orchestration entry point takes an `Option<mpsc::UnboundedSender<AppProgressEvent>>` (channel-based, mirroring `RemoteTransferProgress`'s existing shape)." Also §10: "`AppProgressEvent` is channel-based (`mpsc::UnboundedSender`), mirroring the existing `RemoteTransferProgress` shape."

#### iface-location-enum
**Source**: TUI_REWORK.md §8.1
**Specificity**: high

"`enum Location { Local(PathBuf), Remote(RemoteEndpoint), }`"

#### iface-pane-state-struct
**Source**: TUI_REWORK.md §8.1
**Specificity**: high

"`struct PaneState { id: PaneId, location: Location, entries: Vec<BrowserEntry>, cursor: usize, marked: BTreeSet<EntryId>, filter: String, path_editor: PathEditorState, }`"

#### iface-browser-entry-struct
**Source**: TUI_REWORK.md §8.1
**Specificity**: high

"`struct BrowserEntry { name: String, location: Location, kind: EntryKind, size: Option<u64>, mtime_seconds: Option<i64>, read_only: bool, }`"

#### iface-transfer-draft-struct
**Source**: TUI_REWORK.md §8.1
**Specificity**: high

"`struct TransferDraft { action: TransferAction, sources: Vec<Location>, destinations: Vec<Location>, options: TransferOptions, }`"

#### iface-batch-transfer-draft-struct
**Source**: TUI_REWORK.md §8.1
**Specificity**: high

"`struct BatchTransferDraft { action: TransferAction, sources: Vec<Location>, destinations: Vec<BatchDestination>, options: TransferOptions, }`"

#### iface-browse-provider-trait
**Source**: TUI_REWORK.md §8.2
**Specificity**: high

"`trait BrowseProvider { async fn list(&self, location: &Location) -> Result<Vec<BrowserEntry>>; async fn complete(&self, location: &Location, prefix: &str) -> Result<Vec<BrowserEntry>>; }`"

#### iface-required-providers
**Source**: TUI_REWORK.md §8.2
**Specificity**: high

"Required providers: `LocalBrowseProvider`: lists local filesystem paths. Local metadata must be translated into the same display shape as remote `DirEntry` rows: size, mtime, directory/file/symlink kind, and read-only hints where available. `RemoteBrowseProvider`: reuses existing daemon RPCs and `blit-app` admin helpers for modules, `ls`, `find`, `du`, and `df`."

### behavior

#### behavior-cli-verb-map
**Source**: TUI_DESIGN.md §2
**Specificity**: high

CLI verb -> TUI home parity table: `blit scan`→F1 Daemons; `blit list-modules <host>`→F1 selection drilldown; `blit ls <target>`→F3 Browse; `blit find <target>`→F3 Browse `/` hotkey; `blit du <target>`→F3 Browse sidebar; `blit df <module>`→F1 daemon detail pane; `blit rm <target>`→F3 Browse Delete key; `blit copy <src> <dst>`→F2 Transfer; `blit mirror <src> <dst>`→F2 Transfer with --mirror; `blit move <src> <dst>`→F2 Transfer; `blit check <a> <b>`→F4 Verify; `blit completions ...`→N/A (shell-only); `blit profile`→F4 Profile/status bar; `blit diagnostics perf`→F4 settings panel; `blit diagnostics dump`→F4 dump snapshot action.

#### behavior-no-verb-unrepresented
**Source**: TUI_DESIGN.md §2 (closing paragraph)
**Specificity**: high

"This map drives the screen list (§5). No verb is unrepresented; nothing in the TUI requires a fresh code path the CLI doesn't already exercise."

#### behavior-detach-default-on-tui-remote-remote
**Source**: TUI_DESIGN.md §5.2, §6.5
**Specificity**: high

"Transfers kicked off from the TUI use `detach=true` on the `DelegatedPullRequest` **for remote→remote transfers only**. For local-endpoint transfers (local↔daemon, local↔local) the TUI stays attached for the duration and shows a banner on the transfer-options modal noting that closing the TUI cancels."

#### behavior-cli-detach-only-on-remote-remote
**Source**: TUI_DESIGN.md §6.5 (1), §6.5 CLI surface
**Specificity**: high

"The CLI's `--detach` flag is **only accepted on remote→remote transfers**; on push / pull / pull_sync it produces a clean error message ('--detach only applies to remote→remote transfers; the CLI is in the byte path for this transfer shape')."

#### behavior-canceljob-timeout
**Source**: TUI_DESIGN.md §6.5
**Specificity**: high

"[`CancelJob`] Implementation reaches into the `ActiveJobs` entry, fires a `CancellationToken` the spawn closure is watching, and waits briefly for the transfer to wind down (sink finish, partial-file cleanup if any). Returns once the job is removed from the active table or after a 5-second timeout, whichever first."

#### behavior-broadcast-lagged-handling
**Source**: TUI_DESIGN.md §6.2
**Specificity**: high

"Daemon-side: a `tokio::broadcast` inside `BlitService` collects events from the existing push/pull/purge handlers. `Subscribe` clients subscribe to the channel. Slow consumers drop with a `Lagged` notification (TUI re-fetches via `GetState`)."

#### behavior-progress-event-cadence
**Source**: TUI_DESIGN.md §6.2 (step 1)
**Specificity**: high

"Per-payload byte counters in the data-plane write loop. Add a `report_payload_bytes(transfer_id, delta_bytes)` call inside `receive_stream_double_buffered` (data_plane.rs:135) and the equivalent local-sink write loops. Cadence: every N bytes (e.g. 1 MiB) OR every M milliseconds, whichever fires first. Bounded so we don't drown the broadcast channel on a fast NVMe-to-NVMe local transfer."

#### behavior-active-transfers-table-keying
**Source**: TUI_DESIGN.md §6.2 (step 2)
**Specificity**: high

"Per-transfer state machine in `BlitService`. Today the service has no notion of 'transfer X is running, here's its cumulative byte count.' Milestone C introduces an `ActiveTransferTable: HashMap<transfer_id, ActiveState>` keyed by the UUID minted at `TransferStarted` time."

#### behavior-throughput-ewma-server-side
**Source**: TUI_DESIGN.md §6.2 (step 3)
**Specificity**: high

"Throughput EWMA daemon-side. 1-second exponential moving average over the byte counter so every subscriber sees identical numbers. Computed in the table updater, not in each subscriber."

#### behavior-transfer-id-allocation-site
**Source**: TUI_DESIGN.md §6.2 (step 4), §10 (5)
**Specificity**: high

"`transfer_id` is minted at the dispatch boundary in `service/core.rs` (next to where `metrics.inc_push()` fires) and threaded through `handle_push_stream` / `stream_pull` / `handle_pull_sync_stream` / `handle_delegated_pull` to the write loops." §10: "Transfer-id allocation. UUIDv4 daemon-side at the dispatch boundary."

#### behavior-counters-reads-from-existing-atomics
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"`Counters` reads directly from the existing `TransferMetrics` atomics — no new bookkeeping."

#### behavior-activejobs-drained-on-completion
**Source**: TUI_DESIGN.md §6.3
**Specificity**: high

"Drained on TransferComplete / TransferError. `GetState.active[]` reads from this table directly. Recent transfers live in a parallel in-memory ring of size `recent_limit` (default 50), pushed when an `ActiveJob` drains."

#### behavior-tui-aggregation-client-side
**Source**: TUI_DESIGN.md §6.5 deferred section, §10 (4)
**Specificity**: medium

"Cross-daemon job aggregation. Each daemon owns its own jobs. A TUI that wants 'show me everything running on every daemon I can see' opens N parallel Subscribe streams and aggregates client-side."

#### behavior-spawn-closure-disarm-on-detach
**Source**: TUI_DESIGN.md §6.5 (3)
**Specificity**: high

"Spawn-closure lifecycle change. The `delegated_pull` dispatch site consults `req.detach`. When false (default), behavior is unchanged — `tx.closed()` cancellation race still arms (R30-F2). When true, the cancellation race is **disarmed**: the daemon owns the transfer through to completion or `CancelJob(id)`. Push / pull / pull_sync sites are unchanged; they cannot detach."

#### behavior-jobs-watch-implementation
**Source**: TUI_DESIGN.md §6.5 (5), §8 Milestone M-Jobs
**Specificity**: high

"`blit jobs watch <remote> <transfer_id>` CLI verb. Polls `GetState` until the transfer drains into `recent[]` or `--timeout-secs` fires. Exit codes 0 (finished + ok), 1 (finished + failed), 2 (not found), 3 (timeout). Stopgap until milestone C's `Subscribe` stream lands; the verb name stays, the implementation upgrades."

#### behavior-action-bar-context-sensitive
**Source**: TUI_REWORK.md §6 Actions
**Specificity**: high

"The action bar is context-sensitive. It derives enabled actions from the active selection, inactive pane, daemon capabilities, read-only module status, and operation kind."

#### behavior-action-labels-change-with-focus
**Source**: TUI_REWORK.md §4 Main Shell
**Specificity**: high

"The labels change with focus. If the right pane is active, the directional actions read `Copy -> Left`, `Mirror -> Left`, and `Move -> Left`."

#### behavior-selection-rules
**Source**: TUI_REWORK.md §4 Pane Behavior
**Specificity**: high

"If rows are marked, the marked set is the source selection. If nothing is marked, the highlighted row is the source selection."

#### behavior-places-jump-tools-not-sidebar
**Source**: TUI_REWORK.md §4 Places, Favorites, And Recents
**Specificity**: medium

"Favorites and recents are jump tools, not a permanent third sidebar. They should help navigation without stealing the transfer model away from the two panes."

#### behavior-places-content
**Source**: TUI_REWORK.md §4 Places, Favorites, And Recents
**Specificity**: high

"Each pane has a `Places` control... It contains: Current launch directory (`$PWD`) as the default local start. Home. Filesystem root. Configured favorites. Recent locations. Discovered remotes. Known daemon endpoints from config."

#### behavior-large-dir-listing-nonblocking
**Source**: TUI_REWORK.md §8.2
**Specificity**: high

"Local listing must not block the render loop on large directories. Use `spawn_blocking` or an async worker once listing cost can exceed a small threshold."

#### behavior-action-table
**Source**: TUI_REWORK.md §6 Actions baseline
**Specificity**: high

Baseline visible actions: `Copy -> Other Pane` (copies active selection into inactive pane's current directory); `Mirror -> Other Pane` (mirrors after review); `Move -> Other Pane` (copies then deletes after review); `Delete` (after confirmation); `Verify` (opens verify draft using active selection and inactive pane); `Copy/Mirror/Move to Multiple...` (opens fan-out batch table); `Places`; `Transfers`; `Profile/Diagnostics`; `More`.

#### behavior-batch-fanout-rules
**Source**: TUI_REWORK.md §7 Fan-Out Batch Table
**Specificity**: high

"Every destination is shown before execution. Destinations default from the inactive pane, recent destinations, or the selected daemon's closest matching module/path. Each checked daemon has its own destination. Shared defaults are allowed, but per-destination override is required. `[Change]` opens a destination editor using the same browsable pane model plus an editable path bar. Typing is allowed; browsing is always available. `Run Batch` submits a single `BatchTransferDraft` to the app layer."

#### behavior-batch-progress-tree
**Source**: TUI_REWORK.md §7
**Specificity**: medium

"The transfer view shows a collapsible parent row for the batch with child rows per destination."

#### behavior-existing-transfer-screen-from-action-bar
**Source**: TUI_REWORK.md §8.4
**Specificity**: medium

"Transfers: a drawer or full-screen list opened from the action bar or Places list. It continues to consume `GetState` and `Subscribe`."

#### behavior-aliases-during-migration
**Source**: TUI_REWORK.md §8.4, §11
**Specificity**: medium

"Existing F-keys and old letter shortcuts may remain as aliases during migration, but help text must present the visible action model first." §11: "The old F1/F2/F3/F4 model may remain as aliases during migration, but it is no longer the product model."

#### behavior-small-terminal-fallback
**Source**: TUI_REWORK.md §8.5
**Specificity**: medium

"Below a practical width threshold, the shell may collapse to one visible pane with an inactive-pane summary line and a `Swap Pane` / `Show Other Pane` action. This is a responsive fallback only; the state model still tracks two panes."

#### behavior-tui-uses-detach-true-by-default
**Source**: TUI_DESIGN.md §5.2
**Specificity**: high

"The TUI uses `detach=true` on every transfer it initiates against a remote→remote pair; for one-local-endpoint transfers initiated from the TUI, the TUI stays connected for the duration (same constraint as the CLI)."

#### behavior-cli-detach-error-message
**Source**: TUI_DESIGN.md §6.5 CLI surface
**Specificity**: high

"`blit copy <hostA>:/src <hostB>:/dst --detach` — accepted **only on remote→remote transfers**. Initiates, prints `transfer_id`, exits. Destination daemon owns the transfer. Push (local→remote) and pull (remote→local) reject `--detach` with a clean error message ('CLI is in the byte path for this transfer shape; --detach only applies to remote→remote transfers')."

#### behavior-jobs-cli-verbs
**Source**: TUI_DESIGN.md §6.5 CLI surface
**Specificity**: high

CLI surface: `blit jobs list <remote>` calls `GetState` and prints active+recent. `blit jobs cancel <remote> <transfer_id>` calls `CancelJob`. `blit jobs watch <remote> <transfer_id>` follows a transfer to completion; ships in M-Jobs as `GetState` polling loop with `--interval-ms`, `--timeout-secs`, `--json`; milestone C upgrades to `Subscribe` stream with `transfer_id_filter`.

#### behavior-cli-keeps-working-during-refactor
**Source**: TUI_DESIGN.md §8 Milestone A.0
**Specificity**: high

"CLI keeps working unchanged. Library now consumable." Also: "Workspace test suite must stay green; CI / packaging scripts that reference module paths get updated."

### scope

#### scope-tui-screen-list
**Source**: TUI_DESIGN.md §5
**Specificity**: high

"Four primary screens, switchable with hotkeys. Status bar at the bottom; modal overlays for confirmation / option entry. [F1] Daemons [F2] Transfers [F3] Browse [F4] Profile/Verify"

#### scope-f4-verify-local-only
**Source**: TUI_DESIGN.md §5.4, §10 (6)
**Specificity**: high

"Verify pane wraps the existing `blit check` code path, rendered as a diff (matches / size-diff / mtime-diff / missing-on-side rows). **Local paths only** in Milestone D — matches current `blit check` semantics."

#### scope-a0-extracts-full-verb-surface
**Source**: TUI_DESIGN.md §7.3
**Specificity**: high

"A.0 moves **every** orchestration verb, not just transfer. The TUI's F1 / F3 / F4 screens drive scan / ls / find / du / df / rm / list-modules / profile in addition to transfers, and all of those modules are binary-only in `blit-cli` today. Splitting A.0 into 'transfer core now, admin/browser later' would just defer the same refactor a second time before A.1 could start — not worth it."

#### scope-what-stays-in-blit-cli
**Source**: TUI_DESIGN.md §7.3
**Specificity**: high

"What stays in `blit-cli`: `cli.rs` — clap definitions and arg structs; `main.rs` — the dispatcher mapping clap args to `blit-app`; `completions.rs` — `clap_complete` shell completion generation (clap-coupled by definition); `context.rs` — config / env wiring (CLI-shaped); the indicatif progress integration; the human / JSON output formatters."

#### scope-what-blit-tui-adds
**Source**: TUI_DESIGN.md §7.3
**Specificity**: high

"What `blit-tui` adds: ratatui screens (§5); its own consumer of `AppProgressEvent` that pushes events to the UI event loop; its own rendering layer (no shared formatters with the CLI; the TUI wants tables / progress bars, not stdout text); a small input-modal system for collecting source / destination / option choices."

#### scope-blit-app-prerequisite
**Source**: TUI_DESIGN.md §7.2
**Specificity**: high

"Today `blit-cli` is a **binary-only** crate (no `[lib]` in `crates/blit-cli/Cargo.toml`). Every reusable orchestration entry point lives under `crates/blit-cli/src/<module>.rs` and is reachable only from `main.rs`."

#### scope-a0-effort-estimate
**Source**: TUI_DESIGN.md §7.3, §7.5
**Specificity**: medium

"Adding the nine admin/browser/profile modules to A.0 grows the refactor from '~2–3 days' to **~4–5 days**." §7.5: "Rough estimate: 2–3 days of focused work. **Belongs in Milestone A**, before any TUI screen code is written. Treat it as the 'Milestone A.0' preamble." (Note inconsistent estimates; see contradictions.)

#### scope-three-paths-considered
**Source**: TUI_DESIGN.md §7.2
**Specificity**: medium

"The three available paths: 1. Add `[lib]` to `blit-cli`. Cheapest. But the existing modules are coupled to `clap::Args` shapes, indicatif progress bars, and `println!` summary formatting. 2. Extract a clean `blit-app` library. More work, but right architecture. 3. Shell out from `blit-tui` to `blit`. Subprocess management, parsing `--json` output, no shared progress channel, fragile error handling. Rejected. **Decision: option 2 — extract `blit-app`.**"

#### scope-phase5-budget
**Source**: TUI_DESIGN.md §11
**Specificity**: medium

"Total roughly 7–8 kLOC for the full feature surface (plus the A.0 refactor, which nets near zero new code)."

#### scope-rework-workflows
**Source**: TUI_REWORK.md §5
**Specificity**: high

"Core Workflows: W1 - Remote To Local; W2 - Local To Local; W3 - Local To Many Remotes; W4 - Remote A To Remote B."

#### scope-rework-supersedes-shipped-text-input
**Source**: TUI_REWORK.md header, TUI_DESIGN.md Rework note 2026-05-31
**Specificity**: high

"Supersedes: the shipped `TUI_DESIGN.md` text-input trigger flows and the first `TUI_REWORK.md` picker-modal plan." DESIGN: "the trigger-modal text inputs, F-key-centric pane model, and F3 free-text pull destination prompt are superseded by `TUI_REWORK.md`."

#### scope-phasing-table
**Source**: TUI_DESIGN.md §11
**Specificity**: high

Phasing table: 1) A.0 — extract `blit-app` (no wire changes, ~4–5 days mechanical moves). 2) B — `GetState` + `ActiveJobs` table + recent ring (+`GetState`, ~500 daemon + ~100 CLI). 3) M-Jobs — daemon-owned lifecycle (delegated-only) + `CancelJob` + `detach` (+`CancelJob`, +`detach` on `DelegatedPullRequest`, +`transfer_id` on `DelegatedPullStarted`, ~500 daemon + ~200 CLI). 4) C — `Subscribe` + per-job event ring + byte-level instrumentation (+`Subscribe`, +`SubscribeRequest.transfer_id_filter`, ~1500 daemon + ~100 CLI). 5) A.1 — the TUI itself (none, ~3000). 6) D — Verify + diagnostics (none, ~400 TUI). 7) E — polish (none, ~600).

### non-goal

#### non-goal-http-prometheus-direct
**Source**: TUI_DESIGN.md §9
**Specificity**: high

"HTTP / Prometheus endpoints directly on the daemon. A separate bridge program can scrape `GetState` if Prometheus is needed."

#### non-goal-web-ui-bundled
**Source**: TUI_DESIGN.md §9
**Specificity**: high

"Web UI bundled with the daemon. Same wire works for a future browser client, but the browser client is a separate codebase."

#### non-goal-daemon-initiated-push
**Source**: TUI_DESIGN.md §9
**Specificity**: high

"Daemon-initiated push. Subscribe is server→client streaming over a client-initiated connection."

#### non-goal-structured-logs-stream
**Source**: TUI_DESIGN.md §9
**Specificity**: high

"Live structured logging stream. F15 deferred; if structured logs are wanted in the TUI later, they ride on Subscribe."

#### non-goal-authentication
**Source**: TUI_DESIGN.md §9
**Specificity**: high

"Authentication. Same scope decision as §5.2 of the release plan — operator network controls, no app-level auth."

#### non-goal-rework-prometheus-unrelated
**Source**: TUI_REWORK.md §11
**Specificity**: medium

"The Prometheus bridge question is unrelated."

#### non-goal-rework-no-daemon-wire-change
**Source**: TUI_REWORK.md §11
**Specificity**: high

"No daemon wire change is required for the shell itself."

#### non-goal-rework-no-hot-path-change
**Source**: TUI_REWORK.md §11
**Specificity**: high

"No transfer hot-path change is required by the UI rework. The rework must not slow copy/mirror/move data paths."

#### non-goal-cli-completions-shell-only
**Source**: TUI_DESIGN.md §2 (table)
**Specificity**: high

"`blit completions ...` — N/A — Shell-only; the TUI has its own input affordances."

### deferred

#### deferred-perjob-event-ring-to-c
**Source**: TUI_DESIGN.md §6.5
**Specificity**: high

"Deferred from M-Jobs to milestone C (they only feed `Subscribe`, which doesn't exist yet): Per-job event ring (~50 events). Each `ActiveJob` will keep the last N events seen for that job in addition to the global event ring. `Subscribe` with `transfer_id_filter` set replays this ring on connect, then attaches to the live broadcast."

#### deferred-transfer-id-filter-to-c
**Source**: TUI_DESIGN.md §6.5
**Specificity**: high

"`transfer_id_filter` field on `SubscribeRequest`. Can't add a field to a message that doesn't exist yet; C defines both the message and the field together."

#### deferred-durability-to-0-2-0
**Source**: TUI_DESIGN.md §6.5, §10 (7)
**Specificity**: high

"Durability across daemon restart. Job state lives in memory only. If the daemon restarts, in-flight detached transfers fail; the user re-kicks; block-level resume picks up where it left off. On-disk job state + replay-on-startup is a heavier 0.2.0 design."

#### deferred-recent-persistence
**Source**: TUI_DESIGN.md §6.3, §10 (1)
**Specificity**: medium

"Persistence (durability across daemon restart) is deferred to 0.2.0+." §10: "Recent-transfer persistence... in-memory ring for B; if persistence is wanted later, reuse `perf_local` in Milestone E."

#### deferred-remote-verify
**Source**: TUI_DESIGN.md §10 (6)
**Specificity**: medium

"Recommend: ship F4 Verify local-only in Milestone D (matches today's `blit check`); revisit remote-verify in a later milestone if operator demand justifies it."

#### deferred-cross-daemon-aggregation
**Source**: TUI_DESIGN.md §6.5
**Specificity**: medium

"Cross-daemon job aggregation. Each daemon owns its own jobs. A TUI that wants 'show me everything running on every daemon I can see' opens N parallel Subscribe streams and aggregates client-side." (Server-side aggregation deferred.)

#### deferred-cli-byte-level-progress
**Source**: TUI_DESIGN.md §6.2 (closing)
**Specificity**: low

"What this implicitly buys for the CLI: once byte-level progress is on the wire, the CLI's existing progress bar (today driven by file-complete events from `report_file_complete`) can become a true byte-level bar... That's not a Phase 5 deliverable, but it's a free byproduct."

### rejected

#### rejected-tui-specific-rpcs
**Source**: TUI_DESIGN.md §4 (2)
**Specificity**: high

(Re: design principle 2) — TUI-specific RPCs are explicitly disallowed; every new RPC must be generally useful.

#### rejected-shell-out-from-blit-tui
**Source**: TUI_DESIGN.md §7.2
**Specificity**: high

"Shell out from `blit-tui` to `blit`. Subprocess management, parsing `--json` output, no shared progress channel, fragile error handling. Rejected."

#### rejected-add-lib-to-blit-cli
**Source**: TUI_DESIGN.md §7.2
**Specificity**: high

"Add `[lib]` to `blit-cli`. Cheapest. But the existing modules are coupled to `clap::Args` shapes (`TransferArgs`, `CheckArgs`, etc.), indicatif progress bars, and `println!` summary formatting. The TUI would inherit all of that — it needs custom progress + custom rendering, not the CLI's." (Implicitly rejected by Decision: option 2.)

#### rejected-metrics-gates-event-emission
**Source**: TUI_DESIGN.md §6.4
**Specificity**: high

"This is a refinement of the original draft, which claimed `--metrics` gated event emission. Doing that would make the TUI mostly useless against daemons that hadn't opted in, which is the wrong default for the single-pane-of-glass model."

#### rejected-detach-on-transfer-operation-spec
**Source**: TUI_DESIGN.md §6.5 (1)
**Specificity**: high

"Field lives on the delegated-pull request itself, **not** on `TransferOperationSpec`. Reason: the spec is shared with PullSync, where the bit can never mean anything; putting it on the universal spec would create a footgun where a `pull_sync` client could set `detach=true` and confuse the daemon."

#### rejected-detach-on-push-or-pullsync
**Source**: TUI_DESIGN.md §6.5
**Specificity**: high

"Only DelegatedPull (remote→remote) is structurally detachable — the CLI isn't in the byte path, so it can drop the control stream and the transfer continues. Push and PullSync put the CLI directly in the byte path; detach is architecturally impossible without first lifting the bytes off the CLI (which would mean re-architecting push/pull as daemon-to-daemon DelegatedPull variants — out of Phase 5 scope)."

#### rejected-thin-shim-progress
**Source**: TUI_DESIGN.md §6.2
**Specificity**: medium

"A 'thin shim at 10 Hz over SinkOutcome' wouldn't help: SinkOutcome is also coarse, computed from completed payloads. Milestone C therefore needs real byte-level instrumentation, not just a broadcast shim."

#### rejected-modal-picker-flows
**Source**: TUI_REWORK.md §1, §2
**Specificity**: high

"Normal transfer flow must not depend on modal path pickers." §2: "The first rework proposal fixed path recall with destination pickers, but it still made users leave their visual context and walk through modal flows. That is not the right primary model for a file transfer tool."

#### rejected-letter-commands-as-primary
**Source**: TUI_REWORK.md §1, §3 (2)
**Specificity**: high

"It rejects the bad part: primary workflows must not require users to remember `c`, `m`, `d`, `y`, `p`, or any other letter command. Letter accelerators may exist later, but only as optional shortcuts shown next to visible controls."

#### rejected-permanent-third-sidebar
**Source**: TUI_REWORK.md §4 Places
**Specificity**: medium

"Favorites and recents are jump tools, not a permanent third sidebar."

#### rejected-asking-owner-for-low-level-key-choices
**Source**: TUI_REWORK.md §12 Review History
**Specificity**: medium

"2026-05-31: Owner feedback rejected asking for low-level key choices and reinforced that UI should make the tool simpler without requiring the user to design it."

### shipped

#### shipped-mdns-discovery
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"Daemon discovery — mDNS — `_blit._tcp.local.` — ✅ Already advertised"

#### shipped-mdns-txt-keys
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"Daemon identity at-a-glance — mDNS TXT — `version`, `modules`, `module_count`, `delegation_enabled` — ✅ Shipped `0d76c4f`"

#### shipped-list-modules-rpc
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"Module list — gRPC `ListModules` — per-daemon RPC — ✅ Already exists"

#### shipped-browse-rpcs
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"File browse — gRPC `List` / `Find` / `DiskUsage` / `FilesystemStats` — per-target RPC — ✅ Already exist"

#### shipped-transfer-rpcs
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"Trigger transfer — gRPC `Push` / `PullSync` / `DelegatedPull` — normal client flow — ✅ Already exists"

#### shipped-purge-rpc
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"Remove files — gRPC `Purge` — normal client flow — ✅ Already exists"

#### shipped-blit-check-local-only
**Source**: TUI_DESIGN.md §3, §5.4
**Specificity**: high

"Tree comparison — (none yet — CLI runs this locally) — Local read of both endpoints — ✅ Today's `blit check` is local-only and read-only"

#### shipped-transfer-metrics-counters
**Source**: TUI_DESIGN.md §3
**Specificity**: high

"Aggregate counters — `TransferMetrics` on `BlitService` — currently exposed only via `--metrics` stderr summaries (§3.1) — Exposed over wire by `GetState`"

#### shipped-dual-pane-default
**Source**: TUI_REWORK.md header
**Specificity**: high

"dual-pane is the default `blit-tui` shell as of M1-M3 work on 2026-05-31."

#### shipped-v0-1-0-tui-baseline
**Source**: TUI_DESIGN.md Rework note 2026-05-31
**Specificity**: medium

"this document describes the shipped v0.1.0 TUI baseline."

### decision

#### decision-crate-split
**Source**: TUI_DESIGN.md §10
**Specificity**: high

"Crate split: separate `blit-tui` crate using `ratatui`. Not bundled into the default `blit` binary."

#### decision-foundation-first-order
**Source**: TUI_DESIGN.md §10, §8
**Specificity**: high

"Foundation-first milestone order. A.0 → B → M-Jobs → C → A.1 → D → E. TUI ships as a real network resource from its first release."

#### decision-cancellation-server-side
**Source**: TUI_DESIGN.md §10
**Specificity**: high

"Cancellation: server-side via `CancelJob`. TUI's cancel hotkey fires `CancelJob(transfer_id)` (M-Jobs, §6.5). Works on transfers the TUI didn't initiate. Replaces the original draft's 'client-side Ctrl-C' approach."

#### decision-cli-detach-ships-with-mjobs
**Source**: TUI_DESIGN.md §10
**Specificity**: high

"CLI `--detach` ships with M-Jobs. Plus `blit jobs list / cancel / watch` admin verbs."

#### decision-app-progress-event-channel
**Source**: TUI_DESIGN.md §10
**Specificity**: high

"`AppProgressEvent` is channel-based (`mpsc::UnboundedSender`), mirroring the existing `RemoteTransferProgress` shape. See §7.4."

#### decision-local-mode-first-class
**Source**: TUI_DESIGN.md §10
**Specificity**: high

"Local-only TUI mode is first-class. `blit-tui` works without any daemon on the network."

#### decision-extract-blit-app
**Source**: TUI_DESIGN.md §7.2
**Specificity**: high

"**Decision: option 2 — extract `blit-app`.** This is the right boundary, and it pays off again as soon as a third presenter (web/GUI) becomes interesting."

#### decision-rework-adopts-dual-pane
**Source**: TUI_REWORK.md §1, §12
**Specificity**: high

"The Phase 6 TUI rework adopts a **dual-pane file-manager model** as the primary interface." §12: "This spec adopts the dual-pane shell, visible actions, navigable fan-out destinations, and app/orchestrator-owned scheduling."

### decision (open questions / blockers)

#### open-q1-recent-persistence
**Source**: TUI_DESIGN.md §10 (1)
**Specificity**: medium

"Recent-transfer persistence. `GetState.recent[]` populated from an in-memory ring (cheap, lost on restart) or from `perf_local.jsonl` (durable, reuses existing storage)? **Blocker for: Milestone B.** Recommendation: in-memory ring for B; if persistence is wanted later, reuse `perf_local` in Milestone E."

#### open-q2-subscriber-cap
**Source**: TUI_DESIGN.md §10 (2)
**Specificity**: medium

"`Subscribe` subscriber cap. N concurrent subscribers, N=? Default 8 seems reasonable for a single-operator TUI. **Blocker for: Milestone C.**"

#### open-q3-progress-cadence
**Source**: TUI_DESIGN.md §10 (3)
**Specificity**: medium

"`TransferProgress` cadence. 10 Hz default, configurable via `SubscribeRequest`? Or fixed? Higher cadence is nice for the TUI but costs more in broadcast traffic AND in the byte-counter write-loop overhead. **Blocker for: Milestone C.**"

#### open-q4-multi-daemon-subscribe
**Source**: TUI_DESIGN.md §10 (4)
**Specificity**: medium

"Multi-daemon Subscribe. TUI watches N daemons simultaneously by opening N Subscribe streams; aggregation happens client-side. Is that ok or should the TUI talk to a designated 'primary' daemon that fans out? Recommend simple N-streams approach. **Blocker for: Milestone C.**"

#### open-q5-transfer-id-allocation
**Source**: TUI_DESIGN.md §10 (5)
**Specificity**: medium

"Transfer-id allocation. UUIDv4 daemon-side at the dispatch boundary (next to `metrics.inc_push()`), stable for the duration of the RPC, surfaced to the initiating client via the existing summary path. **Decision: take this default unless owner objects — daemon-internal, low-risk.**"

#### open-q6-remote-verify-scope
**Source**: TUI_DESIGN.md §10 (6)
**Specificity**: medium

"Remote tree verify (F4 Verify scope expansion). Either (a) the TUI streams two `Find`/`List` manifests, diffs them client-side, and renders the result; or (b) the daemon grows a `CompareTrees` RPC that does the diff server-side. (a) is cheaper but slower for large trees; (b) needs new wire. **Recommend: ship F4 Verify local-only in Milestone D**."

#### open-q7-job-durability
**Source**: TUI_DESIGN.md §10 (7)
**Specificity**: medium

"Job durability across daemon restart. Out of Phase 5 scope per §6.5. If the daemon restarts, in-flight detached jobs fail; the TUI shows them as errored; the user re-kicks and block-level resume picks up. Worth revisiting for 0.2.0 with an on-disk job-state design if operators report pain."

#### decision-not-locked-in
**Source**: TUI_DESIGN.md §12
**Specificity**: medium

"What this design intentionally does NOT lock in: Render library (ratatui is the recommendation; swap is local to `crates/blit-tui`). Layout details (illustrative, not binding; tuned during Milestone A). Key bindings (starting points; final bindings come out of usability testing). Theme (Milestone E)."

### behavior (milestone slicing — REWORK M1-M8)

#### milestone-rework-m1
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M1 — Dual-pane state model and render shell. Acceptance: Two panes render with focus, cursor, marks, path bars, and action bar as the default TUI shell."

#### milestone-rework-m2
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M2 — Local browse provider. Acceptance: Both panes can browse local filesystem from `$PWD`, Home, and root. Large-directory listing does not block the render loop."

#### milestone-rework-m3
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M3 — Remote browse provider and Places. Acceptance: Either pane can switch to discovered/configured remotes, list modules, descend into remote paths, and jump through recents/favorites."

#### milestone-rework-m4
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M4 — Visible action bar and copy to other pane. Acceptance: Copy works local->local, local->remote, remote->local, and remote->remote without typed path fields in the normal flow."

#### milestone-rework-m5
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M5 — Destructive actions with review. Acceptance: Mirror, move, and delete use visible review/confirm surfaces and preserve existing safety gates."

#### milestone-rework-m6
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M6 — Fan-out batch table. Acceptance: Multi-daemon copy/mirror uses one table with per-destination rows and navigable `[Change]` destinations."

#### milestone-rework-m7
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M7 — Transfer drawer and batch progress. Acceptance: Existing F2 data is reachable from the new shell; batch rows collapse/expand and show per-destination progress."

#### milestone-rework-m8
**Source**: TUI_REWORK.md §9
**Specificity**: high

"M8 — Full parity cleanup. Acceptance: Verify, profile, diagnostics, help, settings, aliases, and old trigger-modal removal are complete. Normal flows have no required free-text path prompts."

#### milestone-rework-suggested-order
**Source**: TUI_REWORK.md §9
**Specificity**: high

"Suggested order: M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7 -> M8."

### behavior (DESIGN milestone slicing A.0 / B / M-Jobs / C / A.1 / D / E)

#### milestone-design-a0
**Source**: TUI_DESIGN.md §8 Milestone A.0
**Specificity**: high

"Milestone A.0 — Extract `crates/blit-app` library. Refactor only. Move orchestration glue out of `blit-cli`'s binary modules so a sibling `blit-tui` crate can import it. No behavior change. ~2–3 days. CLI keeps working unchanged. Library now consumable."

#### milestone-design-b
**Source**: TUI_DESIGN.md §8 Milestone B
**Specificity**: high

"Milestone B — `GetState` RPC + `ActiveJobs` table + recent ring. Proto change adding `GetState`. Daemon-side: introduces the always-on `ActiveJobs` table populated from the dispatch boundary, plus the `recent` ring buffer. `GetState.counters` reads from existing `TransferMetrics`. **No** cancellation handles yet, **no** detach lifecycle — just the snapshot surface. CLI gets a new `blit jobs list <remote>` admin verb."

#### milestone-design-mjobs
**Source**: TUI_DESIGN.md §8 Milestone M-Jobs
**Specificity**: high

"Milestone M-Jobs — Daemon-owned transfer lifecycle. Extends the `ActiveJobs` table with the cancellation + detach lifecycle bits. Adds: `detach: bool` field on `DelegatedPullRequest` (delegated-only); `CancellationToken` field on each `ActiveJob` row; spawn-closure lifecycle change in `delegated_pull` only; `CancelJob(transfer_id)` RPC; CLI surface: `--detach` flag (remote→remote only), `blit jobs cancel`, `blit jobs watch` (GetState polling)."

#### milestone-design-c
**Source**: TUI_DESIGN.md §8 Milestone C
**Specificity**: high

"Milestone C — `Subscribe` RPC + byte-level instrumentation. Proto change adding `Subscribe` + `DaemonEvent` family, daemon-side `tokio::broadcast`, the four byte-level progress instrumentation pieces (§6.2 steps 1–4). After C the daemon emits live byte-level progress that any subscriber renders. `blit jobs watch` upgrades from polling to streaming."

#### milestone-design-a1
**Source**: TUI_DESIGN.md §8 Milestone A.1
**Specificity**: high

"Milestone A.1 — The TUI itself. Now the TUI screens land. All foundation is in place: F1 Daemons (mDNS list, per-daemon detail pane lit up by `GetState`, 'local' sentinel endpoint); F2 Transfers (active pane fed by `Subscribe`, history fed by `GetState.recent`. Cancel hotkey fires `CancelJob`); F3 Browse (`List`/`Find`/`DiskUsage`/`FilesystemStats`. Multi-select + c/m/v/D modal actions dispatch through `blit_app` with `detach=true`); F4 Profile (reads `~/.config/blit/perf_local.jsonl` directly). Result: real single-pane-of-glass from day one."

#### milestone-design-d
**Source**: TUI_DESIGN.md §8 Milestone D
**Specificity**: high

"Milestone D — Verify + diagnostics screens. F4 Verify (wraps `blit check`, local-only — see §10 Q7), diagnostics-dump action. TUI-side glue."

#### milestone-design-e
**Source**: TUI_DESIGN.md §8 Milestone E
**Specificity**: high

"Milestone E — Polish. Theme support, dark/light, configurable refresh rates, key remapping, JSON config for default endpoints, optional Prometheus bridge as a separate binary scraping `GetState`."

### invariant (testing contract)

#### test-required-workflows
**Source**: TUI_REWORK.md §10
**Specificity**: high

"Required workflow tests: Remote file -> local directory copy. Local directory -> local directory mirror. Local directory -> two remote destinations fan-out. Remote A -> remote B delegated copy. Move/delete review cannot be bypassed by a single accidental key. Editable path bar and navigated rows produce the same `Location`. Old F-key/letter aliases, while present, route to the same actions as visible controls."

#### test-fake-providers
**Source**: TUI_REWORK.md §10
**Specificity**: high

"Use fake browse providers for deterministic UI state tests. Assert the resulting `TransferDraft` / `BatchTransferDraft`, not only rendered text."

#### test-model-boundary-first
**Source**: TUI_REWORK.md §10
**Specificity**: medium

"Add tests at the model boundary first, then at the key-dispatch level."

### behavior (purpose / scope statements)

#### purpose-tui-mirror-cli-plus
**Source**: TUI_DESIGN.md §1 Purpose
**Specificity**: high

"A single terminal-UI binary that lets an operator do interactively everything `blit` currently does at the command line, plus: See every blit-daemon reachable on the LAN, with module list, free/used capacity, version, and delegation state at a glance. Browse the file tree of any reachable daemon (or local path) with `ls` / `find` / `du` / `df` data inline. Initiate copy / mirror / move / rm between any two endpoints (local↔local, local↔daemon, daemon↔daemon) without re-typing paths. Watch in-flight transfers with live throughput, ETA, and per-file progress; see recent transfer history and per-daemon counters."

#### purpose-cli-verb-tui-parity
**Source**: TUI_DESIGN.md §2 (intro)
**Specificity**: high

"Every existing `blit` verb gets a TUI home. This is the parity contract: a user who knows the CLI must be able to do the same thing in the TUI without dropping back to a shell."

#### scope-no-tui-rpcs-future-clients
**Source**: TUI_DESIGN.md §3 (closing)
**Specificity**: high

"Together these enable the single-pane-of-glass model: any TUI on the LAN can list, watch, cancel, or initiate transfers on any reachable daemon, and transfers survive their initiator disconnecting."

## Contradictions

### contradiction-screen-model-f1-f4-vs-dual-pane
**Sources**: TUI_DESIGN.md §5 (four-screen F1-F4 architecture as structural commitment in §12) vs TUI_REWORK.md §1, §4 (dual-pane model with action bar; F-keys aliases-only during migration).

TUI_DESIGN.md §12 names "Four-screen architecture (F1 / F2 / F3 / F4)" as a structural commitment ("the structural commitments are: Four-screen architecture..."). TUI_REWORK.md §1, §4, §11 says the dual-pane two-browser model is the default `blit-tui` shell, F-keys "may remain as aliases during migration, but it is no longer the product model", and the screen list (§5 of DESIGN) is superseded. TUI_DESIGN.md acknowledges this in its 2026-05-31 rework note, but the §12 structural commitment text was not edited to reflect this.

### contradiction-a0-effort-estimate
**Sources**: TUI_DESIGN.md §7.3 vs §7.5.

§7.3: "Adding the nine admin/browser/profile modules to A.0 grows the refactor from '~2–3 days' to **~4–5 days**." §7.5: "Rough estimate: 2–3 days of focused work." §11 phasing table also says "~4–5 days of mechanical moves". §7.5 appears not to have been updated when the scope grew.

### contradiction-detach-field-location-table-vs-text
**Sources**: TUI_DESIGN.md §3 ("What lives where" table) vs §6.5.

§3 row: "Job lifecycle (cancel + daemon-owned transfers) — gRPC `CancelJob` + `detach` field on transfer specs — **new** — see §6.5". The phrase "on transfer specs" implies `TransferOperationSpec`, but §6.5 explicitly rejects that location: "Field lives on the delegated-pull request itself, **not** on `TransferOperationSpec`." The §3 summary text was not updated to match the §6.5 decision.

### contradiction-rework-no-wire-change-vs-design-wire-changes
**Sources**: TUI_REWORK.md §11 vs TUI_DESIGN.md §6.

TUI_REWORK.md §11: "No daemon wire change is required for the shell itself." TUI_DESIGN.md §6 plans three new RPCs (`GetState`, `Subscribe`, `CancelJob`) and several new message fields. Not strictly contradictory (REWORK speaks to the shell rework only, DESIGN to overall Phase 5), but a reader looking only at REWORK might miss that Phase 5 wire work continues in parallel.

### contradiction-detach-field-name-design
**Sources**: TUI_DESIGN.md §3 ("detach field on transfer specs") vs §11 phasing table ("+`detach` on `DelegatedPullRequest`").

The phasing table in §11 correctly says "+`detach` on `DelegatedPullRequest`"; the §3 "What lives where" row's column "Mechanism" says "`CancelJob` + `detach` field on transfer specs". Minor naming inconsistency.

### contradiction-tui-modal-system-vs-no-modals
**Sources**: TUI_DESIGN.md §7.3 (what blit-tui adds) vs TUI_REWORK.md §1, §3.

DESIGN §7.3: "a small input-modal system for collecting source / destination / option choices". REWORK §1: "Normal transfer flow must not depend on modal path pickers." REWORK §3 (5): only confirmation/focused editing/help/batch review use dialogs. The DESIGN sentence is from the pre-rework era; REWORK supersedes the modal model.

## Coverage attestation

| File | Lines | Notes |
|---|---|---|
| docs/plan/TUI_DESIGN.md | 1118 | Read end-to-end in one pass; covers Purpose (§1), CLI parity map (§2), what-lives-where table (§3), 5 design principles (§4), four screens with sketches (§5.1-5.4), wire surface §6 with full proto specs for Subscribe/GetState/CancelJob (§6.1-6.5), crate/dependency shape §7.1-7.5 including blit-app rationale and module-mapping table, milestones A.0/B/M-Jobs/C/A.1/D/E §8, non-goals §9, open questions §10 (Q1-Q7), phasing summary table §11, structural commitments §12. |
| docs/plan/TUI_REWORK.md | 411 | Read end-to-end; covers Decision §1, Problem §2, 8 Product Principles §3, Main Shell sketch §4 (Pane Behavior, Places/Favorites/Recents, Path Entry), 4 Core Workflows W1-W4 §5, Actions §6 (baseline table + accelerators), Fan-Out Batch Table §7 with rules, Architecture §8 (8.1 UI Model Types with structs/enums, 8.2 Browse Providers, 8.3 Transfer Execution, 8.4 Existing F2/F4 Surfaces, 8.5 Small-Terminal Fallback), Implementation Milestones M1-M8 §9, Testing Contract §10, Compatibility and Non-Goals §11, Review History §12. |

**Total lines**: 1529
