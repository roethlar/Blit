# Drift Findings: TUI design + rework
**Generated**: 2026-06-04
**Claims audited**: ~90 (across principle/invariant/interface/behavior/scope/non-goal/deferred/rejected/shipped/decision/milestone)
**Findings**: 12 (H: 4 / M: 5 / L: 3)

## High severity

### dual-pane-actions-are-display-only — Dual-pane action bar is render-only; no transfer triggers
**Plan says**: "Visible actions first. Copy, mirror, move, delete, verify, and batch operations appear in an action bar or focused menu. Letter keys are optional accelerators, not the product model." (TUI_REWORK §3 Product Principles 2). Also TUI_REWORK §1 Decision: "Transfers are launched from visible action buttons, not from hidden memorized letter commands." And §9 M4 Acceptance: "Copy works local->local, local->remote, remote->local, and remote->remote without typed path fields in the normal flow."
**Code does**: `DualPaneState::action_labels()` (`crates/blit-tui/src/dual_pane.rs:462-472`) returns `["Copy -> {dest}", "Mirror -> {dest}", "Move -> {dest}", "Delete", "Verify", "More"]` as a `Vec<String>`. The dual-pane router (`crates/blit-tui/src/main.rs:2219-2234`) ONLY handles `Refresh`, `SelectNext/Prev/First/Last`, `Descend`, `Ascend`, `DualSwitchPane`, `F3ToggleMark` — no `TransferCopy`, `TransferMirror`, `TransferMove`, `F3DeleteBegin`, or verify dispatch. Action labels are rendered (`screens/dual_pane.rs:183-206`) but there is no input handler that consumes them. The dual-pane test (`screens/dual_pane.rs:246-272`) only asserts that the strings render, not that anything happens when invoked.
**Evidence**:
- `crates/blit-tui/src/dual_pane.rs:462-472` — `action_labels()` definition.
- `crates/blit-tui/src/screens/dual_pane.rs:183-206` — `render_actions` paints the labels.
- `crates/blit-tui/src/main.rs:2219-2234` — Dual screen dispatch is navigation-only.
- `crates/blit-tui/src/main.rs:2123-2171` — `TransferCopy/Mirror/Move` only dispatched in `Screen::F4` (verify) arm, operating on `app.verify.source/destination` text fields, NOT the active pane's selection.
**Notes**: This is the central failure of the rework's "Pick not Type" principle. The default screen is Dual (per `--screen` default value), but the only screen that can launch a transfer (F4) uses text fields. Real transfer launching survives only via F1's `t` trigger modal (`f1trigger.rs`), F3's `p`/`m`/`v` modals (`f3pull.rs`), and F4's verify-form path inputs — all letter-command + free-text-modal flows that the rework explicitly rejects. Remediation: wire `UserAction::TransferCopy/Mirror/Move` (and Delete, Verify) into the Dual dispatch arm using the active pane's selection + inactive pane's path as source/dest; add a draft/confirm surface; add corresponding tests.

### transferdraft-types-missing — Plan-mandated UI model types do not exist
**Plan says**: TUI_REWORK §8.1 "UI Model Types": `enum Location`, `struct PaneState { ... path_editor: PathEditorState, ... }`, `struct BrowserEntry`, `struct TransferDraft { action: TransferAction, sources: Vec<Location>, destinations: Vec<Location>, options: TransferOptions, }`, `struct BatchTransferDraft { action: TransferAction, sources: Vec<Location>, destinations: Vec<BatchDestination>, options: TransferOptions, }`. §10 testing contract: "Assert the resulting `TransferDraft` / `BatchTransferDraft`, not only rendered text."
**Code does**: Workspace-wide search for `TransferDraft`, `BatchTransferDraft`, `TransferAction`, `TransferOptions`, `BatchDestination` returns zero matches in `crates/**` (only the plan doc itself contains the names). The `BrowseProvider` trait, `LocalBrowseProvider`, and `RemoteBrowseProvider` (§8.2) likewise do not exist as types. The shipped `PaneState` (`dual_pane.rs:172-183`) has `path_editor: String`, not a `PathEditorState`; no sort/display-preferences field; no source/dest selection draft.
**Evidence**:
- `crates/blit-tui/src/dual_pane.rs:172-183` — `PaneState` struct.
- `grep -rn "TransferDraft\|BatchTransferDraft\|BrowseProvider\|TransferAction" /Users/michael/Dev/Blit/crates/` returns no hits.
- `crates/blit-tui/src/dual_pane.rs:319-334` — `apply_fetch_result` shape; no provider trait abstraction.
**Notes**: Without `TransferDraft`/`BatchTransferDraft` the testing-contract assertions in §10 ("Assert the resulting `TransferDraft`") are structurally impossible. Without the `BrowseProvider` trait, the "fake browse providers for deterministic UI state tests" (§10 `test-fake-providers`) cannot be wired. The plan model boundary that tests are supposed to target does not exist as code. Remediation: introduce the types per §8.1/§8.2 before continuing the rework; tests can then assert at the model boundary.

