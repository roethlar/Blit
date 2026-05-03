# Remote→Remote Direct Transfer Plan

**Status:** Draft v4, 2026-05-03 (incorporates Round 21 + Round 23 + Round 25 review findings)
**Owner:** mcoelho
**Supersedes:** the "Remote→remote re-evaluation" entry in `TODO.md`'s Deferred design calls.

**v2 changes vs v1 (in response to Round 21):**
- §4.1 — `DelegatedPullRequest` now embeds existing `TransferOperationSpec`
  instead of duplicating fields (R21-F1).
- §4.2 — Daemon handler reuses `RemotePullClient::pull_sync`; removed
  the nonexistent `FilteredSink` (R21-F3).
- §4.3 — New "Delegation gate" subsection: default-disabled config flag
  with host allowlist (R21-F2).
- §4.4–4.5 — Removed silent auto-fallback on `Unimplemented` /
  `Unavailable` / `CONNECT_SOURCE`. `--relay-via-cli` is now the only
  fallback (R21-F5).
- §6 — Byte-path-isolation test redesigned with two independent
  observables; `negotiated_endpoint` is informational only (R21-F7).
- §7 — Removed FilterSpec proto-to-be-added line; clarified
  spec_version/capabilities is the compatibility model (R21-F4, R21-F6).

**v3 changes vs v2 (in response to Round 23):**
- §4.2 — Daemon handler calls a new `RemotePullClient::pull_sync_with_spec`
  that accepts a pre-built `TransferOperationSpec` and forwards it
  unchanged. The existing `pull_sync` becomes a thin wrapper that builds
  the spec from `PullSyncOptions` and delegates. This removes the
  "validate-then-reconstruct" drift surface (R23-F1).
- §4.2, §4.3 — Delegation gate ordering specified explicitly: source
  locator parse → daemon-wide gate → module metadata lookup → per-module
  override → path resolution + F2 containment → outbound connect. No
  filesystem path resolution and no outbound connect before policy
  approves (R23-F2).
- §4.3 — `allowed_source_hosts` matching semantics specified: hostname
  case/dot/punycode normalization, CIDR support, all-resolved-addresses-
  must-pass, IPv6 normalization, DNS-rebinding mitigation via
  resolve-once-connect-by-IP (R23-F3).
- §6 — `source_peer_observed` demoted to diagnostic. CLI-side byte
  counter is the load-bearing byte-path-isolation assertion (R23-F4).

**v4 changes vs v3 (in response to Round 25):**
- §4.2 — `pull_sync_with_spec` extraction widened. Spec construction
  in `pull.rs` includes the endpoint→spec field mapping at lines
  397–409 (module + source_path), not just lines 433–484. The new
  `pull_sync_with_spec` MUST NOT read `self.endpoint.path`; the
  endpoint is transport-only. New endpoint-isolation unit test
  required (R25-F1).
- §4.1, §4.2 — `client_capabilities` is the one field where CLI-
  supplied values are non-authoritative. The destination handler
  mandatorily replaces the field with its own `PeerCapabilities`
  before forwarding the spec to src, because the byte recipient in
  delegation is the dst, not the CLI. Override boundary is
  documented in proto comments and tested explicitly (R25-F2).
- §4.3.3 — Loopback / link-local / unique-local resolved addresses
  require an **IP- or CIDR-form** allowlist entry; a hostname-form
  match alone does not authorize them. Closes the SSRF-via-DNS
  pivot where an attacker-controlled DNS record points an
  allowlisted hostname at the daemon's loopback (R25-F3).

## 1. Context and goal

Today, when a user runs

```
blit copy server-A:/mod/foo server-B:/mod/bar
```

the CLI on the user's host becomes the byte path. It opens a `RemotePullClient` to A
and a `RemotePushClient` to B simultaneously, wires them via `RemoteTransferSource`,
and shuttles every byte through its own memory:

```
                    ┌──── server-A
                    │
   user CLI ◄───────┤  pull data plane (TCP, A is listener)
                    │
                    └──► server-B
                       push data plane (TCP, B is listener)
```

Two independent gRPC streams. Two independent TCP data planes. CLI sits in the middle
holding everything in `BufferPool` chunks. On a LAN where the CLI is on the same fast
network as both daemons, the cost is roughly one extra hop. On a CLI sitting on a slow
link orchestrating two LAN-attached daemons, the CLI link is the bottleneck and pegs
throughput at the slowest of `cli↔A` and `cli↔B` — typically far below the LAN
fabric A and B share.

**Goal:** make `server-A:/x server-B:/y` transfers route the byte path directly
between A and B. The CLI orchestrates and reports; it does not relay bytes.

**Non-goal:** preserve the existing relay path as a fallback (we will keep it as a
fallback for hostile-NAT cases and `--relay-via-cli` benchmarking, but it is not
the default after this work lands).

## 2. Current shape — file/line audit

| What | Where |
|---|---|
| Dispatch table for endpoint pairs | `crates/blit-cli/src/transfers/mod.rs:398` (`(Remote, Remote) → run_remote_push_transfer`) |
| Mirror dispatch | `crates/blit-cli/src/transfers/mod.rs:503` (same path, mirror=true) |
| CLI relay setup | `crates/blit-cli/src/transfers/remote.rs:154-189` builds `RemoteTransferSource` from `RemotePullClient` and feeds it to `RemotePushClient::push` |
| `RemoteTransferSource` (the relay primitive) | `crates/blit-core/src/remote/transfer/source.rs` |
| Push wire protocol | `proto/blit.proto:7` (`Push(stream ClientPushRequest) → stream ServerPushResponse`) |
| PullSync wire protocol | `proto/blit.proto:15` (`PullSync(stream ClientPullMessage) → stream ServerPullMessage`) |
| Data plane: daemon-as-listener | `crates/blit-daemon/src/service/push/data_plane.rs:39` (`TcpListener::bind("0.0.0.0:0")`) |
| Data plane: connector | `crates/blit-core/src/remote/transfer/data_plane.rs:74` (`TcpStream::connect`) |
| Auth surface | `proto/blit.proto:40-42` defined; **zero implementations across the codebase**. No token, no challenge. |
| Per-transfer data-plane token | `DataTransferNegotiation.one_time_token` in `proto/blit.proto:49` — scoped to the single TCP data-plane connect, not to the gRPC control plane. |

Critical observation: the data plane is *already* symmetric. `TcpListener::bind`
on the daemon side and `TcpStream::connect` on the connector side are independent
of which host is "client." A daemon can play the connector role with no protocol
change.

## 3. Design options considered

### Option A — **Push delegation** (recommended)

CLI sends a delegation request to **dst (B)**. B becomes the initiator: it opens a
`RemotePullClient` to A and runs the universal pipeline against itself as the
filesystem sink. Progress streams back to CLI over the original gRPC.

