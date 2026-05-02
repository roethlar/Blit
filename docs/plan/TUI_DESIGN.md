# Blit TUI Design

**Status:** Planning. No code yet.
**Author:** Drafted 2026-05-01 alongside the metrics-counter scaffolding.

## Purpose

A terminal UI for an interactive operator who wants to:
- See what daemons are reachable on the LAN (mDNS-discovered).
- Watch in-progress transfers with live progress, throughput, ETA.
- Browse modules and trigger transfers from the same window.
- Inspect daemon health: uptime, recent errors, aggregate counters.

The TUI is a **gRPC client** of one or more daemons. It is not a daemon itself
and does not introduce new wire formats. Everything it consumes is also
consumable by a future GUI on the same protocol — this design intentionally
avoids TUI-specific RPCs.

## What lives where

| Need | Source | Mechanism |
|---|---|---|
| Daemon discovery | mDNS | `_blit._tcp.local.` (already advertised) |
| Daemon identity | mDNS TXT | **needs enrichment** — see §"mDNS TXT" |
| Module list | gRPC `ListModules` | already exists |
| File browse | gRPC `List` / `Find` / `Du` | already exists |
| Live transfer events | gRPC `Subscribe` | **new RPC** — see §"Subscribe RPC" |
| Snapshot state | gRPC `GetState` | **new RPC** — see §"GetState RPC" |
| Aggregate counters | `TransferMetrics` (already on `BlitService`) | exposed via `GetState` |
| Trigger transfer | gRPC `Push` / `PullSync` | already exists; the TUI just kicks one off as a normal client |

## Design principles

1. **Reuse the unified pipeline.** The TUI's "trigger a transfer" action
   instantiates a normal `RemotePushClient` / `RemotePullClient` and hands
   off to the same code path the CLI uses. No alternate transfer code.

2. **No new wire format.** Everything the TUI needs is structured gRPC.
   Counters travel over the same channel as everything else; no second
   listener, no second protocol.

3. **Progressive enhancement.** The TUI can be useful with what exists
   today (mDNS scan, `List`, `Du`). Live event streaming and rich state
   are additions that ship as the TUI grows. Don't preemptively bolt
   them on the daemon ahead of consumers.

4. **Counters are calculated, not surfaced.** `TransferMetrics` is internal
   state today (already wired). When the TUI needs aggregate counts, the
   daemon exposes them through `GetState` — single chokepoint, no
   side-channel.

## mDNS TXT enrichment (small, do early)

Today's advertisement is bare: `_blit._tcp.local.` with the port. A TUI
listing reachable daemons benefits from richer per-record info **without
opening a connection** to each.

Add to the TXT record:

| Key | Value | Use |
|---|---|---|
| `v` | daemon version (`0.1.0`) | TUI shows "out-of-date" warning |
| `mods` | comma-joined module names, truncated to ~120 bytes | TUI shows modules without an RPC roundtrip |
| `nmods` | numeric module count | accurate total when `mods` was truncated |
| `caps` | bitfield of capability flags (e.g. `grpc=1,ck=1`) | TUI dims options for unsupported features |

**Effort:** ~30 LOC in `blit-core/src/mdns.rs` plus 10 LOC in `blit-utils`-merged
`scan` parsing. No new attack surface (mDNS is already there). Worth doing
ahead of the TUI work — even today's `blit scan` output gets better.

## Subscribe RPC (the live event stream)

Single new server-streaming RPC the TUI needs:

```protobuf
service Blit {
  // ...existing RPCs...
  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
}

message SubscribeRequest {
  // Bitfield of event categories the client wants.
  uint32 event_mask = 1;  // 0=all, else: TRANSFERS=1, ERRORS=2, MODULES=4
  // If set, replay buffered recent events on connect for context.
  bool replay_recent = 2;
}

message DaemonEvent {
  oneof payload {
    TransferStarted   transfer_started   = 1;
    TransferProgress  transfer_progress  = 2;
    TransferComplete  transfer_complete  = 3;
    TransferError     transfer_error     = 4;
    ModuleListChanged module_list_changed = 5;
    DaemonHeartbeat   heartbeat          = 6;
  }
}
```

The daemon already produces these events internally (the push/pull
handlers know when transfers start, when bytes flow, when they complete).
A `tokio::broadcast` inside `BlitService` collects them; `Subscribe`
fans them out to subscribed clients. Clients drop on lag — they're
observers, not part of the transfer hot path.

**When to build:** when the TUI's "live transfer pane" needs implementing.
Not before.

## GetState RPC (the snapshot)