### tui-delegated-detach-false — TUI delegated transfers ship with detach=false, contradicting design decision
**Plan says**: TUI_DESIGN §5.2 — "The TUI uses `detach=true` on every transfer it initiates against a remote→remote pair". §6.5 — "Transfers kicked off from the TUI use `detach=true` on the `DelegatedPullRequest` for remote→remote transfers only." §10 (decision) — "Daemon-owned transfer lifecycle for remote→remote (delegated) transfers when `detach=true`". §12 structural commitment: "Daemon-owned transfer lifecycle for remote→remote (delegated) transfers when `detach=true`".
**Code does**: `crates/blit-tui/src/exec_plan.rs:91-108` `build_delegated_execution` hardcodes `detach: false` with the comment "Always attached (`detach: false`); detached/F2-visible delegation is a follow-up."
**Evidence**:
- `crates/blit-tui/src/exec_plan.rs:106` — `detach: false,`
- `crates/blit-tui/src/exec_plan.rs:89-90` — Comment admits it's a deferred follow-up.
- `proto/blit.proto:828` and downstream — Wire field exists; daemon honors it (`service/core.rs:1314-1320` `if !detach`).
**Notes**: The wire surface, daemon-side detach lifecycle (M-Jobs), CLI's `--detach` flag, `jobs watch`, and `CancelJob` all shipped. The single place that should set `detach=true` — the TUI's delegated execution builder — instead sets it false. This breaks the "single-pane-of-glass survives initiator disconnect" promise (TUI_DESIGN §3 closing: "any TUI on the LAN can list, watch, cancel, or initiate transfers on any reachable daemon, and transfers survive their initiator disconnecting"). Remediation: flip to `detach: true` for remote→remote and surface a banner on the trigger modal for local-endpoint transfers per §5.2.

### no-tui-rework-workflow-tests — Required workflow tests absent
**Plan says**: TUI_REWORK §10 "Testing Contract — Required workflow tests: Remote file -> local directory copy. Local directory -> local directory mirror. Local directory -> two remote destinations fan-out. Remote A -> remote B delegated copy. Move/delete review cannot be bypassed by a single accidental key. Editable path bar and navigated rows produce the same `Location`. Old F-key/letter aliases, while present, route to the same actions as visible controls."
**Code does**: Search for the four workflow scenarios (W1-W4) and the "navigated rows produce the same Location" or "aliases route to the same actions as visible controls" tests yields nothing in `crates/blit-tui/src/`. The existing tests (`dual_pane.rs:749-953`, `screens/dual_pane.rs:229-273`) cover navigation, marking, fetch generation, and that action labels exist and flip direction — but no end-to-end test asserts that a transfer is *launched* (with `TransferDraft` or any other surface) from the dual-pane shell.
**Evidence**:
- `crates/blit-tui/src/dual_pane.rs:749-953` — 8 tests; all are state-model unit tests; none drive a key-press → transfer flow.
- `crates/blit-tui/src/screens/dual_pane.rs:229-273` — 2 render tests; assert label strings but not behavior.
- The workflow names W1-W4 (TUI_REWORK §5) don't appear in any test name or comment in the crate.
**Notes**: Without these tests, the rework principles are unverifiable by CI. The Move/delete-review-cannot-be-bypassed assertion is especially important — d-65/R47-F4 / `feedback-port-cli-safety-guards` already shows that bypassing those gates causes data loss. Remediation: add a `tests/` directory or `mod tests` with W1-W4 scenarios using fake providers (which also requires the `BrowseProvider` trait per drift `transferdraft-types-missing`).

## Medium severity

