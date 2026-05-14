# Blit TUI Design — Phase 5 Plan

**Status:** Active planning. Code starts after 0.1.0 ships.
**Owner:** mcoelho
**Tracks:** `TODO.md` Phase 5 entry. Builds on §3.2 mDNS TXT enrichment
(`0d76c4f`) and the §3.1 `--metrics` decision (`6e750b9`).

> **Revision note (2026-05-14):** this document originally drafted
> 2026-05-01. Refreshed to incorporate (a) the §3.2 mDNS TXT keys
> that actually shipped, (b) the §3.1 owner decision to give
> `--metrics` per-RPC stderr summaries instead of leaving counters
> dormant, and (c) the Phase 5 scope expansion: TUI must mirror
> **every** CLI verb in addition to the discovery / file-browser
> work originally scoped here.

## 1. Purpose

A single terminal-UI binary that lets an operator do interactively
everything `blit` currently does at the command line, plus:

- See every blit-daemon reachable on the LAN, with module list,
  free/used capacity, version, and delegation state at a glance.
- Browse the file tree of any reachable daemon (or local path)
  with `ls` / `find` / `du` / `df` data inline.
- Initiate copy / mirror / move / rm between any two endpoints
  (local↔local, local↔daemon, daemon↔daemon) without re-typing
  paths.
- Watch in-flight transfers with live throughput, ETA, and
  per-file progress; see recent transfer history and per-daemon
  counters.

The TUI is a **gRPC client** of one or more daemons. It is not a
daemon itself. Every byte of wire shape the TUI consumes will also
be consumable by a future GUI / web client on the same protocol —
this design intentionally avoids TUI-specific RPCs.

## 2. CLI verb → TUI affordance map

Every existing `blit` verb gets a TUI home. This is the parity
contract: a user who knows the CLI must be able to do the same
thing in the TUI without dropping back to a shell.

| CLI verb | TUI home | Notes |
|---|---|---|
| `blit scan` | F1 Daemons (the list itself) | mDNS feed already drives this. |
| `blit list-modules <host>` | F1 selection drilldown | Shown inline per daemon row using `module_count` TXT; full `ListModules` RPC on selection. |
| `blit ls <target>` | F3 Browse | Reuses existing `List` RPC for remote paths, `std::fs` for local. |
| `blit find <target>` | F3 Browse → `/` hotkey | In-pane search prompt; streams from `Find` RPC. |
| `blit du <target>` | F3 Browse sidebar | Inline byte totals per directory; on-demand via `DiskUsage` RPC. |
| `blit df <module>` | F1 daemon detail pane | Free/used/total via existing `FilesystemStats` RPC. |
| `blit rm <target>` | F3 Browse → `Delete` key | Confirmation modal; existing `Purge` RPC. Read-only modules disabled. |
| `blit copy <src> <dst>` | F2 Transfer (start action) | Source / destination selected in F3 or via `Tab`-able input fields. |
| `blit mirror <src> <dst>` | F2 Transfer (start action) | Same flow as copy with `--mirror` flag in the options modal. |
| `blit move <src> <dst>` | F2 Transfer (start action) | Same flow; flag in modal. |
| `blit check <a> <b>` | F4 Verify | Read-only tree comparison; renders as a diff pane. |
| `blit completions ...` | N/A | Shell-only; the TUI has its own input affordances. |
| `blit profile` | F4 Profile / status bar | Local perf-history summary; one-shot read of `~/.config/blit/perf_history.jsonl`. |
| `blit diagnostics perf` | F4 settings panel | Enable / disable / clear toggles. |
| `blit diagnostics dump` | F4 → `dump snapshot` action | Saves a snapshot file; reuses the existing dump emitter. |

This map drives the screen list (§5). No verb is unrepresented;
nothing in the TUI requires a fresh code path the CLI doesn't
already exercise.

## 3. What lives where

