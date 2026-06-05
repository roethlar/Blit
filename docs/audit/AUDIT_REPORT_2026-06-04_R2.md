# Blit Codebase + Plan Audit — 2026-06-04 (Revision 2)

**Revision 2** merges Revision 1 (workflow-driven, 30 agents, 92K lines audited) with an
independent review by GPT (4-agent fan-out, working tree including uncommitted files).
GPT's review surfaced 6 HIGH-severity items the R1 audit missed entirely, mostly
because the R1 plan-doc scope omitted `docs/plan/UNIFIED_RECEIVE_PIPELINE.md`, `docs/API.md`,
`docs/plan/README.md`, `docs/plan/REMOTE_TRANSFER_PARITY.md`, `docs/plan/TUI_UX_EXPERT_PROPOSAL.md`,
and a few others, and the R1 code agents leaned toward inventory cataloging rather than
specific bug hunting on hot paths. Every finding below is attributed to its source (R1 / GPT /
both) so the reader can trace provenance.

## TL;DR (revised)

The R1 picture stands: Blit ships a coherent, well-tested 0.1.0 surface, but its plan corpus
has not kept up with what shipped, and TUI rework is the single largest gap. The R2 revision
adds **six new HIGH-severity findings** GPT surfaced:

1. `mirror --relay-via-cli` bypasses the `require_complete_scan` data-loss guard on
   remote→remote mirrors (GPT-1). `move --relay-via-cli` is blocked for this class; `mirror`
   is not.
2. **Esc exits the TUI** at every screen — the operator's universal "back out" instinct kills
   the app instead of stepping back, contradicting `TUI_REWORK.md:60` (GPT-3).
3. The stall guard / idle-timeout story is materially worse than R1's finding #9 suggested:
   `pull.rs:752` gRPC-fallback pulls also lack an idle deadline (GPT-12). The stall guard
   protects ONE of the four receive paths.
4. The "receive-pipeline unification" `UNIFIED_RECEIVE_PIPELINE.md:332` claims complete is
   only complete on the TCP fast path; both gRPC fallbacks still write files/chunks via
   hand-written loops (GPT-13).
5. Delegated transfers emit no live byte progress — daemon `service/delegated_pull.rs:336`
   ships only terminal/manifest events, so the F2/`jobs watch` surface that's supposed to
   show live throughput is silent for the headline remote→remote case (GPT-11).
6. Daemon-side `GetState` hardcodes byte/file counters to zero (`service/core.rs:233`),
   despite the TODO/CHANGELOG claiming byte-level instrumentation shipped (GPT-15).

R1's TUI rework gap is widened by three additional GPT findings: favorites/recents/known-
endpoints data model is absent (GPT-7), fan-out is the old sequential UI rather than the
per-destination batch table (GPT-9), and read-only/capability context is not preserved into
the dual-pane child listings so action labels are unconditional (GPT-10). R1's
ARCHITECTURE.md auth/TLS drift (R1-#15) is corroborated by GPT-19 and joined by GPT-20
(`docs/plan/README.md` still names the shipped 0.1.0 release plan as the live source of
truth), GPT-21 (`README.md:24` overstates 10+ Gbps as achieved despite the release-plan
benchmark deferral), and GPT-23 (REVIEW.md / .review/findings / TODO state are
mutually inconsistent — verified rows also listed open).

R1's broad cross-cutting findings (path encoding fork, Status code stripping, four flavors of
unreadable-paths refusal, three `connect_with_timeout` copies, prompt-phrasing fork) GPT did
not enumerate. They remain valid; this report keeps them.

## Method

Two parallel independent audits:

- **R1** (this workflow, 2026-06-04): 30 agents across 4 phases (plan inventory, code
  inventory, drift detection per plan cluster, cross-cutting inconsistency by dimension).
  9,140 lines of plan documentation + 83,089 lines of code read end-to-end. Output in
  `docs/audit/AUDIT_REPORT_2026-06-04.md`.

- **GPT review** (independent, ~14m runtime): 4-agent fan-out across TUI, transfer/proto,
  CLI/admin, and plan-corpus drift; cross-checked highest-impact items locally against the
  working tree (including uncommitted files — relevant because `dual_pane.rs`,
  `screens/dual_pane.rs`, and several plan docs were untracked).

This R2 file merges both. Each finding cites its source as **[R1]**, **[GPT]**, or **[R1+GPT]**.
The numbering restarts; cross-references to the original R1 number are preserved in parens.

## High-severity findings

The order is approximate priority — start at the top.

### H1. `mirror --relay-via-cli` bypasses the `require_complete_scan` data-loss guard

**Source**: GPT
**Class**: drift / data-loss-class bug
**Where**:
- `crates/blit-cli/src/transfers/mod.rs:230` — `copy`/`mirror --relay-via-cli` allows
  remote→remote
- `crates/blit-core/src/remote/transfer/source.rs:233` — remote source scanner ignores
  `unreadable_paths`
- `crates/blit-core/src/remote/push/client/mod.rs:848` — push derives `scan_complete` from
  the (always-empty) `unreadable_paths` list
