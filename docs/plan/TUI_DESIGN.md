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
| `blit profile` | F4 Profile / status bar | Local perf-history summary; one-shot read of `~/.config/blit/perf_local.jsonl`. |
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
| Job lifecycle (cancel + daemon-owned transfers) | gRPC `CancelJob` + `detach` field on transfer specs | **new** — see §6.5 | ⏳ Not yet on the wire |
| Job-id-scoped event attach | `Subscribe.transfer_id_filter` | **new field** — see §6.2 | ⏳ Not yet on the wire |

The wire-surface gaps:
- `GetState` (snapshot of active + recent + counters)
- `Subscribe` (live event stream) with a `transfer_id_filter` field
- `CancelJob` (cancel without holding the transfer's stream)
- `detach: bool` field on `TransferOperationSpec` (decouples
  transfer lifetime from initiating-client connection)

Together these enable the single-pane-of-glass model: any TUI on
the LAN can list, watch, cancel, or initiate transfers on any
reachable daemon, and transfers survive their initiator
disconnecting.

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
  TransferError`. Transfers the TUI didn't initiate appear here
  too — that's the single-pane-of-glass guarantee.
- History from `GetState.recent[]` snapshot at TUI launch +
  appended from `Subscribe.transfer_complete` events.
- Selecting an active row gives a detail modal with file-level
  list (if available) and "cancel" hotkey, which fires the new
  `CancelJob(transfer_id)` RPC (§6.5). Cancellation works on
  transfers the TUI didn't initiate.
- Transfers kicked off from the TUI always use `detach=true` on
  the spec (§6.5) so closing the TUI doesn't kill the transfer.

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
┌─ Verify (local paths only — see note) ───────────────────────┐
│ Source:      [local path                                  ]  │
│ Destination: [local path                                  ]  │
│ Mode: (•) size+mtime ( ) checksum                            │
│                                                              │
│  [enter] run check    [esc] clear                            │
└──────────────────────────────────────────────────────────────┘
┌─ Diagnostics ────────────────────────────────────────────────┐
│  [d] dump diagnostic snapshot for SRC → DST (saves to disk)  │
└──────────────────────────────────────────────────────────────┘
```

- Profile pane reads `perf_local.jsonl` directly — no RPC
  needed; mirrors what `blit profile` does today.
- Verify pane wraps the existing `blit check` code path,
  rendered as a diff (matches / size-diff / mtime-diff /
  missing-on-side rows). **Local paths only** in Milestone D —
  matches current `blit check` semantics (see
  `crates/blit-cli/src/check.rs`, which calls `Path::exists()`
  directly on both inputs). Remote-side verification requires a
  new "tree compare" affordance that doesn't exist today; see
  §10 open question on remote verify.
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
  // Empty = events for every active transfer (and module /
  // heartbeat / error events per event_mask). Set to a job's
  // transfer_id to receive only that job's events plus a
  // per-job replay buffer if the transfer is already in flight
  // when the subscription opens. Lets a TUI that connects
  // mid-transfer pick up the bytes-completed history of an
  // in-flight job without missing the early progress.
  string transfer_id_filter = 3;
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

**Source of progress events — honest accounting.** Today's
progress pipeline (`crates/blit-core/src/remote/transfer/
progress.rs`) emits three event kinds:

- `ManifestBatch { files }` — fires when the daemon discovers
  a batch of files; good for the "files discovered" denominator.
- `Payload { files, bytes }` — fires per planned payload group
  (tar shard or raw bundle), NOT per byte streamed.
- `FileComplete { path, bytes }` — fires when a file's bytes
  finish landing on the destination.

So **today a 10 GiB single-file transfer produces exactly one
progress event — at completion.** A "thin shim at 10 Hz over
SinkOutcome" wouldn't help: SinkOutcome is also coarse, computed
from completed payloads. Milestone C therefore needs real
byte-level instrumentation, not just a broadcast shim:

1. **Per-payload byte counters in the data-plane write loop.**
   Add a `report_payload_bytes(transfer_id, delta_bytes)` call
   inside `receive_stream_double_buffered` (data_plane.rs:135)
   and the equivalent local-sink write loops. Cadence: every
   N bytes (e.g. 1 MiB) OR every M milliseconds, whichever
   fires first. Bounded so we don't drown the broadcast channel
   on a fast NVMe-to-NVMe local transfer.

2. **Per-transfer state machine in `BlitService`.** Today the
   service has no notion of "transfer X is running, here's its
   cumulative byte count." Milestone C introduces an
   `ActiveTransferTable: HashMap<transfer_id, ActiveState>`
   keyed by the UUID minted at `TransferStarted` time. The
   byte-counter calls feed this table; the broadcast emits
   `TransferProgress` snapshots derived from it.

3. **Throughput EWMA daemon-side.** 1-second exponential moving
   average over the byte counter so every subscriber sees
   identical numbers. Computed in the table updater, not in
   each subscriber.

4. **Plumbing `transfer_id` through the existing handlers.**
   `transfer_id` is minted at the dispatch boundary in
   `service/core.rs` (next to where `metrics.inc_push()` fires)
   and threaded through `handle_push_stream` / `stream_pull` /
   `handle_pull_sync_stream` / `handle_delegated_pull` to the
   write loops. Roughly the same plumbing path the §3.1
   `--metrics` work already added, but with the id rather than
   just a `started` Instant.

Effort estimate moves Milestone C from "~800 daemon + ~500 TUI"
in the earlier draft to **~1500 daemon + ~500 TUI** (table +
state machine + byte-level instrumentation + the broadcast).
TUI side is unchanged — once events arrive the renderer is the
same.

**What this implicitly buys for the CLI:** once byte-level
progress is on the wire, the CLI's existing progress bar
(today driven by file-complete events from
`report_file_complete`) can become a true byte-level bar by
consuming `Subscribe` against the local daemon — or by routing
its own progress events through the same instrumentation. That's
not a Phase 5 deliverable, but it's a free byproduct.

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
atomics — no new bookkeeping.

**`ActiveJobs` table — introduced here in Milestone B.** A
`Mutex<HashMap<String, ActiveJob>>` on `BlitService` keyed by
`transfer_id`. Populated at the dispatch boundary in
`service/core.rs` (next to where `metrics.inc_push()` fires —
same site that mints the transfer_id). Drained on
TransferComplete / TransferError. `GetState.active[]` reads
from this table directly. Recent transfers live in a parallel
in-memory ring of size `recent_limit` (default 50), pushed
when an `ActiveJob` drains.

Always on regardless of `--metrics` (see §6.4). Cost is a small
per-job allocation plus a HashMap insert/remove on transfer
boundaries; cheap relative to the transfer cost itself.

In Milestone B the table carries only the snapshot fields shown
in `ActiveTransfer` above. M-Jobs (§6.5) extends each row with
a `CancellationToken` so `CancelJob` can fire it; C (§6.2)
extends the row with byte-level progress data fed from the
write-loop instrumentation. Same table, growing payload across
milestones.

Persistence (durability across daemon restart) is deferred to
0.2.0+ — see §10 open questions.

### 6.4 Wire-surface impact on `--metrics`

`--metrics` ships unchanged from the §3.1 owner decision:
- gates atomic counter increments (`push_operations` &c.)
- gates the per-RPC stderr summary lines

It does **not** gate the TUI observability surface. `GetState`
and `Subscribe` always work; an operator running the daemon
expects to be able to ask it what it's doing without a separate
opt-in. When `--metrics` is off the `GetState.counters` block
returns zero (because the underlying atomics weren't
incremented), but `GetState.active[]` / `GetState.recent[]` and
the Subscribe event stream remain fully populated — those come
from the always-on `ActiveJobs` table (§6.5), not from
`TransferMetrics`.

This is a refinement of the original draft, which claimed
`--metrics` gated event emission. Doing that would make the TUI
mostly useless against daemons that hadn't opted in, which is
the wrong default for the single-pane-of-glass model.

### 6.5 `CancelJob` + `detach` — daemon-owned transfer lifecycle

Today the daemon ties transfer lifetime to the initiating
client's connection. The push/pull/pull_sync spawn closures plus
the `delegated_pull` cancellation race (R30-F2) all kill the
transfer when the originating stream closes. That's the right
default for CLI users (`blit copy ... && echo done` should mean
done) but exactly wrong for a TUI that wants to act as a network
monitor.

**Why `--detach` is delegated-pull-only** (correction to the
earlier draft).

Three transfer shapes exist on the wire today:

| Shape | Byte source | Byte sink | Initiator role | Detachable? |
|---|---|---|---|---|
| Push | local (CLI) | remote daemon | initiator IS the byte source | ❌ CLI exit → no bytes |
| PullSync | remote daemon | local (CLI) | initiator IS the byte sink | ❌ CLI exit → no recipient |
| DelegatedPull | remote daemon | remote daemon | initiator is control-plane only | ✅ CLI exit → bytes still flow |

Only DelegatedPull (remote→remote) is structurally detachable
— the CLI isn't in the byte path, so it can drop the control
stream and the transfer continues. Push and PullSync put the
CLI directly in the byte path; detach is architecturally
impossible without first lifting the bytes off the CLI (which
would mean re-architecting push/pull as daemon-to-daemon
DelegatedPull variants — out of Phase 5 scope).

So `detach` is a **delegated-only** feature in M-Jobs.

**M-Jobs introduces** (in this order):

1. **`detach: bool` field on `DelegatedPullRequest`.** Field
   lives on the delegated-pull request itself, **not** on
   `TransferOperationSpec`. Reason: the spec is shared with
   PullSync, where the bit can never mean anything; putting it
   on the universal spec would create a footgun where a
   `pull_sync` client could set `detach=true` and confuse the
   daemon. Local-endpoint paths (push / pull_sync) don't get
   the field at all.

   The CLI's `--detach` flag is **only accepted on
   remote→remote transfers**; on push / pull / pull_sync it
   produces a clean error message ("--detach only applies to
   remote→remote transfers; the CLI is in the byte path for
   this transfer shape"). The TUI uses `detach=true` on every
   transfer it initiates against a remote→remote pair; for
   one-local-endpoint transfers initiated from the TUI, the
   TUI stays connected for the duration (same constraint as
   the CLI).

   Note: the local↔daemon transfer shapes in the TUI's F3
   "trigger transfer" modal will need a UI affordance to make
   clear that closing the TUI cancels the transfer — different
   from the remote→remote case. Probably a banner on the
   transfer-options modal. Detail in A.1.

2. **Cancellation-handle field on `ActiveJob`** (table itself
   ships in Milestone B — see §6.3 ownership note). Each
   `ActiveJob` row gains a `CancellationToken` field. Milestone
   B populated the table for `GetState.active[]` visibility;
   M-Jobs extends each row with the cancel handle that
   `CancelJob` fires. Same per-job allocation, just one more
   field.

3. **Spawn-closure lifecycle change.** The `delegated_pull`
   dispatch site consults `req.detach`. When false (default),
   behavior is unchanged — `tx.closed()` cancellation race
   still arms (R30-F2). When true, the cancellation race is
   **disarmed**: the daemon owns the transfer through to
   completion or `CancelJob(id)`. Push / pull / pull_sync sites
   are unchanged; they cannot detach (see "Why detach is
   delegated-only" above).

4. **`CancelJob` RPC.**

   ```protobuf
   service Blit {
     // ...
     rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
   }

   message CancelJobRequest {
     string transfer_id = 1;
   }

   message CancelJobResponse {
     // True if the job existed and cancellation was initiated.
     // False if the transfer_id wasn't found (already complete,
     // never existed, or wrong daemon).
     bool cancelled = 1;
     // Human-readable reason; empty when cancelled = true.
     string reason = 2;
   }
   ```

   Implementation reaches into the `ActiveJobs` entry, fires a
   `CancellationToken` the spawn closure is watching, and waits
   briefly for the transfer to wind down (sink finish, partial-
   file cleanup if any). Returns once the job is removed from
   the active table or after a 5-second timeout, whichever
   first.

5. **Per-job event ring (~50 events).** Each `ActiveJob` keeps
   the last N events seen for that job in addition to the
   global event ring. `Subscribe` with `transfer_id_filter`
   set replays this ring on connect, then attaches to the live
   broadcast. Gives a TUI that connects mid-transfer the
   bytes-completed history it needs to render a sensible
   progress bar.

**Explicitly NOT in M-Jobs** (deferred to 0.2.0+):

- **Durability across daemon restart.** Job state lives in
  memory only. If the daemon restarts, in-flight detached
  transfers fail; the user re-kicks; block-level resume picks
  up where it left off. On-disk job state + replay-on-startup
  is a heavier 0.2.0 design.
- **Cross-daemon job aggregation.** Each daemon owns its own
  jobs. A TUI that wants "show me everything running on every
  daemon I can see" opens N parallel Subscribe streams and
  aggregates client-side.

**CLI surface (`--detach` + `blit jobs`):**

- `blit copy <src> <dst> --detach` — initiates, prints
  `transfer_id`, exits. Daemon owns the transfer.
- `blit jobs list <remote>` — calls `GetState` against
  `<remote>`, prints `active` + `recent`.
- `blit jobs cancel <remote> <transfer_id>` — calls `CancelJob`.
- `blit jobs watch <remote> <transfer_id>` — opens `Subscribe`
  with `transfer_id_filter` and renders progress until the job
  completes. Useful for "I detached, want to follow now."

All four are thin CLI wrappers over the new RPCs; the TUI uses
the same wire surface.

## 7. Crate / dependency shape

### 7.1 Target shape

Two new crates land in this phase:

- **`crates/blit-app`** — library only. Houses the orchestration
  glue currently buried inside `blit-cli`'s binary modules:
  endpoint parsing, transfer dispatch (local vs push vs pull vs
  delegated), filter assembly from inputs, the local-tree
  comparison core, the diagnostics-dump emitter. **No** clap,
  **no** indicatif, **no** stdout formatting — pure
  programmatic API returning structured results.
- **`crates/blit-tui`** — binary, depends on `blit-app` +
  `blit-core` + `ratatui`. Renders four screens (§5), wires
  events into the `ratatui` event loop, dispatches transfer
  actions into `blit-app`.

`blit-cli` is rewritten as a thin wrapper over `blit-app`:
- clap argument parsing (unchanged)
- indicatif progress-bar rendering driven by `blit-app` events
- stdout formatters for human / JSON output (unchanged)
- routes every verb through the `blit_app::*` entry points

### 7.2 Why `blit-app` is a prerequisite (not an optional polish)

Today `blit-cli` is a **binary-only** crate (no `[lib]` in
`crates/blit-cli/Cargo.toml`). Every reusable orchestration
entry point lives under `crates/blit-cli/src/<module>.rs` and is
reachable only from `main.rs`. Examples:

- `transfers::mod::run_transfer_dispatch` (the local↔remote↔
  remote-remote routing core)
- `transfers::local::run_local_transfer`
- `transfers::remote::run_remote_push_transfer`,
  `run_remote_pull_transfer`
- `transfers::remote_remote_direct::run_remote_remote_direct`
- `transfers::endpoints::Endpoint::parse`
- `check::run_check` (local tree compare)
- `diagnostics::run_dump`

A sibling `blit-tui` crate **cannot import any of these
directly** — they're not exported as a library. The reviewer
flagged this in the round that produced this section.

The three available paths:

1. **Add `[lib]` to `blit-cli`.** Cheapest. But the existing
   modules are coupled to `clap::Args` shapes (`TransferArgs`,
   `CheckArgs`, etc.), indicatif progress bars, and `println!`
   summary formatting. The TUI would inherit all of that — it
   needs custom progress + custom rendering, not the CLI's.
2. **Extract a clean `blit-app` library.** More work, but right
   architecture: orchestration owns the work, presentation
   owns the rendering. The CLI becomes one presenter; the TUI
   becomes another; a future GUI / web client becomes a third.
3. **Shell out from `blit-tui` to `blit`.** Subprocess
   management, parsing `--json` output, no shared progress
   channel, fragile error handling. Rejected.

**Decision: option 2 — extract `blit-app`.** This is the
right boundary, and it pays off again as soon as a third
presenter (web/GUI) becomes interesting.

### 7.3 What moves to `blit-app` (concretely)

A.0 moves **every** orchestration verb, not just transfer. The
TUI's F1 / F3 / F4 screens drive scan / ls / find / du / df /
rm / list-modules / profile in addition to transfers, and all
of those modules are binary-only in `blit-cli` today. Splitting
A.0 into "transfer core now, admin/browser later" would just
defer the same refactor a second time before A.1 could start —
not worth it.

| From | To |
|---|---|
| **Transfer core** | |
| `crates/blit-cli/src/transfers/mod.rs` (dispatch + filter helpers) | `blit-app::transfers::{dispatch, filter}` |
| `crates/blit-cli/src/transfers/local.rs` (core; sans indicatif) | `blit-app::transfers::local` |
| `crates/blit-cli/src/transfers/remote.rs` (core; sans indicatif/println) | `blit-app::transfers::remote` |
| `crates/blit-cli/src/transfers/remote_remote_direct.rs` | `blit-app::transfers::remote_remote_direct` |
| `crates/blit-cli/src/transfers/endpoints.rs` | `blit-app::endpoints` |
| Progress events the orchestration emits | `blit-app::progress::AppProgressEvent` (new — channel-based) |
| **Browser / admin verbs** | |
| `crates/blit-cli/src/scan.rs` (mDNS discovery core) | `blit-app::scan` |
| `crates/blit-cli/src/list_modules.rs` (ListModules RPC client) | `blit-app::admin::list_modules` |
| `crates/blit-cli/src/ls.rs` (List RPC client + local fallback) | `blit-app::admin::ls` |
| `crates/blit-cli/src/find.rs` (Find RPC client) | `blit-app::admin::find` |
| `crates/blit-cli/src/du.rs` (DiskUsage RPC client) | `blit-app::admin::du` |
| `crates/blit-cli/src/df.rs` (FilesystemStats RPC client) | `blit-app::admin::df` |
| `crates/blit-cli/src/rm.rs` (Purge RPC client) | `blit-app::admin::rm` |
| **Verify + diagnostics + profile** | |
| `crates/blit-cli/src/check.rs` (local-tree compare core; sans clap args) | `blit-app::check` |
| `crates/blit-cli/src/diagnostics.rs` (dump emitter + perf toggles) | `blit-app::diagnostics::{dump, perf}` |
| `crates/blit-cli/src/profile.rs` (perf-history summarizer) | `blit-app::profile` |

What stays in `blit-cli`:
- `cli.rs` — clap definitions and arg structs
- `main.rs` — the dispatcher mapping clap args to `blit-app`
- `completions.rs` — `clap_complete` shell completion generation
  (clap-coupled by definition)
- `context.rs` — config / env wiring (CLI-shaped)
- the indicatif progress integration (consumes
  `AppProgressEvent`)
- the human / JSON output formatters for every verb (stdout
  rendering is presenter-specific)

What `blit-tui` adds:
- ratatui screens (§5)
- its own consumer of `AppProgressEvent` that pushes events to
  the UI event loop
- its own rendering layer (no shared formatters with the CLI;
  the TUI wants tables / progress bars, not stdout text)
- a small input-modal system for collecting source / destination
  / option choices

**Scope impact.** Adding the nine admin/browser/profile modules
to A.0 grows the refactor from "~2–3 days" to **~4–5 days**.
Most of those modules are thin RPC-client wrappers (du / df /
ls / find / rm), so the per-module move is mechanical:
extract the core function, drop the clap-args parameter, drop
the `println!` formatting, leave a stub in `blit-cli` that wires
clap args to the new function and renders the result. Test
coverage in `blit-cli/tests/` and `blit_utils.rs` must continue
to pass unchanged — that's the green-light condition for A.0
landing.

### 7.4 Progress event boundary

Today the CLI's progress rendering is wired in two places:
indicatif `ProgressBar` updates driven by core `ProgressEvent`s
(`crates/blit-core/src/remote/transfer/progress.rs`), and ad-hoc
`println!` calls in `transfers/*.rs` around start / completion.

`blit-app` exposes a single event channel pattern. Every
orchestration entry point takes an
`Option<mpsc::UnboundedSender<AppProgressEvent>>` (channel-based,
mirroring `RemoteTransferProgress`'s existing shape). Both
`blit-cli` and `blit-tui` plug their own consumers; the
orchestration code doesn't care which.

State-machine ownership splits across two milestones:
- **M-Jobs** introduces the always-on `ActiveJobs` table
  (§6.5). After M-Jobs every transfer has a daemon-side
  identity, a cancellation handle, and a place to drop events.
  No byte-level data flowing yet — the table is fed by today's
  file-complete-granularity events.
- **C** adds the byte-level instrumentation that fills the
  `ActiveJob.bytes_completed` field meaningfully and lets
  `TransferProgress` carry useful throughput. See §6.2 for the
  four pieces of write-loop work.

This split is why M-Jobs lands before C in the new sequencing:
the table is what `CancelJob` cancels against and what
`Subscribe.transfer_id_filter` indexes into. Byte-level fill is
a quality-of-progress upgrade on top.

### 7.5 Scope of the prerequisite

This isn't a one-evening refactor — it's a few-day chunk:
- Decide the trait shape for `AppProgressSink` (or pick a
  channel-based design).
- Move each module, splitting clap/output coupling out.
- Re-wire `blit-cli/src/main.rs` to call through `blit-app`.
- Re-run the workspace test suite. Most of the test surface is
  in `blit-core` and integration tests under `blit-cli/tests/`
  — those need to keep working without changes.
- Update CI / packaging scripts that reference module paths.

Rough estimate: 2–3 days of focused work. **Belongs in
Milestone A**, before any TUI screen code is written. Treat it
as the "Milestone A.0" preamble.

## 8. Milestones (each independently shippable)

**Sequencing (owner-confirmed 2026-05-14):** foundation-first.
The TUI's first release ships as a real single-pane-of-glass —
discover daemons, watch transfers initiated anywhere, cancel
them, kick off new ones that survive closing the laptop. That
requires job-lifecycle work landed daemon-side before TUI
screens exist. Hence A.0 → B → M-Jobs → C → A.1 → D → E.

### Milestone A.0 — Extract `crates/blit-app` library

Refactor only. Move orchestration glue out of `blit-cli`'s
binary modules so a sibling `blit-tui` crate can import it. No
behavior change. See §7.2 for rationale and §7.3 for the
file-level mapping. Workspace test suite must stay green; CI /
packaging scripts that reference module paths get updated.

CLI keeps working unchanged. Library now consumable.

~2–3 days.

### Milestone B — `GetState` RPC + `ActiveJobs` table + recent ring

Proto change adding `GetState`. Daemon-side: introduces the
always-on `ActiveJobs` table (§6.3) populated from the dispatch
boundary, plus the `recent` ring buffer. `GetState.counters`
reads from existing `TransferMetrics`. **No** cancellation
handles yet, **no** detach lifecycle — just the snapshot
surface.

CLI gets a new `blit jobs list <remote>` admin verb that
consumes the new RPC.

No TUI screens yet, but the daemon now has the per-transfer
identity infrastructure M-Jobs builds on. Each subsequent
milestone (M-Jobs, C) extends rows in the same table.

### Milestone M-Jobs — Daemon-owned transfer lifecycle

Extends the `ActiveJobs` table (which Milestone B introduced)
with the cancellation + detach lifecycle bits. Adds (see §6.5):

- `detach: bool` field on `DelegatedPullRequest`
  (delegated-only — see "Why detach is delegated-only" in §6.5)
- `CancellationToken` field on each `ActiveJob` row
- Spawn-closure lifecycle change in `delegated_pull` only:
  when `detach=true`, the `tx.closed()` cancellation race
  disarms; transfer owned by the daemon to completion or
  `CancelJob(id)`
- `CancelJob(transfer_id)` RPC
- Per-job event ring inside each `ActiveJob` row
- `transfer_id_filter` field on `SubscribeRequest` (defined
  here even though `Subscribe` itself lands in C)
- CLI surface:
  - `blit copy / mirror / move ... --detach` — accepted only
    on remote→remote transfers; rejected with a clear error
    on push / pull / pull_sync transfers (CLI is in the byte
    path for those)
  - `blit jobs cancel <remote> <transfer_id>`
  - `blit jobs watch <remote> <transfer_id>` — uses Subscribe
    (when C lands) or `GetState` polling as a stopgap

After M-Jobs the daemon is fully ready to be a network resource
the TUI can drive — for remote→remote workloads. Local-endpoint
transfers still tie their lifetime to the TUI / CLI process by
necessity.

### Milestone C — `Subscribe` RPC + byte-level instrumentation

Proto change adding `Subscribe` + `DaemonEvent` family, daemon-
side `tokio::broadcast`, the four byte-level progress
instrumentation pieces (§6.2 steps 1–4). After C the daemon
emits live byte-level progress that any subscriber renders.

`blit jobs watch` upgrades from polling to streaming.

### Milestone A.1 — The TUI itself

Now the TUI screens land. All foundation is in place:
- F1 Daemons: mDNS list, per-daemon detail pane lit up by
  `GetState`, "local" sentinel endpoint as a first-class row
- F2 Transfers: active pane fed by `Subscribe`, history fed by
  `GetState.recent`. Watches transfers initiated by anyone on
  the network. Cancel hotkey fires `CancelJob`.
- F3 Browse: `List`/`Find`/`DiskUsage`/`FilesystemStats` per
  selected daemon. Multi-select + `c`/`m`/`v`/`D` modal actions
  dispatch through `blit_app` with `detach=true`.
- F4 Profile: reads `~/.config/blit/perf_local.jsonl` directly.

Result: real single-pane-of-glass from day one.

### Milestone D — Verify + diagnostics screens

F4 Verify (wraps `blit check`, local-only — see §10 Q7),
diagnostics-dump action. TUI-side glue.

### Milestone E — Polish

Theme support, dark/light, configurable refresh rates, key
remapping, JSON config for default endpoints, optional
Prometheus bridge as a separate binary scraping `GetState`.

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

Decisions already taken (owner sign-off 2026-05-14):

- **Crate split:** separate `blit-tui` crate using `ratatui`.
  Not bundled into the default `blit` binary.
- **Local-only TUI mode is first-class.** `blit-tui` works
  without any daemon on the network. "Local" appears in the F1
  daemon list as a sentinel row so the F3 / F2 flows treat it
  symmetrically with remote daemons. Driving a `blit copy`
  between two local paths must work from inside the TUI.

More decisions taken (owner sign-off 2026-05-14):

- **Foundation-first milestone order.** A.0 → B → M-Jobs → C →
  A.1 → D → E. TUI ships as a real network resource from its
  first release.
- **Cancellation: server-side via `CancelJob`.** TUI's cancel
  hotkey fires `CancelJob(transfer_id)` (M-Jobs, §6.5). Works
  on transfers the TUI didn't initiate. Replaces the original
  draft's "client-side Ctrl-C" approach.
- **CLI `--detach` ships with M-Jobs.** Plus `blit jobs list /
  cancel / watch` admin verbs.
- **`AppProgressEvent` is channel-based** (`mpsc::UnboundedSender`),
  mirroring the existing `RemoteTransferProgress` shape. See
  §7.4.

Still open, listed in the order they become decision-blockers:

1. **Recent-transfer persistence.** `GetState.recent[]` populated
   from an in-memory ring (cheap, lost on restart) or from
   `perf_local.jsonl` (durable, reuses existing storage)?
   **Blocker for: Milestone B.** Recommendation: in-memory ring
   for B; if persistence is wanted later, reuse `perf_local`
   in Milestone E.

2. **`Subscribe` subscriber cap.** N concurrent subscribers,
   N=? Default 8 seems reasonable for a single-operator TUI.
   **Blocker for: Milestone C.**

3. **`TransferProgress` cadence.** 10 Hz default, configurable
   via `SubscribeRequest`? Or fixed? Higher cadence is nice for
   the TUI but costs more in broadcast traffic AND in the
   byte-counter write-loop overhead (see §6.2 step 1).
   **Blocker for: Milestone C.**

4. **Multi-daemon Subscribe.** TUI watches N daemons
   simultaneously by opening N Subscribe streams; aggregation
   happens client-side. Is that ok or should the TUI talk to a
   designated "primary" daemon that fans out? Recommend simple
   N-streams approach. **Blocker for: Milestone C.**

5. **Transfer-id allocation.** UUIDv4 daemon-side at the
   dispatch boundary (next to `metrics.inc_push()`), stable for
   the duration of the RPC, surfaced to the initiating client
   via the existing summary path. **Decision: take this default
   unless owner objects — daemon-internal, low-risk.**

6. **Remote tree verify (F4 Verify scope expansion).** Today's
   `blit check` (`crates/blit-cli/src/check.rs`) calls
   `Path::exists()` directly on both inputs — local paths only.
   Extending F4 Verify to remote endpoints requires a new tree-
   compare affordance: either (a) the TUI streams two
   `Find`/`List` manifests, diffs them client-side, and renders
   the result; or (b) the daemon grows a `CompareTrees` RPC that
   does the diff server-side and returns a structured report.
   (a) is cheaper but slower for large trees; (b) needs new
   wire. **Recommend: ship F4 Verify local-only in Milestone D
   (matches today's `blit check`); revisit remote-verify in a
   later milestone if operator demand justifies it.**

7. **Job durability across daemon restart.** Out of Phase 5
   scope per §6.5. If the daemon restarts, in-flight detached
   jobs fail; the TUI shows them as errored; the user re-kicks
   and block-level resume picks up. Worth revisiting for 0.2.0
   with an on-disk job-state design if operators report pain.

## 11. Phasing summary

| # | Milestone | Wire changes | LOC band | Independently useful? |
|---|---|---|---|---|
| 1 | A.0 — extract `blit-app` (full verb surface; see §7.3) | none (refactor only) | ~0 net; ~4–5 days of mechanical moves | ✅ CLI keeps working; library now consumable |
| 2 | B — `GetState` + `ActiveJobs` table + recent ring | +`GetState` | ~500 daemon + ~100 CLI (`jobs list`) | ✅ Daemon introspectable from `blit jobs list` |
| 3 | M-Jobs — daemon-owned lifecycle (delegated-only) + `CancelJob` + `detach` | +`CancelJob`, +`detach` on `DelegatedPullRequest`, +`transfer_id_filter` on `SubscribeRequest` (defined here) | ~500 daemon + ~200 CLI (`--detach` on remote→remote only, `jobs cancel/watch`) | ✅ Remote→remote transfers detachable; CLI gains cancel; daemon ready for TUI |
| 4 | C — `Subscribe` + byte-level instrumentation | +`Subscribe` | ~1500 daemon + ~100 CLI (`jobs watch` upgrade) | ✅ Live byte-level progress on the wire |
| 5 | A.1 — the TUI itself | none | ~3000 (new `blit-tui` crate, four screens, event integration) | ✅ Single-pane-of-glass |
| 6 | D — Verify + diagnostics | none | ~400 TUI | ✅ |
| 7 | E — polish | none (optional Prometheus bridge is a separate binary) | ~600 | ✅ |

Total roughly 7–8 kLOC for the full feature surface (plus the
A.0 refactor, which nets near zero new code — it relocates
existing modules into a library crate). The TUI itself (A.1)
ships as **the fifth milestone**, on top of a daemon that has
already been turned into a network resource by milestones 2–4.
The CLI gets meaningful capability upgrades at every milestone
along the way (`jobs list`, `--detach` on remote→remote,
`jobs cancel`, `jobs watch`), so the runway to A.1 is itself
useful work — not purely scaffolding for the TUI.

**Detach scope reminder**: `--detach` and daemon-owned
lifecycle only apply to remote→remote (delegated) transfers.
Push and pull transfers tie their lifetime to the CLI / TUI
process by architectural necessity — the initiator is in the
byte path. See §6.5 "Why detach is delegated-only."

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
- Three new RPCs (`GetState`, `Subscribe`, `CancelJob`) plus
  the `detach: bool` field on `DelegatedPullRequest` and the
  `transfer_id_filter` field on `SubscribeRequest` — names and
  message fields are the contract.
- Always-on `ActiveJobs` table on `BlitService` (decoupled from
  `--metrics`). Ships in Milestone B; M-Jobs and C extend rows.
- Daemon-owned transfer lifecycle for remote→remote (delegated)
  transfers when `detach=true`; client-owned (today's behavior)
  for everything else and for `detach=false` (CLI default).
- Separate `blit-app` library crate and separate `blit-tui`
  binary crate.
- Reuse the unified pipeline; no shadow transfer code.
- Foundation-first milestone order: A.0 → B → M-Jobs → C → A.1
  → D → E.

Everything else is a tuning knob.