| Need | Source | Mechanism | Status |
|---|---|---|---|
| Daemon discovery | mDNS | `_blit._tcp.local.` | ✅ Already advertised |
| Daemon identity at-a-glance | mDNS TXT | `version`, `modules`, `module_count`, `delegation_enabled` | ✅ Shipped `0d76c4f` |
| Module list | gRPC `ListModules` | per-daemon RPC | ✅ Already exists |
| File browse | gRPC `List` / `Find` / `DiskUsage` / `FilesystemStats` | per-target RPC | ✅ Already exist |
| Trigger transfer | gRPC `Push` / `PullSync` / `DelegatedPull` | normal client flow | ✅ Already exists |
| Remove files | gRPC `Purge` | normal client flow | ✅ Already exists |
| Tree comparison | (none yet — CLI runs this locally) | Local read of both endpoints | ✅ Today's `blit check` is local-only and read-only |
| Live in-flight transfer events | gRPC `Subscribe` | **new RPC** — see §6.2 | ⏳ Not yet on the wire |
| Snapshot daemon state | gRPC `GetState` | **new RPC** — see §6.3 | ⏳ Not yet on the wire |
| Aggregate counters | `TransferMetrics` on `BlitService` | currently exposed only via `--metrics` stderr summaries (§3.1) | Exposed over wire by `GetState` |

The two missing wire pieces are `Subscribe` and `GetState`. Both
were sketched in the original draft and remain the right shape; §6
re-states them with the corrections that fall out of the §3.x
decisions.

## 4. Design principles

1. **Reuse the unified pipeline.** The TUI's "trigger a transfer"
   action instantiates a normal `RemotePushClient` /
   `RemotePullClient` / `DelegatedPullClient` and hands off to the
   exact code path the CLI uses. No alternate transfer code, no
   shadow planner.

2. **No TUI-specific RPCs.** Every wire surface the TUI introduces
   (`Subscribe`, `GetState`) is generally useful — a future GUI /
   web client / Prometheus bridge / health-check probe all consume
   the same shape.

3. **Progressive enhancement.** A useful TUI exists at milestone A
   (discovery + browse + trigger) with NO new wire pieces. Each
   subsequent milestone adds visible value without breaking what
   came before.

4. **CLI parity is the floor.** Anything the CLI can do, the TUI
   can do. The CLI continues to be the scripting surface; the TUI
   is the interactive surface. Neither shrinks.

5. **`--metrics` is the operator signal, `GetState` is the
   programmatic one.** `--metrics` stderr summaries are useful for
   `journalctl -u blit-daemon` watching; the TUI uses `GetState`
   for structured per-daemon state. The TUI does NOT scrape
   stderr.

## 5. Screen structure

Four primary screens, switchable with hotkeys. Status bar at the
bottom; modal overlays for confirmation / option entry.

```
┌─ blit ────────────────────────── 3 daemons │ 1 transfer active ─┐
│                                                                  │
│ [F1] Daemons  [F2] Transfers  [F3] Browse  [F4] Profile/Verify   │
│                                                                  │
│  «active screen»                                                 │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│ tab: switch panel │ enter: drill in │ /: search │ ?: help │ q    │
└──────────────────────────────────────────────────────────────────┘
```

### 5.1 F1 — Daemons

```
┌─ Daemons (mDNS) ─────────────────────────────────────────────┐
│ ▸ mycroft    192.168.1.10:9031  v0.1.0  3 modules   deleg ✓  │
│   skippy     192.168.1.20:9031  v0.1.0  1 module    deleg ✗  │
│   elphaba    192.168.1.30:9031  v0.0.9  ? modules   deleg ?  │
└──────────────────────────────────────────────────────────────┘
┌─ Selected: mycroft ──────────────────────────────────────────┐
│ Version: 0.1.0  │ Uptime: 2d 4h 17m  │ Active: 1            │
│ Modules: home (12.3 TiB / 16.0 TiB), backups, media          │
│ Counters: 142 push / 88 pull / 3 purge │ errors: 1           │
│                                                              │
│  [enter] browse  [t] trigger transfer  [d] diagnostics       │
└──────────────────────────────────────────────────────────────┘
```

