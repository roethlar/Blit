# audit-15-grpc-missing-connection-timeouts: Missing request/idle timeouts on tonic gRPC server control plane

**Severity**: Robustness
**Status**: Response / recommend-defer (pending reviewer grade) — see Resolution
**Branch**: `phase5/a1`
**Commit**: (analysis-only; no code change — see Resolution)

## What

Audit of [`crates/blit-daemon/src/main.rs`](file:///Users/michael/Dev/Blit/crates/blit-daemon/src/main.rs) identified that standard non-streaming RPC endpoints in the `blit-daemon` control plane do not have any configured timeouts or maximum concurrency bounds.

In `main.rs` (lines 137-142):
```rust
    Server::builder()
        .http2_keepalive_interval(Some(std::time::Duration::from_secs(30)))
        .http2_keepalive_timeout(Some(std::time::Duration::from_secs(20)))
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;
```

While TCP-level and HTTP/2-level keepalives are configured (which are useful for reclaiming dead client connection streams like Subscribe), standard gRPC requests (e.g. `ListModules`, `List`, `DiskUsage`, `FilesystemStats`) can be kept open indefinitely by a misbehaving, stuck, or malicious client. 
If a client opens a connection, initiates a request, and then refuses to send metadata/frames or hangs indefinitely, it will hold a worker thread or connection slot. Over time, this can lead to thread pool/connection starvation or file descriptor exhaustion (SSRF or DoS potential).

## Approach

Configure a default request/connection timeout on the gRPC server using `Server::builder().timeout(...)`. This limits the maximum duration any single RPC request is allowed to take before being aborted by the server. 
Setting a request timeout of e.g. 30 seconds for standard control plane endpoints protects the daemon from resource starvation.

Proposed configuration:
```rust
    Server::builder()
        .timeout(std::time::Duration::from_secs(30))
        .http2_keepalive_interval(Some(std::time::Duration::from_secs(30)))
        .http2_keepalive_timeout(Some(std::time::Duration::from_secs(20)))
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;
```

*Note: For long-running streaming endpoints (such as streaming data plane transfers or remote-to-remote delegation), the timeout needs to be handled appropriately or kept long-running. Standard gRPC streams in Tonic inherit the Server's request timeout unless overridden or managed separately. In Blit, data transfers utilize direct TCP data connections rather than the gRPC control plane, so the gRPC control plane request timeout is primarily relevant for short-lived administrative and metadata query RPCs.*

## Files changed

- `crates/blit-daemon/src/main.rs`

## Tests

- Validate that client connections are terminated if standard RPC requests exceed the configured timeout.
- Existing daemon integration tests must still pass within the timeout boundary.

## Resolution — DO NOT apply the proposed fix (recommend defer)

The proposed `Server::builder().timeout(Duration::from_secs(30))` is
**harmful** and must not be applied as written.

### Why the proposed fix is wrong

`tonic`'s `Server::timeout()` applies a per-request deadline to **every**
RPC, including server-streaming and bidi-streaming ones. The `Blit`
service has 7 streaming RPCs that legitimately run far longer than 30s:

- `Subscribe` — the TUI F2 view / `jobs watch` hold this open for the
  whole session (minutes to hours).
- `DelegatedPull` — streams progress for the entire delegated
  daemon→daemon transfer.
- `Pull` / `PullSync` / `Push` — stream the transfer itself.
- `Find` / `DiskUsage` — stream entries for a whole recursive walk.

A 30s server timeout would abort all of these mid-flight. That's exactly
the idle-vs-total-deadline trap audit-1c was careful to avoid (it used an
*idle* StallGuard, never a total cap). The finding's own note ("data
transfers utilize direct TCP rather than the gRPC control plane") is only
partly true — `Subscribe` and `DelegatedPull` are long-lived gRPC streams
on the control plane.

### Why there is no real gap to fix here

The dead/stuck/partitioned-peer case the finding worries about is
**already handled** by the audit-1b HTTP/2 keepalive
(`http2_keepalive_interval(30s)` + `http2_keepalive_timeout(20s)`,
main.rs:137-139): a peer that stops answering PINGs is reaped within
~50s, reclaiming the stream + broadcast Receiver + forwarder task.

The only residual is a TCP-alive-but-misbehaving client that opens a
connection, issues a request, and never reads the response. A per-request
*server-side* timeout doesn't actually fix that (the handler completes;
the response just buffers), and the daemon's threat model is
authenticated / operator-controlled peers, not arbitrary internet hosts —
so the DoS surface is low.

### If a narrow mitigation is wanted later (deferred)

The *correct* shape — should the owner/reviewer want defense-in-depth —
is a timeout scoped to ONLY the 8 unary admin RPCs (`List`, `Purge`,
`CompletePath`, `ListModules`, `FilesystemStats`, `GetState`,
`CancelJob`, `ClearRecent`), never the 7 streaming ones. That means
wrapping each unary handler body in `tokio::time::timeout(...)` (or a
tower layer keyed on the method path) — a non-trivial change with its own
regression surface. A `concurrency_limit_per_connection` is a weak lever
(a flood uses many connections, not many streams on one), so it's not
recommended either.

**Recommendation:** reject the blanket timeout; keep the keepalive as the
mechanism; treat a unary-only timeout as a low-priority, opt-in
defense-in-depth follow-up rather than a fix. No code change shipped for
this finding.

## Reviewer comments

(empty — pending grade of the recommend-defer decision)