### invariant-foundation-first-violated — Foundation-first milestone order partially violated by parallel work
**Plan says**: TUI_DESIGN §12 structural commitment: "Foundation-first milestone order: A.0 → B → M-Jobs → C → A.1 → D → E." §10: "Foundation-first milestone order. A.0 → B → M-Jobs → C → A.1 → D → E. TUI ships as a real network resource from its first release."
**Code does**: blit-app exists (A.0 ✓), `GetState`/`ActiveJobs`/recent ring exist (B ✓), `CancelJob` + `detach` field exist (M-Jobs ✓), `Subscribe` exists (C ✓). But A.1 ("the TUI itself" — TUI_DESIGN §8 — listing screens F1 Daemons / F2 Transfers / F3 Browse / F4 Profile) shipped while the Dual screen (TUI_REWORK §1 rework, scheduled M1-M8) became the default *without* the rework's transfer-execution layer wired (see `dual-pane-actions-are-display-only`). The default screen is `ScreenArg::Dual` (`main.rs:105`), but the productive transfer paths remain on F1/F3/F4 (the pre-rework model).
**Evidence**:
- `crates/blit-tui/src/main.rs:105` — `default_value_t = ScreenArg::Dual`.
- `crates/blit-tui/src/main.rs:163-307` — `AppState` carries F1/F2/F3/F4 substate + dual_pane substate side-by-side.
- TUI_REWORK §9 milestone list M1-M8; only M1-M3 shipped per the memory `phase6_tui_dual_pane_m1_m2_2026_05_31.md` + `phase6_tui_dual_pane_m3_*` and the TUI_REWORK header "dual-pane is the default `blit-tui` shell as of M1-M3 work on 2026-05-31."
**Notes**: The default-screen flip happened before the rework's M4+ landed, so the user-facing default surfaces are the partially-implemented shell. Not strictly a wire/foundation violation, but the "TUI ships as a real network resource from its first release" promise is at risk: a fresh-install user sees the action bar but it doesn't transfer anything. Remediation: either gate `--screen dual` behind a `default_value_t = ScreenArg::F1` until M4 lands, or finish M4 (the action-bar wiring) before default-flipping.

### subscribe-event-mask-not-implemented — SubscribeRequest.event_mask parsed but ignored
**Plan says**: TUI_DESIGN §6.2 "SubscribeRequest has `event_mask` (uint32, bitfield with TRANSFERS=1, ERRORS=2, MODULES=4, HEARTBEAT=8; 0 = all)".
**Code does**: `proto/blit.proto:863` defines `uint32 event_mask = 1;` with the comment "field is parsed and ignored. Locking the tag here keeps the wire shape stable." The daemon-side Subscribe handler (`service/core.rs:353-455`) doesn't filter on `event_mask`.
**Evidence**:
- `proto/blit.proto:857-863` — Wire field with explanatory comment that today's emitters only fire transfer-family events, so the mask is unused.
- Plan iface-subscribe-rpc specifies semantics for the mask.
**Notes**: Documented as deferred-but-tag-reserved, which is reasonable; flagging as drift because the plan does not call this out as deferred. ModuleListChanged / DaemonHeartbeat event variants are reserved at the wire level (proto §901-904) but not emitted. Future Subscribe consumers wanting to filter on category will not get the documented behavior. Remediation: either implement the filter when ModuleListChanged/Heartbeat ship, or mark explicitly deferred in TUI_DESIGN §6.2.

### tui-clear-recent-result-discarded — Clear-recent fan-out swallows per-daemon errors
**Plan says**: TUI_REWORK §11: "No daemon wire change is required for the shell itself." Implicit from `principle-cli-parity-floor`: TUI surfaces must not silently hide failures the CLI would show. TUI_DESIGN §6.3 / shipped behavior: `ClearRecent` returns `cleared: u32`.
**Code does**: `crates/blit-tui/src/main.rs:3926-3930` `spawn_clear_recent` runs `_ = blit_app::admin::jobs::clear_recent(&endpoint).await;` — result discarded. Mass fan-out with all daemons failing produces no operator-visible signal beyond the local clear. The code inventory flags this as "intentional per responsiveness" but the plan does not authorize hiding failures.
**Evidence**:
- `crates/blit-tui/src/main.rs:3927` — `_ = blit_app::admin::jobs::clear_recent(&endpoint).await;`
- Code-inventory smell: `code-tui-main.md:188` flags this.
**Notes**: Memory `feedback-port-cli-safety-guards` requires CLI-grade safety porting; here the CLI's `blit jobs clear-recent` would surface errors. Remediation: collect per-daemon Result and at least banner the count of failed daemons.