```
   user CLI ───── delegation gRPC ────► server-B
                                         │
                                         │ RemotePullClient (B is now initiator)
                                         ▼
                                       server-A
                                         │
                                         │ pull data plane (TCP, A listener)
                                         ▼
                                       server-B (sink)
```

Why dst, not src: every existing diff/manifest path on Blit lives on the destination
side. `Push` has the dst computing the NeedList; `PullSync` has the dst sending its
manifest and src returning what's missing. Either way, dst is the brain. Making dst
the delegated initiator reuses the most existing code.

The pipeline already supports this — that was the point of the universal pipeline
refactor. We only need to expose "daemon-as-initiator" over the wire.

### Option B — Pull delegation (rejected)

Symmetric: CLI tells **src (A)** to push to B. Rejected because:
1. A would need write-side scheduling and diffing logic it doesn't carry today.
2. Existing pull-from-A semantics would need re-derivation in push-to-B form.
3. Auth credential delegation is harder when the credential issuer (CLI/operator)
   has no relationship with A's view of B.

### Option C — Co-orchestration with TCP rendezvous (rejected)

CLI does control-plane handshakes with both daemons, mints data-plane tokens on
each, and tells one to connect to the other. Rejected because:
1. Two control-plane streams to manage, two error surfaces, more state in CLI.
2. Half the diff/manifest planning ends up on the wrong side or duplicated.
3. The "dst as initiator" model in Option A is strictly simpler and reuses 100%
   of the existing pull/push internals.

### Option D — Keep status quo (rejected per user direction)

Current relay shape works, but the user's standing directive is that pipeline
optimization is the next priority once the case for it is sound. The case is sound:
operator workflows that orchestrate LAN-attached daemons from a remote laptop are
the primary cross-host pattern, and they pay 2–10× CLI-link tax.

## 4. Recommended design — Push delegation in detail

### 4.1 New RPC — embeds the existing `TransferOperationSpec`

The proto comment at `proto/blit.proto:312` already names remote→remote as the
case `TransferOperationSpec` was designed for. We embed that message directly
rather than introducing a parallel flag bag.

```proto
// dst-side delegated initiator. CLI calls this on the destination daemon
// when both endpoints are remote. The server opens a normal pull against
// the named source daemon and runs the universal pipeline locally.
rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);

message DelegatedPullRequest {
  // ── Destination-side fields (this daemon) ─────────────────────────────
  // Module on this (destination) daemon to write into. Resolved through
  // the same module table push uses; F2 canonical-path containment
  // applies to dst_destination_path.
  string dst_module = 1;
  string dst_destination_path = 2;

  // ── Source-side / universal contract ──────────────────────────────────
  // Where this daemon should pull from. Strictly typed; see below.
  RemoteSourceLocator src = 10;

  // The universal transfer contract. Reuses the existing message at
  // proto/blit.proto:314, including FilterSpec, ComparisonMode,
  // MirrorMode, ResumeSettings, force_grpc, ignore_existing, and
  // spec_version. Receivers normalize via
  // `NormalizedTransferOperation::from_spec` exactly like push/pull.
  //
  // OVERRIDE BOUNDARY: `spec.client_capabilities` describes the byte
  // recipient's capabilities. In delegation, the byte recipient is
  // the destination daemon, not the CLI. The destination handler
  // mandatorily REPLACES `client_capabilities` with its own
  // PeerCapabilities before forwarding the spec to the source —
  // any CLI-supplied value here is non-authoritative. Every other
  // field flows through unchanged. (See §4.2 step 8 + spec-authorship
  // subsection.)
  TransferOperationSpec spec = 20;

  // Diagnostics-only.
  bool trace_data_plane = 31;
}

message RemoteSourceLocator {
  // Strict Blit remote endpoint, not arbitrary URI. The daemon parses
  // this through the same RemoteEndpoint code the CLI uses; rejects
  // schemes other than the Blit gRPC control-plane scheme.
  string host = 1;
  uint32 port = 2;

  // Auth-passthrough hook. Empty in 0.1.0 (no auth implemented yet).
  // When BlitAuth lands, the operator-minted, dst-scoped bearer token
  // travels here and dst presents it on its outbound connection to src.
  bytes delegated_credential = 10;
}

message DelegatedPullProgress {
  oneof payload {
    DelegatedPullStarted started = 1;
    ManifestBatch        manifest_batch = 2; // existing message
    BytesProgress        bytes_progress = 3;
    DelegatedPullSummary summary = 4;
    DelegatedPullError   error = 5;
  }
}

message DelegatedPullStarted {
  // Diagnostic: the source-side data-plane endpoint this daemon's pull
  // client observed (its remote-socket peer). "tcp:host:port" when TCP
  // data plane is used; "grpc-fallback" otherwise. Informational only —
  // the load-bearing byte-path-isolation assertion in tests is the
  // CLI-side byte counter (see §6).
  string source_data_plane_endpoint = 1;
  uint32 stream_count = 2;
}

message BytesProgress {
  uint64 files_completed = 1;
  uint64 files_total = 2;
  uint64 bytes_completed = 3;
  uint64 bytes_total = 4;
}

message DelegatedPullSummary {
  // Mirrors PullSummary at proto/blit.proto:149-155.
  uint64 files_transferred = 1;
  uint64 bytes_transferred = 2;
  uint64 bytes_zero_copy = 3;
  bool   tcp_fallback_used = 4;
  uint64 entries_deleted = 5;
  // Diagnostic: the data-plane peer address this dst observed when
  // connecting to src. Useful for operator audit logs. Not a proof of
  // byte-path isolation (a destination can only observe its own local
  // and remote socket addresses; what src actually accepted is what
  // matters). Tests must rely on the CLI-side byte counter and the
  // assertion that RemoteTransferSource is not constructed on the
  // direct path — see §6.
  string source_peer_observed = 6;
}

message DelegatedPullError {
  string upstream_message = 1;
  enum Phase {
    UNKNOWN = 0;
    DELEGATION_REJECTED = 1; // gate denied (see §4.3)
    CONNECT_SOURCE = 2;
    NEGOTIATE = 3;
    TRANSFER = 4;
    APPLY = 5;
  }
  Phase phase = 2;
}
```