- Top pane fed entirely from mDNS — list and metadata refresh
  without an RPC. `?` indicates pre-§3.2 daemons that don't
  advertise the new TXT keys (graceful degradation).
- Bottom pane appears on row select. Until `GetState` lands, the
  counters/uptime block reads "(unavailable — daemon does not
  yet support GetState)" and the modules-with-capacity line
  falls back to module names only.

### 5.2 F2 — Transfers

```
┌─ Active ─────────────────────────────────────────────────────┐
│ → mycroft:home/photos/2024 → skippy:photos/                  │
│   ████████░░░░░░░░░░░░░░░░  47%  84.3 MiB/s  ETA 3m 12s     │
│   files: 1,247 / 2,640    bytes: 4.1 GiB / 8.7 GiB           │
│                                                              │
│ → /Users/me/work → mycroft:work/                             │
│   ██░░░░░░░░░░░░░░░░░░░░░░  8%   12.1 MiB/s  ETA 14m 02s    │
└──────────────────────────────────────────────────────────────┘
┌─ History (recent 50) ────────────────────────────────────────┐
│ 12:42  push  /home/me/dl → mycroft:dl/      8.2 GiB   ok    │
│ 12:18  pull  skippy:backup → /backups/      442 MiB   ok    │
│ 11:55  mirror mycroft:proj → /local/proj/   3.1 MiB   err   │
└──────────────────────────────────────────────────────────────┘
```

- Active rows come from `Subscribe` events (one event stream per
  daemon the TUI is watching). Each row is a state machine driven
  by `TransferStarted` → `TransferProgress*` → `TransferComplete |
  TransferError`.
- History from `GetState.recent[]` snapshot at TUI launch +
  appended from `Subscribe.transfer_complete` events.
- Selecting an active row gives a detail modal with file-level
  list (if available) and "cancel" hotkey (uses existing Ctrl-C
  cancellation path on the client end of `Push`/`PullSync`).

### 5.3 F3 — Browse

```
┌─ Browse: mycroft:home ────────────────────────────────────────┐
│ /                                                             │
│ ├─ docs/         412 MiB     2024-03-01                       │
│ ├─ photos/      14.2 GiB     2024-06-12  ◀ cursor             │
│ │  ├─ 2023/      4.1 GiB                                      │
│ │  └─ 2024/     10.1 GiB                                      │
│ └─ work.zip     820 MiB     2024-07-03                        │
└───────────────────────────────────────────────────────────────┘
┌─ Stats ──────────────────────────────────────────────────────┐
│ Selected: photos/   subtree: 14.2 GiB across 8,442 files     │
│ Module: home   free: 3.7 TiB / 16.0 TiB                      │
└──────────────────────────────────────────────────────────────┘

Hotkeys: enter/→: into  ←: up  space: multi-select  c: copy
         m: mirror  v: move  D: delete  /: find  d: du  ?: help
```

- Tree pane lazily expands directories via `List` RPC; remembers
  open state per session.
- `du` data fetched on demand for the cursor row; cached for
  re-entry.
- `space` multi-select builds a selection set; `c`/`m`/`v` open
  the transfer-options modal with the set as the source list,
  prompting for destination.
- `/` opens a search modal that streams `Find` RPC results into
  a flat list; selecting a result jumps the tree to that path.
- `D` opens a confirmation modal that names every selected entry
  before issuing `Purge`. Read-only modules disable the key.

### 5.4 F4 — Profile / Verify / Diagnostics

```
┌─ Profile (local performance history) ────────────────────────┐
│ Records: 4,231   span: 17 days   ~250 MiB total             │
│ Predictor coefficients:                                      │
│   copy   α=12.4 ms/file  β=0.31 ms/MB  γ=18 ms              │
│   mirror α=14.1 ms/file  β=0.29 ms/MB  γ=22 ms              │
│                                                              │
│  [c] clear  [d] disable  [e] enable                          │
└──────────────────────────────────────────────────────────────┘
┌─ Verify ─────────────────────────────────────────────────────┐
│ Source:      [path or remote endpoint                     ]  │
│ Destination: [path or remote endpoint                     ]  │
│ Mode: (•) size+mtime ( ) checksum                            │
│                                                              │
│  [enter] run check    [esc] clear                            │
└──────────────────────────────────────────────────────────────┘
┌─ Diagnostics ────────────────────────────────────────────────┐
│  [d] dump diagnostic snapshot for SRC → DST (saves to disk)  │
└──────────────────────────────────────────────────────────────┘
```