```protobuf
service Blit {
  // ...
  rpc GetState(GetStateRequest) returns (DaemonState);
}

message DaemonState {
  string version = 1;
  uint64 uptime_seconds = 2;
  repeated ModuleInfo modules = 3;
  repeated ActiveTransfer active = 4;       // currently running
  repeated TransferRecord recent = 5;        // last N completed (configurable)
  Counters counters = 6;
}

message Counters {
  uint64 push_operations_total = 1;
  uint64 pull_operations_total = 2;
  uint64 purge_operations_total = 3;
  uint64 active_transfers = 4;
  uint64 transfer_errors_total = 5;
  // Future: bytes_transferred_total, files_transferred_total once the
  // sink layer naturally feeds them.
}
```

`Counters` reads directly from the existing `TransferMetrics` struct.
Active and recent transfers are tracked in `BlitService` via the same
mechanism that produces `Subscribe` events. No new bookkeeping.

**When to build:** when the TUI's "dashboard" view needs implementing.

## TUI structure (sketch)

Three primary panes, switchable with hotkeys:

```
┌─ blit ────────────────────────────────── 3 daemons reachable ─┐
│                                                                │
│  [F1] Daemons   [F2] Transfers   [F3] Browse                   │
│                                                                │
│  ┌─ Daemons ──────────────────────────────────────────────┐   │
│  │ * mycroft    192.168.1.10:9031  v0.1.0  3 modules     │   │
│  │   skippy     192.168.1.20:9031  v0.1.0  1 module      │   │
│  │   elphaba    192.168.1.30:9031  v0.0.9  unknown       │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                │
│  Selected: mycroft                                             │
│  ┌─ State ────────────────────────────────────────────────┐   │
│  │ uptime: 2d 4h 17m         active transfers: 1          │   │
│  │ ops: 142 push / 88 pull / 3 purge  errors: 1           │   │
│  │ modules: home, backups, media                          │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                │
│  ┌─ Active ───────────────────────────────────────────────┐   │
│  │ → home/photos/2024  47%  84.3 MiB/s  ETA 3m 12s        │   │
│  └────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
```

- **F1 Daemons:** mDNS-driven list; per-daemon TXT info renders inline
  (no RPC needed for the list itself, only on selection).
- **F2 Transfers:** subscribed `Subscribe` stream renders rolling history;
  per-row progress bar from `TransferProgress` events.
- **F3 Browse:** uses existing `List` / `Du` / `Find` RPCs against the
  selected daemon. Trigger pull/push from here.

## What to build, in order

1. **mDNS TXT enrichment** — useful today, no consumer dependency. Do early.
2. **TUI scaffolding** (probably new `blit-tui` crate using `ratatui`).
   F1 (daemon list) works against the existing daemon — no daemon changes.
3. **`GetState` RPC + daemon-side bookkeeping** for active/recent transfers.
   F1's per-daemon info pane lights up.
4. **`Subscribe` RPC + broadcast plumbing.** F2 (live transfers) lights up.
5. **F3 Browse + trigger.** Mostly TUI-side; reuses existing RPCs.

Each step is independently shippable — you get a useful TUI at step 2.

## Non-goals

- Prometheus / OpenMetrics scraping. Counters live behind gRPC `GetState`.
  If an operator wants Prometheus, write a small external bridge that
  scrapes `GetState` and re-exposes — that bridge is a separate program,
  not the daemon's responsibility.
- HTTP endpoints of any kind on the daemon.
- A web UI. The browser-shaped client comes later, on the same gRPC.
- Daemon-initiated push (i.e. daemon-to-TUI). Subscribe is server→client
  streaming over a client-initiated connection.

## Open questions

- **Where do recent transfers persist?** In-memory ring buffer (lost on
  restart) is simplest. JSONL on disk is durable but overlap with
  `perf_history`. Probably reuse `perf_history` and surface via `GetState`.
- **How many subscribers?** Cap to N concurrent (configurable). Drop the
  oldest on overflow. TUI is single-user usually; defensive bound is fine.
- **mDNS TXT size limits.** Standard is ~255 bytes per record, multiple
  records allowed. Truncate `mods` aggressively; rely on `nmods` for the
  true count.
- **Daemon-side broadcast costs.** A `tokio::broadcast` channel per
  service is cheap. If subscribers lag, they see "missed N events" and
  re-fetch via `GetState`.

## Why this shape

- **No HTTP** — keeps the daemon's surface single-protocol (gRPC + mDNS).
  Reviewer flagged HTTP as needing hardening; we sidestep that entirely.
- **Counters opt-in** — daemons that don't expose state still pay nothing
  for collection. When the TUI is the consumer, the operator opts in.
- **mDNS does heavy lifting for discovery** — the TUI's start-up cost
  is "listen for advertisements, render list." No RPC roundtrip for the
  daemon list, no central registry.
- **Progressive** — every step adds visible value without requiring the
  next step. mDNS enrichment alone makes `blit scan` better today.
