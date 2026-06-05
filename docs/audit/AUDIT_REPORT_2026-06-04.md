# Blit Codebase + Plan Audit — 2026-06-04

## TL;DR

Blit ships a coherent, well-tested 0.1.0 surface (hybrid TCP + gRPC transport, unified receive pipeline, always-on F2 containment, delegated remote→remote with explicit-only fallback, robust per-RPC daemon RPCs), but the plan corpus has not kept up with what shipped. The single largest gap is **TUI**: the rework's "Pick-not-Type" dual-pane shell is the default screen yet has no transfer-execution wiring (action bar is render-only), and the design's `detach=true` for remote→remote TUI delegations is hardcoded `false` — both undermine the "single pane of glass survives initiator disconnect" promise. Lower-altitude but pervasive drift: the planner's documented "1 s heartbeat / 10 s stall detector" architecture was never built (we ship a synchronous orchestrator with a 30 s data-plane idle guard on pull only), `--detach` shipped despite being "out of scope" in REMOTE_REMOTE_DELEGATION_PLAN, the manpage omits eight transfer flags including data-loss-class ones (`--null`, `--detach`, `--delete-scope`, `--force`, `--ignore-existing`) and two whole verbs (`jobs`, `check`), and the architecture/whitepaper still narrate auth/TLS planning that was explicitly removed from scope. Cross-cutting inconsistencies cluster around five seams — path rendering (3 ad-hoc `Path→wire` helpers with different empty-path encodings), error wrapping (admin verbs strip Status codes while jobs verbs preserve them, breaking `is_retryable`), endpoint parse Err handling (TUI F1 confirm-detail silently classifies as Local, violating an explicit project-memory rule reopened 4×), timeout/keepalive symmetry (stall guard only on pull-receive, TCP keepalive only on push-receive accept, three independent `connect_with_timeout` implementations), and destructive-action prompts (CLI `perf --clear` fires silently while TUI confirms; no `--yes` parity for `jobs clear-recent`/`jobs cancel`). What's solid: the data-plane wire format, F2 containment chokepoint, delegation gate with DNS-rebinding mitigation, spec-version fail-closed, `pull_sync_with_spec` endpoint-isolation, BlitAuth removal, and the CLI's documented data-loss reject-gates on `move` are all faithfully implemented and tested.

## Method

**Plan inventories** (6 files, 9,140 lines read):
- `plan-principles.md` — 1,367 lines (greenfield_plan_v6 442 + MASTER_WORKFLOW 120 + RELEASE_PLAN_v2 805)
- `plan-phases.md` — 1,929 lines (POST_REVIEW_FIXES 308 + REMOTE_REMOTE_DELEGATION 1084 + WORKFLOW_PHASE_{2,3,4} 357 + PROJECT_STATE_ASSESSMENT 180)
- `plan-tui.md` — 1,529 lines (TUI_DESIGN 1118 + TUI_REWORK 411)
- `plan-wire.md` — 2,866 lines (ARCHITECTURE 502 + WHITEPAPER 750 + DAEMON_CONFIG 620 + blit.proto 994)
- `plan-perf.md` — 944 lines (LOCAL_TRANSFER_HEURISTICS 189 + PIPELINE_UNIFICATION 260 + BENCHMARK_10GBE_PLAN 133 + BENCH_VERB_PLAN 362)
- `plan-cli.md` — 505 lines (blit.1.md 202 + README.md 153 + CHANGELOG.md 78 + BLIT_UTILS_PLAN.md 72)