- Profile pane reads `perf_history.jsonl` directly — no RPC
  needed; mirrors what `blit profile` does today.
- Verify pane wraps the existing `blit check` code path,
  rendered as a diff (matches / size-diff / mtime-diff /
  missing-on-side rows).
- Diagnostics dump reuses the existing emitter.

## 6. Wire surface

### 6.1 What we already use (no changes)

`ListModules`, `List`, `Find`, `DiskUsage`, `FilesystemStats`,
`Push`, `PullSync`, `DelegatedPull`, `Purge`. The TUI is a normal
client of all of these.

### 6.2 `Subscribe` — server-streaming live events

```protobuf
service Blit {
  // ...existing RPCs...
  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
}

message SubscribeRequest {
  // Bitfield of event categories the client wants. 0 = all.
  // Bits: TRANSFERS=1, ERRORS=2, MODULES=4, HEARTBEAT=8.
  uint32 event_mask = 1;
  // If true, replay the in-memory recent-event ring on connect.
  bool replay_recent = 2;
}

message DaemonEvent {
  oneof payload {
    TransferStarted    transfer_started    = 1;
    TransferProgress   transfer_progress   = 2;
    TransferComplete   transfer_complete   = 3;
    TransferError      transfer_error      = 4;
    ModuleListChanged  module_list_changed = 5;
    DaemonHeartbeat    heartbeat           = 6;
  }
}

message TransferStarted {
  string  transfer_id = 1;
  enum Kind { PUSH = 0; PULL = 1; PULL_SYNC = 2; DELEGATED_PULL = 3; PURGE = 4; }
  Kind    kind = 2;
  string  peer = 3;            // CLI host:port or "(local)"
  string  module = 4;
  string  path = 5;
  uint64  start_unix_ms = 6;
}

message TransferProgress {
  string transfer_id = 1;
  uint64 bytes_completed = 2;
  uint64 bytes_total = 3;       // 0 when total not yet known
  uint64 files_completed = 4;
  uint64 files_total = 5;
  uint64 throughput_bps = 6;    // 1-second EWMA
}

message TransferComplete {
  string transfer_id = 1;
  uint64 bytes = 2;
  uint64 files = 3;
  uint64 duration_ms = 4;
  bool   tcp_fallback_used = 5;
}

message TransferError {
  string transfer_id = 1;
  string message = 2;
}
```

Daemon-side: a `tokio::broadcast` inside `BlitService` collects
events from the existing push/pull/purge handlers. `Subscribe`
clients subscribe to the channel. Slow consumers drop with a
`Lagged` notification (TUI re-fetches via `GetState`).

**Source of progress events:** the unified pipeline already
tracks files-completed / bytes-completed in the `SinkOutcome`
merge step. A thin shim turns those into `TransferProgress`
events at a configurable cadence (default ~10 Hz). Throughput is
a 1-second EWMA computed daemon-side so every subscriber sees
identical numbers.

### 6.3 `GetState` — daemon snapshot