`FilterSpec` is **already** defined at `proto/blit.proto:367-392` and the CLI
already produces one through `build_filter_spec` in
`crates/blit-cli/src/transfers/mod.rs`. We reuse both without modification.
Version drift is handled through `TransferOperationSpec.spec_version` and
`PeerCapabilities`, which `NormalizedTransferOperation::from_spec` already
validates at the boundary. We do **not** rely on detecting unknown protobuf
fields (proto3 silently preserves them; that's not a compatibility strategy).

### 4.2 New code paths

**CLI side (`crates/blit-cli/src/transfers/`):**

- New module `remote_remote_direct.rs`:
  ```rust
  pub async fn run_remote_to_remote_direct(
      args: &TransferArgs,
      src: RemoteEndpoint,
      dst: RemoteEndpoint,
      mirror_mode: bool,
  ) -> Result<()>;
  ```
  Builds a `TransferOperationSpec` from `args` using the same helpers
  push/pull use today (`build_filter_spec`, mirror/comparison/resume
  normalization), wraps it in a `DelegatedPullRequest`, opens a control
  stream to dst, and forwards progress events to the existing
  `RemoteTransferProgress` monitor.

  Note on `client_capabilities`: the CLI is **not** the byte recipient
  in delegation, so any `client_capabilities` it puts on the spec is
  non-authoritative. The CLI may leave the field default-zero or fill
  it with conservative values; the destination daemon mandatorily
  rewrites it before forwarding the spec to src (§4.2 step 8). All
  other fields the CLI sets on the spec flow through unchanged.
- `mod.rs:398` and `mod.rs:503` dispatch updated:
  ```rust
  (Endpoint::Remote(src), Endpoint::Remote(dst)) => {
      if args.relay_via_cli {
          // explicit operator escape hatch; preserved for hostile-NAT
          // cases and side-by-side benchmarking.
          run_remote_push_transfer(args, Endpoint::Remote(src.clone()), dst, mirror_mode).await
      } else {
          run_remote_to_remote_direct(args, src, dst, mirror_mode).await
      }
  }
  ```
  No silent fallback. If the destination daemon doesn't support
  `DelegatedPull`, fail with a clear "destination daemon does not implement
  DelegatedPull; use --relay-via-cli or upgrade the destination" error.
  Per the no-backcompat release directive, we do not transparently support
  stale daemons.
- New flag: `--relay-via-cli` (operator-selected; forces the old CLI-relay
  path). Lives on `TransferArgs`.

**Daemon side (`crates/blit-daemon/src/service/`):**

We **reuse the existing target-side pull machinery** rather than building
a parallel sink pipeline. There is no `FilteredSink` (filters are
source-side via `FilteredSource`); the destination's job in a normal pull
is to (a) compute its local manifest, (b) call
`RemotePullClient::pull_sync`, (c) apply incoming chunks and any returned
delete list. The delegated handler does exactly that.

- New file `delegated_pull.rs` with the RPC handler. Steps run in this
  exact order; the invariant is **no path resolution and no outbound
  connect before policy approves** (R23-F2):

  1. **Parse `RemoteSourceLocator`** through the existing
     `RemoteEndpoint` parser. Reject schemes other than the Blit gRPC
     control plane scheme. Reject malformed host/port.
  2. **Spec validation**: validate `spec.spec_version`,
     `PeerCapabilities`, and convert via
     `NormalizedTransferOperation::from_spec` — exactly like the existing
     push and pull handlers. No parallel normalizer. Reject unknown
     versions explicitly.
  3. **Daemon-wide gate** (§4.3): if `allow_delegated_pull == false`,
     return `DelegatedPullError{phase=DELEGATION_REJECTED}` immediately.
     If `allowed_source_hosts` is non-empty, **resolve the source
     hostname to an IP set, validate every resolved address against the
     allowlist, and bind the connection to that resolved IP** (see §4.3
     for full semantics). On failure: same error phase.
  4. **Module metadata lookup** for `dst_module` (no path resolution
     yet). Reject if module unknown or read-only (returning the existing
     read-only-module error code).
  5. **Per-module override**: if the module's delegation-allowed flag is
     `false`, return `DELEGATION_REJECTED`. (The override only narrows
     the daemon-wide policy; it cannot widen it.)
  6. **F2 canonical-path containment** on `dst_destination_path` via
     `resolve_contained_path` in `crates/blit-daemon/src/service/util.rs`.
     Reject contained-path violations at the boundary, before any
     outbound connect.
  7. **Metrics + RAII**: `let _guard = Arc::clone(&metrics).enter_transfer();`
     so the active-transfer gauge and attempts/errors counters stay
     coherent with push/pull (F5 contract).
  8. **Mandatory `client_capabilities` override**: replace
     `spec.client_capabilities` with this destination daemon's actual
     `PeerCapabilities`. The CLI cannot honestly speak for what the
     destination's byte-receive side supports (tar shards, data-plane
     TCP, resume, filter pipeline). This override is unconditional —
     the field is rewritten regardless of what the CLI sent — because
     `client_capabilities` describes the byte recipient and in
     delegation that is the destination, not the CLI. Performed before
     outbound connect so the override is independent of network state.
     (See spec-authorship subsection below for full rationale and
     unit-test requirement.)
  9. **Outbound connect**: open a `RemotePullClient` to the
     IP/port resolved in step 3 using the existing client code in
     `crates/blit-core/src/remote/pull.rs`. If `delegated_credential`
     is non-empty, attach it (Phase 4 wiring; ignored in 0.1.0).
  10. **Run delegated pull**: call
      `RemotePullClient::pull_sync_with_spec(dest_root, local_manifest,
      spec, track_paths, progress)`. This new entry point accepts the
      already-normalized `TransferOperationSpec` (with dst's
      `client_capabilities` from step 8) and forwards it on the wire
      unchanged. Same PullSync framing, ack negotiation (F11/R15-F1),
      data-plane setup, mirror delete list semantics, and tar-shard
      receive safety (R5-F2/R6-F1/R6-F3 via
      `tar_safety::safe_extract_tar_shard`) as a normal pull.
  11. **Progress forwarding**: adapt `pull_sync_with_spec`'s progress
      events into `DelegatedPullProgress` envelopes on the gRPC return
      stream. Backpressure-friendly bounded channel; throttle if needed
      (same pattern as `RemoteTransferProgress`).
  12. **Cancellation**: if the gRPC return stream closes (CLI Ctrl-C),
      drop the `pull_sync_with_spec` future, which propagates
      cancellation through the existing pull-side cleanup. Document that
      delegated pulls are CLI-session-bound; `--detach` is out of
      scope (§9).

#### Required core-side refactor — `pull_sync_with_spec` extraction

`RemotePullClient::pull_sync` (`crates/blit-core/src/remote/pull.rs:374`)
currently takes `PullSyncOptions` and builds a `TransferOperationSpec`
internally. Spec construction in the existing function spans **two**
non-contiguous regions:

1. **Lines 397–409** — derive `module` and `source_path` (as `path_str`)
   from `self.endpoint.path`. These become the spec's `module` and
   `source_path` fields.
2. **Lines 433–484** — derive `compare_mode`, `mirror`, `filter_spec`,
   `resume`, `client_capabilities`, `force_grpc`, `ignore_existing`
   from `options`, and assemble the `TransferOperationSpec`.

The delegation handler must not validate a spec on the wire and then
have those fields silently reconstructed under it. The whole spec —
including module and source_path — must travel verbatim.

Refactor:

```rust
impl RemotePullClient {
    /// Build the spec from CLI-style options. Pure function; testable
    /// in isolation. Reads endpoint to derive module + source_path,
    /// reads options to derive everything else.
    pub(crate) fn build_spec_from_options(
        endpoint: &RemoteEndpoint,
        options: &PullSyncOptions,
    ) -> Result<TransferOperationSpec> {
        // Lift BOTH the lines 397–409 block (module/path derivation
        // from endpoint.path) AND the lines 433–484 block (rest of
        // spec). Returns Result because the Discovery variant of
        // RemotePath bails (line 401).
    }

    /// Pull using a pre-built, normalized spec. The spec travels
    /// over the wire unchanged.
    ///
    /// IMPORTANT: this method MUST NOT read `self.endpoint.path` to
    /// derive any spec field. The endpoint is purely a transport
    /// handle (host:port for the gRPC connection); the spec is
    /// authoritative for module + source_path + every other field.
    /// Touching `endpoint.path` here would reopen the validate-then-
    /// reconstruct hole that motivated this refactor.
    ///
    /// Used by the delegated_pull handler AND by the existing CLI pull
    /// entry point (via the wrapper below).
    pub async fn pull_sync_with_spec(
        &mut self,
        dest_root: &Path,
        local_manifest: Vec<FileHeader>,
        spec: TransferOperationSpec,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
    ) -> Result<RemotePullReport> {
        // Body lifted from current pull.rs lines 411-end, EXCLUDING
        // the lines 397-409 endpoint.path lookup and the lines 433-484
        // spec construction. The first message sent on the channel is
        // the supplied `spec` verbatim.
    }

    /// Existing entry point — preserved for CLI call sites that still
    /// build PullSyncOptions. Trivially delegates.
    pub async fn pull_sync(
        &mut self,
        dest_root: &Path,
        local_manifest: Vec<FileHeader>,
        options: &PullSyncOptions,
        track_paths: bool,
        progress: Option<&RemotePullProgress>,
    ) -> Result<RemotePullReport> {
        let spec = Self::build_spec_from_options(&self.endpoint, options)?;
        self.pull_sync_with_spec(dest_root, local_manifest, spec, track_paths, progress).await
    }
}
```

The seam is the *concatenation* of the two extracted regions plus the
endpoint→spec field mapping in the first region. All existing CLI call
sites keep their current signature unchanged. No behavior change in any
pre-existing code path.

Required unit tests:

1. **Wire-equivalence**: `build_spec_from_options(endpoint, opts)`
   followed by `pull_sync_with_spec(... spec ...)` produces an identical
   on-the-wire spec to today's `pull_sync(... opts ...)` for a
   representative options matrix (each `ComparisonMode`, each
   `MirrorMode`, with/without resume, with/without filter, all
   `RemotePath` variants).
2. **Endpoint-isolation**: a hand-built spec with `module = "alpha"`,
   `source_path = "x/y"` produces those exact values on the wire
   when handed to `pull_sync_with_spec`, even when the
   `RemotePullClient`'s endpoint was constructed with a different
   `RemotePath::Module { module: "beta", rel_path: "z" }`. The spec
   wins; the endpoint is transport-only. (Regression guard for
   R25-F1.)

#### Spec authorship in the delegated path — `client_capabilities` is dst's

`TransferOperationSpec.client_capabilities` (`proto/blit.proto:449-458`)
describes "the initiator's side of the wire" — what the host that
will *receive* payload bytes from the origin can handle. In a normal
pull, that's the CLI. In a delegated pull, the byte recipient is the
**destination daemon**, not the CLI. The CLI is one step removed: it
expresses operator intent, but it is not in the byte path and cannot
honestly speak for what tar-shard / data-plane / resume payloads the
destination supports.

The wire contract therefore requires that the spec arriving at src has
`client_capabilities` populated by the dst, not the CLI. To reconcile
this with the embedded-`TransferOperationSpec` design (R21-F1), we make
the override explicit:

- The CLI fills the rest of `TransferOperationSpec` from operator
  intent (filter, mirror, comparison, resume, ignore_existing,
  force_grpc, source identity). It MAY leave
  `client_capabilities` unset or fill it with the CLI's own
  conservative values; those values are **not authoritative**.
- The dst handler, in §4.2 step 8 (between "metrics RAII" and
  "outbound connect"), **mandatorily replaces**
  `spec.client_capabilities` with `PeerCapabilities` describing the
  destination daemon's actual byte-receive capabilities. No conditional
  override, no field merging — the field is rewritten regardless of
  what the CLI sent.
- Phase 1 unit test required: feed a `DelegatedPullRequest` whose
  embedded spec has `client_capabilities {supports_tar_shards: false}`
  to a daemon that supports tar shards; assert the spec sent on the
  outbound `pull_sync_with_spec` call has `supports_tar_shards: true`.

This is the only field for which CLI-supplied values are not
authoritative. Every other field of `TransferOperationSpec` flows
through unchanged. The override boundary is documented in the proto
comment on `DelegatedPullRequest.spec` and tested explicitly.

**Core side (`crates/blit-core/src/remote/`):**

- `RemoteTransferSource` (`crates/blit-core/src/remote/transfer/source.rs`)
  is the relay primitive used by the legacy CLI-relay path. Keep it: the
  `--relay-via-cli` escape hatch still uses it, and removing it would
  remove the explicit fallback. Document in the module docstring that this
  is the relay primitive, not the default remote→remote path.
- Confirm `RemotePullClient::pull_sync` is async-clean for use inside a
  `tonic` server handler (no nested `block_on`, no thread-local runtime
  assumptions). Verified at `crates/blit-core/src/remote/pull.rs:128`.

### 4.3 Authentication and the delegation gate

#### 4.3.1 Why this needs a gate

Today a caller that reaches daemon B can read/write only B's configured
modules. Adding `DelegatedPull` lets the same caller make B initiate a
TCP connection to an arbitrary source endpoint chosen by the caller.
That is a **new outbound-network capability** from B's network position —
an SSRF/network-pivot primitive. It is not "no new privileges."

The 0.1.0 trust model can still accept direct remote→remote, but the
capability has to be opt-in and operator-controlled at the daemon, not
implicit.

#### 4.3.2 Gate design

Add a `delegation` block to daemon config (`blit-daemon.toml`):

```toml
[delegation]
# Master switch. Default: false. Setting to true allows the daemon to
# act as a delegated pull initiator on behalf of CLI clients that can
# reach its control plane.
allow_delegated_pull = false

# Source allowlist. When non-empty, every resolved IP of the source
# host must match at least one entry. Empty means "any host" (only
# honored when allow_delegated_pull is true and the operator has
# accepted that posture).
#
# Accepted entry forms (see §4.3.3 for matching semantics):
#   - hostname        e.g. "server-a.lan"
#   - CIDR (IPv4/v6)  e.g. "10.0.0.0/8", "fd00::/8"
#   - bare IP         e.g. "10.1.2.3", "::1"
allowed_source_hosts = ["server-a.lan", "10.0.0.0/8"]

# Per-module narrowing override. A module may opt OUT of being a
# delegation destination even if the daemon-wide setting allows it.
# Configured under [[modules]] entries; the override can only narrow
# the daemon-wide policy, never widen it.
```