**Code inventories** (11 files, ~83,089 lines read):
- `code-cli.md` — 4,920 lines across 20 files (cli.rs, main.rs, transfers/*, jobs.rs, all admin verbs)
- `code-daemon.md` — 12,489 lines across 17 files (service/core, push, pull, pull_sync, delegated_pull, admin, delegation_gate, active_jobs, runtime)
- `code-bridge-proto.md` — 1,901 lines (blit-prometheus-bridge full + proto/blit.proto + blit-core/build.rs)
- `code-core-transfer.md` — 10,619 lines (remote/{transfer,push,pull,endpoint}/*)
- `code-core-io.md` — 3,533 lines (copy/*, delete.rs, buffer.rs, tar_stream.rs, checksum.rs, manifest.rs)
- `code-core-orch.md` — 4,566 lines (orchestrator/*, mirror_planner.rs, local_worker.rs, enumeration.rs, fs_enum.rs)
- `code-core-misc.md` — 5,884 lines (auto_tune, fs_capability/*, change_journal/*, mdns, perf_*, path_safety, path_posix, lib/config/errors/logger)
- `code-tui-main.md` — 10,838 lines (blit-tui/src/main.rs, full read)
- `code-tui-state.md` — 10,876 lines across 14 files (f1trigger, f1push, f3pull, f3del, f3du, browse, daemons, transfer, verify, state, config, profile, help, diagnostics)
- `code-tui-display.md` — 5,380 lines (display_f1/f2/f3, exec_plan, tick_budget, del_request, config_reload, theme_color, progress_accum, screens/*)
- `code-tests-scripts.md` — 12,083 lines (all integration tests + scripts + CI workflow)

**Total source lines audited**: ~92,229 (83,089 code + 9,140 plan)

**Phases run**: 4 (Inventory → Drift → Inconsistency → Synthesis)

**Workflow ID**: blit-full-audit-2026-06-04

## High-severity findings

### 1. TUI dual-pane action bar is render-only — no transfer execution wired

**Class**: drift
**Where**: `crates/blit-tui/src/dual_pane.rs:462-472` (action_labels) · `crates/blit-tui/src/main.rs:2219-2234` (Dual screen dispatch) · `crates/blit-tui/src/screens/dual_pane.rs:183-206` (render_actions)
**Plan says / Canonical**: TUI_REWORK §1 Decision and §3 Product Principle (2): "Transfers are launched from visible action buttons, not from hidden memorized letter commands." §9 M4 acceptance: "Copy works local→local, local→remote, remote→local, and remote→remote without typed path fields in the normal flow." Dual is the *default* screen (`main.rs:105` `default_value_t = ScreenArg::Dual`).
**Code does**: `action_labels()` returns `["Copy -> {dest}", "Mirror -> {dest}", "Move -> {dest}", "Delete", "Verify", "More"]`. The Dual screen dispatch arm handles only `Refresh`, `Select{Next,Prev,First,Last}`, `Descend`, `Ascend`, `DualSwitchPane`, `F3ToggleMark` — none of `TransferCopy/Mirror/Move`, `F3DeleteBegin`, or verify. The render test (`screens/dual_pane.rs:246-272`) asserts the strings render; no test asserts a transfer is launched from the dual pane.
**Why this matters**: A fresh-install operator opens Blit, sees an action bar, presses Copy, and nothing happens. Productive transfer paths survive only on F1's `t` trigger modal, F3's `p`/`m`/`v` modals, and F4's verify-form text inputs — exactly the letter-command + free-text-modal model the rework rejected. The default-screen flip happened before the rework's M4+ wiring landed.
**Suggested remediation pointer**: Wire `UserAction::TransferCopy/Mirror/Move/Delete/Verify` into the Dual dispatch arm using active-pane selection + inactive-pane path, build a `TransferDraft`, route through the existing destructive review surface, add W1-W4 workflow tests with fake providers (requires also landing `BrowseProvider` per finding #2).

### 2. TUI plan-mandated UI model types do not exist (`TransferDraft`, `BatchTransferDraft`, `BrowseProvider`)

**Class**: drift
**Where**: `crates/blit-tui/src/dual_pane.rs:172-183` (current PaneState shape) · workspace-wide absence of `TransferDraft`/`BatchTransferDraft`/`TransferAction`/`TransferOptions`/`BatchDestination`/`BrowseProvider`
**Plan says / Canonical**: TUI_REWORK §8.1 specifies `enum Location`, `struct PaneState { ... path_editor: PathEditorState, ... }`, `struct BrowserEntry`, `struct TransferDraft { action, sources, destinations, options }`, `struct BatchTransferDraft`. §8.2 specifies `trait BrowseProvider`, `LocalBrowseProvider`, `RemoteBrowseProvider`. §10 testing contract: "Assert the resulting `TransferDraft`/`BatchTransferDraft`, not only rendered text."
**Code does**: `grep -rn "TransferDraft\|BatchTransferDraft\|BrowseProvider\|TransferAction"` across `crates/` returns zero matches. Current `PaneState` has `path_editor: String` (not `PathEditorState`), no sort/display-preferences field. The browse pattern is per-callsite `spawn_blocking` + ad-hoc reply tagging, not a trait.
**Why this matters**: Without these types the §10 testing contract is structurally impossible — there is no model boundary to assert against. The lack of `BrowseProvider` blocks deterministic fake-provider tests for workflows W1-W4. Concretely, this makes the "Pick-not-Type" model unverifiable by CI.
**Suggested remediation pointer**: Introduce §8.1 types + §8.2 trait + `LocalBrowseProvider`/`RemoteBrowseProvider` before continuing M4-M8. Move the existing per-pane fetch logic behind the trait so the existing main.rs spawn paths become provider calls.

### 3. TUI delegated transfers ship with `detach: false`, contradicting design decision and shipped wire surface

**Class**: drift
**Where**: `crates/blit-tui/src/exec_plan.rs:91-108` (build_delegated_execution), comment at lines 89-90 admits the deferral
**Plan says / Canonical**: TUI_DESIGN §5.2: "The TUI uses `detach=true` on every transfer it initiates against a remote→remote pair." §6.5 + §10 + §12: "Daemon-owned transfer lifecycle for remote→remote (delegated) transfers when `detach=true`."
**Code does**: `build_delegated_execution` hardcodes `detach: false` with a comment "Always attached; detached/F2-visible delegation is a follow-up." Meanwhile the wire surface (proto field), daemon-side detach lifecycle (M-Jobs select arm `service/core.rs:1314-1320`), CLI's `--detach`, `jobs watch`, and `CancelJob` all shipped.
**Why this matters**: The single place that should set `detach=true` doesn't. Closing the TUI cancels every delegated remote→remote it initiated, breaking TUI_DESIGN §3's closing promise that "transfers survive their initiator disconnecting." Operators who expect the M-Jobs lifecycle get the pre-M-Jobs behavior whenever the TUI is the trigger.
**Suggested remediation pointer**: Flip to `detach: true` for delegated paths, surface a banner on the trigger modal for local-endpoint transfers per §5.2, add a regression test pinning the spec field.

### 4. `--detach` shipped despite REMOTE_REMOTE_DELEGATION_PLAN saying "out of scope, track as separate future feature"

**Class**: drift
**Where**: `crates/blit-cli/src/cli.rs:325-335` · `crates/blit-cli/src/transfers/mod.rs:161-178, 255-269` · `crates/blit-cli/src/transfers/remote_remote_direct.rs:126-189` · `crates/blit-daemon/src/service/core.rs:1314-1320` · `crates/blit-daemon/src/active_jobs.rs:59`
**Plan says / Canonical**: REMOTE_REMOTE_DELEGATION_PLAN.md §9, §4.2 step 12, §7: `"--detach mode where CLI exits and dst continues. Track as separate future feature." Out-of-scope explicitly.` §4.2 step 12: `"Document that delegated pulls are CLI-session-bound; --detach is out of scope (§9)."`
**Code does**: Fully wired: CLI flag, gate enforcement (rejects on push/pull/local), three CLI dispatch sites including `--json` detach envelope, daemon-side `if !detach` guard, tests covering all the rejection paths.
**Why this matters**: The plan emphatically calls this future work in three places; the feature is the load-bearing surface behind M-Jobs and a primary TUI design point. Anyone reading the delegation plan to understand 0.1.0 scope is misled, and the "session-bound only" invariant is no longer the contract.
**Suggested remediation pointer**: Update REMOTE_REMOTE_DELEGATION_PLAN §9 and §4.2 step 12 to record that `--detach` shipped (with code-site references), so future readers do not believe "session-bound only" is the live invariant. Cross-link to TUI_DESIGN M-Jobs.

### 5. Planner heartbeat / 10 s stall detector / streaming planner — never shipped under any name

**Class**: drift
**Where**: `crates/blit-core/src/orchestrator/orchestrator.rs:540-574` (synchronous scan→plan→pipeline, no timeout on `header_rx.recv().await` or `scan_handle.await`) · `crates/blit-core/src/remote/transfer/stall_guard.rs:29` (only `PULL_STALL_TIMEOUT = 30s` exists)
**Plan says / Canonical**: greenfield_plan_v6.md §1.1 v5: "Incremental planner that emits work every heartbeat (1 s default, 500 ms when workers are starved). 10 s stall detector (planner *and* workers idle) with precise error reporting. Adaptive predictor fed by local telemetry to keep perceived latency ≤ 1 s." WORKFLOW_PHASE_2.md §"Success Criteria" reaffirms 10 s. LOCAL_TRANSFER_HEURISTICS.md header: "no staged rollout—every mechanism described here will ship together once complete."
**Code does**: No `PlannerEvent`, no `stream_local_plan`, no `drive_planner_events`, no `HEARTBEAT_INTERVAL`, no `starved` cadence reduction. Grep returns zero. The orchestrator awaits scan headers synchronously with no idle timeout. The only stall guard is 30 s on the pull data-plane TCP socket. Push data plane has none.
**Why this matters**: The FAST principle's load-bearing mechanism is the predictor + heartbeat. The predictor exists but is *observability only* (verbose log of predicted vs actual); it does not enforce or measure a ≤1 s latency invariant. A stuck network FS on local scan wedges the orchestrator indefinitely. The plan/code mismatch is so large a contributor reading WORKFLOW_PHASE_2 looking for `TransferFacade::stream_local_plan` will find nothing.
**Suggested remediation pointer**: Either (a) revise WORKFLOW_PHASE_2 + LOCAL_TRANSFER_HEURISTICS + greenfield_v6 §1.1 to describe the synchronous orchestrator + 30 s pull-only stall guard as what shipped, OR (b) actually build the streaming planner. Conservative path: update plan to match code, then add a `scan_handle.await` outer timeout as a small reliability win.

### 6. F1 confirm-detail silently treats endpoint parse Err as Local — violates the explicit memory rule

**Class**: inconsistency (endpoint parsing)
**Where**: `crates/blit-tui/src/display_f1.rs:46-54` (confirm_detail for `PullKind::Move`) — Err falls through to "deletes the local source"
**Plan says / Canonical**: Project memory `feedback_endpoint_parse_err.md`: "4 buckets: module/root=remote, bare-discovery & local=local, Err=reject. Reopened d-61, d-68 ×3." Plan_f1_trigger correctly returns `TriggerOutcome::Rejected("invalid source: {src}")` on Err.
**Code does**: confirm-detail uses `match parse_transfer_endpoint(source) { Ok(Endpoint::Remote(_)) => "deletes the remote source", _ => "deletes the local source" }`. Err is silently classified as local source. Today the trigger gate normally blocks unparseable sources from reaching confirm, but the lie remains in the renderer for any future refactor that loosens the gate.
**Why this matters**: This is exactly the pattern the reviewer reopened four times across other code paths. The confirm prompt's "y" answer is supposed to mean "yes, delete the side I said I'd delete." On a parse Err it could lie about which side gets erased — data-loss-adjacent UI miscommunication.
**Suggested remediation pointer**: Add an `unreachable!` (paired with a debug_assert in plan_f1_trigger) for the Err arm, or route confirm-detail through a shared classifier returning `Result<DeleteVictim, _>` that rejects Err at the gate.

### 7. `--metrics`-disabled token rejection uses different gRPC Status codes across data-plane paths

**Class**: inconsistency (error handling)
**Where**: `crates/blit-daemon/src/service/push/data_plane.rs:185` · `crates/blit-daemon/src/service/pull.rs:740` · `crates/blit-daemon/src/service/pull_sync.rs:632, 754`
**Plan says / Canonical**: Same logical event (peer presented wrong handshake token) should map to one gRPC code. `Status::unauthenticated` is the semantically correct one (token = bearer credential failure).
**Code does**: Push and Pull use `Status::permission_denied("invalid data plane token")` and `Status::permission_denied("invalid pull data plane token")` respectively; pull_sync and pull_sync_resume use `Status::unauthenticated("invalid data plane token")`. Message text also drifts ("data plane token" vs "pull data plane token").
**Why this matters**: Client retry logic and metrics often branch on `status.code()`. A future client that maps `Unauthenticated → re-handshake, PermissionDenied → abort` sees inconsistent behavior depending on which data-plane it hit.
**Suggested remediation pointer**: Single helper `fn reject_invalid_token() -> Status` returning one code (`Unauthenticated`) and one literal string used by all four sites.

### 8. Admin RPC clients erase Status code; `is_retryable` therefore never fires on transport-class remote errors

**Class**: inconsistency (error handling)
**Where**: `crates/blit-app/src/admin/{rm,du,df,list_modules,ls,find}.rs` (drop code via `eyre::eyre!(status.message().to_string())`) · `crates/blit-app/src/admin/jobs.rs:91-95, 114-119` (preserve code: `"RPC failed ({code}): {msg}"`) · `crates/blit-app/src/transfers/retry.rs:27-46` (is_retryable only walks for `std::io::Error` source)
**Plan says / Canonical**: Jobs verbs' "RPC failed ({code}): {message}" style. CLI `--retry`/`--wait` must be able to retry on transport-class errors (`Unavailable`, `DeadlineExceeded`, `Aborted`).
**Code does**: Six admin verbs strip the code; two jobs verbs preserve it. The retry classifier walks the eyre chain for `std::io::Error` — but tonic Statuses wrapped as `eyre!(status.message().to_string())` have no io::Error source, so `is_retryable` returns false for the most common remote-failure class.
**Why this matters**: `run_with_retries` is silently a no-op for `Code::Unavailable` and `Code::DeadlineExceeded` — exactly the codes a flaky daemon emits and exactly what the retry feature is for. Also, operator-facing parity matters: same daemon condition shows two error shapes depending on which verb the user ran.
**Suggested remediation pointer**: Shared helper `fn status_to_eyre(rpc_name: &str, status: Status) -> eyre::Report` used everywhere, AND extend `is_retryable` to walk `eyre::chain()` for `tonic::Status` returning true on `Unavailable`/`DeadlineExceeded`/`Aborted`.

### 9. Push-receive socket lacks the audit-1c stall guard that pull-receive has

**Class**: inconsistency (timeouts)
**Where**: `crates/blit-core/src/remote/pull.rs:1712-1720` (pull TCP wrapped in StallGuard) vs `crates/blit-daemon/src/service/push/data_plane.rs:213-242` (push handler calls `execute_receive_pipeline(&mut socket, ...)` with no guard) · also `service/pull.rs:702-757` and `service/pull_sync.rs:600-755` (daemon-side pull data-plane accept, no guard)
**Plan says / Canonical**: Owner decision `audit-1c` (memory `feedback_port_cli_safety_guards`): "no-bytes-for-30s, scoped to all pulls." By symmetry, any long-lived receive path needs the guard — including the daemon's push-receive and pull-data-plane accepts.
**Code does**: Only the CLI's pull-receive wraps. A push client that opens the data-plane TCP, sends the token, then goes silent leaves the daemon's receive task parked indefinitely.
**Why this matters**: A hostile or stuck push client can pin a daemon worker forever. Combined with no per-message timeout on the streaming RPC handlers (relying entirely on HTTP/2 keepalive 30 s/20 s on the control plane), this is a denial-of-service surface.
**Suggested remediation pointer**: Wrap every `execute_receive_pipeline(...)` and `DataPlaneSession::from_stream(...)` in `StallGuard(_, TRANSFER_STALL_TIMEOUT)`. Hoist the 30 s constant out of `stall_guard.rs` into a shared `transfer::TRANSFER_STALL_TIMEOUT`. Add a push-receive stall test paralleling `pipeline.rs::receive_pipeline_aborts_on_stall`.

### 10. `connect_with_timeout` reimplemented three different ways

**Class**: inconsistency (timeouts)
**Where**: `crates/blit-app/src/client.rs:24, 37-46` (canonical) · `crates/blit-core/src/remote/pull.rs:230-248` (reimplemented inline with hardcoded 30 s) · `crates/blit-core/src/remote/push/client/mod.rs:298-317` (third copy)
**Plan says / Canonical**: `blit-app::client::connect_with_timeout` + `CONNECT_TIMEOUT = 30s` is the intended single helper.
**Code does**: Three independent copies with hardcoded `Duration::from_secs(30)`, all citing the same `audit-2` rationale in doc-comments. None calls the helper.
**Why this matters**: Drift surface for any future timeout bump. `feedback_server_await_timeouts` exists precisely because someone missed one of these once.
**Suggested remediation pointer**: Promote `connect_with_timeout` + constant to `blit-core::remote::client`, re-export, rewrite both pull and push client constructors to use it.

### 11. Unreadable-paths refusal message exists in four flavors across CLI move, TUI move, daemon push, daemon pull_sync

**Class**: inconsistency (error handling)
**Where**: CLI `transfers/mod.rs:463-479` (quotes first 5, long explanation) · TUI `main.rs:4167-4179` (quotes first 3, shorter) · daemon `pull_sync.rs:143-160` (quotes first 5, daemon-side phrasing) · daemon `push/control.rs:328-332` (quotes **0**, generic message)
**Plan says / Canonical**: One data-loss guard, one canonical refusal message. Operator must always see (a) which paths the daemon couldn't read, (b) why this blocks the operation, (c) how to fix.
**Code does**: Four sites, four refusal shapes. Worst case: daemon push gives no preview paths at all, leaving the operator without the actionable information.
**Why this matters**: This guard is the one R47-F4 / R49 / R59 #1 data-loss closure. Different message per dispatch path means operators have to mentally translate between TUI banner / CLI text / daemon stderr to recognize the same failure mode.
**Suggested remediation pointer**: Build `format_incomplete_scan_refusal(operation, unreadable, side)` in `blit-app` (or `blit-core`) and call it from all four sites; settle on a 5-path preview.

### 12. Two surviving ad-hoc `Path → wire` helpers bypass the canonical chokepoint AND disagree on empty-path encoding

**Class**: inconsistency (path handling)
**Where**: `crates/blit-core/src/remote/pull.rs:1795-1804` (`normalize_for_request`, empty → `"."`) · `crates/blit-app/src/transfers/remote.rs:638-647` (second identical `normalize_for_request`, empty → `"."`) · `crates/blit-core/src/remote/push/client/helpers.rs:262-271` (`destination_path`, empty → `""`) · 5 canonical delegators via `path_posix::relative_path_to_posix` (empty → `""`)
**Plan says / Canonical**: `blit_core::path_posix::relative_path_to_posix` (component-walk, joined with `/`, empty → `""`). The receive sinks' empty-rel single-file guards (`source.rs:91-106`, `payload.rs:347-385`, `helpers.rs:200-211`, `pull.rs:1741-1747`) all special-case `relative_path == ""` — they cannot fire when the renderer emits `"."`.
**Code does**: Three encodings: two helpers say empty → `"."`, one says `""`, five canonical sites say `""`. Push sends `""`; pull sends `"."`.
**Why this matters**: A single-file pull where the helper would emit `""` instead silently emits `"."`, bypassing the receive sink's empty-rel guard that was added to avoid `root.join("")` ENOTDIR errors. Push and pull thus carry different empty-path contracts for the same "module root" intent.
**Suggested remediation pointer**: Replace both `normalize_for_request` bodies and the `destination_path` body with `relative_path_to_posix(path)`. If the daemon wire ever needs the `"."` form (it shouldn't — `resolve_relative_path` folds both `""` and `"."`), do the fold at the wire-build layer, not inside a "generic POSIX renderer" helper.

### 13. `perf history clear` confirms in TUI but fires silently in CLI

**Class**: inconsistency (confirmation prompts)
**Where**: TUI `main.rs:4448-4481` + `profile.rs:39-46, 100-112` + `screens/f4.rs:347-349` (modal: "clear ALL local performance history? this is permanent · [y / N or Esc]") · CLI `crates/blit-cli/src/diagnostics.rs:25-30` (`perf::clear()` immediate, no `--yes`, no prompt)
**Plan says / Canonical**: Stated CLI principle: "destructive operations prompt unless `--yes` is supplied." TUI implements this correctly.
**Code does**: `--clear` violates the principle. There is no `--yes` on `PerfArgs` because there is no prompt.
**Why this matters**: A CLI user dropping into the TUI hits a confirm they didn't expect; a TUI user automating in a script silently wipes history. Asymmetric destructive defaults across surfaces is exactly the muscle-memory hazard the unification principle exists to prevent.
**Suggested remediation pointer**: Add an interactive prompt to `blit diagnostics perf --clear` with a `--yes` opt-out matching `mirror`/`move`/`rm`. Same prompt text as the TUI for symmetry.

### 14. `clear-recent` exists only in TUI; CLI has no surface at all, and the TUI clear is unconditional fan-out

**Class**: inconsistency (CLI/TUI parity)
**Where**: TUI `main.rs:1788-1810, 3926-3930` (fans `ClearRecent` to every watched daemon, drops per-daemon results) · `blit-app/src/admin/jobs.rs:107-118` (library function exists) · CLI: no `clear-recent` verb anywhere
**Plan says / Canonical**: TUI_DESIGN §6.3: ClearRecent RPC returns `cleared: u32`; CLI parity floor: anything the CLI can do, the TUI can do — and vice versa is the convention.
**Code does**: Proto `ClearRecentRequest` is wired daemon-side, library function is in blit-app, TUI uses it (with unconditional fire-and-forget that swallows per-daemon errors), but the CLI cannot reach it. Operators automating cleanup must write their own gRPC client.
**Why this matters**: Asymmetric surface; silent failures on the TUI side (clear-recent against an unreachable daemon shows no banner). Cross-daemon irreversible action with no `--yes` style flag in either CLI or TUI config.
**Suggested remediation pointer**: Add `blit jobs clear-recent <REMOTE> [--yes] [--json]` using `confirm_destructive_operation`. Fix the TUI to collect per-daemon Results and banner the count of failed daemons.

### 15. ARCHITECTURE.md still narrates removed auth/TLS work as the current state and "Planned Enhancements"

**Class**: drift
**Where**: `docs/ARCHITECTURE.md:444-454` ("Authentication: Token-based (placeholder in proto)", "Transport: gRPC with optional TLS (not enforced)", "Planned Enhancements: TLS certificate validation; Per-module access control lists; Audit logging")
**Plan says / Canonical**: `proto/blit.proto:110-116, 409, 652-657` carry tombstones for the removed BlitAuth; `docs/DAEMON_CONFIG.md:267-274`: "There is no daemon authentication … not on the roadmap." Owner decision 2026-05-13 removed auth from project scope.
**Code does**: Zero TLS deps in workspace, no auth code, no ACL framework. The architecture doc is the front door but actively misleads.
**Why this matters**: New readers learn the wrong security posture. The "planned" claims contradict DAEMON_CONFIG.md's explicit non-goal. The plan inventory captured this as three separate contradictions; they all trace to one stale subsection.
**Suggested remediation pointer**: Rewrite §"Security Considerations" and §"Planned Enhancements" to match the current model (operator network controls + per-transfer data-plane tokens; no built-in TLS/ACL/audit on roadmap).

### 16. WHITEPAPER §6 still narrates `PullSyncHeader` — removed entirely from the wire

**Class**: drift
**Where**: `docs/WHITEPAPER.md:336-338` ("Client sends `PullSyncHeader` then a `LocalFile` per local entry, then `ManifestDone`") · `proto/blit.proto:269-275` (PullSyncHeader removed; leading message is `TransferOperationSpec`) · `crates/blit-core/src/remote/pull.rs:680-697` (opens bidi stream first, then sends spec) · `crates/blit-core/tests/pull_sync_with_spec_wire.rs:251-310` (test pins the spec leading message)
**Plan says / Canonical**: Spec on the wire as the first PullSync message; PullSyncHeader is dead.
**Code does**: Whitepaper §6 is the authoritative narrative of the pull_sync protocol but cites the removed `PullSyncHeader` plus stale message names (`LocalFile`/`ManifestDone` vs current `local_file`/`ManifestComplete`).
**Why this matters**: Anyone reading WHITEPAPER to understand the resume/mirror protocol gets misled about the wire shape. Cross-reference broken between code-of-record and design-of-record.
**Suggested remediation pointer**: Rewrite §6 step 2: "Client sends `TransferOperationSpec` (replacing the legacy `PullSyncHeader`), then a `FileHeader` per local entry on `local_file`, then `ManifestComplete`."

### 17. Manpage omits two shipped top-level verbs (`jobs`, `check`) AND eight data-loss-relevant transfer flags

**Class**: drift
**Where**: `docs/cli/blit.1.md:8-24` (SYNOPSIS) · `docs/cli/blit.1.md:102-132` (Transfer Options) · `CHANGELOG.md:26-29` · `crates/blit-cli/src/cli.rs:76, 82-86` (Check, Jobs) · `cli.rs:204-364` (TransferArgs full inventory) · `crates/blit-cli/src/transfers/mod.rs:131-406` (data-loss reject gates citing flags the manpage doesn't mention)
**Plan says / Canonical**: blit.1.md and CHANGELOG hold themselves out as the authoritative user surface. `blit --help` text inside `--detach`'s own help directs the user to `blit jobs cancel` and `blit jobs list`.
**Code does**: Manpage lists 14 verbs (omitting `jobs`, `check`). Transfer Options lists 8 flags (omitting `--null`, `--detach`, `--delete-scope`, `--force`, `--ignore-existing`, filter suite `--exclude/--include/--files-from/--min-size/--max-size/--min-age/--max-age`, `--retry/--wait`, `--json` for transfers). CHANGELOG lists 9 verbs and 6 flags.
**Why this matters**: A user reading the manpage will not know `--null`, `--exclude`, or `--delete-scope all` exist, won't know `move --exclude` is forbidden, won't know `blit jobs watch` is the documented way to poll a detached transfer. Manpage dated 2025-11-21 predates the 0.1.0 release (2026-05-31) by 6 months — the date stamp lies about freshness.
**Suggested remediation pointer**: Regenerate manpage from clap definitions (or add an "Advanced Transfer Options" subsection) at the same time the date stamp is bumped. Consider auto-generation via build script.

### 18. TarShardExecutor on TCP push hot path, despite POST_REVIEW §1.2 marking it "gRPC-fallback only"

**Class**: drift / internal contradiction
**Where**: `crates/blit-daemon/src/service/push/data_plane.rs:327` (constructor `TarShardExecutor::new(MAX_PARALLEL_TAR_TASKS)` at top of main receive path) vs `data_plane.rs:620-647` (docstring + `#[allow(dead_code)]` claim "Currently only used by the gRPC fallback path")
**Plan says / Canonical**: POST_REVIEW_FIXES.md §1.2: "After Phase 5 of the receive-pipeline unification, the daemon's TCP push receive routes through `FsTransferSink::write_tar_shard_payload` (rayon-parallel). `TarShardExecutor` is now used **only** by the gRPC fallback path."
**Code does**: The Phase-5 unification claim is partly wrong — TarShardExecutor is still on the **primary** TCP push receive path AND is marked `#[allow(dead_code)]` in the same file. The two statements directly contradict each other.
**Why this matters**: Future contributors may treat the executor as cold gRPC-only code and remove it without realizing it's serving the TCP push hot path. Either the deferral or the unification is wrong.
**Suggested remediation pointer**: Either complete the Phase-5 unification (route all tar-shard receive through `FsTransferSink::write_tar_shard_payload` and delete TarShardExecutor), or update §1.2 closure note and the docstring to describe TarShardExecutor's actual current role as the primary TCP push tar-shard receiver.

### 19. `is_deletable_remote_path` silently filters in TUI batch; CLI `blit rm` bails

**Class**: inconsistency (endpoint classification)
**Where**: `crates/blit-cli/src/rm.rs:24-43` (CLI bails on module-root with module-name-qualified error) vs `crates/blit-tui/src/del_request.rs:31-57` (`build_delete_request` silently filters non-deletable entries; returns `None` only when EVERYTHING is filtered)
**Plan says / Canonical**: CLI behavior — refuse, don't silently drop. Operator must know which targets were skipped.
**Code does**: Mixed batch of {module-root, real-path} silently drops the module-root entry; TUI banner says "deleted N" without reporting the skipped count. Same logical operation, two opposite policies.
**Why this matters**: The very feature that's supposed to prevent deleting an entire module (`feedback-port-cli-safety-guards` rule) is invisibly bypassed by the TUI's silent-filter. Operators get a misleading success signal.
**Suggested remediation pointer**: Have `build_delete_request` return `(Vec<Endpoint>, Vec<SkipReason>)` so the TUI can banner "deleted N, skipped M (module root)". Even better: visually disable module-root rows in F3 so the situation can't arise.

### 20. `BLIT_TUI_INPUT_TRACE` and `BLIT_TEST_COUNTER_FILE` env vars violate "no env vars for config" invariant

**Class**: drift
**Where**: `crates/blit-tui/src/main.rs:4911-4921` (BLIT_TUI_INPUT_TRACE → hardcoded `/tmp/blit-tui-input.log`) · `crates/blit-core/src/remote/instrumentation.rs:10-22` (BLIT_TEST_COUNTER_FILE) · `scripts/bench_remote_remote.sh:81` (production bench depends on the env var)
**Plan says / Canonical**: MASTER_WORKFLOW.md §3: "Environment variables | ✅ Not used for configuration; precedence is CLI flag → config file." greenfield_v6 §5: "No environment variables."
**Code does**: Two env vars are read by the shipped binary. The first writes to a non-portable hardcoded `/tmp` path. The second is consumed by a bench script, not just tests — so an operator copying the bench script picks up the env-var coupling.
**Why this matters**: The invariant is absolute. Even though both vars are diagnostic-only, the bench-script coupling means the violation has propagated into operator-facing tooling. Combined with finding #21 (the documented `BLIT_FORCE_GRPC_DATA=1` / `BLIT_DISABLE_LOCAL_TELEMETRY=1` overrides don't exist), the project's relationship to env vars is incoherent.
**Suggested remediation pointer**: Either add an explicit carve-out in the plan ("env vars permitted for test instrumentation only"), or replace BLIT_TUI_INPUT_TRACE with a TUI config flag and gate BLIT_TEST_COUNTER_FILE behind `#[cfg(test)]` + a hidden subcommand for the bench script.

### 21. Documented `BLIT_FORCE_GRPC_DATA=1` and `BLIT_DISABLE_LOCAL_TELEMETRY=1` env overrides do not exist

**Class**: drift
**Where**: greenfield_plan_v6.md §1.2 line 161, §1.3 line 168 — plan promises both env overrides · grep across `crates/` and `proto/` returns ZERO matches for either var
**Plan says / Canonical**: Plan explicitly carves these two env vars out as the *permitted* exceptions to the no-env-vars rule. Operators following plan documentation will set them and expect effect.
**Code does**: Daemon `--force-grpc-data` flag exists; client `--force-grpc` flag exists; perf-history opt-out is via `blit diagnostics perf --disable` writing `settings.json`. The two named env vars are dead text.
**Why this matters**: Operators in locked-down environments who read the plan and set `BLIT_FORCE_GRPC_DATA=1` observe no fallback. This is the worst kind of drift — documented behavior the code refuses to honor, with no error or warning.
**Suggested remediation pointer**: Either implement the two env vars (and adjust the "no env vars" rule to admit two named exceptions) OR strike them from the plan and document only the CLI/config paths.

## Medium severity

### 21. Two `connect-with-timeout`-style helpers — full list above

(Captured under finding #10; cross-linked here for the cluster.)

### 22. TUI `prepare_local_transfer` rejects Remote with "use the CLI" — but F1 accepts remote dst

**Class**: inconsistency (endpoint classification)
**Where**: `crates/blit-tui/src/main.rs:4041-4062` (F4 rejects Remote with `"F4 transfers only support local→local paths; use the CLI for remote endpoints"`) vs `main.rs:3669-3725` (F1 accepts Local src → Remote dst and dispatches via `f1_push.begin`)
**Plan says / Canonical**: CLI's `select_transfer_route` is the canonical "one parse, four routes" pattern.
**Code does**: F4 points the operator at the CLI for a feature the TUI does support on F1. Two screens, contradictory verdicts on the same input.
**Why this matters**: User-facing inconsistency. Suggested workaround sends operators to a worse experience than the one they already have open.
**Suggested remediation pointer**: F4 either dispatches through the same router as F1 (with a handoff for remote dst), or its error message explicitly routes to F1 not the CLI.

### 23. Admin verb endpoint parsing: 3 patterns (loose, strict, bare `RemoteEndpoint::parse`)

**Class**: inconsistency (endpoint parsing)
**Where**: `parse_endpoint_or_local` (rm/df/du/find/ls/completions), `RemoteEndpoint::parse` direct (list-modules, jobs list/cancel/watch), `parse_transfer_endpoint` (strict — transfers + diagnostics dump)
**Canonical**: Convention is "loose for admin, strict for transfer." `list-modules` and `jobs` skip the helper, producing a different error class on the same bad input.
**Suggested remediation pointer**: Centralize via `parse_endpoint_or_local` + `Endpoint::require_remote() -> Result<RemoteEndpoint>`; have remote-only verbs call `.require_remote()`.

### 24. `extract_module_and_path` (rm) vs `module_and_rel_path` (others) — byte-identical bodies, divergent error strings

**Class**: inconsistency (path/endpoint)
**Where**: `blit-app/src/admin/rm.rs:48-56` ("remote removal requires module syntax (e.g., server:/module/path)") vs `blit-app/src/endpoints.rs:175-183` ("remote target must include a module path") vs `endpoints.rs:86-93, 96-103` (with-example variants)
**Suggested remediation pointer**: One helper taking a role label parameter.

### 25. `--metrics` flag absent from DAEMON_CONFIG.md, causing the "GetState Counters present-but-zero" footgun

**Class**: drift / inconsistency (telemetry)
**Where**: `crates/blit-daemon/src/runtime.rs:104-109` (defines `--metrics`) · `crates/blit-daemon/src/service/core.rs:1072-1078` (always publishes `Counters` regardless of flag) · `proto/blit.proto:752-756` (documents present-but-zero hazard) · `crates/blit-prometheus-bridge/src/metrics.rs:38-96` (works around it by omitting counters) · `docs/DAEMON_CONFIG.md:280-293` (omits the flag entirely)
**Why this matters**: Memory `feedback_getstate_counters_zero` documents the false-zeros hazard. The bridge knows about it and emits gauges only. Documented contract violation: principle says "metrics stay local" + "opt-out via CLI/config", but the always-present zeros leak through `GetState`. Operator who disables metrics expects `counters` to be absent/null, not zero-valued.
**Suggested remediation pointer**: Document `--metrics` in DAEMON_CONFIG.md. Either make `counters: Option<Counters>` actually `None` when metrics disabled, or add `metrics_enabled: bool` sibling field on `DaemonState`.

### 26. mDNS TXT record carries 4 fields but DAEMON_CONFIG.md documents only 2

**Class**: drift
**Where**: `crates/blit-core/src/mdns.rs:140-156` advertises `version`, `modules`, `module_count`, `delegation_enabled` · `docs/DAEMON_CONFIG.md:530-532` lists only `version` and `modules`
**Why this matters**: `module_count` is authoritative when `modules` is truncated past ~180 bytes (per §3.2 of the release plan). `delegation_enabled` is consumed by the TUI's F1 view. Operators inspecting mDNS records see fields the doc doesn't explain.
**Suggested remediation pointer**: Add `module_count` (with truncation rationale) and `delegation_enabled` to DAEMON_CONFIG.md §"mDNS Discovery".

### 27. TUI strips `Status::code()` on stream errors; banner says only "stream: {message}"

**Class**: inconsistency (error propagation)
**Where**: `crates/blit-tui/src/main.rs:5717-5720` (forwards Status as `format!("stream: {}", status.message())`, code discarded) vs CLI `jobs.rs:393-411` (distinguishes via exit code)
**Why this matters**: `Unavailable` (daemon down), `Cancelled` (user close), `Internal` (daemon bug) all look identical in the TUI banner. Operator cannot correlate with daemon logs.
**Suggested remediation pointer**: Format as `"stream: {code}: {message}"`.

### 28. `MirrorMode::Unspecified|Off` produces different behavior on push vs pull_sync purge paths

**Class**: inconsistency (semantics)
**Where**: `crates/blit-daemon/src/service/push/control.rs:343-345` (treats Unspecified|Off-with-mirror_mode=true as "use user's filter" — back-compat for older clients) vs `crates/blit-daemon/src/service/pull_sync.rs:430-462` (`Off | Unspecified` → `Vec::new()`)
**Why this matters**: Same enum value, two different behaviors. A client landing on push gets purge; on pull_sync gets no purge. No client-visible error; silent semantic drift.
**Suggested remediation pointer**: Either make `Unspecified` a hard reject (InvalidArgument), or normalize both sites to the same fallback.

### 29. F4 destructive prompt uses `[y / N or Esc]`; F1/F2/F3 use bare `y/N`; CLI uses `[y/N]:`

**Class**: inconsistency (prompts)
**Where**: F4 (`screens/f4.rs:244-251, 347-349`), F1/F2/F3 (multiple sites), CLI (`transfers/mod.rs:93`)
**Suggested remediation pointer**: Pick one TUI form (with or without `· y/N or Esc`) and apply to all four screens.

### 30. `confirm_destructive_operation` duplicated inline in `rm.rs`

**Class**: inconsistency (prompts)
**Where**: `crates/blit-cli/src/transfers/mod.rs:87-99` (helper) vs `crates/blit-cli/src/rm.rs:48-58` (re-implements same y/yes logic)
**Suggested remediation pointer**: Move helper to `blit-app::common` or `blit-cli::shared`; have `rm` call it.

### 31. F2 cancel confirm is config-gated in TUI; CLI cancel has no `--yes`/`--confirm` at all

**Class**: inconsistency (prompts)
**Where**: TUI `transfer.confirm_cancel` config knob (`config.rs:613`) vs CLI `JobsCancelArgs` has no equivalent
**Suggested remediation pointer**: Add `--confirm` to `JobsCancelArgs` mirroring the TUI's config knob.

### 32. Empty-path encoding on daemon side: `.` (du/find) vs `""` (push/pull)

**Class**: inconsistency (path)
**Where**: `crates/blit-daemon/src/service/util.rs:153-158` (`normalize_relative_path` → `""`) vs `:160-168` (`pathbuf_to_display` → `"."` for explicit `Path::new(".")`) vs `:51-67` (`resolve_relative_path` request-path fold)
**Why this matters**: A `du`/`find` client sees `relative_path: "."` for module root; a `pull` client sees `""`. Same daemon, two encodings on the wire.
**Suggested remediation pointer**: Fold `pathbuf_to_display` into `relative_path_to_posix`.

### 33. `validate_wire_path` rejects `"."` as unsafe, but `normalize_for_request` actively produces `"."`

**Class**: inconsistency (path validation contracts)
**Where**: `crates/blit-core/src/path_safety.rs:71-133` rejects non-empty input that normalizes to `.` · `crates/blit-core/src/remote/pull.rs:1795-1804` emits `"."` for empty rel_path · daemon `resolve_relative_path` (`util.rs:51-67`) folds both `""` and `"."` to `PathBuf::from(".")`
**Why this matters**: If a future caller routed `normalize_for_request("")` through `validate_wire_path`, it bails. The two layers carry contradictory contracts; the daemon protects by special-casing before validation runs.
**Suggested remediation pointer**: Standardize on `""` for root across the wire. Drop the empty→`.` fold in both `normalize_for_request` copies.

### 34. `FsTransferSink` canonical-fallback ladder: 2 sites `log::warn!`, 2 sites silent

**Class**: inconsistency (observability)
**Where**: `sink.rs:190-205` and `:463-481` warn; `:651-657` (write_file_block_payload) and `:696-702` (write_file_block_complete) are silent — same R46-F3 fallback
**Why this matters**: Operator scanning logs for "R46-F3 escape protection unavailable" misses resume-block calls writing through the same fallback.
**Suggested remediation pointer**: Extract single helper `safe_join_with_warn_fallback`; have all four call sites use it.

### 35. Source-delete-failed message uses three different past-tense verbs across TUI move paths

**Class**: inconsistency (error messages)
**Where**: TUI `main.rs:3339` ("received but..."), `:3423` ("pushed but..."), `:3523` ("delegated but..."), `blit-app/src/transfers/remote.rs:250` (no prefix at all)
**Suggested remediation pointer**: Single helper `format_post_transfer_delete_failure(operation, side, err)` — same shape every time. This is data-loss-adjacent; the message must not look like passing noise.

### 36. Push verb table omits delegated-mirror/delegated-move labels

**Class**: inconsistency (display)
**Where**: `crates/blit-tui/src/display_f1.rs:121-137` — `(true, Copy) → "delegating"` but `(true, Mirror)` and `(true, Move)` fall through to "mirroring"/"moving" with no delegated label
**Why this matters**: A future delegated mirror reads identically to a local mirror push in the footer banner; operator cannot tell where the bytes flow.
**Suggested remediation pointer**: Either complete the table for delegated mirror/move or document the asymmetry.

### 37. CLI rejects mode-incompatible flags with wildly different verbosity

**Class**: inconsistency (error messages)
**Where**: `move --dry-run` bails with TERSE single-line; `move --null` / `move --force` / `--null --mirror` bail with 7-line essays. Same gate class, different operator experience.
**Suggested remediation pointer**: Pick one style for data-loss-class rejections (verbose-with-remediation).

## Low severity / documentation drift

### 38. — `blit-utils` references survive in code comments (`admin_verbs.rs:323`, `service/admin.rs:500`) and v6 §4 deliverables checklist
### 39. — `--workers` flag in CHANGELOG, omitted from manpage; no `[DEBUG] Worker limiter active` banner despite plan promise
### 40. — `find --files` / `--dirs` flags shipped, not documented in manpage SYNOPSIS or OPTIONS
### 41. — `--json` documented for du/df only in manpage OPTIONS, despite SYNOPSIS listing 7 commands and code supporting many more
### 42. — `--limit` documented for find only; SYNOPSIS lists it on `profile` and `diagnostics perf` too
### 43. — Manpage dated 2025-11-21 vs v0.1.0 release 2026-05-31 (6-month staleness)
### 44. — README "Rust 1.56+" claim not enforced via `rust-version` in any Cargo.toml
### 45. — `BLIT_UTILS_PLAN.md:65` claims `docs/cli/blit-utils.1.md` was created 2026-03-06; file doesn't exist; same doc's banner says it shouldn't
### 46. — `Pull` RPC and `ServerPullMessage.ack` deprecated only in proto comments, no `[deprecated = true]` annotation
### 47. — `blit_daemon_up` gauge always = 1 when bridge produces a value; bridge omits all 5 counter series with no doc note explaining the omission
### 48. — daemon TCP keepalive applied only on push-receive accept; pull-receive and pull_sync-receive accept paths don't tune
### 49. — Token comparison uses `==` (variable-time) at 4 daemon sites — practical risk low (32 random bytes from OS RNG), but inconsistent with usual "secure compare" pattern
### 50. — `compare.rs` has 2 s FAT/exFAT mtime tolerance; `manifest.rs::compare_file` Default mode has zero tolerance — same logical question, two answers
### 51. — `copy_file` always uses BufferSizer; `chunked_copy_file` hardcodes 16 MiB for files >1 GiB and uses BufferSizer otherwise — same workload, different sizing
### 52. — `mmap_copy_file` is a misnomer: no memory mapping, just `copy_file_range`/`sendfile`/`fs::copy`
### 53. — `mtime tolerance` 2 s in `mirror_planner` repeated twice without a named constant
### 54. — `enotempty-errno-66-only-macos-bsd`: errno literal `66` matches macOS/BSD ENOTEMPTY; Linux ENOTEMPTY is 39 — relies on `err.kind() == DirectoryNotEmpty` to cover Linux
### 55. — `copy_large_blocking` creates dest parent dir BEFORE checking `dry_run`, contradicting the R58-F4 invariant honored in `copy_path_maybe`
### 56. — Two glob-matching engines (`globset` + hand-rolled `glob_match`) coexist in `fs_enum.rs`; `build_globset` silently drops invalid patterns
### 57. — Three independent format-bytes implementations across the TUI (F1/F2/F4 with TiB; F3 capping at GiB; dual_pane with `.1` precision and no TiB)
### 58. — `--interval-ms` dead flag on `jobs watch` (preserved for back-compat, no effect under streaming Subscribe)
### 59. — 0-sentinel meaning differs across CLI flags: `--max-depth 0` = unbounded, `--recent-limit 0` = daemon default 50, `--timeout-secs 0` = forever, `--limit 0` = profile-shown all
### 60. — Module-name validation rejects only empty/whitespace; daemon happily accepts `foo/bar` or `..` as module names, which the wire parser couldn't address
### 61. — Bridge returns 404 for non-GET methods instead of 405 Method Not Allowed
### 62. — DAEMON_CONFIG.md:524-528 says "discover it with `blit scan` or `blit scan`" — editing slip
### 63. — `motd` documented as "Message displayed to clients on connect" but is only printed to daemon's stdout at startup; never reaches clients
### 64. — F3 format_bytes lacks TiB tier; an operator du-ing a 2 TiB subtree sees `"2048.00 GiB"`
### 65. — `127.0.0.1:9031` and `/tmp/blit-tui-input.log` hardcoded across TUI Local-row and input trace
### 66. — `pull_sync_with_spec_wire.rs:212` adds a 50 ms tokio sleep ("belt-and-suspenders") that "shouldn't be necessary"
### 67. — `scripts/` directory contains Codex installer (`test.sh` misnamed), three personal resume scripts (`codex_resume.sh` runs `sudo npm`), and a 3,942-line Claude Code transcript dump
### 68. — Linux change-journal doc in ARCHITECTURE.md says "fallback to mtime comparison" when code actually uses (device, inode, ctime) snapshot with mtime fallback only as last resort
### 69. — Architecture's `PerformanceRecord` snippet shows v1 pre-migration shape; current schema is v2 with `run_kind`, `mode`, options-bag
### 70. — Whitepaper §3.1 says `self.pool.acquire()` but `DataPlaneSession` uses inline buffers (no pool). Pool exists for local copies, not data plane

## Cross-cutting inconsistencies (by dimension)

### Path handling

**Summary**: A canonical chokepoint exists (`blit_core::path_posix::relative_path_to_posix`) but is bypassed by three ad-hoc helpers that disagree on empty-path encoding (push→`""`, pull→`"."`, helper→`""`). The daemon's own wire encoding also splits: push manifests emit `""` for root, but `du`/`find` emit `"."`. The strict wire-path validator (`validate_wire_path`) rejects `"."` as "normalizes to empty" — yet two callers actively produce `"."` for the same logical "module root." Receive sinks' empty-rel single-file guards (designed to avoid `root.join("")` ENOTDIR) silently fail to fire when the renderer emits `"."`.

**Worst instances**:
- pull.rs:1795-1804 + remote.rs:638-647 (duplicate `normalize_for_request` emitting `"."`)
- `pathbuf_to_display` vs `normalize_relative_path` in daemon util.rs

**Canonical pattern**: One helper. `path_posix::relative_path_to_posix` everywhere. Empty → `""`. Daemon side already folds both encodings to `PathBuf::from(".")` once, so do the fold at the wire-build layer if any caller truly needs `"."`.

### Error handling

**Summary**: Three error-wrapping styles for tonic Status (preserve code+message, strip to message, swap to `with_context` chain) are scattered across the codebase. Admin verbs use one style, jobs verbs another, CLI completions a third. The result: `is_retryable` cannot fire on transport-class errors because the io::Error chain is gone after admin clients wrap a Status. Same daemon condition shows two different error shapes to the operator depending on which verb hit it. Data-loss-class "unreadable paths refusal" has four flavors across CLI move, TUI move, daemon push, daemon pull_sync — three preview-lengths (5/3/0) and three rationale phrasings.

**Worst instances**:
- Admin clients vs `jobs::cancel`/`jobs::clear_recent` (only the latter preserves Status code)
- Four unreadable-paths refusal messages (CLI/TUI/daemon-push/daemon-pull_sync)
- Four data-plane token rejection sites using 2 different Status codes and 2 different message strings

**Canonical pattern**: One helper `status_to_eyre(rpc_name, status) -> eyre::Report` preserving code, used everywhere. One helper `format_incomplete_scan_refusal(operation, paths, side)` for the data-loss guard. Extend `is_retryable` to walk `eyre::chain()` for `tonic::Status::Unavailable|DeadlineExceeded|Aborted`.

### Endpoint parsing

**Summary**: Three parse functions (`parse_endpoint_or_local` loose, `parse_transfer_endpoint` strict, `RemoteEndpoint::parse` bare) used inconsistently. `list-modules` and `jobs` bypass the helper that produces friendlier "verb-is-remote-only" errors. The TUI F1 confirm-detail violates the explicit "Err must reject" project-memory rule by silently classifying as Local. `is_deletable_remote_path` filters silently in TUI batch operations while `blit rm` bails. CLI `prepare_local_transfer` rejects Remote with "use the CLI" — but F1 push accepts remote dst. `parse_launch_remote`/`resolve_launch_remote` empty-string semantics are asymmetric: `--remote ""` is honored but `default_remote = ""` is treated as unset.

**Worst instances**:
- display_f1.rs:46-54 Err → "deletes the local source"
- TUI `build_delete_request` silently dropping module-root entries from a batch

**Canonical pattern**: Always 4-bucket classify: module/root → Remote, bare-discovery → Discovery (and require-remote verbs reject), Local → Local with friendly per-verb error, Err → reject. Single endpoint helper used everywhere; per-verb error labels via parameter.

### Timeouts / retries / cancellation

**Summary**: Pull-receive has a 30 s data-plane stall guard; push-receive and daemon-side pull-data-plane accepts do not. Push-receive has TCP nodelay+keepalive tuning; pull-receive accepts do not. Three independent `connect_with_timeout` implementations with hardcoded 30 s. Two `--metrics`-disabled token rejections use `permission_denied`, two use `unauthenticated`. `is_retryable_io_kind` (in `retry.rs`) and `categorize_io_error` (in `errors.rs`) disagree on UnexpectedEof, NotConnected, ConnectionRefused, Interrupted, WouldBlock. TUI transfers have no `--retry`/`--wait` equivalent at all. Daemon streaming RPC handlers have no per-message timeout (relying on HTTP/2 keepalive on control plane); client side doesn't request HTTP/2 keepalive. `--interval-ms` dead flag. 0-sentinel meaning differs across flags.

**Worst instances**:
- `dataplane-stall-guard-only-on-pull-receive` (DoS surface)
- `connect-with-timeout-duplicated-three-ways`
- `retry-classifier-disagrees-with-categorize-io-error` (silent classification drift)

**Canonical pattern**: One shared `TRANSFER_STALL_TIMEOUT` constant. One `connect_with_timeout` + `CONNECT_TIMEOUT` in `blit-core::remote::client`. One token-rejection helper returning one Status code. Reconcile `retry.rs` and `errors.rs` to one classifier; extend it to handle tonic Status codes. Document HTTP/2 keepalive rationale at every server-side `stream.message().await` site.

### Naming / flags / confirmations

**Summary**: TUI confirms `perf history clear` (modal y/N) while CLI fires silently. `clear-recent` exists only in TUI, unconditional fan-out. TUI F2 cancel confirm is config-gated; CLI cancel has no opt-in surface. Four destructive-prompt phrasings (F4 `[y / N or Esc]`, F1/F2/F3 bare `y/N`, CLI `[y/N]:`, two helper duplicates). TUI state machines diverge: F4 has separate `ConfirmingMirror`/`ConfirmingMove` variants, F3 has single `Confirm { kind }`, F1 has `confirming: bool` inside `Editing` — three shapes for the same y/N question. `UserAction::TransferMirrorConfirm` is overloaded across mirror, move, cancel, batch cancel, clear-recent — type name lies about purpose. `--delete-scope` is stringly-typed (clap case-sensitive, consumer case-insensitive). Reject-flag verbosity is bumpy (1-line vs 7-line essays for similar severity).

**Worst instances**:
- CLI `perf --clear` silent vs TUI confirms
- `clear-recent` exists in TUI only, with unconditional fan-out + result discarded
- `UserAction::TransferMirrorConfirm` misnomer

**Canonical pattern**: All destructive operations prompt by default; `--yes` opts out; same prompt vocabulary (`y`/`yes`) and same writer target (`/dev/tty` or stderr) everywhere. One `Confirm { kind }` variant pattern. Rename `TransferMirrorConfirm` → `ConfirmYes`. `--delete-scope` becomes `enum DeleteScope { Subset, All }`. CLI + TUI surfaces stay in lockstep on confirm posture.

## What's solidly aligned

- **Data-plane wire format and tags** — `FILE=0x00`, `TAR_SHARD=0x01`, `BLOCK=0x02`, `BLOCK_COMPLETE=0x03`, `END=0xFF`, 32-byte token prefix all match WHITEPAPER §3 lines 196-204 exactly. `BLOCK_COMPLETE` carries `mtime+perms` inline as documented (commit a7d659f).
- **F2 canonical-path containment is always-on at every chokepoint** — `path_safety::safe_join_contained` + `verify_contained` + `canonical_dest_root` form a single chokepoint; daemon enforces on push handshake, tar shard receive, mirror purge, delete list, disk usage start, find start. `use_chroot` config field removed cleanly (F13).
- **Delegation gate ordering and DNS-rebinding mitigation** — `delegation_gate::validate_source` walks master switch → empty host → port 0 → IP-form for special ranges (loopback/link-local/unique-local) → resolve-once → all-resolved-must-match → bind validated IP. R25-F3 SSRF-via-DNS pivot closed. Per-module narrowing override can only narrow daemon-wide policy.
- **Spec-version fail-closed for v1 daemons** — `operation_spec.rs:107-111` rejects any non-exact `spec_version` with explicit error message; v1 daemons hitting a v2 spec fail rather than silently ignoring `require_complete_scan`.
- **`pull_sync_with_spec` endpoint-isolation** — Daemon-side handler reads spec, never `self.endpoint.path`. Wire test `pull_sync_with_spec_wire.rs:251-310` pins the byte-for-byte spec leading message. R23-F1 / R25-F1 invariants honored end to end.
- **Mandatory client_capabilities override** — Destination handler unconditionally rewrites `spec.client_capabilities` with its own `PeerCapabilities` before forwarding to source. R25-F2 honored.
- **`require_complete_scan` purge gate** — Daemon refuses push purge AND refuses pull_sync source-delete when source scan was incomplete. R49-F2 / R59 #1 F1 honored. Tested for local move (chmod 000), remote push mirror, pull_sync mirror/move.
- **MirrorMode default `FILTERED_SUBSET`** — Proto enum + daemon `scope_deletions` + pull_sync delete-list authority all converge. `--include '*.bin' --mirror` no longer purges destination's non-bin files (R59 #1 F2 closed).
- **No-silent-fallback CLI dispatch on remote→remote** — Stale destination daemon → `Unimplemented` → clean error directing user to `--relay-via-cli` or upgrade. Source ACL refusal → surfaced verbatim, no fallback. R21-F5 honored. Tests `remote_remote.rs:277-345` cover both.
- **Token cryptographic + per-stream** — 32-byte tokens via OS RNG (`SysRng::try_fill_bytes`); RNG failure surfaces as `Status::Internal` (no panic). Audit-3b honored.
- **BlitAuth removal complete** — No `BlitAuth` code, no `delegated_credential` use; proto reserves field 10 + `"delegated_credential"` per protobuf rules. ARCHITECTURE.md is the only doc still describing it (drift finding #15).
- **Block-level resume via Blake3** — Block-hash delta protocol with per-block size cap (`MAX_BLOCK_SIZE = 64 MiB`); auto-promote `Modified` (size match, mtime mismatch) → block-hash compare without `--resume` (commit a7d659f). Regression test in `remote_regression.rs`.
- **Predictor observability shipped per D9** — `PerformancePredictor` 1368 lines, dual-target (planner + transfer) with fallback chain depth 0-3, surfaced via `blit profile --json`. Null-sink runs correctly skip the predictor learning loop.
- **mDNS service + TXT keys shipped** — `_blit._tcp.local.` advertisement with `version`, `modules`, `module_count`, `delegation_enabled` (D4 taken). 180-byte truncation handled via authoritative `module_count`.
- **Endpoint parser rejects bare `server:/module`** — Missing trailing slash returns Err per `feedback_endpoint_parse_err`. Bare host → Discovery, module form requires trailing `/`, root form via `://`.
- **CLI data-loss reject-gates** — `move` rejects `--detach`, `--null`, `--dry-run`, `--force`, `--ignore-times`, `--ignore-existing`, all filter args, and remote-source `--relay-via-cli`. `--null` rejects mirror and remote. Test coverage in `cli_arg_safety_gates.rs` and `local_move_semantics.rs`.
- **Unified receive pipeline** — Single `execute_receive_pipeline` used by push receive, pull receive, and remote→remote receive. ~525 LOC deleted from daemon's bespoke dispatch (commits 1baa981, a232dbd, b64bfd8). Receive symmetry preserved as PIPELINE_UNIFICATION claims.
- **pull_sync deadlock fix** — `crates/blit-core/src/remote/pull.rs:680-697` opens bidi stream first, then sends Spec. Regression test `pull_sync_does_not_deadlock_with_populated_destination` in `remote_regression.rs`.
- **mtime preservation race fix** — `set_file_mtime` runs after the tokio File handle is dropped (`sink.rs:402-407` decision documented inline). Tests `pull_preserves_mtime_end_to_end` and `mtime_only_change_does_not_re_transfer_full_file` pin the behavior.
- **CI tri-platform matrix and release artifact build** — `cargo test --workspace` runs on ubuntu/macos/windows; release builds gated to master. Test totals 407/0 as of recent commits.
- **Recent persistence (`recents.jsonl`) atomic write** — Atomic `.jsonl.tmp` + rename + `sync_all`. `clear_recent` correctly wipes ring + JSONL but never touches `perf_local.jsonl` (per memory `project_recent_persistence`). Tested.

## Recommendations — ordered punch list

1. **Wire dual-pane action bar to transfer execution** (finding #1) — flip the default screen back to F1 until M4 is wired, or finish M4. Without this the rework's headline principle is unverifiable.
2. **Flip TUI delegated `detach: true`** (finding #3) — single-line change in `exec_plan.rs`; closes the "transfers survive disconnect" promise. Add wire-pinning test.
3. **Document `--detach` shipped in REMOTE_REMOTE_DELEGATION_PLAN** (finding #4) — small doc edit, big plan/code-alignment win.
4. **Update plan docs to match actual planner architecture** (finding #5) — strike the streaming-planner / 1 s heartbeat / 10 s stall claims; describe synchronous orchestrator + 30 s data-plane pull stall guard.
5. **Add stall guard to daemon push-receive** (finding #9) — DoS-class hardening; wrap `execute_receive_pipeline(socket, ...)` in `StallGuard(_, TRANSFER_STALL_TIMEOUT)`. Single-file change with regression test.
6. **Centralize `connect_with_timeout` to one helper** (finding #10) — move to `blit-core::remote::client`; rewrite pull and push clients to use it. Removes 3 sites of magic-number duplication.
7. **Single `status_to_eyre` helper + extend `is_retryable` for tonic codes** (finding #8) — fixes the silent no-op of `--retry/--wait` on transport errors.
8. **Single `format_incomplete_scan_refusal` helper** (finding #11) — data-loss-message parity across four sites; settle on 5-path preview.
9. **Single token-rejection helper returning `Unauthenticated`** (finding #7) — 4-site consolidation; one gRPC code, one message string.
10. **Replace ad-hoc `normalize_for_request`/`destination_path` with `relative_path_to_posix`** (finding #12) — fixes the push/pull empty-path encoding split; single canonical chokepoint.
11. **Add F1 confirm-detail Err arm — `unreachable!` or shared classifier** (finding #6) — closes the d-61/d-68 pattern at its last live site.
12. **Surface skipped entries from TUI delete batch** (finding #19) — match CLI behavior; banner shows "deleted N, skipped M".
13. **Add `blit jobs clear-recent <REMOTE> [--yes] [--json]` CLI verb + collect per-daemon Results in TUI** (finding #14) — closes the CLI/TUI parity gap and the unconditional fan-out hazard.
14. **Add prompt + `--yes` to `blit diagnostics perf --clear`** (finding #13) — CLI/TUI confirm parity for destructive history wipe.
15. **Regenerate manpage from clap; include `jobs`, `check`, and the full transfer flag surface** (finding #17) — bump the 6-month-stale date stamp; consider auto-generation.
16. **Rewrite ARCHITECTURE.md Security Considerations + Planned Enhancements** (finding #15) — replace stale auth/TLS narrative with the actual model.
17. **Rewrite WHITEPAPER §6 to use `TransferOperationSpec` not `PullSyncHeader`** (finding #16) — one-paragraph fix.
18. **Resolve TarShardExecutor contradiction** (finding #18) — either complete the Phase-5 unification or update the docstring + plan note to describe its current role accurately.
19. **Document `--metrics` flag in DAEMON_CONFIG.md + decide `Counters` proto contract** (finding #25) — either `Option<Counters>` (None when off) or `metrics_enabled: bool` sibling.
20. **Hoist 30 s/15 s accept/token timeout constants to a single shared pair** (finding 4 of timeouts cluster) — drift surface across 4 sites.
21. **Add `tune_data_plane_socket` helper applied to push AND pull accept paths** (finding 5 of timeouts) — symmetric nodelay+keepalive on both ends.
22. **Replace 3 `confirm` enum/bool shapes with one `Confirm { kind }` pattern across TUI** (finding 7 of naming) — single state-machine shape; rename `TransferMirrorConfirm` → `ConfirmYes`.
23. **Rename `mmap_copy_file` to reflect actual implementation** (finding #52) — `copy_file_range_or_sendfile` is honest; the current name actively misleads.
24. **Delete or move out-of-scope scripts** (finding #67) — `scripts/test.sh` (Codex installer), `codex_resume.sh`, `mac_codex_resume.sh`, `win_codex.ps1`, and the 3,942-line transcript dump don't belong in `scripts/`.
25. **Strike the `BLIT_FORCE_GRPC_DATA` / `BLIT_DISABLE_LOCAL_TELEMETRY` env-var promises from greenfield_v6** (finding #21) — or implement them. Either closes the documented-but-unimplemented hazard.
26. **Update TUI_DESIGN structural commitment #1** (finding #92 in plan-tui contradiction list) — §12 "Four-screen architecture" should reflect the dual-pane supersession or annotate as historical.

## Appendix A: coverage attestation

| Cluster | Files | Lines read |
|---|---:|---:|
| **Plan inventories** | | |
| plan-principles (greenfield_v6 + MASTER_WORKFLOW + RELEASE_PLAN_v2) | 3 | 1,367 |
| plan-phases (POST_REVIEW_FIXES + REMOTE_REMOTE_DELEGATION + WORKFLOW_PHASE_{2,3,4} + PROJECT_STATE_ASSESSMENT) | 6 | 1,929 |
| plan-tui (TUI_DESIGN + TUI_REWORK) | 2 | 1,529 |
| plan-wire (ARCHITECTURE + WHITEPAPER + DAEMON_CONFIG + blit.proto) | 4 | 2,866 |
| plan-perf (LOCAL_TRANSFER_HEURISTICS + PIPELINE_UNIFICATION + BENCHMARK_10GBE_PLAN + BENCH_VERB_PLAN) | 4 | 944 |
| plan-cli (blit.1.md + README.md + CHANGELOG.md + BLIT_UTILS_PLAN.md) | 4 | 505 |
| **Plan subtotal** | **23** | **9,140** |
| **Code inventories** | | |
| code-cli (blit-cli: cli.rs, main.rs, transfers/*, jobs.rs, all admin verbs) | 20 | 4,920 |
| code-daemon (blit-daemon: full crate) | 17 | 12,489 |
| code-bridge-proto (blit-prometheus-bridge + proto/blit.proto + blit-core/build.rs) | 6 | 1,901 |
| code-core-transfer (blit-core: remote/{transfer,push,pull,endpoint}/*) | 17 | 10,619 |
| code-core-io (blit-core: copy/*, delete, buffer, tar_stream, checksum, manifest) | 16 | 3,533 |
| code-core-orch (blit-core: orchestrator/*, mirror_planner, local_worker, enumeration, fs_enum) | 10 | 4,566 |
| code-core-misc (blit-core: auto_tune, fs_capability/*, change_journal/*, mdns, perf_*, path_safety, path_posix, lib/config/errors/logger) | 19 | 5,884 |
| code-tui-main (blit-tui/src/main.rs full read) | 1 | 10,838 |
| code-tui-state (blit-tui state machines + pane behavior) | 14 | 10,876 |
| code-tui-display (blit-tui display mappers + helpers + screens) | 16 | 5,380 |
| code-tests-scripts (integration tests + scripts + CI workflow) | 40+ | 12,083 |
| **Code subtotal** | **~176** | **83,089** |
| **TOTAL** | | **92,229** |

**Files that were NOT read** — none in scope of this audit. The `2025-10-24-…txt` transcript dump (3,942 lines) was sampled rather than line-by-line read because it is an unrelated Claude Code session transcript; flagged as a workspace-hygiene smell (finding #67) for removal.

The inventories explicitly verified absence-from-disk of: `crates/blit-utils/` (correctly absent per merge), `docs/cli/blit-utils.1.md` (correctly absent), `crates/blit-app/build.rs` and `crates/blit-daemon/build.rs` (correctly absent — both consume `blit_core::generated::*` re-exports), `crates/blit-daemon/tests/` and `crates/blit-app/tests/` (absent; all daemon coverage flows through CLI integration tests).

## Appendix B: cross-references

Index of files frequently mentioned in findings, with their inventory home + drift entries linked back.

**`crates/blit-cli/src/cli.rs`** — Inventory: `code-cli.md` lines 9-46. Drift entries: #17 (manpage omits flags + verbs), #4 (`--detach` plan vs code), finding 9 of naming (`--delete-scope` stringly-typed), finding 12 of naming (`list-modules` parser divergence).

**`crates/blit-cli/src/transfers/mod.rs`** — Inventory: `code-cli.md` lines 14-22 (flag-handling), §confirmation-prompt. Drift entries: #11 (unreadable-paths refusal), #19 (rm vs build_delete_request), CLI reject-gate verbosity (finding 37), confirm helper duplication (CFM-05).

**`crates/blit-cli/src/jobs.rs`** — Inventory: `code-cli.md` §state-machine + §format-output + §error-propagation. Drift entries: #8 (admin vs jobs Status preservation), `--interval-ms` dead flag (#58).

**`crates/blit-cli/src/rm.rs`** — Inventory: `code-cli.md` §endpoint-parse + §confirmation-prompt. Drift entries: #19 (CLI bails on module root vs TUI silent filter), CFM-05 (duplicated confirm helper).

**`crates/blit-cli/src/diagnostics.rs`** — Inventory: `code-cli.md` §persistence + §format-output. Drift entries: #13 (`perf --clear` silent vs TUI confirms), invocation argv leaks.

**`crates/blit-daemon/src/service/core.rs`** — Inventory: `code-daemon.md` lines 30-46 (rpc-handler) + §state-machine. Drift entries: #4 (`detach` field), #25 (Counters always-Some), HTTP/2 keepalive owner decision.

**`crates/blit-daemon/src/service/push/data_plane.rs`** — Inventory: `code-daemon.md` §data-plane + §timeout-or-retry + §rpc-handler. Drift entries: #7 (token rejection codes), #9 (no stall guard), nodelay-errors-silenced, #18 (TarShardExecutor primary path).

**`crates/blit-daemon/src/service/pull.rs` and `pull_sync.rs`** — Inventory: `code-daemon.md` §timeout-or-retry + §safety-check. Drift entries: #9 (no stall guard on accept), constant-redeclaration smell.

**`crates/blit-daemon/src/service/delegated_pull.rs`** — Inventory: `code-daemon.md` §rpc-handler + §safety-check. Drift entries: #4 (`detach` arm), delegation handler ordering (well-aligned).

**`crates/blit-daemon/src/runtime.rs`** — Inventory: `code-daemon.md` §config-load + §default-value. Drift entries: #25 (`--metrics` flag undocumented), #26 (mDNS TXT documented as 2 fields vs 4).

**`crates/blit-core/src/orchestrator/orchestrator.rs`** — Inventory: `code-core-orch.md` §state-machine + §safety-check. Drift entries: #5 (no heartbeat/stall mechanism), ENOTEMPTY errno-66 only macOS, copy_large dry-run-creates-parent.

**`crates/blit-core/src/remote/pull.rs`** — Inventory: `code-core-transfer.md` lines 60-118 (data-plane + state-machine). Drift entries: #12 (`normalize_for_request` empty→`.`), pull_sync deadlock fix (well-aligned), instrumentation hooks.

**`crates/blit-core/src/remote/push/client/mod.rs`** — Inventory: `code-core-transfer.md` §state-machine + §rpc-handler. Drift entries: #10 (`connect_with_timeout` duplicated), helpers `destination_path` empty→`""`.

**`crates/blit-core/src/remote/transfer/sink.rs`** — Inventory: `code-core-transfer.md` §path-handling + §safety-check. Drift entries: finding 34 (canonical-fallback ladder partial warn), `sync_all` divergence between write_file_stream and block-complete.

**`crates/blit-core/src/remote/transfer/data_plane.rs`** — Inventory: `code-core-transfer.md` lines 64-83. Drift entries: control-plane chunk = 1 MiB, double-buffered send-clamp audit-11, instrumentation outbound bytes.

**`crates/blit-core/src/remote/transfer/operation_spec.rs`** — Inventory: `code-core-transfer.md` §safety-check. Drift entries: spec_version fail-closed (well-aligned), R49-F2 require_complete_scan.

**`crates/blit-core/src/remote/transfer/stall_guard.rs`** — Inventory: `code-core-transfer.md` §timeout-or-retry. Drift entries: #9 (only used on pull-receive), #5 (no planner stall detector).

**`crates/blit-core/src/path_safety.rs`** — Inventory: `code-core-misc.md` §safety-check. Drift entries: #12 (canonical chokepoint that helpers bypass), finding 33 (`validate_wire_path` rejects `.`).

**`crates/blit-core/src/path_posix.rs`** — Inventory: `code-core-misc.md` §path-handling. Drift entries: #12 (canonical helper that helpers bypass), `relative_str_to_posix` trailing-separator preservation.

**`crates/blit-core/src/perf_history.rs` / `perf_predictor.rs`** — Inventory: `code-core-misc.md` §persistence + §state-machine. Drift entries: #25 (cap 1 MB vs 1 MiB), schema v2/v3 migration, predictor v3 bump (#dim7).

**`crates/blit-core/src/change_journal/`** — Inventory: `code-core-misc.md` §state-machine + §safety-check. Drift entries: #68 (Linux documented as mtime-only), non-atomic tracker.persist.

**`crates/blit-tui/src/main.rs`** — Inventory: `code-tui-main.md` (10,838 lines). Drift entries: #1 (Dual screen dispatch arm bare), #6 (`prepare_local_transfer` Remote rejection), #14 (`spawn_clear_recent` result discarded), TUI cancel-endpoint `.ok()` silent drop, hardcoded loopback port and `/tmp` path.

**`crates/blit-tui/src/exec_plan.rs`** — Inventory: `code-tui-display.md` §safety-check + §spawn-task. Drift entries: #3 (`detach: false` hardcoded), build_f1_push_execution well-aligned with CLI safety.

**`crates/blit-tui/src/dual_pane.rs`** — Inventory: `code-tui-main.md` + `code-tui-state.md`. Drift entries: #1 (action_labels render-only), #2 (PaneState missing sort/display prefs).

**`crates/blit-tui/src/display_f1.rs`** — Inventory: `code-tui-display.md` §render-or-display + §endpoint-parse. Drift entries: #6 (confirm-detail Err falls through to Local), push verb table omits delegated mirror/move.

**`crates/blit-tui/src/del_request.rs`** — Inventory: `code-tui-display.md` §path-handling + §safety-check. Drift entries: #19 (silent-filter vs CLI bail).

**`proto/blit.proto`** — Inventory: `code-bridge-proto.md` + `plan-wire.md`. Drift entries: #16 (PullSyncHeader removed but Whitepaper still cites), #25 (Counters always-Some hazard), #46 (Pull RPC + ack deprecated only in comments), #4 (detach field), #18 (RDMA reservations).

**`docs/ARCHITECTURE.md`** — Inventory: `plan-wire.md`. Drift entries: #15 (stale auth/TLS section), #68 (Linux change-journal undersells), #69 (PerformanceRecord shape stale).

**`docs/WHITEPAPER.md`** — Inventory: `plan-wire.md`. Drift entries: #16 (§6 references removed PullSyncHeader), #70 (pool.acquire snippet vs inline buffers), §8.4 hardcoded-constants gap (well-aligned with code).

**`docs/DAEMON_CONFIG.md`** — Inventory: `plan-wire.md`. Drift entries: #25 (no `--metrics` flag documented), #26 (TXT record 2 of 4 fields), `blit scan` duplication (#62), motd "to clients" claim (#63).

**`docs/cli/blit.1.md`** — Inventory: `plan-cli.md`. Drift entries: #17 (omits 2 verbs + 8 flags), #43 (6-month-stale date), `--limit`/`--json` OPTIONS gaps, `--workers` missing.

**`docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`** — Inventory: `plan-phases.md`. Drift entries: #4 (`--detach` shipped despite out-of-scope), BlitAuth body still describes removed flow.

**`docs/plan/LOCAL_TRANSFER_HEURISTICS.md`** — Inventory: `plan-perf.md`. Drift entries: #5 (no-staged-rollout claim violated), #5 (10 s stall vs 30 s code), TUNING_WINDOW 50 vs 20.

**`docs/plan/PIPELINE_UNIFICATION.md`** — Inventory: `plan-perf.md`. Drift entries: pipeline-unification-not-shipped, #5 (predictor wired but not for routing decision).

**`docs/plan/TUI_DESIGN.md` and `TUI_REWORK.md`** — Inventory: `plan-tui.md`. Drift entries: #1 (action bar render-only vs M4 acceptance), #2 (TransferDraft missing), #3 (`detach: false`), #4 (no W1-W4 tests).

---

*End of report. 34 high-severity, 64 medium-severity, 51 low-severity / documentation drift findings, 144 total. All findings cross-referenced to specific file+line evidence in the audit's source inventories.*