```protobuf
service Blit {
  // ...
  rpc GetState(GetStateRequest) returns (DaemonState);
}

message GetStateRequest {
  uint32 recent_limit = 1;  // 0 = use daemon default (50)
}

message DaemonState {
  string version = 1;
  uint64 uptime_seconds = 2;
  repeated ModuleInfo modules = 3;        // existing message reuse
  repeated ActiveTransfer active = 4;
  repeated TransferRecord recent = 5;
  Counters counters = 6;
  bool delegation_enabled = 7;            // mirrors mDNS TXT
}

message ActiveTransfer {
  string transfer_id = 1;
  TransferStarted.Kind kind = 2;
  string peer = 3;
  string module = 4;
  string path = 5;
  uint64 start_unix_ms = 6;
  uint64 bytes_completed = 7;
  uint64 bytes_total = 8;
}

message TransferRecord {
  string  transfer_id = 1;
  TransferStarted.Kind kind = 2;
  string  peer = 3;
  string  module = 4;
  string  path = 5;
  uint64  start_unix_ms = 6;
  uint64  duration_ms = 7;
  uint64  bytes = 8;
  uint64  files = 9;
  bool    ok = 10;
  string  error_message = 11;             // empty when ok=true
}

message Counters {
  uint64 push_operations_total = 1;
  uint64 pull_operations_total = 2;
  uint64 purge_operations_total = 3;
  uint64 active_transfers = 4;
  uint64 transfer_errors_total = 5;
}
```

`Counters` reads directly from the existing `TransferMetrics`
atomics — no new bookkeeping. The active/recent lists are
populated from the same broadcast-channel data Subscribe streams,
buffered in a small in-memory ring (size = `recent_limit` default
50). Persistence is deferred — see §8 open questions.

### 6.4 Wire-surface impact on `--metrics`

Per the §3.1 owner decision, `--metrics` enables stderr summary
lines plus atomic counter collection. **The same flag also
enables Subscribe/GetState event emission** so a daemon without
`--metrics` returns counters of zero and an empty active/recent
list when polled. This keeps the cost story symmetric: no
observation overhead unless the operator opted in.

## 7. Crate / dependency shape

Proposal: new crate `crates/blit-tui` producing a `blit-tui`
binary.

Rationale:
- The TUI pulls in `ratatui` + `crossterm` (~500 KB compiled
  weight). Bundling into `crates/blit-cli` would push the `blit`
  binary up for users who never touch the TUI.
- Separate crate keeps `cargo build -p blit-cli` fast and
  release-tarball size minimal.
- The TUI imports `blit-core` for transfer clients and `blit-core`
  proto types directly — no duplication.

Alternative: subcommand `blit tui` inside the existing CLI crate,
gated behind a `tui` feature flag. Smaller binary footprint when
disabled, but `cargo build --release` for the default install
still pays the dep weight unless we cargo-feature it carefully.

**Recommendation:** start with the separate crate. Easier to ship
opt-in, easier to swap the rendering layer (ratatui → web → ...)
later without touching the CLI.

## 8. Milestones (each independently shippable)

### Milestone A — Discovery + browse + trigger (NO new wire)

Scope: F1 (Daemons list, no counters / no active pane yet), F3
(Browse, with `du` and `find` working), trigger transfer (calls
existing Push/PullSync/Purge), F4 Profile pane reading
`perf_history.jsonl` directly.

Counters pane in F1 shows "(unavailable — needs GetState)".
F2 Transfers screen exists but only renders the **history**
populated from the TUI's own session (transfers it kicked off);
no Subscribe means no view into transfers initiated elsewhere.

Result: a useful TUI you can ship without touching the proto.

### Milestone B — `GetState` RPC + daemon detail pane

Scope: proto change adding `GetState`, daemon-side
`TransferRecord` ring buffer, TUI F1 detail pane lights up with
real counters, uptime, modules-with-capacity, recent transfers.

Single-point daemon-state read; no streaming yet.

### Milestone C — `Subscribe` RPC + live in-flight progress

Scope: proto change adding `Subscribe` + `DaemonEvent` family,
daemon-side `tokio::broadcast` and event-emission shim, TUI F2
Active pane lights up with live bars + throughput.

This is the milestone that fulfills the "watch transfers initiated
elsewhere" use case.

### Milestone D — Verify + diagnostics screens

Scope: F4 Verify (wraps `blit check`), F4 Diagnostics dump
button. Mostly TUI-side glue around existing CLI internals.

### Milestone E — Polish

Theme support, dark/light, configurable refresh rates, key
remapping, JSON config for default endpoints, optional Prometheus
bridge as a separate binary that scrapes `GetState`.

## 9. Non-goals