#### 4.3.3 Gate ordering and matching semantics

Ordering is the load-bearing security invariant. The handler runs the
checks in §4.2 steps 1–6 in that exact order; the rule is **no
filesystem path resolution and no outbound connect before policy
approves**. Policy approval requires both the daemon-wide gate (step 3)
and the per-module override (step 5) to pass.

`allowed_source_hosts` matching:

1. **Hostname normalization** — case-insensitive comparison; trailing
   dots stripped; non-ASCII names normalized through IDNA (punycode)
   before comparison. `Server-A.Lan` and `server-a.lan.` both match
   `server-a.lan`.
2. **Bare-IP and CIDR entries** — parsed once at config load using the
   `ipnet` crate. Invalid entries fail config load (loud failure
   beats silent permissive default).
3. **Hostname-entry matches** — exact post-normalization equality only.
   No wildcards in 0.1.0 (suffix matchers would be a footgun against
   `evil-server-a.lan`-style typos).
4. **Resolution** — when the source locator is a hostname, the daemon
   resolves it once to an IP set via the standard async resolver.
   **Every resolved address** must match either a CIDR entry or a bare
   IP entry (the literal hostname can also match per rule 3 — but only
   if listed). If any resolved address is unmatched, the gate denies.
5. **DNS-rebinding mitigation (load-bearing)** — the IP set produced by
   the resolution in step 4 is bound to the connection. The outbound
   `RemotePullClient` connects to a specific resolved IP, not to the
   hostname. A second resolution at connect time would let a malicious
   DNS authority swap the IP between check and connect; binding to the
   already-resolved IP closes that window. Implementation note: pass an
   already-resolved `SocketAddr` to the gRPC connector; do not pass a
   hostname URI that would re-resolve.
6. **IPv6 normalization** — bracketed forms (`[::1]:8443`) are stripped
   before comparison; IPv4-mapped IPv6 (`::ffff:10.1.2.3`) is flattened
   to its IPv4 form for matching.

7. **Loopback and link-local addresses require IP/CIDR authorization,
   not hostname authorization.** A resolved address in any of the
   following ranges is rejected unless an **IP-form or CIDR-form**
   allowlist entry covers it. A hostname-form allowlist match is
   **insufficient** for these ranges:
     - IPv4 loopback: `127.0.0.0/8`
     - IPv4 link-local: `169.254.0.0/16`
     - IPv6 loopback: `::1`
     - IPv6 link-local: `fe80::/10`
     - IPv6 unique-local: `fc00::/7`
     - "this network": `0.0.0.0/8`
     - IPv6 unspecified: `::`

   Rationale: hostname allowlist entries authorize a *name*, but DNS
   answers can be attacker-controlled. If `evil.example.com` is in
   `allowed_source_hosts` and resolves to `127.0.0.1`, accepting that
   on the strength of the name alone would let any actor with control
   of `evil.example.com`'s A record point the daemon at its own
   loopback services (a classic SSRF-via-DNS pivot). Requiring an
   IP-form authorization for these ranges means the operator must
   explicitly opt into reaching anything sensitive via name. To
   delegate against `127.0.0.1` (e.g., for a same-host integration
   test), the operator writes `allowed_source_hosts = ["127.0.0.1"]`
   or `["127.0.0.0/8"]`, not just the hostname.

The `delegated_pull` handler emits
`DelegatedPullError{phase=DELEGATION_REJECTED}` with a clear reason
string for any rejected case (master-switch off, no allowlist match,
per-module opt-out, IDNA failure, unresolvable host, mixed-result
resolution where some addresses are allowed and some aren't,
loopback/link-local resolved without IP-form authorization). The CLI
surfaces the reason verbatim. No silent denials.

Tests required (Phase 1 unit + integration):
- Hostname case/trailing-dot/IDNA equivalence.
- CIDR matching for IPv4 and IPv6.
- Multi-A-record resolution where one address is outside the allowlist
  → denied.
- IPv4-mapped IPv6 normalization.
- Hostname matches but resolves to `127.0.0.1` with no IP-form entry
  → **denied** (regression guard for R25-F3).
- Hostname matches and resolves to `127.0.0.1` with `127.0.0.0/8` in
  the allowlist → permitted.
- Public IP (e.g., `1.2.3.4`) authorized purely by hostname match
  → permitted (only loopback/link-local need IP-form authorization).
- DNS-rebinding simulation: resolver returns IP-A on first call,
  IP-B on second call → daemon connects to IP-A (the one it validated),
  not IP-B.
- Bracketed IPv6 literal in locator.
- Invalid CIDR / unparseable entry at config load → daemon refuses to
  start.

#### 4.3.4 What the gate does not solve

The gate is policy, not authentication. With auth disabled (today's
default), anyone reaching the daemon's control plane can request a
delegation against any allowlisted source. That is acceptable for the
"trusted LAN" deployment model `DAEMON_CONFIG.md` already documents.
For internet-exposed deployments, both `BlitAuth` (separate work) and
the gate are required. The gate does not pretend to be auth.

#### 4.3.5 Documentation

Add an "Outbound delegation" section to `docs/DAEMON_CONFIG.md`'s "Trust
Model and Network Exposure" that explains:

- Direct remote→remote requires the destination daemon to opt in.
- Enabling it gives clients of the destination daemon the ability to
  cause it to dial source endpoints from the destination's network
  position. The gate's host allowlist is the operator's primary control.
- Allowlist matching: hostnames are case-insensitive after IDNA
  normalization; CIDR entries match resolved IPs; **all** resolved
  addresses must match (mixed-result resolution is denied); the daemon
  connects to the resolved IP, not the hostname (DNS-rebinding
  mitigation).
- Loopback and link-local addresses (`127.0.0.0/8`, `169.254.0.0/16`,
  `::1`, `fe80::/10`, etc.) require an **IP- or CIDR-form** allowlist
  entry. A hostname-form match alone does not authorize them — this
  blocks SSRF-via-DNS pivots where an attacker-controlled DNS record
  points an allowlisted hostname at the daemon's own loopback.
- When auth lands, delegation will additionally require an
  operator-minted, dst-scoped bearer token presented by the CLI and
  forwarded through `RemoteSourceLocator.delegated_credential`.

#### 4.3.6 Forward-compatible auth hook

`RemoteSourceLocator.delegated_credential` is defined now and ignored
in 0.1.0. When `BlitAuth.Authenticate` becomes real, the flow is:

1. CLI calls `BlitAuth.Authenticate` against src; receives a token bound
   to "operator + delegate scope to host {dst-host}."