**Plan/canonical**: The `require_complete_scan` safety is the R49-F2 / R59 #1 data-loss
closure. `move --relay-via-cli` is correctly blocked from this whole class (R1's "solidly
aligned" #11 confirms the `move` reject-gate); `mirror --relay-via-cli` is not.
**Code does**: A mirror via CLI relay produces a push handshake where `scan_complete` is
True even when the remote source had unreadable paths, because the relay-source scanner
never populates the list the push handshake derives from.
**Why this matters**: The whole point of `require_complete_scan` is preventing a mirror
purge from running against an incomplete source view. The exact data-loss class the safety
exists to prevent is reachable through the relay path.
**Remediation**: Plumb `unreadable_paths` through the remote source scanner (or, simpler,
refuse `mirror --relay-via-cli` symmetric with `move`).

### H2. Esc exits the TUI at every screen — universal back-out instinct is broken

**Source**: GPT
**Class**: drift / UX
**Where**:
- `crates/blit-tui/src/main.rs:5141` — global quit dispatch maps Esc before per-screen
  back-out behavior
- `docs/plan/TUI_REWORK.md:60` — spec says Esc backs out
**Code does**: Pressing Esc on any screen quits the application. The dual-pane shell has no
back-step behavior at all.
**Why this matters**: Every desktop / TUI operator's reflex is Esc to back out. In Blit
that reflex destroys the session. Combined with the dual-pane action bar being render-only
(H4 below) and Esc being the only way to "leave" a state, the UX failure is total.
**Remediation**: Wire Esc to a per-screen back action; reserve quit to `q` or `Ctrl-C`. Add
a regression test pinning Esc behavior on each screen.

### H3. Stall guard misses three of four data-plane paths — daemon push-receive, daemon pull-data-plane write progress, and CLI gRPC-fallback pulls all silent on stuck peers

**Source**: R1+GPT (R1 #9 + GPT-12). Wording revised 2026-06-05 per h3b
implementation review: R1's original "daemon-side pull-data-plane accepts"
phrasing was imprecise — the accept and token-read phases on those paths are
already bounded by `PULL_ACCEPT_TIMEOUT` / `PULL_TOKEN_TIMEOUT`. The missing
guard on the daemon pull side is **write progress after token acceptance**,
because the daemon is the sender on a pull and the stall surface is TCP write
backpressure from a slow/wedged reader, not a stuck receive.
**Class**: inconsistency (timeouts) — DoS-class hardening gap
**Where**:
- Covered by guard: `crates/blit-core/src/remote/pull.rs:1712-1720` (CLI pull-receive TCP
  wraps in `StallGuard`) [R1+GPT]
- **Missing** on: `crates/blit-daemon/src/service/push/data_plane.rs:213-242` (daemon
  push-receive socket) [R1 #9] — closed by **audit-h3a** (master `dd51a1c`).
- **Missing** on: `crates/blit-daemon/src/service/pull.rs:743` and
  `pull_sync.rs:641, 765` (daemon pull-data-plane **write progress after token
  acceptance** — the daemon writes to the puller and a stalled reader fills
  the kernel send buffer indefinitely) [R1 #9] — closed by **audit-h3b** via
  a write-side `StallGuardWriter` wired inside `DataPlaneSession`.
- **Missing** on: `crates/blit-core/src/remote/pull.rs:752` (CLI gRPC-fallback pull awaits
  messages without idle deadline) [GPT-12] — pending audit-h3c.
**Plan/canonical**: Owner decision `audit-1c` (memory `feedback_port_cli_safety_guards`):
"no-bytes-for-30 s." Extended 2026-06-05 by owner to apply symmetrically to
write progress on sender paths (audit-h3b).
**Why this matters**: Three independent attack/failure surfaces — a hostile push client
pinning a daemon push worker (h3a), a slow/wedged puller pinning a daemon pull
worker via TCP write backpressure (h3b), and a silent gRPC fallback going
unbounded (h3c). The reviewed claim that "all pulls abort on no-byte stalls"
covered only the CLI receive side; daemon-side coverage shipped piecewise
across h3a/h3b.
**Remediation status**: `TRANSFER_STALL_TIMEOUT` constant hoisted (audit-h3a).
Daemon push-receive wrapped in `StallGuard` (audit-h3a). Daemon pull-data-plane
write progress guarded by `StallGuardWriter` inside `DataPlaneSession`
(audit-h3b). gRPC-fallback path (audit-h3c) still pending — it sits below
`tonic::Streaming<T>` rather than `AsyncRead`/`AsyncWrite` and needs a
different mechanism (per-message `tokio::time::timeout` or a `Stream` adapter).

### H4. TUI dual-pane action bar is render-only — Copy/Mirror/Move/Delete/Verify do nothing

**Source**: R1+GPT (R1 #1 + GPT-2)
**Class**: drift
**Where**:
- `crates/blit-tui/src/dual_pane.rs:462-472` (`action_labels`) returns
  `["Copy -> {dest}", "Mirror -> {dest}", "Move -> {dest}", "Delete", "Verify", "More"]`
- `crates/blit-tui/src/main.rs:2219-2234` (Dual screen dispatch arm) handles only
  navigation actions; none of `TransferCopy`/`Mirror`/`Move`, `F3DeleteBegin`, or verify
- `crates/blit-tui/src/screens/dual_pane.rs:183-206, 246-272` (renderer + render test) —
  test asserts strings render, no test asserts a transfer launches
**Plan**: `TUI_REWORK.md` §1 + §3 Principle 2: "Transfers are launched from visible
action buttons, not from hidden memorized letter commands"; §9 M4 acceptance: "Copy works
local→local, local→remote, remote→local, remote→remote without typed path fields." Dual
is the *default screen* (`main.rs:105` `default_value_t = ScreenArg::Dual`).
**Why this matters**: Fresh-install operator opens Blit on the rework's default screen,
sees an action bar, presses Copy, nothing happens. Productive transfer paths survive only
on F1's `t` trigger modal, F3's `p`/`m`/`v` modals, and F4's verify-form text inputs —
exactly the letter-command + free-text model the rework rejected.
**Remediation**: Wire `UserAction::TransferCopy`/`Mirror`/`Move`/`Delete`/`Verify` into the
Dual dispatch arm using active-pane selection + inactive-pane path. Build a
`TransferDraft` (which doesn't yet exist — see H5). Add W1-W4 workflow tests with fake
providers. Until landed, flip the default screen back to F1.

### H5. Plan-mandated UI model types do not exist (`TransferDraft`, `BatchTransferDraft`, `BrowseProvider`, `LocationProvider`)

**Source**: R1+GPT (R1 #2 + GPT-8)
**Class**: drift / structural
**Where**: `crates/blit-tui/src/dual_pane.rs:172-183` (current `PaneState` shape) +
workspace-wide absence of `TransferDraft` / `BatchTransferDraft` / `TransferAction` /
`TransferOptions` / `BatchDestination` / `BrowseProvider` / `LocationProvider`. Verified by
`grep -rn` returning zero matches across `crates/`.
**Plan**: `TUI_REWORK.md` §8.1 specifies `enum Location`, `struct PaneState { ...
path_editor: PathEditorState, ... }`, `struct BrowserEntry`, `struct TransferDraft`,
`struct BatchTransferDraft`. §8.2 specifies `trait BrowseProvider`, `LocalBrowseProvider`,
`RemoteBrowseProvider`. §10 testing contract: "Assert the resulting `TransferDraft`/
`BatchTransferDraft`, not only rendered text."
**Code does**: Current `PaneState` has `path_editor: String` (not `PathEditorState`), no
sort/display-preferences field, no trait abstraction; the browse pattern is per-callsite
`spawn_blocking` + ad-hoc reply tagging.
**Why this matters**: Without these types the §10 testing contract is structurally
impossible — there is no model boundary to assert against. Concretely makes "Pick-not-Type"
unverifiable by CI. Also blocks M4-M8 cleanly.
**Remediation**: Introduce §8.1 types + §8.2 trait + `LocalBrowseProvider` /
`RemoteBrowseProvider` before continuing M4-M8. Move existing per-pane fetch logic behind
the trait so existing main.rs spawn paths become provider calls.

### H6. TUI delegated transfers ship with `detach: false` — closing the TUI cancels every delegated transfer

**Source**: R1 #3
**Class**: drift
**Where**: `crates/blit-tui/src/exec_plan.rs:91-108` — hardcoded `detach: false` with a
comment "Always attached; detached/F2-visible delegation is a follow-up"
**Plan**: TUI_DESIGN §5.2: "TUI uses `detach=true` on every transfer it initiates against
a remote→remote pair"; §6.5 / §10 / §12: "Daemon-owned transfer lifecycle for remote→remote
(delegated) transfers when `detach=true`."
**Code does**: The single place that should set `detach=true` doesn't. Meanwhile the wire
surface (proto field), daemon-side detach lifecycle (`service/core.rs:1314-1320`), CLI's
`--detach`, `jobs watch`, and `CancelJob` all shipped.
**Why this matters**: Closing the TUI cancels every delegated remote→remote it initiated.
Breaks TUI_DESIGN §3's closing promise "transfers survive their initiator disconnecting."
**Remediation**: Flip to `detach: true` for delegated paths, surface a banner on the
trigger modal for local-endpoint transfers per §5.2, add a regression test pinning the spec
field.

### H7. Delegated remote→remote transfers emit no live byte progress

**Source**: GPT-11
**Class**: drift
**Where**:
- `crates/blit-daemon/src/service/delegated_pull.rs:336` — daemon emits only terminal /
  manifest / summary events; no `BytesProgress` stream messages
- `proto/blit.proto` defines the progress field shape; consumer (F2 / `jobs watch`) renders
  whatever arrives
**Plan**: M-Jobs / Milestone C wording: byte-level progress shipped, surfaced through
`Subscribe`/`GetState` for any active transfer.
**Code does**: For delegated remote→remote (the headline TUI use case), only
manifest-level events fire. F2 active table shows the transfer started and finished; it
shows zero bytes in between.
**Why this matters**: The headline TUI surface (real-time fan-out monitoring) is silent
on its headline workload. Combined with H6 (TUI ships `detach: false` so delegated
transfers die when the TUI quits), the F2 "watching transfers initiated by anyone on the
network" promise is doubly broken.
**Remediation**: Stream `BytesProgress` from `delegated_pull` for both `--detach`=true
(daemon-owned) and false paths. Add an integration test asserting at least one progress
event lands between Started and Done.

### H8. `Subscribe`/`GetState` byte/file counters hardcoded to zero despite shipped-progress claim

**Source**: GPT-15
**Class**: drift
**Where**: `crates/blit-daemon/src/service/core.rs:233` — `ActiveJob.bytes_transferred` /
`files_transferred` written as 0 in `to_proto`
**Plan**: TODO marks byte-level progress shipped; CHANGELOG §"Daemon" claims live byte
counters via `Subscribe`.
**Code does**: The `to_proto` serializer used by both `GetState` and `Subscribe` snapshots
returns 0 for both fields, even on transfers that have moved many bytes. Related to but
distinct from R1 #25 (Counters always-Some when `--metrics` disabled): #25 is about per-RPC
totals; this is about per-transfer live progress.
**Why this matters**: Anything that reads `GetState` to render a transfer (TUI F2, `jobs
list --json`, the Prometheus bridge) shows 0/0 for live byte/file counts. The bridge
already works around this by omitting counter series (R1 #47).
**Remediation**: Plumb the `BytesProgress` updates that the data-plane already produces
into the `ActiveJob` row and have `to_proto` serialize current values. Same code path that
H7 wants for delegated.

### H9. `--detach` shipped despite REMOTE_REMOTE_DELEGATION_PLAN §9 calling it explicit out-of-scope future work

**Source**: R1 #4
**Class**: drift
**Where**: `crates/blit-cli/src/cli.rs:325-335` · `crates/blit-cli/src/transfers/mod.rs:161-178,
255-269` · `crates/blit-cli/src/transfers/remote_remote_direct.rs:126-189` ·
`crates/blit-daemon/src/service/core.rs:1314-1320` · `crates/blit-daemon/src/active_jobs.rs:59`
**Plan**: `REMOTE_REMOTE_DELEGATION_PLAN.md` §9, §4.2 step 12, §7: "out of scope, track as
separate future feature" — said three times. §4.2 step 12: "Document that delegated pulls
are CLI-session-bound; `--detach` is out of scope (§9)."
**Code does**: Fully wired: CLI flag, gate enforcement (rejects on push/pull/local), three
CLI dispatch sites including `--json` detach envelope, daemon-side `if !detach` guard,
tests covering all rejection paths.
**Why this matters**: The plan emphatically calls this future work in three places; the
feature is the load-bearing surface behind M-Jobs and a primary TUI design point. Anyone
reading the delegation plan to understand 0.1.0 scope is misled, and the "session-bound
only" invariant is no longer the contract.
**Remediation**: Update REMOTE_REMOTE_DELEGATION_PLAN §9 and §4.2 step 12 to record that
`--detach` shipped (with code-site references). Cross-link to TUI_DESIGN M-Jobs.

### H10. Planner heartbeat / 10 s stall detector / streaming planner — never built under any name

**Source**: R1 #5
**Class**: drift
**Where**: `crates/blit-core/src/orchestrator/orchestrator.rs:540-574` (synchronous
scan → plan → pipeline, no idle timeout on `header_rx.recv().await` or `scan_handle.await`) ·
`crates/blit-core/src/remote/transfer/stall_guard.rs:29` (only `PULL_STALL_TIMEOUT = 30s`
exists)
**Plan**: `greenfield_plan_v6.md` §1.1 v5: "Incremental planner that emits work every
heartbeat (1 s default, 500 ms when workers are starved). 10 s stall detector (planner
*and* workers idle) with precise error reporting." `WORKFLOW_PHASE_2.md` reaffirms 10 s.
`LOCAL_TRANSFER_HEURISTICS.md` header: "no staged rollout—every mechanism described here
will ship together once complete."
**Code does**: No `PlannerEvent`, no `stream_local_plan`, no `drive_planner_events`, no
`HEARTBEAT_INTERVAL`, no `starved` cadence reduction. Grep returns zero. The orchestrator
awaits scan headers synchronously with no idle timeout. The only stall guard is 30 s on
the pull data-plane TCP socket. Push data plane has none.
**Why this matters**: The FAST principle's load-bearing mechanism is the predictor +
heartbeat. The predictor exists but is *observability only*; it does not enforce or
measure a ≤1 s latency invariant. A stuck network FS on local scan wedges the orchestrator
indefinitely.
**Remediation**: Either (a) revise WORKFLOW_PHASE_2 + LOCAL_TRANSFER_HEURISTICS +
greenfield_v6 §1.1 to describe the synchronous orchestrator + 30 s pull-only stall guard
as what shipped, OR (b) actually build the streaming planner. Conservative path: update
plan to match code, then add a `scan_handle.await` outer timeout.

### H11. F1 confirm-detail silently treats endpoint parse Err as Local — explicitly forbidden by `feedback_endpoint_parse_err`

**Source**: R1 #6
**Class**: inconsistency (endpoint parsing) — data-loss-adjacent
**Where**: `crates/blit-tui/src/display_f1.rs:46-54` — confirm_detail for `PullKind::Move`
uses `match parse_transfer_endpoint(source) { Ok(Endpoint::Remote(_)) => "deletes the
remote source", _ => "deletes the local source" }`. Err is silently classified as local.
**Plan**: Project memory `feedback_endpoint_parse_err.md`: "4 buckets: module/root=remote,
bare-discovery & local=local, Err=reject. Reopened d-61, d-68 ×3."
**Code does**: `plan_f1_trigger` correctly returns `TriggerOutcome::Rejected("invalid
source: {src}")` on Err — but the confirm-detail renderer silently falls through to local.
The gate normally blocks unparseable sources from reaching confirm, but the lie remains in
the renderer for any future refactor that loosens the gate.
**Why this matters**: Exactly the pattern the reviewer reopened four times. Confirm
prompt's "y" is supposed to mean "yes, delete the side I said I'd delete." On a parse Err
it could lie about which side gets erased — data-loss-adjacent UI miscommunication.
**Remediation**: Add `unreachable!` (paired with `debug_assert` in `plan_f1_trigger`) for
the Err arm, or route confirm-detail through a shared classifier returning
`Result<DeleteVictim, _>` that rejects Err at the gate.

### H12. Admin RPC clients erase Status code; `is_retryable` therefore never fires on transport-class remote errors

**Source**: R1 #8
**Class**: inconsistency (error handling)
**Where**: `crates/blit-app/src/admin/{rm,du,df,list_modules,ls,find}.rs` (drop code via
`eyre::eyre!(status.message().to_string())`) · `crates/blit-app/src/admin/jobs.rs:91-95,
114-119` (preserve code) · `crates/blit-app/src/transfers/retry.rs:27-46` (`is_retryable`
walks for `std::io::Error` source only)
**Why this matters**: `run_with_retries` is silently a no-op for `Code::Unavailable` and
`Code::DeadlineExceeded` — exactly the codes a flaky daemon emits and exactly what the
retry feature is for. Six admin verbs vs two jobs verbs: same daemon condition shows two
error shapes depending on which verb the user ran.
**Remediation**: Shared helper `fn status_to_eyre(rpc_name: &str, status: Status) ->
eyre::Report` used everywhere. Extend `is_retryable` to walk `eyre::chain()` for
`tonic::Status` and return true on `Unavailable`/`DeadlineExceeded`/`Aborted`.

### H13. Unreadable-paths refusal message exists in four flavors across CLI move, TUI move, daemon push, daemon pull_sync

**Source**: R1 #11
**Class**: inconsistency (error messages) — data-loss-class guard
**Where**:
- CLI `transfers/mod.rs:463-479` (quotes first 5 paths, long explanation)
- TUI `main.rs:4167-4179` (quotes first 3, shorter)
- Daemon `pull_sync.rs:143-160` (quotes first 5, daemon-side phrasing)
- Daemon `push/control.rs:328-332` (quotes **0**, generic message)
**Why this matters**: This guard is the R47-F4 / R49 / R59 #1 data-loss closure. Different
message per dispatch path means operators have to mentally translate between TUI banner /
CLI text / daemon stderr to recognize the same failure mode. Worst case (daemon push) gives
no preview paths at all.
**Remediation**: `format_incomplete_scan_refusal(operation, unreadable, side)` in
`blit-app` (or `blit-core`); 5-path preview convention.

### H14. Push verb table omits delegated-mirror/delegated-move labels — operator can't tell delegated mirror from local mirror

**Source**: R1 #36 (raising severity)
**Class**: inconsistency (display) — data-loss-adjacent
**Where**: `crates/blit-tui/src/display_f1.rs:121-137` — `(true, Copy) → "delegating"` but
`(true, Mirror)` and `(true, Move)` fall through to "mirroring"/"moving"
**Why this matters**: A delegated mirror (daemon→daemon, bytes don't touch operator's host)
reads identically in the F1 footer to a local mirror push from this host. Operator cannot
tell where the bytes flow during a destructive operation. Reclassified to HIGH because of
the destructive-action context.
**Remediation**: Complete the (delegated, kind) table.

### H15. Stale architecture/whitepaper narrative misleads readers about removed security model and current wire shape

**Source**: R1+GPT (R1 #15 + R1 #16 + GPT-19)
**Class**: drift
**Where**:
- `docs/ARCHITECTURE.md:444-454` narrates removed auth/TLS as the current/planned state
- `docs/WHITEPAPER.md:336-338` still narrates `PullSyncHeader` (removed from wire)
- `README.md` + `CHANGELOG.md` still mention token auth / TLS placeholders [GPT-19]
- `proto/blit.proto:110-116, 409, 652-657` carries the tombstones; `docs/DAEMON_CONFIG.md:
  267-274` explicitly: "no daemon authentication … not on the roadmap"
**Why this matters**: ARCHITECTURE.md is the front door for new readers. Whitepaper §6 is
the protocol's authoritative narrative. Both actively mislead about wire shape and security
posture, while DAEMON_CONFIG.md says the opposite.
**Remediation**: Rewrite ARCHITECTURE §"Security Considerations" + §"Planned Enhancements"
to match the current model (operator network controls + per-transfer data-plane tokens;
no built-in TLS/ACL/audit on roadmap). Rewrite WHITEPAPER §6 step 2 to use
`TransferOperationSpec` not `PullSyncHeader`.

### H16. Manpage omits two shipped verbs (`jobs`, `check`) and eight data-loss-relevant transfer flags

**Source**: R1 #17 + GPT-17
**Class**: drift
**Where**:
- `docs/cli/blit.1.md:8-24, 102-132` — SYNOPSIS lists 14 verbs (omits `jobs`, `check`);
  Transfer Options lists 8 flags
- Omitted flags: `--null`, `--detach`, `--delete-scope`, `--force`, `--ignore-existing`,
  filter suite (`--exclude`/`--include`/`--files-from`/`--min-size`/`--max-size`/
  `--min-age`/`--max-age`), `--retry`/`--wait`, `--json` for transfers
- `crates/blit-cli/src/cli.rs:76, 82-86, 204-364` — verb + flag inventory in code
- `crates/blit-cli/src/transfers/mod.rs:131-406` — reject-gates cite flags the manpage
  doesn't mention
**Why this matters**: User reading the manpage cannot know `--null`, `--exclude`, or
`--delete-scope all` exist, won't know `move --exclude` is forbidden, won't know `blit
jobs watch` is the documented way to poll a detached transfer. Manpage dated 2025-11-21
vs v0.1.0 (2026-05-31) — 6-month stale date stamp lies about freshness.
**Remediation**: Regenerate manpage from clap. Consider build-script auto-generation.

### H17. TarShardExecutor on TCP push hot path despite POST_REVIEW §1.2 marking it "gRPC-fallback only"

**Source**: R1 #18
**Class**: drift / internal contradiction
**Where**: `crates/blit-daemon/src/service/push/data_plane.rs:327` (constructor on the
primary receive path) vs `data_plane.rs:620-647` (docstring + `#[allow(dead_code)]` claim
"Currently only used by the gRPC fallback path"). The two statements in the same file
directly contradict each other.
**Plan**: `POST_REVIEW_FIXES.md` §1.2 closure: "Phase 5 of the receive-pipeline unification
… `TarShardExecutor` is now used **only** by the gRPC fallback path."
**Why this matters**: Future contributors may treat the executor as cold gRPC-only code
and remove it without realizing it's serving the TCP push hot path. Either the deferral or
the unification is wrong.
**Remediation**: Either complete the unification (route all tar-shard receive through
`FsTransferSink::write_tar_shard_payload`, delete TarShardExecutor) or update §1.2 closure
+ docstring to describe its actual current role.

### H18. Receive-pipeline unification incomplete on both gRPC fallbacks — hand-written receive loops still live in two places

**Source**: GPT-13 (extends H17)
**Class**: drift
**Where**:
- `docs/plan/UNIFIED_RECEIVE_PIPELINE.md:332` — claims hand-written receive loops should be
  gone
- Push gRPC fallback receive — writes files directly, not via the unified pipeline
- Pull gRPC fallback receive — writes chunks directly
- (My R1 audit missed UNIFIED_RECEIVE_PIPELINE.md entirely — it wasn't in any plan cluster)
**Why this matters**: The unification was the largest single architectural decision after
the 0.1.0 release plan. It is materially less complete than the plan claims.
**Remediation**: Either (a) route both gRPC fallbacks through `execute_receive_pipeline`
(symmetric with TCP push/pull), OR (b) update UNIFIED_RECEIVE_PIPELINE.md to describe what
actually shipped (TCP fast path unified, gRPC fallback still ad-hoc).

### H19. Connect-with-timeout reimplemented three different ways

**Source**: R1 #10
**Class**: inconsistency (timeouts)
**Where**: `crates/blit-app/src/client.rs:24, 37-46` (canonical, exported) ·
`crates/blit-core/src/remote/pull.rs:230-248` (reimplemented inline with hardcoded 30 s) ·
`crates/blit-core/src/remote/push/client/mod.rs:298-317` (third copy with same hardcoded
constant)
**Why this matters**: Drift surface for any future timeout bump. `feedback_server_await_
timeouts` exists precisely because someone missed one of these once.
**Remediation**: Promote `connect_with_timeout` + constant to
`blit-core::remote::client`; rewrite both pull and push client constructors to use it.

### H20. `--metrics`-disabled token rejection uses two different gRPC Status codes across the four data-plane paths

**Source**: R1 #7
**Class**: inconsistency (error handling)
**Where**:
- `service/push/data_plane.rs:185` and `service/pull.rs:740` — `Status::permission_denied`
- `service/pull_sync.rs:632, 754` — `Status::unauthenticated`
- Message text also drifts ("data plane token" vs "pull data plane token")
**Why this matters**: Client retry logic and metrics often branch on `status.code()`. A
future client that maps `Unauthenticated → re-handshake, PermissionDenied → abort` sees
inconsistent behavior depending on which data-plane it hit.
**Remediation**: One helper `fn reject_invalid_token() -> Status` returning one code
(`Unauthenticated`) and one literal string used by all four sites.

### H21. Two ad-hoc `Path → wire` helpers bypass the canonical chokepoint AND disagree on empty-path encoding

**Source**: R1 #12
**Class**: inconsistency (path handling)
**Where**: `crates/blit-core/src/remote/pull.rs:1795-1804` (`normalize_for_request`, empty
→ `"."`) · `crates/blit-app/src/transfers/remote.rs:638-647` (second identical
`normalize_for_request`, empty → `"."`) · `crates/blit-core/src/remote/push/client/
helpers.rs:262-271` (`destination_path`, empty → `""`) · 5 canonical delegators via
`path_posix::relative_path_to_posix` (empty → `""`)
**Why this matters**: The receive sinks' empty-rel single-file guards (designed to avoid
`root.join("")` ENOTDIR) silently fail to fire when the renderer emits `"."`. Push sends
`""`; pull sends `"."` for the same "module root" intent.
**Remediation**: Replace both `normalize_for_request` bodies and `destination_path` body
with `relative_path_to_posix(path)`.

### H22. `is_deletable_remote_path` silently filters in TUI batch; CLI `blit rm` bails

**Source**: R1 #19
**Class**: inconsistency (endpoint classification)
**Where**: `crates/blit-cli/src/rm.rs:24-43` (CLI bails on module-root with module-name-
qualified error) vs `crates/blit-tui/src/del_request.rs:31-57` (`build_delete_request`
silently filters non-deletable entries; returns `None` only when EVERYTHING is filtered)
**Why this matters**: The very feature that's supposed to prevent deleting an entire
module (`feedback-port-cli-safety-guards` rule) is invisibly bypassed by the TUI's silent
filter. Operators get a misleading success signal.
**Remediation**: Have `build_delete_request` return `(Vec<Endpoint>, Vec<SkipReason>)` so
the TUI can banner "deleted N, skipped M (module root)". Visually disable module-root rows
in F3.

### H23. `BLIT_TUI_INPUT_TRACE` and `BLIT_TEST_COUNTER_FILE` env vars violate "no env vars for config" invariant; documented `BLIT_FORCE_GRPC_DATA` / `BLIT_DISABLE_LOCAL_TELEMETRY` overrides do not exist

**Source**: R1 #20 + #21 (merged)
**Class**: drift
**Where**:
- Violations: `crates/blit-tui/src/main.rs:4911-4921` (BLIT_TUI_INPUT_TRACE →
  hardcoded `/tmp/blit-tui-input.log`); `crates/blit-core/src/remote/instrumentation.rs:
  10-22` (BLIT_TEST_COUNTER_FILE consumed by production bench script
  `scripts/bench_remote_remote.sh:81`)
- Missing: greenfield_plan_v6.md §1.2 line 161, §1.3 line 168 promise both
  `BLIT_FORCE_GRPC_DATA=1` and `BLIT_DISABLE_LOCAL_TELEMETRY=1`; grep across the workspace
  returns ZERO matches for either
**Why this matters**: The "no env vars" invariant is absolute. The shipped binary reads
two env vars; the bench-script coupling means the violation has propagated into operator-
facing tooling. Meanwhile two documented escape hatches don't exist — operator setting
them observes no effect with no warning. The project's relationship to env vars is
incoherent.
**Remediation**: Pick a policy. Either an explicit carve-out in the plan ("env vars
permitted for test instrumentation only") + implement the two documented overrides, or
strike both documented overrides from the plan + replace BLIT_TUI_INPUT_TRACE with a
TUI config flag + gate BLIT_TEST_COUNTER_FILE behind `#[cfg(test)]`.

## Medium-severity findings

### M1. CancelJob `Unauthorized` outcome is not modeled by the CLI / app consumer

**Source**: GPT-4
**Class**: drift / structural gap
**Where**: `crates/blit-daemon/src/active_jobs.rs:182` defines `CancelOutcome::Unauthorized`
and maps to `PermissionDenied` vs `crates/blit-app/src/admin/jobs.rs:39`
`CancelJobOutcome` enum only models `Cancelled` / `NotFound` / `Unsupported`
**Why this matters**: A `PermissionDenied` cancel response would be classified as a
generic RPC error rather than the documented `Unauthorized` outcome the daemon defines.
Currently latent (no auth in shipped daemon) but the type model is incoherent and will
break the day any auth path lands.
**Remediation**: Add `Unauthorized` to the app-layer enum + map it in `jobs::cancel`. Add
a test that uses a mock daemon returning `PermissionDenied`.

### M2. TUI favorites / recents / known-endpoints schema is absent

**Source**: GPT-7
**Class**: drift
**Where**: `crates/blit-tui/src/main.rs:2928` populates cwd / Home / root + limited
discovered remotes; no favorites / location-recents schema; no config-file backing
**Plan**: TUI_REWORK §119 requires configured favorites, recents, discovered remotes, and
known endpoints in the picker.
**Why this matters**: Without this the picker UX (the rework's flagship surface) cannot
reach the spec.
**Remediation**: Define `[favorites] paths = [...]` in `tui.toml`, plus a separate
`favorites.jsonl` / `picker_recents.jsonl` for runtime recents (mirroring the
`recents.jsonl` separation). Add a small "Pin to Favorites" action in the picker.

### M3. Fan-out is still the old sequential UI rather than the batch table

**Source**: GPT-9
**Class**: drift
**Where**: Current batch pull advances one remote source at a time (`crates/blit-tui/src/
main.rs` batch pull loop)
**Plan**: TUI_REWORK §167 requires a batch table with per-destination rows showing each
target's status concurrently.
**Why this matters**: Multi-destination push (the workflow you flagged earlier this
session — "Sync a folder from local to multiple remotes") still presents as one transfer
at a time, not the spec'd batch view.
**Remediation**: Land the batch-table renderer + per-destination progress aggregation.
Likely co-requires H7 (delegated progress events) and H8 (per-transfer counter
serialization) to feed the rows.

### M4. Read-only / capability context is lost on descent — dual-pane action labels are unconditional

**Source**: GPT-10
**Class**: drift
**Where**: `crates/blit-tui/src/dual_pane.rs:462` action labels are unconditional; module
`read_only` context from the daemon's `ListModules` response is not preserved into child
listings
**Why this matters**: The Copy/Mirror/Move buttons on the rework's default screen offer
themselves even when the active pane is a read-only module. Operator gets misleading
affordances.
**Remediation**: Thread a `Capabilities { read_only: bool, supports_delete: bool, ... }`
along with each `BrowserEntry`; gate `action_labels` on it. Probably easiest after H5
(provider trait) lands.

### M5. Delegated diagnostics report control-plane address as data-plane endpoint

**Source**: GPT-14
**Class**: drift / observability bug
**Where**: `crates/blit-daemon/src/service/delegated_pull.rs:305` — `Started` event sets
`source_data_plane_endpoint` to the resolved source control-plane address rather than the
observed data-plane TCP endpoint or "grpc-fallback" sentinel
**Plan / proto**: The field documents itself as "observed data-plane TCP endpoint or grpc-
fallback sentinel."
**Why this matters**: Anyone reading the diagnostic stream (TUI F2 detail, `jobs watch
--json`) sees a value labeled as data-plane that is actually control-plane. Misleads
troubleshooting.
**Remediation**: Capture the actual data-plane address inside the delegated pull worker
and forward it on the Started event.

### M6. README overstates 10+ Gbps as achieved performance

**Source**: GPT-21
**Class**: drift / marketing accuracy
**Where**: `README.md:24` claims 10+ Gbps transfers; `RELEASE_PLAN_v2_2026-05-04.md` §2.6
explicitly defers benchmark capture to 0.1.1 (hardware-bound)
**Why this matters**: Top-of-README claim contradicts the released plan's own statement of
what shipped. A 10 GbE benchmark is a falsifiable claim and operators evaluating Blit will
read this.
**Remediation**: Soften to "designed for 10 GbE links; benchmarks in 0.1.1" or attach a
data point only after §2.6 closes.

### M7. TUI `prepare_local_transfer` rejects Remote with "use the CLI" — but F1 accepts remote dst

**Source**: R1 #22
**Class**: inconsistency (endpoint classification)
**Where**: `crates/blit-tui/src/main.rs:4041-4062` (F4 rejects Remote with "F4 transfers
only support local→local paths; use the CLI for remote endpoints") vs `main.rs:3669-3725`
(F1 accepts Local src → Remote dst)
**Why this matters**: F4 points the operator at the CLI for a feature the TUI does
support on F1.
**Remediation**: F4 dispatches through the same router as F1, or its error message
explicitly routes to F1 not the CLI.

### M8. Admin verb endpoint parsing uses three patterns

**Source**: R1 #23
**Class**: inconsistency (endpoint parsing)
**Where**: `parse_endpoint_or_local` (rm/df/du/find/ls/completions), `RemoteEndpoint::parse`
direct (list-modules, jobs list/cancel/watch), `parse_transfer_endpoint` (strict —
transfers + diagnostics dump)
**Remediation**: Centralize via `parse_endpoint_or_local` + `Endpoint::require_remote() ->
Result<RemoteEndpoint>`.

### M9. `extract_module_and_path` (rm) vs `module_and_rel_path` (others) — byte-identical bodies, divergent error strings

**Source**: R1 #24
**Where**: `blit-app/src/admin/rm.rs:48-56` vs `blit-app/src/endpoints.rs:175-183, 86-93,
96-103`
**Remediation**: One helper taking a role label parameter.

### M10. `--metrics` flag absent from DAEMON_CONFIG.md; `Counters` always-Some footgun

**Source**: R1 #25
**Where**: `crates/blit-daemon/src/runtime.rs:104-109` defines `--metrics` ·
`crates/blit-daemon/src/service/core.rs:1072-1078` always publishes `Counters` regardless ·
`docs/DAEMON_CONFIG.md:280-293` omits the flag entirely · `crates/blit-prometheus-bridge/
src/metrics.rs:38-96` works around it by omitting counter series
**Plan/memory**: `feedback_getstate_counters_zero` documents the hazard.
**Remediation**: Document `--metrics`. Either make `counters: Option<Counters>` actually
`None` when metrics disabled, or add `metrics_enabled: bool` sibling field.

### M11. mDNS TXT record carries 4 fields but DAEMON_CONFIG.md documents 2

**Source**: R1 #26 + GPT-22
**Where**: `crates/blit-core/src/mdns.rs:140-156` advertises `version`, `modules`,
`module_count`, `delegation_enabled` · `docs/DAEMON_CONFIG.md:530-532` lists only `version`
and `modules`
**Remediation**: Add `module_count` (with truncation rationale) and `delegation_enabled`
to DAEMON_CONFIG.md.

### M12. TUI strips `Status::code()` on stream errors

**Source**: R1 #27
**Where**: `crates/blit-tui/src/main.rs:5717-5720` forwards Status as `format!("stream:
{}", status.message())`
**Remediation**: Format as `"stream: {code}: {message}"`.

### M13. `MirrorMode::Unspecified|Off` produces different behavior on push vs pull_sync

**Source**: R1 #28
**Where**: `crates/blit-daemon/src/service/push/control.rs:343-345` (back-compat: use
user's filter) vs `crates/blit-daemon/src/service/pull_sync.rs:430-462` (`Off | Unspecified`
→ `Vec::new()`)
**Remediation**: Either make `Unspecified` a hard reject (`InvalidArgument`), or normalize
both sites.

### M14. F4 destructive prompt phrasing diverges from F1/F2/F3 and CLI

**Source**: R1 #29
**Where**: F4 uses `[y / N or Esc]`; F1/F2/F3 use bare `y/N`; CLI uses `[y/N]:`
**Remediation**: Pick one TUI form, apply to all four screens, match CLI.

### M15. `confirm_destructive_operation` duplicated inline in `rm.rs`

**Source**: R1 #30
**Where**: `crates/blit-cli/src/transfers/mod.rs:87-99` (helper) vs `crates/blit-cli/src/
rm.rs:48-58` (re-implements y/yes logic)
**Remediation**: Move helper to `blit-app::common` or `blit-cli::shared`.

### M16. CLI cancel has no `--yes`/`--confirm`; TUI cancel confirm is config-gated

**Source**: R1 #31
**Where**: TUI `transfer.confirm_cancel` config knob vs CLI `JobsCancelArgs` has no
equivalent
**Remediation**: Add `--confirm` to `JobsCancelArgs` mirroring the TUI knob.

### M17. Empty-path encoding on the daemon side: `.` (du/find) vs `""` (push/pull)

**Source**: R1 #32
**Where**: `crates/blit-daemon/src/service/util.rs:153-158` (`normalize_relative_path` →
`""`) vs `:160-168` (`pathbuf_to_display` → `"."` for explicit `Path::new(".")`)
**Remediation**: Fold `pathbuf_to_display` into `relative_path_to_posix`.

### M18. `validate_wire_path` rejects `"."` as unsafe, but `normalize_for_request` produces `"."`

**Source**: R1 #33
**Remediation**: Standardize on `""` for root. Drop the empty→`.` fold in both
`normalize_for_request` copies.

### M19. `FsTransferSink` canonical-fallback ladder: 2 sites `log::warn!`, 2 silent

**Source**: R1 #34
**Where**: `sink.rs:190-205` and `:463-481` warn; `:651-657` and `:696-702` silent — same
R46-F3 fallback
**Remediation**: Extract single helper.

### M20. Source-delete-failed message uses three different past-tense verbs across TUI move paths

**Source**: R1 #35
**Where**: TUI `main.rs:3339` ("received but..."), `:3423` ("pushed but..."),
`:3523` ("delegated but..."), `blit-app/src/transfers/remote.rs:250` (no prefix)
**Remediation**: Single helper `format_post_transfer_delete_failure(operation, side, err)`.

### M21. CLI rejects mode-incompatible flags with wildly different verbosity

**Source**: R1 #37
**Where**: `move --dry-run` bails with a terse single-line; `move --null` / `--force` /
`--null --mirror` bail with 7-line essays
**Remediation**: One style for data-loss-class rejections.

### M22. `perf history clear` confirms in TUI but fires silently in CLI

**Source**: R1 #13
**Where**: TUI confirms via modal; `crates/blit-cli/src/diagnostics.rs:25-30`
`perf::clear()` immediate, no `--yes`, no prompt
**Plan**: Stated CLI principle: "destructive operations prompt unless `--yes` is supplied."
**Remediation**: Add prompt + `--yes` opt-out matching `mirror`/`move`/`rm`.

### M23. `clear-recent` exists only in TUI; CLI has no surface

**Source**: R1 #14
**Where**: TUI `main.rs:1788-1810, 3926-3930` (fans `ClearRecent` to every watched daemon,
drops per-daemon results) · `blit-app/src/admin/jobs.rs:107-118` (library function exists)
· CLI: no `clear-recent` verb
**Remediation**: Add `blit jobs clear-recent <REMOTE> [--yes] [--json]`. Fix TUI to collect
per-daemon Results.

## Low-severity findings / documentation drift

### L1. `docs/API.md` is not a valid API reference

**Source**: GPT-16
**Where**: `docs/API.md:11` lists 9 RPCs vs `proto/blit.proto:5` exposes 15 (adds PullSync,
DelegatedPull, GetState, CancelJob, ClearRecent, Subscribe)
**Remediation**: Regenerate from proto or replace with a pointer to it.

### L2. RELEASE_PLAN_v2 contradicts itself on §2.6

**Source**: GPT-18
**Where**: `docs/plan/RELEASE_PLAN_v2_2026-05-04.md:22` says §2.6 is deferred and
non-blocking, line 36 calls it the last P0 release blocker
**Remediation**: One status, one place.

### L3. Plan index points agents at stale live docs

**Source**: GPT-20
**Where**: `docs/plan/README.md:8` still names the 0.1.0 release plan as live source of
truth, even though TODO/CHANGELOG say 0.1.0 shipped and Phase 6 is active
**Remediation**: Point at TUI_REWORK.md as the active plan; mark RELEASE_PLAN as shipped
0.1.0 reference.

### L4. REVIEW.md, .review/findings/, and TODO.md state are mutually inconsistent

**Source**: GPT-23
**Where**: Verified rows are also listed as open; finding files still say "pending" after
verification; TODO header says only F15 remains while later unchecked work exists
**Remediation**: Sweep all three sources; the bookkeeping protocol exists, was not
followed end-to-end after the bug-mirror-literal-backslash round-2 close.

### L5. Historical artifacts need superseded banners

**Source**: GPT-24
**Where**: Old audit/bug docs still contain active-looking statements about missing blit-
tui, missing Subscribe, existing BlitAuth, and unresolved single-file push bugs
**Remediation**: Add a Superseded banner to each of these old artifacts pointing at the
current state.

### L6. blit-utils references survive in comments + checklists

**Source**: R1 #38 + GPT-25
**Where**: `admin_verbs.rs:323`, `service/admin.rs:500`, greenfield_v6 §4 deliverables
checklist, TODO entries
**Remediation**: Sweep `grep -rn blit-utils` across `crates/` and `docs/`.

### L7-L33. Remaining R1 low-severity items (verbatim from R1)

L7. `--workers` flag in CHANGELOG, omitted from manpage; no `[DEBUG] Worker limiter active`
banner. (R1 #39)
L8. `find --files` / `--dirs` flags shipped, not documented. (R1 #40)
L9. `--json` documented for du/df only; code supports more. (R1 #41)
L10. `--limit` documented for find only; SYNOPSIS lists it on `profile` and `diagnostics
perf` too. (R1 #42)
L11. Manpage dated 2025-11-21 vs v0.1.0 release 2026-05-31 (6-month staleness). (R1 #43)
L12. README "Rust 1.56+" claim not enforced via `rust-version` in any Cargo.toml. (R1 #44)
L13. `BLIT_UTILS_PLAN.md:65` claims `docs/cli/blit-utils.1.md` was created 2026-03-06; file
doesn't exist; same doc's banner says it shouldn't. (R1 #45)
L14. `Pull` RPC and `ServerPullMessage.ack` deprecated only in proto comments, no
`[deprecated = true]` annotation. (R1 #46)
L15. `blit_daemon_up` gauge always = 1; bridge omits all 5 counter series with no doc note.
(R1 #47)
L16. Daemon TCP keepalive applied only on push-receive accept; pull-receive and pull_sync-
receive accept paths don't tune. (R1 #48)
L17. Token comparison uses `==` (variable-time) at 4 daemon sites. (R1 #49)
L18. `compare.rs` has 2 s FAT/exFAT mtime tolerance; `manifest.rs::compare_file` Default
mode has zero tolerance. (R1 #50)
L19. `copy_file` uses BufferSizer; `chunked_copy_file` hardcodes 16 MiB for files >1 GiB.
(R1 #51)
L20. `mmap_copy_file` is a misnomer: no memory mapping, just `copy_file_range`/`sendfile`/
`fs::copy`. (R1 #52)
L21. `mtime tolerance` 2 s in `mirror_planner` repeated twice without a named constant.
(R1 #53)
L22. `enotempty-errno-66-only-macos-bsd`: relies on `err.kind() == DirectoryNotEmpty` to
cover Linux. (R1 #54)
L23. `copy_large_blocking` creates dest parent dir BEFORE checking `dry_run`. (R1 #55)
L24. Two glob-matching engines (`globset` + hand-rolled `glob_match`) coexist;
`build_globset` silently drops invalid patterns. (R1 #56)
L25. Three independent format-bytes implementations across the TUI (F1/F2/F4 with TiB; F3
capping at GiB; dual_pane with `.1` precision and no TiB). (R1 #57)
L26. `--interval-ms` dead flag on `jobs watch`. (R1 #58)
L27. 0-sentinel meaning differs across CLI flags. (R1 #59)
L28. Module-name validation rejects only empty/whitespace; daemon accepts `foo/bar` or
`..` as module names. (R1 #60)
L29. Bridge returns 404 for non-GET methods instead of 405. (R1 #61)
L30. DAEMON_CONFIG.md:524-528 has an editing slip ("discover it with `blit scan` or `blit
scan`"). (R1 #62)
L31. `motd` documented as "Message displayed to clients on connect" but is only printed to
daemon's stdout at startup. (R1 #63)
L32. F3 format_bytes lacks TiB tier; a 2 TiB subtree shows `"2048.00 GiB"`. (R1 #64)
L33. `127.0.0.1:9031` and `/tmp/blit-tui-input.log` hardcoded. (R1 #65)
L34. `pull_sync_with_spec_wire.rs:212` adds a 50 ms tokio sleep ("belt-and-suspenders").
(R1 #66)
L35. `scripts/` contains a Codex installer (`test.sh` misnamed), three personal resume
scripts, and a 3,942-line Claude Code transcript dump. (R1 #67)
L36. Linux change-journal doc claims "fallback to mtime comparison" — code actually uses
(device, inode, ctime) snapshot with mtime fallback only as last resort. (R1 #68)
L37. Architecture's `PerformanceRecord` snippet shows v1 pre-migration shape; current
schema is v2. (R1 #69)
L38. Whitepaper §3.1 references `self.pool.acquire()`; `DataPlaneSession` uses inline
buffers. (R1 #70)

## Cross-cutting inconsistencies (by dimension)

Carried verbatim from R1. GPT did not enumerate these; they remain valid.

### Path handling

A canonical chokepoint (`blit_core::path_posix::relative_path_to_posix`) exists but is
bypassed by three ad-hoc helpers that disagree on empty-path encoding. Push sends `""`;
pull sends `"."` for the same "module root" intent. Receive sinks' empty-rel guards
silently fail to fire when the renderer emits `"."`. Strict `validate_wire_path` rejects
`"."` but two callers actively produce it.

**Canonical pattern**: One helper, everywhere. `relative_path_to_posix(path)`. Empty →
`""`. Daemon side already folds both encodings.

### Error handling

Three error-wrapping styles for `tonic::Status` (preserve code+message / strip to message /
swap to `with_context` chain) scattered across the codebase. Admin verbs strip; jobs verbs
preserve. The result: `is_retryable` cannot fire on transport-class errors because the
`std::io::Error` chain is gone after admin clients wrap a Status. Data-loss-class
"unreadable paths refusal" has four flavors across CLI/TUI/daemon push/daemon pull_sync.
Four data-plane token rejection sites use 2 Status codes and 2 message strings.

**Canonical pattern**: One `status_to_eyre(rpc_name, status)` preserving code. One
`format_incomplete_scan_refusal(operation, paths, side)`. Extend `is_retryable` to walk
`eyre::chain()` for `tonic::Status::Unavailable|DeadlineExceeded|Aborted`.

### Endpoint parsing

Three parse functions used inconsistently. `list-modules` and `jobs` bypass the helper
that produces friendlier "verb-is-remote-only" errors. TUI F1 confirm-detail violates the
explicit "Err must reject" project-memory rule. `is_deletable_remote_path` filters
silently in TUI batch while `blit rm` bails. CLI `prepare_local_transfer` rejects Remote
with "use the CLI" while F1 push accepts remote dst.

**Canonical pattern**: Always 4-bucket classify. Single helper used everywhere; per-verb
error labels via parameter.

### Timeouts / retries / cancellation

Stall guard covers ONE of FOUR receive paths (H3). Three `connect_with_timeout` copies
with hardcoded 30 s (H19). Two `permission_denied`, two `unauthenticated` for the same
event (H20). `is_retryable_io_kind` and `categorize_io_error` disagree on UnexpectedEof,
NotConnected, ConnectionRefused, Interrupted, WouldBlock. TUI transfers have no
`--retry`/`--wait` equivalent. Daemon streaming RPC handlers have no per-message timeout.

**Canonical pattern**: One `TRANSFER_STALL_TIMEOUT` constant. One `connect_with_timeout` +
`CONNECT_TIMEOUT` in `blit-core::remote::client`. One token-rejection helper. Reconcile
`retry.rs` and `errors.rs` to one classifier.

### Naming / flags / confirmations

CLI `perf --clear` silent vs TUI confirms (M22). `clear-recent` exists only in TUI, with
unconditional fan-out + results discarded (M23). TUI F2 cancel confirm is config-gated;
CLI cancel has no opt-in (M16). Four destructive-prompt phrasings (M14). TUI state
machines diverge: F4 separate `ConfirmingMirror`/`ConfirmingMove` variants, F3 single
`Confirm { kind }`, F1 `confirming: bool` inside `Editing`. `UserAction::TransferMirrorConfirm`
is overloaded across mirror, move, cancel, batch cancel, clear-recent.

**Canonical pattern**: All destructive operations prompt by default; `--yes` opts out;
same prompt vocabulary and same writer target everywhere. One `Confirm { kind }` pattern.
Rename `TransferMirrorConfirm` → `ConfirmYes`. CLI + TUI stay in lockstep.

## What's solidly aligned

Carried from R1. GPT didn't list these but corroborated several spot-checks.

- **Data-plane wire format and tags** match WHITEPAPER §3 exactly.
- **F2 canonical-path containment is always-on** at every chokepoint.
- **Delegation gate ordering + DNS-rebinding mitigation** are honored end to end.
- **Spec-version fail-closed** for v1 daemons.
- **`pull_sync_with_spec` endpoint-isolation** (R23-F1 / R25-F1).
- **`require_complete_scan` purge gate** — but see H1 for the relay-mirror bypass.
- **MirrorMode default `FILTERED_SUBSET`** + scope_deletions.
- **No-silent-fallback CLI dispatch on remote→remote** — but the manpage doesn't say so
  (H16).
- **Token cryptographic + per-stream** (audit-3b).
- **BlitAuth removal complete in code** — but ARCHITECTURE.md hasn't caught up (H15).
- **Block-level resume via Blake3** (a7d659f).
- **Predictor observability shipped** per D9.
- **mDNS service + TXT keys shipped** — but DAEMON_CONFIG lists only 2 of 4 fields (M11).
- **Endpoint parser rejects bare `server:/module`**.
- **CLI data-loss reject-gates** comprehensive on `move`.
- **Unified receive pipeline** — on the TCP fast path. gRPC fallbacks still hand-written
  (H18).
- **pull_sync deadlock fix**.
- **mtime preservation race fix**.
- **CI tri-platform matrix and release artifact build**.
- **Recent persistence atomic write**.

## Audit cross-comparison

| GPT # | Title | R1 mapping | Status |
|---:|---|---|---|
| 1 | Relay mirror unreadable_paths bypass | none | **NEW** → H1 |
| 2 | Dual-pane action bar render-only | R1 #1 | match → H4 |
| 3 | Esc exits TUI | none | **NEW** → H2 |
| 4 | CancelJob Unauthorized not propagated | none | **NEW** → M1 |
| 5 | TUI source-of-truth split | R1 partially (rec #26) | merge → H4/H15 |
| 6 | Dual-pane path bars/search display-only | R1 #2 (implicit) | merge → H5 |
| 7 | Places/favorites/recents incomplete | none | **NEW** → M2 |
| 8 | TransferDraft / BatchTransferDraft missing | R1 #2 | match → H5 |
| 9 | Fan-out still sequential | none | **NEW** → M3 |
| 10 | Capability context can't drive dual-pane | none | **NEW** → M4 |
| 11 | Delegated live progress not emitted | none | **NEW** → H7 |
| 12 | Pull stall timeout misses gRPC fallback | R1 #9 (partial) | extend → H3 |
| 13 | Receive-pipeline unification incomplete | R1 #18 (partial) | extend → H18 |
| 14 | Delegated diagnostics endpoint mismatch | none | **NEW** → M5 |
| 15 | Subscribe/GetState byte counters hardcoded zero | none | **NEW** → H8 |
| 16 | docs/API.md stale (9 vs 15 RPCs) | none | **NEW** → L1 |
| 17 | Manpages stale | R1 #17 | match → H16 |
| 18 | Release plan contradicts itself §2.6 | none | **NEW** → L2 |
| 19 | Security/auth docs disagree | R1 #15 | match → H15 |
| 20 | Plan index points to stale docs | none | **NEW** → L3 |
| 21 | README 10+ Gbps overstated | none | **NEW** → M6 |
| 22 | mDNS TXT docs stale | R1 #26 | match → M11 |
| 23 | REVIEW.md/findings/TODO unreliable | none | **NEW** → L4 |
| 24 | Historical artifacts need superseded banners | partially R1 #38, #45 | **NEW** → L5 |
| 25 | blit-utils references remain | R1 #38 | match → L6 |

**Tally**: 11 GPT findings are NEW (not in R1); 7 directly match; 4 extend an R1 finding;
3 are partial / merged. The 6 new HIGH-severity GPT findings (1, 3, 11, 12, 13, 15) plus
the partial-merge upgrade of R1 #36 to HIGH (H14) are the most consequential R2 additions.

## Gaps in R1 audit scope (the docs my workflow missed)

R1's plan-doc inventory clusters didn't include:

- `docs/plan/UNIFIED_RECEIVE_PIPELINE.md` — directly cited by GPT-13 (H18). Would have
  surfaced the gRPC-fallback unification gap as a HIGH drift in R1.
- `docs/API.md` — directly cited by GPT-16 (L1). Caught only because GPT looked at the
  top-level `docs/` tree.
- `docs/plan/README.md` — cited by GPT-20 (L3).
- `docs/plan/REMOTE_TRANSFER_PARITY.md` — present on disk; not in R1 plan inventory.
- `docs/plan/TUI_UX_EXPERT_PROPOSAL.md` — present on disk; not in R1 plan inventory.
- `docs/plan/WORKFLOW_PHASE_2.5.md` and `WORKFLOW_V2.md` — present on disk; not in R1.
- `docs/grok_review.md` and `docs/forklift_audit/` and `docs/reviews/` — older review
  artifacts that GPT-24 (L5) wants supersede-banner'd.

In future audits, the plan-corpus inventory should `find docs/ -name '*.md'` and route
every result into exactly one cluster rather than rely on a hand-maintained list.

## Recommendations — merged ordered punch list

Priority order = severity × ease × user impact.

**Round 1 — data-loss-class / DoS-class fixes** (do before anything else):

1. **H1** Plumb `unreadable_paths` through the relay-mirror source scanner so
   `require_complete_scan` actually fires. Optionally, refuse `mirror --relay-via-cli`
   symmetric with `move --relay-via-cli` until the plumb-through is verified.
2. **H3** One `TRANSFER_STALL_TIMEOUT` constant + wrap every receive path in it (daemon
   push-receive, daemon pull-data-plane accepts, CLI gRPC-fallback). Single regression
   test per path.
3. **H11** Fix `display_f1.rs:46-54` confirm-detail Err arm — `unreachable!` paired with
   `debug_assert` in `plan_f1_trigger`, or shared classifier.
4. **H22** Surface skipped entries from TUI delete batch — match `blit rm` behavior.
5. **H13** Single `format_incomplete_scan_refusal` helper, used at all four sites,
   5-path preview.

**Round 2 — TUI rework alignment** (unblocks M3/M4/M-series):

6. **H4** Wire `TransferCopy`/`Mirror`/`Move`/`Delete`/`Verify` into Dual screen dispatch.
   Until landed, flip default screen back to F1.
7. **H5** Introduce `TransferDraft` / `BatchTransferDraft` / `BrowseProvider`. Move
   existing per-pane fetch logic behind the trait.
8. **H2** Wire Esc to per-screen back; reserve quit to `q` / Ctrl-C.
9. **H6** Flip TUI delegated `detach: true` + banner + regression test.
10. **H7** Stream `BytesProgress` from delegated_pull.
11. **H8** Plumb data-plane byte counters into `ActiveJob.to_proto`.
12. **M2** Define favorites / recents / known-endpoints schema in `tui.toml` + dedicated
    JSONL.
13. **M3** Batch-table renderer for fan-out (depends on H7/H8).
14. **M4** Thread capabilities through `BrowserEntry`; gate action labels.
15. **H14** Complete the (delegated, kind) verb table.

**Round 3 — error and timeout consolidation**:

16. **H19** Promote `connect_with_timeout` to `blit-core::remote::client`.
17. **H12** Shared `status_to_eyre`; extend `is_retryable` for tonic codes.
18. **H20** Single token-rejection helper returning `Unauthenticated`.
19. **H21** Replace ad-hoc `normalize_for_request` / `destination_path` with
    `relative_path_to_posix`.
20. **M1** Add `Unauthorized` to app-layer `CancelJobOutcome`.
21. **M17 + M18** Fold daemon-side `pathbuf_to_display` into `relative_path_to_posix`;
    standardize empty-root encoding on `""`.

**Round 4 — observability + doc-of-record alignment**:

22. **H10** Decide: revise WORKFLOW_PHASE_2 / LOCAL_TRANSFER_HEURISTICS / greenfield_v6
    §1.1 to describe the synchronous orchestrator, OR build the streaming planner.
    Conservative: doc-update + add outer `scan_handle.await` timeout.
23. **H18** Decide: complete the gRPC-fallback unification, OR update
    UNIFIED_RECEIVE_PIPELINE.md to describe what shipped.
24. **H17** Decide: complete the Phase-5 TarShardExecutor unification, OR update §1.2
    closure + docstring.
25. **H9** Update REMOTE_REMOTE_DELEGATION_PLAN §9 / §4.2 step 12 to record that
    `--detach` shipped.
26. **H15** Rewrite ARCHITECTURE Security Considerations + Planned Enhancements; rewrite
    WHITEPAPER §6 to use `TransferOperationSpec`.
27. **H16** Regenerate manpage from clap. Auto-generation via build script.
28. **H23** Decide the env-var policy. Implement the two documented overrides or strike
    them from the plan.
29. **M10** Document `--metrics` in DAEMON_CONFIG.md; decide Counters proto contract.
30. **M11** Add `module_count` + `delegation_enabled` to DAEMON_CONFIG.md §"mDNS Discovery".
31. **M5** Capture actual data-plane address in delegated pull, forward on Started event.
32. **M6** Soften README 10+ Gbps claim.

**Round 5 — surface parity** (CLI/TUI):

33. **M22** Prompt + `--yes` for `blit diagnostics perf --clear`.
34. **M23** Add `blit jobs clear-recent`; fix TUI to collect per-daemon Results.
35. **M16** Add `--confirm` to `JobsCancelArgs`.
36. **M14** Pick one TUI destructive-prompt phrasing.
37. **M7** F4 routes through same router as F1, or its error message points to F1 not the
    CLI.

**Round 6 — documentation sweep**:

38. **L1** Regenerate `docs/API.md` from proto, or point at proto.
39. **L2** Fix RELEASE_PLAN §2.6 self-contradiction.
40. **L3** Update `docs/plan/README.md` to point at TUI_REWORK.md as the active plan.
41. **L4** Sweep REVIEW.md / .review/findings/ / TODO.md for verified/open consistency.
42. **L5** Superseded-banner historical artifacts.
43. **L6** Sweep `grep -rn blit-utils` and remove remaining references.
44. Plus L7-L38 — small individual edits.

## Appendix A — coverage attestation

(Inherits the R1 attestation table verbatim. Total 92,229 lines of plan + code read in R1;
GPT's review re-read targeted areas of the same tree plus the docs R1 missed listed in
"Gaps in R1 audit scope" above.)

| Cluster | Files | Lines read |
|---|---:|---:|
| Plan — principles (greenfield_v6 + MASTER_WORKFLOW + RELEASE_PLAN_v2) | 3 | 1,367 |
| Plan — phases (POST_REVIEW_FIXES + REMOTE_REMOTE_DELEGATION + WORKFLOW_PHASE_{2,3,4} + PROJECT_STATE_ASSESSMENT) | 6 | 1,929 |
| Plan — TUI (TUI_DESIGN + TUI_REWORK) | 2 | 1,529 |
| Plan — wire (ARCHITECTURE + WHITEPAPER + DAEMON_CONFIG + blit.proto) | 4 | 2,866 |
| Plan — perf (LOCAL_TRANSFER_HEURISTICS + PIPELINE_UNIFICATION + BENCHMARK_10GBE_PLAN + BENCH_VERB_PLAN) | 4 | 944 |
| Plan — CLI (blit.1.md + README.md + CHANGELOG.md + BLIT_UTILS_PLAN.md) | 4 | 505 |
| **Plan subtotal (R1)** | **23** | **9,140** |
| Plan — *missed by R1*, caught by GPT | 7+ | not measured |
| Code — blit-cli | 20 | 4,920 |
| Code — blit-daemon | 17 | 12,489 |
| Code — bridge + proto + build.rs | 6 | 1,901 |
| Code — blit-core remote/transfer + push + pull + endpoint | 17 | 10,619 |
| Code — blit-core copy + delete + buffer + tar_stream + checksum + manifest | 16 | 3,533 |
| Code — blit-core orchestrator + mirror_planner + local_worker + enumeration + fs_enum | 10 | 4,566 |
| Code — blit-core remaining (auto_tune, fs_capability, change_journal, mdns, perf_*, path_*) | 19 | 5,884 |
| Code — blit-tui main.rs | 1 | 10,838 |
| Code — blit-tui state machines + pane behavior | 14 | 10,876 |
| Code — blit-tui display mappers + helpers + screens | 16 | 5,380 |
| Code — tests + scripts + CI | 40+ | 12,083 |
| **Code subtotal (R1)** | **~176** | **83,089** |
| **TOTAL (R1)** | | **92,229** |

## Appendix B — file-level cross-references

Inherits R1 Appendix B verbatim; key additions for R2 findings:

- `crates/blit-core/src/remote/transfer/source.rs:233` — H1 (relay-mirror
  `unreadable_paths` bypass)
- `crates/blit-tui/src/main.rs:5141` — H2 (Esc maps to quit before back-out)
- `crates/blit-tui/src/main.rs:2928` — M2 (favorites/recents schema absent)
- `crates/blit-tui/src/dual_pane.rs:462` — M4 (action labels unconditional)
- `crates/blit-daemon/src/service/delegated_pull.rs:336` — H7 (no live BytesProgress)
- `crates/blit-daemon/src/service/delegated_pull.rs:305` — M5 (wrong endpoint reported)
- `crates/blit-daemon/src/service/core.rs:233` — H8 (counters hardcoded to zero)
- `crates/blit-core/src/remote/pull.rs:752` — H3 (gRPC fallback no idle deadline)
- `crates/blit-app/src/admin/jobs.rs:39` — M1 (CancelJobOutcome missing Unauthorized)
- `docs/plan/UNIFIED_RECEIVE_PIPELINE.md:332` — H18 (claims gRPC unification, untrue)
- `docs/API.md:11` — L1 (lists 9 of 15 RPCs)
- `docs/plan/README.md:8` — L3 (live SoT pointer stale)
- `docs/plan/RELEASE_PLAN_v2_2026-05-04.md:22` vs `:36` — L2 (§2.6 self-contradiction)
- `README.md:24` — M6 (10+ Gbps overstated)

---

*End of Revision 2 report. 23 high-severity, 23 medium-severity, ~38 low-severity / doc
drift findings. Every finding cross-referenced to specific file+line evidence in the
audit's source inventories + GPT review verbatim. R3 to integrate any GPT R2 fan-out.*