### tui-recent-persistence-shipped-not-in-plan — Recent persistence shipped despite plan deferral
**Plan says**: TUI_DESIGN §6.3 / §10 (1): "Persistence (durability across daemon restart) is deferred to 0.2.0+." Open Q1 recommendation: "in-memory ring for B; if persistence is wanted later, reuse `perf_local` in Milestone E." Phasing table §11: "recent ring (+`GetState`, ~500 daemon + ~100 CLI)" with no persistence.
**Code does**: Recent persistence to `recents.jsonl` (separate from `perf_local.jsonl` per `project_recent_persistence` memory) is shipped: `crates/blit-daemon/src/recents_store.rs:29` defines `RECENTS_FILE = "recents.jsonl"`; `crates/blit-daemon/src/active_jobs.rs:777-844` plumb `arm_persistence`/`RecentsWriter` for daemon-lifetime persistence; `recents_store::write_atomic` writes `.jsonl.tmp` + rename. Shipped per audit memory.
**Evidence**:
- `crates/blit-daemon/src/recents_store.rs:1-177` — full module.
- `crates/blit-daemon/src/active_jobs.rs:777-844` — armed in daemon lifecycle.
- TUI_DESIGN §10 (1) and §11 explicitly defer this to 0.2.0+ / Milestone E.
**Notes**: Plan was conservative; the implementation went further. The discrepancy is mostly a planning/doc out-of-date issue rather than a code defect, but readers consulting TUI_DESIGN to understand current daemon behavior will get the wrong picture. Remediation: update TUI_DESIGN §6.3 / §10 / §11 / §12 to reflect that recents.jsonl persistence shipped earlier than planned.

### tui-no-app-progress-event-channel — AppProgressEvent shape not exposed as a single channel
**Plan says**: TUI_DESIGN §7.4 / §10 (decision): "`blit-app` exposes a single event channel pattern. Every orchestration entry point takes an `Option<mpsc::UnboundedSender<AppProgressEvent>>` (channel-based, mirroring `RemoteTransferProgress`'s existing shape)."
**Code does**: blit-app exposes per-call progress senders (e.g. `RemoteTransferProgress` flows through the remote pull/push paths), and the TUI feeds them via dedicated channels per pane (`f1_push_progress_tx`, `transfers_event_tx`, etc.) — but there is no unified `AppProgressEvent` type/channel pattern wrapping all entry points. The TUI's `accumulate_pull_progress` / `accumulate_push_progress` / `accumulate_delegated_progress` (`progress_accum.rs`) each interpret slightly different event sources (Payload-only vs FileComplete-only) — three accumulators for three subtly different shapes.
**Evidence**:
- `crates/blit-tui/src/progress_accum.rs:21-88` — three different accumulators.
- `crates/blit-tui/src/main.rs:200, 3361-3433, 3461-3538` — separate progress senders per launch site.
- No `AppProgressEvent` enum/struct in blit-app or anywhere in the workspace.
**Notes**: The pattern works (each entrypoint has its own channel) but the unified `AppProgressEvent` abstraction the plan calls for never materialized; consumers (TUI, future GUI) duplicate the three-accumulator pattern. Remediation: either land the unified enum (matches §7.4 / §10) or update §7.4 to document the per-call shape that actually exists.

## Low severity

### contradiction-screen-model-not-resolved-in-design — TUI_DESIGN §12 still names four-screen architecture as structural commitment
**Plan says**: TUI_DESIGN §12: "Four-screen architecture (F1 / F2 / F3 / F4)." Plan inventory flags this as `contradiction-screen-model-f1-f4-vs-dual-pane`. TUI_REWORK §1 supersedes the screen list but the §12 commitment text wasn't edited to reflect this.
**Code does**: The code carries both models simultaneously — `Screen::{F1, F2, F3, F4, Dual}` (`main.rs:125-132`), and Dual is the default (`main.rs:105`). The four-screen model is no longer a structural commitment in practice — Dual is.
**Evidence**:
- `crates/blit-tui/src/main.rs:125-132` — five `Screen` variants.
- TUI_DESIGN §12 unedited.
**Notes**: Doc drift only — the rework note at the top of TUI_DESIGN.md acknowledges supersession but §12 should be updated to either delist "Four-screen architecture" or annotate it as superseded. Remediation: edit §12.

### a0-effort-estimate-inconsistent — TUI_DESIGN §7.5 estimate not updated to match §7.3 / §11
**Plan says**: §7.3 — "Adding the nine admin/browser/profile modules to A.0 grows the refactor from '~2–3 days' to **~4–5 days**." §7.5 — "Rough estimate: 2–3 days of focused work." §11 phasing — "~4–5 days of mechanical moves". Internal inconsistency.
**Code does**: blit-app exists with the full admin verb surface (`crates/blit-app/src/admin/{jobs,ls,list_modules,rm,du,df,find}.rs`) — A.0 shipped per its broader §7.3 / §11 scope.
**Evidence**:
- `crates/blit-app/src/admin/` directory listing.
- TUI_DESIGN §7.3, §7.5, §11 cited above.
**Notes**: Pure doc drift; §7.5 was apparently not updated when §7.3 expanded the scope. Remediation: edit §7.5.