2. CLI passes that token through `delegated_credential`.
3. Dst attaches it as the bearer credential on its outbound
   `RemotePullClient` connection to src.

The operator remains the trust anchor; daemons never trust each other
implicitly.

### 4.4 Failure modes — no silent fallback

The release directive ("carry no tech debt for the sake of backwards
compatibility") rules out automatic fallback. Auto-falling-back on
`Unimplemented` is exactly stale-daemon support. Auto-falling-back on
`CONNECT_SOURCE` is exactly the kind of silent reroute that masks real
misconfiguration. The operator gets one explicit escape hatch
(`--relay-via-cli`); other failures surface verbatim.

| Failure | Detection | Behavior |
|---|---|---|
| Dst daemon doesn't implement `DelegatedPull` (stale daemon) | `tonic::Code::Unimplemented` from RPC | Fail with: "destination daemon does not implement DelegatedPull; upgrade or pass `--relay-via-cli`" |
| Dst gate denies (delegation disabled, source not allowlisted) | `DelegatedPullError{phase=DELEGATION_REJECTED}` | Fail with the gate's reason string. Suggest `--relay-via-cli` if applicable. |
| Dst can't reach src (network partition, ACL block) | `tonic::Code::Unavailable` or `DelegatedPullError{phase=CONNECT_SOURCE}` | Fail with: "destination daemon cannot reach source ({addr}); pass `--relay-via-cli` to route through the CLI host" |
| Src refuses dst's connection (auth/ACL) | `DelegatedPullError{phase=NEGOTIATE}` with upstream message | Surface verbatim. Do **not** fall back — silently rerouting through CLI would defeat intentional ACLs. |
| Mid-transfer failure | `DelegatedPullError{phase=TRANSFER}` | Same handling as a failed direct pull; non-zero exit, message verbatim. |
| Apply-phase failure on dst (disk full, permissions, F2 containment) | `DelegatedPullError{phase=APPLY}` | Surface verbatim; partial state left as-is (same as today's pull). |
| CLI loses connection to dst mid-transfer | gRPC stream closed on either side | Dst aborts (`pull_sync` future dropped); CLI returns "delegation lost". Document; `--detach` is out of scope (§9). |

### 4.5 Fallback policy: explicit only

```rust
// Pseudocode: there is no auto-fallback predicate. The dispatch is
// purely a function of the user-set flag.
match (args.relay_via_cli, ...) {
    (true,  _) => run_remote_push_transfer(...),         // operator chose relay
    (false, _) => run_remote_to_remote_direct(...),      // surface failures verbatim
}
```

This is intentional. Every fallback heuristic that we removed has a
clear failure mode that's better surfaced than papered over:

- **Stale destination daemon** → upgrade message, not silent demotion.
- **Network partition between dst and src** → operator-visible, with a
  documented escape hatch (`--relay-via-cli`).
- **ACL refusal at src** → surfaces the operator's intentional security
  boundary instead of routing around it.

`--relay-via-cli` exists for genuine topology cases (CLI is the only host
that can talk to both daemons) and for benchmarking. It is the single
operator-controlled escape, not a fallback automaton.

## 5. Phased implementation

### Phase 1 — Wire protocol, delegation gate, daemon handler (3 days)

1. Extend `proto/blit.proto` with `DelegatedPull`, `DelegatedPullRequest`
   (embedding `TransferOperationSpec`), `RemoteSourceLocator`,
   `DelegatedPullProgress`, `DelegatedPullStarted`, `DelegatedPullSummary`,
   `DelegatedPullError`. Bump `TransferOperationSpec.spec_version` if
   delegation requires new fields, otherwise reuse as-is.
2. **Delegation gate config** (R21-F2 + R23-F2 + R23-F3):
   - Extend daemon config (`runtime.rs` config types) with
     `[delegation]` block: `allow_delegated_pull` (default `false`),
     `allowed_source_hosts` (default empty list, accepts hostnames /
     CIDRs / bare IPs), per-module narrowing override.
   - At config load: parse each `allowed_source_hosts` entry into a
     typed enum (`Hostname(String) | Cidr(IpNet) | BareIp(IpAddr)`),
     applying IDNA + lowercasing + trailing-dot stripping for
     hostnames. Invalid entries fail config load loudly. Add `ipnet`
     to dependencies.
   - Implement `validate_source(&Locator) -> Result<SocketAddr, GateDenial>`
     per the §4.3.3 matching semantics: resolve hostname once, validate
     **every** resolved address, return a single `SocketAddr` to bind
     the outbound connection to (DNS-rebinding mitigation).
   - Wire daemon-wide gate into the new handler; wire per-module
     override into `ModuleConfig` resolution (looked up after module
     metadata, before path resolution per §4.2 step 5).
   - Document in `docs/DAEMON_CONFIG.md` Trust Model section.
3. **`pull_sync_with_spec` refactor in core** (R23-F1, R25-F1):
   - Spec construction in the existing `pull_sync` spans **two
     non-contiguous regions**: lines 397–409 (derive `module` and
     `source_path` from `self.endpoint.path`; bail on
     `RemotePath::Discovery`) and lines 433–484 (derive everything
     else from `options`). Lift **both** regions into
     `RemotePullClient::build_spec_from_options(endpoint, options) ->
     Result<TransferOperationSpec>`. Lift the remainder of the body
     into `RemotePullClient::pull_sync_with_spec(...)`.
   - **Endpoint-isolation invariant:** `pull_sync_with_spec` MUST NOT
     read `self.endpoint.path`. The endpoint is transport-only;
     module + source_path + every other spec field is authoritative
     from the supplied spec. Document this in the method rustdoc.
   - Existing `pull_sync` becomes a thin wrapper that calls
     `build_spec_from_options(&self.endpoint, options)?` and delegates.
     No CLI call site changes.
   - Unit tests:
     - **Wire-equivalence** (R23-F1): `pull_sync(opts)` and
       `pull_sync_with_spec(build_spec_from_options(opts)?)` emit
       byte-identical specs on the wire for a representative
       `PullSyncOptions` matrix (each `ComparisonMode`, each
       `MirrorMode`, with/without resume, with/without filter, all
       `RemotePath` variants).
     - **Endpoint-isolation** (R25-F1): hand-built spec with
       `module = "alpha"`, `source_path = "x/y"` produces those exact
       values on the wire when handed to `pull_sync_with_spec`, even
       when the `RemotePullClient`'s endpoint was constructed with a
       different `RemotePath::Module { module: "beta", rel_path: "z" }`.
4. **No new FilterSpec proto** — `FilterSpec` already exists at
   `proto/blit.proto:367-392` and is produced by `build_filter_spec` in
   `crates/blit-cli/src/transfers/mod.rs`. Reuse both.
5. Implement `delegated_pull.rs` handler in
   `crates/blit-daemon/src/service/` per §4.2 ordered steps 1–12.
   Calls `pull_sync_with_spec`, not `pull_sync`. Forwards progress via
   bounded channel. Cancels on dropped gRPC return stream.
6. Wire the handler into the gRPC service in
   `crates/blit-daemon/src/service/core.rs`.
7. Unit tests:
   - Delegation gate matrix per §4.3.3 (disabled / enabled /
     hostname-equality / CIDR / multi-A-record where one address is
     denied / IPv4-mapped IPv6 / bracketed IPv6 / per-module
     narrowing override).
   - **Loopback/link-local IP-form-authorization rule (R25-F3):**
     hostname matches + resolves to `127.0.0.1` with no IP-form
     entry → denied; same hostname with `127.0.0.0/8` in allowlist →
     permitted; public IP via hostname-only entry → permitted.
   - DNS-rebinding simulation: resolver returns IP-A then IP-B; daemon
     connects to IP-A.
   - Invalid CIDR / unparseable allowlist entry fails config load.
   - Containment violation rejected after gate but before connect.
   - Stale `spec_version` rejected by `from_spec` normalizer.
   - `pull_sync` ↔ `pull_sync_with_spec` wire-equivalence test (per
     step 3).
   - **`pull_sync_with_spec` endpoint-isolation test (R25-F1):**
     hand-built spec with `module = "alpha"` produces `"alpha"` on
     the wire even when client's endpoint was constructed with
     `module = "beta"`.
   - **`client_capabilities` mandatory override test (R25-F2):**
     `DelegatedPullRequest` with `client_capabilities {supports_tar_shards: false}`
     forwarded to a tar-shard-supporting daemon → spec sent to src
     has `supports_tar_shards: true`.
   - Mock `pull_sync_with_spec` to assert progress passthrough
     integrity in the delegated handler.

### Phase 2 — CLI dispatch and integration tests (2 days)

1. Add `run_remote_to_remote_direct` in
   `crates/blit-cli/src/transfers/remote_remote_direct.rs`. Builds
   `TransferOperationSpec` from `args` using existing `build_filter_spec`
   and the same mirror/comparison/resume normalization push/pull use.
2. Update dispatch in `transfers/mod.rs:398` and `:503` per §4.2 (no
   silent fallback predicate).
3. Add `--relay-via-cli` flag on `TransferArgs`.
4. Integration test `crates/blit-cli/tests/remote_remote_direct.rs`
   (see §6 for byte-path-isolation test design — this is the load-bearing
   correctness test).
5. Integration test `remote_remote_no_silent_fallback.rs`:
   - Stale dst (returns `Unimplemented`): assert CLI fails with explicit
     upgrade message; assert CLI did **not** route through relay path.
   - Gate-rejected dst (allow_delegated_pull = false): assert CLI surfaces
     the gate's reason string verbatim.
   - Src refuses dst (NEGOTIATE phase): assert CLI surfaces upstream
     error verbatim; assert CLI did **not** retry through relay.
6. Integration test `remote_remote_explicit_relay.rs`:
   - With `--relay-via-cli`, assert legacy path runs and bytes flow
     through CLI (counterpart to the byte-path-isolation test).

### Phase 3 — Cleanup and benchmarking (1 day)

1. Audit `RemoteTransferSource` usages. If only the legacy CLI relay path
   uses it, **leave it** (relay is a real fallback). Document this in the
   module docstring.
2. Update `docs/DAEMON_CONFIG.md`:
   - "Trust Model" section: direct remote→remote, ACL implications.
   - "Path containment" section: F2 still applies to dst-side resolution
     in delegated handler.
3. Update `docs/CLI_USAGE.md` (or wherever `--relay-via-cli` belongs) and
   `man` pages.
4. Update `TODO.md`: move "Remote→remote re-evaluation" out of Deferred,
   mark closed with reference to commit.
5. Add benchmark script under `benches/` or `scripts/bench/`:
   - Run identical workload through `--relay-via-cli` vs default direct.
   - Measure wallclock, bytes/sec, CLI-host network bytes (proves CLI is
     out of the byte path).
   - Capture results in `docs/perf/remote_remote_benchmarks.md`.

### Phase 4 — Future-proofing (parallel, can land separately)

1. `RemoteSourceLocator.delegated_credential` honored end-to-end once
   `BlitAuth` becomes real. Track as separate task; this plan does not
   block on it.
2. `--detach` mode where CLI exits and dst continues. Track as separate
   future feature.

## 6. Test strategy

**Unit:**
- `TransferOperationSpec` round-trip across delegated handler boundary —
  ensure nothing is dropped/flattened (regression guard for R21-F1).
- Delegation gate matrix (disabled / enabled / enabled+allowlist /
  allowlist mismatch / per-module override).
- Containment check on `dst_module + dst_destination_path` rejects
  symlink escapes (extends F2 test pattern).
- `spec_version` normalizer rejects unknown versions explicitly
  (regression guard for R21-F6 — we do not rely on unknown-field
  detection).

**Integration (CLI tests calling real daemon binaries):**

*Byte-path isolation (load-bearing — addresses R21-F7 + R23-F4):*

The destination's view of "who connected to whom" is not authoritative.
A destination daemon can only read its own local and remote socket
addresses; what the source daemon actually accepted is what matters,
and across NAT/loopback aliases those views can disagree. So the
proof-of-isolation observables are designed around what the **CLI
host** itself does, not what the destination reports.

Primary observables (both must hold for the test to pass):

1. **CLI-side byte counter (load-bearing).** Wrap the CLI's outbound
   transports in a test-only byte counter (compile-time `#[cfg(test)]`
   hook on `RemotePushClient`/`RemotePullClient`/data-plane connect
   constructors). On the direct path the counter must observe only the
   small `DelegatedPull` control-plane gRPC traffic — **no data-plane
   bytes**, no significant payload-sized traffic. On the
   `--relay-via-cli` counterpart test, the same counter must observe
   approximately the full payload size (sanity check that the
   instrumentation works).
2. **Construction guard.** The test fails if `RemoteTransferSource` is
   constructed at all on the direct path. This catches a regression
   where the dispatcher accidentally falls back to the relay path
   without flagging it.

Diagnostic-only observables (logged for triage, not asserted as
correctness invariants):

- `DelegatedPullStarted.source_data_plane_endpoint`: what dst's pull
  client *thinks* its peer is. Useful to debug NAT cases.
- `DelegatedPullSummary.source_peer_observed`: dst's view of the
  remote socket. Surface in audit logs; do not gate test outcomes
  on this.

Optional future work: have the **source** daemon report the accepted
data-plane peer address back through the pull protocol so the dst can
forward it through `DelegatedPullSummary`. Then the operator audit log
contains a source-attested fact, not a destination-attested one. Out
of scope for 0.1.0; the CLI byte counter is sufficient for byte-path
isolation correctness today.

*Other integration tests:*

- Direct happy path: tree A → B, A/B/CLI on three distinct loopback
  ports; both observables above pass; result tree on B matches A.
- Stale dst (`Unimplemented`): explicit upgrade message; **assert no
  fallback**.
- Gate denies: explicit reason string; **assert no fallback**.
- Src refuses dst (NEGOTIATE): upstream message verbatim; **assert no
  fallback**.
- `--relay-via-cli` happy path: legacy relay still works, byte-counter
  observes payload (counterpart sanity).
- Mirror mode (FilteredSubset and All) end-to-end with delete list
  applied on dst (regression guard against R21-F1's data-loss class).
- ComparisonMode coverage: SIZE_MTIME, CHECKSUM, SIZE_ONLY, IGNORE_TIMES,
  FORCE — at least one delegated transfer per mode succeeds.
- `ignore_existing` honored end-to-end.
- Filter pass-through: include/exclude globs work identically to direct
  pull.
- Mid-transfer cancellation: CLI Ctrl-C → dst aborts → partial state
  visible on dst but no corruption (no half-written files, no orphaned
  delete-list applies).

**Performance (manual, hardware-bound):**
- Linux/Linux pair, 10 GbE LAN: measure throughput delta vs CLI relay
  with CLI on same LAN. Expect parity or modest improvement (CLI relay
  hop time is small on equal-speed links).
- Linux/Linux pair, 10 GbE LAN, CLI on 100 Mbit link: expect 50–100×
  improvement (CLI was the bottleneck).
- Linux/Linux pair, RDMA-capable: defer to Phase 3.5.

## 7. Risks and open questions

| Risk | Mitigation |
|---|---|
| `TransferOperationSpec` evolves and dst sees a newer version than it understands | Use `spec_version` + `PeerCapabilities`; `NormalizedTransferOperation::from_spec` rejects unknown versions explicitly with a clear error. We do not depend on protobuf unknown-field detection (proto3 silently preserves unknowns). |
| Dst daemon becomes a network client to attacker-supplied source URIs (network-pivot/SSRF risk) | §4.3 delegation gate: default-disabled, host allowlist with strict matching semantics (§4.3.3), DNS-rebinding mitigation by binding the connection to the resolved IP. Per-module narrowing override. Documented in `DAEMON_CONFIG.md`. |
| Spec-construction drift between CLI `pull_sync` and daemon `delegated_pull` paths | §4.2 refactor extracts `pull_sync_with_spec`; both CLI and daemon use the same target-side pull body. Wire-equivalence unit test guards the seam (R23-F1). The seam includes the endpoint→spec mapping at `pull.rs:397-409`; `pull_sync_with_spec` is contractually forbidden to read `self.endpoint.path`, with an endpoint-isolation unit test guarding the boundary (R25-F1). |
| Allowlist matching gotchas (DNS aliases, CIDR off-by-one, IPv6 forms, rebinding) | §4.3.3 specifies exact semantics; Phase 1 unit-test list covers each form including DNS-rebinding simulation (R23-F3). Loopback/link-local addresses additionally require IP- or CIDR-form authorization, never hostname-only (R25-F3) — closes the SSRF-via-DNS pivot. |
| CLI claims destination's capabilities incorrectly (e.g., asserts dst supports tar shards when it doesn't) | `client_capabilities` is the one spec field where CLI-supplied values are non-authoritative. Dst handler mandatorily replaces it with own `PeerCapabilities` before outbound connect (§4.2 step 8). Mandatory-override unit test guards the boundary (R25-F2). |
| Progress event volume grows (every delegated pull pushes events) | Apply existing `RemoteTransferProgress` throttling; same as a normal pull. Bounded channel + stream backpressure handle overload. |
| Operator runs `blit copy` against three daemons (A→B and B→C in same script): does B as dst handle re-entry as src cleanly? | B is just a daemon, no special state. Two delegated pulls can land on it concurrently; metrics gauge counts both. Document as supported. |
| Dst aborts but src has already buffered chunks | Same failure mode as today's pull. Existing `pull_sync_with_spec` cleanup covers it. |
| **Open:** does `--checksum` work end-to-end? | The dst-as-initiator pull semantics are identical to a direct pull. F11/R15-F1 ack negotiation lives in `pull.rs` — should work unchanged. Verified in integration test. |
| **Open:** mid-stream CLI Ctrl-C semantics | gRPC stream cancellation propagates; dst observes via dropped `pull_sync_with_spec` future; src observes via dropped data-plane connection. Explicit test required. |
| **Open:** `--detach` mode interaction | Out of scope. Sync delegation only. Note in §9. |

## 8. Decision summary

- **Approach:** Push delegation (Option A). Dst becomes the initiator.
- **Wire shape:** one new RPC (`DelegatedPull`); request **embeds existing
  `TransferOperationSpec`**; reuses existing `FilterSpec`. No parallel
  flag-bag, no duplicated semantics.
- **Daemon implementation:** new `pull_sync_with_spec` extracted from
  `pull_sync`; both CLI and daemon call it. The new entry point is
  contractually forbidden to read `self.endpoint.path`; the endpoint
  is transport-only and the spec is authoritative. No custom sink
  pipeline, no `FilteredSink` (which doesn't exist).
- **Spec authorship:** CLI fills all fields except `client_capabilities`,
  which the destination mandatorily overrides with its own
  `PeerCapabilities` (the byte recipient in delegation is dst, not CLI).
- **Security:** explicit `[delegation]` config gate, default-disabled,
  with host allowlist. Strict matching semantics (IDNA, CIDR,
  multi-A-record validation, DNS-rebinding mitigation by connecting to
  the validated IP, loopback/link-local require IP/CIDR-form
  authorization). Documented in `DAEMON_CONFIG.md`.
- **Gate ordering:** locator parse → daemon-wide gate → module
  metadata lookup → per-module override → path resolution + F2
  containment → outbound connect. No filesystem path resolution and
  no outbound connect before policy approves.
- **CLI flag:** `--relay-via-cli` is the **only** fallback, operator-set.
  No silent automatic fallback on `Unimplemented` / `Unavailable` /
  `CONNECT_SOURCE`.
- **Byte-path isolation proof:** CLI-side byte counter is the
  load-bearing assertion; destination's view of socket addresses is
  diagnostic only.
- **Auth posture:** zero change in 0.1.0; forward-compatible
  `delegated_credential` passthrough field for when `BlitAuth` lands.
- **Estimated implementation:** 6–8 working days end-to-end including
  delegation gate (with allowlist matching, including loopback IP-form
  rule), `pull_sync_with_spec` refactor (with endpoint-isolation
  invariant), client_capabilities mandatory-override, tests, and docs.

## 9. Out-of-scope (explicitly)

- `BlitAuth.Authenticate` implementation. Tracked separately.
- `--detach` mode. Tracked separately.
- RDMA/RoCE data plane (Phase 3.5).
- Direct daemon-to-daemon connections that bypass the CLI control plane
  entirely (CLI as orchestrator is preserved — this is not a peer mesh).