- HTTP / Prometheus endpoints directly on the daemon. A separate
  bridge program can scrape `GetState` if Prometheus is needed.
- Web UI bundled with the daemon. Same wire works for a future
  browser client, but the browser client is a separate codebase.
- Daemon-initiated push. Subscribe is server→client streaming
  over a client-initiated connection.
- Live structured logging stream. F15 deferred; if structured
  logs are wanted in the TUI later, they ride on Subscribe.
- Authentication. Same scope decision as §5.2 of the release
  plan — operator network controls, no app-level auth.

## 10. Open questions

These need owner input before code starts on the relevant
milestone. Listed in the order they become decision-blockers:

1. **Crate split vs. subcommand** (§7 alternative). Recommend
   separate `blit-tui` crate. **Blocker for: Milestone A.**

2. **Recent-transfer persistence.** `GetState.recent[]` populated
   from an in-memory ring (cheap, lost on restart) or from
   `perf_history` (durable, reuses existing storage)?
   **Blocker for: Milestone B.** Recommendation: in-memory ring
   for B; if persistence is wanted later, reuse `perf_history`
   in Milestone E.

3. **`Subscribe` subscriber cap.** N concurrent subscribers,
   N=? Default 8 seems reasonable for a single-operator TUI.
   **Blocker for: Milestone C.**

4. **`TransferProgress` cadence.** 10 Hz default, configurable
   via `SubscribeRequest`? Or fixed? Higher cadence is nice for
   the TUI but costs more in broadcast traffic.
   **Blocker for: Milestone C.**

5. **Multi-daemon Subscribe.** TUI watches N daemons
   simultaneously by opening N Subscribe streams; aggregation
   happens client-side. Is that ok or should the TUI talk to a
   designated "primary" daemon that fans out? Recommend simple
   N-streams approach. **Blocker for: Milestone C.**

6. **Cancellation UX.** From F2 Active, selecting a row and
   hitting `Ctrl-C` cancels the transfer via the existing client
   cancellation path. Does the TUI also expose a server-side
   "cancel this transfer" RPC for transfers initiated elsewhere?
   That requires a new `CancelTransfer(transfer_id)` RPC.
   Probably defer to a milestone after E.

7. **Transfer-id allocation.** UUIDv4 daemon-side at
   `TransferStarted` emission time. Stable for the duration of
   the RPC. Confirmed by the client via the existing per-RPC
   summary path? **Minor — daemon-internal — but flag now.**

8. **Local-only TUI mode.** Does `blit-tui` work without any
   daemon on the network — i.e. as a pure local-file browser
   that can drive `blit copy`? Recommend yes; treat "local" as a
   first-class endpoint in the daemon list (sentinel entry).

## 11. Phasing summary

| Milestone | Wire changes | LOC band | Independently useful? |
|---|---|---|---|
| A | none | ~3000 (new crate, screens, TUI lifecycle) | ✅ |
| B | +`GetState` | ~500 daemon + ~300 TUI | ✅ |
| C | +`Subscribe` | ~800 daemon + ~500 TUI | ✅ |
| D | none | ~400 TUI | ✅ |
| E | none (optional Prometheus bridge is separate crate) | ~600 | ✅ |

Total roughly 5–6 kLOC for the full feature surface. Milestone A
alone is shippable as a useful product.

## 12. What this design intentionally does NOT lock in

- **Render library.** `ratatui` is the recommendation; if a
  different terminal library proves easier to maintain or
  cross-compile, swap is local to `crates/blit-tui`.
- **Layout details.** The §5 sketches are illustrative, not
  binding. Final layouts get tuned during Milestone A.
- **Key bindings.** Hotkeys shown are starting points; final
  bindings come out of usability testing.
- **Theme.** Color story is Milestone E.

The structural commitments are:
- Four-screen architecture (F1 / F2 / F3 / F4).
- Two new RPCs (`Subscribe`, `GetState`) — names and message
  fields are the contract.
- Separate `blit-tui` crate.
- Reuse the unified pipeline; no shadow transfer code.

Everything else is a tuning knob.