### tui-sort-display-prefs-missing — Pane state lacks "sort and display preferences"
**Plan says**: TUI_REWORK §4 Pane Behavior — "Each pane owns: A `Location`: ... An editable path bar. A row list. A cursor. A marked set. Filter/search state. **Sort and display preferences**."
**Code does**: `PaneState` (`dual_pane.rs:172-183`) has location, entries, cursor, marked, path_editor, filter, status, pending_request_id — but no sort field, no display-preference field. Sorting is hardcoded in `sort_browser_entries` (`dual_pane.rs:605-622`) by (kind priority, lowercase name, name).
**Evidence**:
- `crates/blit-tui/src/dual_pane.rs:172-183` — PaneState fields.
- `crates/blit-tui/src/dual_pane.rs:605-622` — hardcoded sort.
**Notes**: Minor partial — sort works correctly but isn't operator-tunable per pane as the plan implies. Remediation: add sort/display-prefs to PaneState when M4+ wires the action bar.

## Claims that align well

- **Crate split**: `blit-app` library crate + `blit-tui` binary crate exist (TUI_DESIGN §7.2, §12 — `crates/blit-app/Cargo.toml`, `crates/blit-tui/Cargo.toml`). `blit-tui` consumes `blit-app`; CLI keeps working.
- **mDNS discovery + TXT keys**: `_blit._tcp.local.` advertised with `version` / `modules` / `module_count` / `delegation_enabled` per shipped-mdns-* claims (`crates/blit-core/src/mdns.rs:43-152`).
- **GetState/Subscribe/CancelJob/ClearRecent RPCs**: All four shipped on the wire per spec (`proto/blit.proto:57, 76, 89, 107`); daemon implementations land in `service/core.rs:353-1146`.
- **detach field placement**: Field lives on `DelegatedPullRequest` only (proto), not on `TransferOperationSpec` — matches the §6.5 (1) decision and the `rejected-detach-on-transfer-operation-spec` reasoning.
- **Daemon-side cancel race**: `resolve_delegated_pull_outcome` handler-first biased select per `behavior-spawn-closure-disarm-on-detach` (`service/core.rs:741-783`).
- **CLI parity floor for new RPCs**: `blit jobs list / cancel / watch` + `blit copy --detach` (delegated-only) exist with documented exit codes (`crates/blit-cli/src/jobs.rs`, `transfers/mod.rs:161-178`).
- **Counters always-present-but-zero**: Documented in proto (`proto/blit.proto:752-756`) and matches `feedback_getstate_counters_zero` memory; bridge works around it (`metrics.rs:38-96`).
- **Dual-pane state model basics (M1)**: `DualPaneState` + `PaneId::{Left,Right}` + active/inactive + action-label direction flip + Tab switch all present and unit-tested (`crates/blit-tui/src/dual_pane.rs:401-484, 749-953`).
- **Local browse provider (M2)**: `list_local_entries` reads metadata, builds entries with kind/size/mtime/read_only, sorts; large directories use `spawn_blocking` (`main.rs:2959-3011`, `dual_pane.rs:486-517`).
- **Remote browse provider (M3, partial)**: modules + ls + descend/ascend implemented for the dual-pane shell (`dual_pane.rs:519-560, 651-706`), backed by existing daemon RPCs as the plan required (no new wire).
- **Local-mode first class**: `daemons.rs:330-339` synthesizes a Local row pinned at index 0; F1 / F2 / F3 treat Local symmetrically with remote daemons (matches `invariant-local-mode-first-class`).
- **F4 Verify local-only**: `compare_trees` from `blit_app::check` (`main.rs:4816-4821`); remote-verify not implemented (correctly per `deferred-remote-verify`).
- **No shadow transfer code**: TUI launches go through `blit_app::transfers::{local::run, remote::run_remote_push, remote_remote_direct::run_delegated_pull, ...}` — same code path the CLI uses (`main.rs:3238-3538`, exec_plan.rs).
- **Reload-on-Ctrl+R + collision-disabling keymap**: Hot-reload via `reload_tui_config` + `classify_reload` (`config_reload.rs:19-57`); collision policy in `config.rs:187-237` with explicit precedence and per-collision warnings.
