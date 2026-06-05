# Inconsistency Findings: Timeouts, retries, cancellation, stalls

**Generated**: 2026-06-04
**Findings**: 13 (H: 4, M: 6, L: 3)

## High severity

### dataplane-stall-guard-only-on-pull-receive — Push-receive socket lacks the audit-1c stall guard

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-core/src/remote/pull.rs:1712-1720` — every pull data-plane TCP socket is wrapped in `StallGuard(stream, PULL_STALL_TIMEOUT)` (30s idle) before being handed to `execute_receive_pipeline`. A silent peer (no bytes for 30s) trips `io::ErrorKind::TimedOut`, surfaces via `is_retryable`, and the CLI retry loop catches it.
  2. `crates/blit-daemon/src/service/push/data_plane.rs:213-242` — the daemon's push data-plane handler also calls `execute_receive_pipeline(&mut socket, sink, None)` but does NOT wrap `socket` in `StallGuard`. A push client that opened the data stream + sent the token + then went silent leaves the receive task parked on `socket.read_*()` forever. There is no per-RPC timeout on the daemon side; the only fallback is HTTP/2 keepalive on the *control* plane, which does not cover the separate data-plane TCP socket.
  3. `crates/blit-daemon/src/service/pull.rs:702-757` (deprecated Pull) and `crates/blit-daemon/src/service/pull_sync.rs:600-755` (pull_sync data-plane accept paths) construct `DataPlaneSession::from_stream(socket, ...)` directly with no stall guard. Same blind spot.

**Canonical**: The CLI's wrapped-stream pattern (pull.rs:1712-1720). Per the audit-1c owner decision in `feedback_port_cli_safety_guards.md`, "no-bytes-for-30s, scoped to all pulls" — but the contract also applies to any long-lived receive path, including the daemon's push receive and the daemon's pull data-plane accept paths. The current asymmetry means a hostile (or stuck) push client can pin a daemon worker indefinitely.

**Recommendation**: Wrap every `execute_receive_pipeline(stream, ...)` and every `DataPlaneSession::from_stream(stream, ...)` call (both sender- and receiver-side) in `StallGuard` with the existing `PULL_STALL_TIMEOUT` constant. Hoist the constant out of `stall_guard.rs` into a shared `transfer::TRANSFER_STALL_TIMEOUT`. Add a test analogous to `pipeline.rs::receive_pipeline_aborts_on_stall` for the push-receive path.

---

### connect-with-timeout-duplicated-three-ways — Three independent "connect-with-timeout" implementations

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-app/src/client.rs:24,37-46` — defines `pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(30)` and `pub async fn connect_with_timeout(uri)`. The intended canonical helper.
  2. `crates/blit-core/src/remote/pull.rs:230-248` — inlines the same `tokio::time::timeout(30s, Endpoint::from_shared(uri).connect_timeout(30s).connect())` pattern with hardcoded magic numbers, NOT using `blit_app::client::connect_with_timeout`. Doc-comment cites `audit-2` but reimplements it.
  3. `crates/blit-core/src/remote/push/client/mod.rs:298-317` — same inline pattern, same hardcoded `Duration::from_secs(30)`s, same audit-2 doc-comment. Third copy.

**Canonical**: `blit-app::client::connect_with_timeout` (it's the function the rest of `blit-app::admin` uses for control-plane RPCs). The pull/push core clients should go through the same helper; if `blit-core` can't depend on `blit-app`, the helper + constant should live in `blit-core::remote::endpoint` or a new `blit-core::remote::client` module and be re-exported.

**Recommendation**: Promote `connect_with_timeout` + `CONNECT_TIMEOUT` to `blit-core::remote` and rewrite both `RemotePullClient::connect` and `RemotePushClient::connect` to call it. Bumps a value once instead of three times. Per `feedback_server_await_timeouts.md` "audit all awaits up front" — three sites = three places a future audit may forget to bump.

---

### retry-classifier-disagrees-with-categorize-io-error — Two definitions of "retryable I/O" disagree on UnexpectedEof / ConnectionRefused / NotConnected

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-app/src/transfers/retry.rs:35-46` — `is_retryable_io_kind` classifies `TimedOut`, `ConnectionReset`, `ConnectionAborted`, `ConnectionRefused`, `BrokenPipe`, `UnexpectedEof`, `NotConnected` as **Retryable**. This drives the CLI `--retry/--wait` loop.
  2. `crates/blit-core/src/errors.rs:90-117` — `categorize_io_error` classifies the SAME `UnexpectedEof`, `NotConnected`, `ConnectionRefused` as **Fatal** ("default to fatal to avoid infinite loops"). `WriteZero`, `AddrInUse`, `AddrNotAvailable` also go to Fatal. Comment explicitly: "These could go either way - default to fatal".
  3. The two implementations also disagree on `Interrupted` (retry.rs ignores it; errors.rs calls it Retryable) and `WouldBlock` (errors.rs Retryable; retry.rs ignores it).

**Canonical**: Neither is wired into both code paths today. `is_retryable` is what actually gates retry decisions in `run_with_retries`; `categorize_io_error` carries a `TransferError::attempts` counter (see `errors.rs:34-75`) that is currently used only by the test suite. So in practice `retry.rs` wins for CLI behavior. But the duplicated decision logic is a high-severity drift surface — adding a new retryable kind in one place will silently misalign the other.

**Recommendation**: Pick one classifier. Either delete `categorize_io_error` (and `ErrorCategory`/`TransferError`) as dead since `run_with_retries` doesn't use them, OR rewrite `retry.rs::is_retryable_io_kind` to delegate to `categorize_io_error` and reconcile the membership list to one truth. Per `RULES.md` DRY: "Abstract common functionality, eliminate duplication".

---

### tui-vs-cli-no-retry-parity — TUI transfers have no `--retry`/`--wait` equivalent

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-cli/src/main.rs:46-63` — Copy/Mirror/Move are wrapped in `run_with_retries(args.retry, wait, |_n| run_transfer(...))`. Default retries=0, wait=5s; user can opt in to `--retry 3 --wait 5`.
  2. `crates/blit-tui/src/main.rs:3238-3354` (F3 pull), `:3361-3433` (F1 push), `:3461-3538` (F1 delegated), `:4076-4101` (F4 local copy/mirror), `:4121-4196` (F4 local move) — every TUI spawn path calls the transfer function once. No retry loop, no `--retry`/`--wait` config field, no equivalent.
  3. `crates/blit-tui/src/config.rs` — TUI config covers cancel/pull/delete/push status-line TTLs and the live-tick interval, but exposes NO retry knob. The TUI user cannot ask for the robocopy-style retry the CLI offers.

**Canonical**: The CLI retry semantics — these are documented in `cli.rs:264-272` as owner-decided and tied to the retry loop in `retry.rs`. Per the project memory `project_audit_decisions` ("transfer stall-timeout 30s no-bytes + robocopy --retry/--wait follow-up"), the CLI's behavior is the spec.

**Recommendation**: Either wrap the TUI's `spawn_f3_pull` / `spawn_f1_push` / `spawn_f1_delegated_pull` task bodies in the same `run_with_retries` driver with a TUI-config-default of `(retry=0, wait=5s)`, OR document explicitly that TUI transfers are intentionally single-attempt and recommend `:! blit copy --retry` in the help screen. Today the parity gap is silent: a transient ConnectionReset that the CLI would retry simply fails the TUI transfer with no follow-up. Per `feedback_port_cli_safety_guards.md` "replicating a CLI transfer in the TUI? port its safety guards" — `--retry/--wait` qualifies.

## Medium severity

### dataplane-accept-token-timeouts-redeclared — Same accept/token timeouts redeclared under multiple names

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-daemon/src/service/push/data_plane.rs:67,74` — `DATA_PLANE_ACCEPT_TIMEOUT = 30s`, `DATA_PLANE_TOKEN_TIMEOUT = 15s`.
  2. `crates/blit-daemon/src/service/pull.rs:698-699` — `PULL_ACCEPT_TIMEOUT: StdDuration = 30s`, `PULL_TOKEN_TIMEOUT: StdDuration = 15s`. Locally-scoped `const` inside the function body.
  3. `crates/blit-daemon/src/service/pull_sync.rs:597-598` — `PULL_SYNC_ACCEPT_TIMEOUT = 30s`, `PULL_SYNC_TOKEN_TIMEOUT = 15s` in `stream_via_data_plane`.
  4. `crates/blit-daemon/src/service/pull_sync.rs:719-720` — REDECLARED inside the same file as `ACCEPT_TIMEOUT = 30s`, `TOKEN_TIMEOUT = 15s` (no `PULL_SYNC_` prefix) in `stream_via_data_plane_resume`. Doc-comment explicitly cites "same rationale as the streaming pull-sync path" but doesn't share the constant.

**Canonical**: One shared `DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT` pair in `blit-daemon` (or `blit-core::remote::transfer::data_plane`). All four sites encode the SAME 30s/15s pair.

**Recommendation**: Hoist into `blit-daemon::service::data_plane_timeouts` (or `blit-core::remote::transfer::data_plane`) and reference from all four sites. Drift risk is real — `feedback_server_await_timeouts.md` says "Audit all awaits up front", and a partial bump to the push value without the pull pair would create asymmetric behavior between the two transfer directions.

---

### tcp-keepalive-only-on-push-data-plane — Daemon-side TCP keepalive enabled for push, not pull

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-daemon/src/service/push/data_plane.rs:108-120` — after `listener.accept()`, the accepted socket is converted to `socket2::Socket`, then `s2.set_tcp_nodelay(true)` + `s2.set_keepalive(true)` are called (errors silently dropped via `let _ =`). Comment says "Enable nodelay + keepalive to prevent idle stream timeouts during long transfers on other streams."
  2. `crates/blit-daemon/src/service/pull.rs:701-721` — after `listener.accept()` for the pull data plane, the socket is used directly with no TCP_NODELAY or KEEPALIVE configuration.
  3. `crates/blit-daemon/src/service/pull_sync.rs:600-735` — same omission for both `stream_via_data_plane` and `stream_via_data_plane_resume` accept paths.
  4. `crates/blit-core/src/remote/transfer/data_plane.rs:79-90` — the CLIENT side of every data-plane connection DOES set `set_tcp_nodelay(true)` (propagates errors) and `set_keepalive(true)` (logs on failure). Both sides of push are tuned; only the client side of pull is tuned.

**Canonical**: The push-receive pattern (both ends tune nodelay+keepalive). The client-side `data_plane.rs:78-103` pattern is more robust (logs failures instead of swallowing).

**Recommendation**: Add the same nodelay+keepalive tuning to the daemon-side pull / pull_sync accepted sockets, using a shared helper `transfer::tune_data_plane_socket(socket)` so the tuning policy lives in one place. Replace the daemon's `let _ = s2.set_tcp_nodelay(true)` with `log::warn!`-on-failure parity with the client side, per `feedback_server_await_timeouts.md`-style "fail loudly on configuration errors".

---

### server-streaming-rpcs-no-per-message-timeout — Daemon-side streaming RPC handlers lack per-message timeouts

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-daemon/src/service/push/control.rs:62` — `while let Some(request) = stream.message().await?` in the push control loop. No `tokio::time::timeout`. Relies entirely on HTTP/2 keepalive (30s interval + 20s timeout from `main.rs:138-139`) to reap a silent client.
  2. `crates/blit-daemon/src/service/pull_sync.rs:307,341,798,966` — multiple `stream.message().await` sites in pull_sync (receive_spec, receive_client_manifest, etc.) all unbounded.
  3. `crates/blit-daemon/src/service/core.rs:2075,2230,2276,2429,2489` — test sites use `tokio::time::timeout(50ms/100ms/2s, stream.next())` (test sentries), confirming the production handler has no equivalent.
  4. `crates/blit-cli/src/jobs.rs:290` — by contrast, the CLI's `jobs watch` loop DOES wrap `stream.message()` in `tokio::time::timeout(remaining, ...)` to honor `--timeout-secs`.

**Canonical**: Per the design (`audit-1` HTTP/2 keepalive owner decision in `feedback_keymap_collisions.md` / project memory `project_audit_decisions`), the daemon side intentionally relies on HTTP/2 keepalive rather than per-message timeouts: "subscribe→HTTP/2 keepalive (not idle-close)". This IS the canonical answer — but it's not documented at the streaming call sites, only in main.rs.

**Recommendation**: Add a single-line doc-comment at every server-side `stream.message().await` site cross-referencing `main.rs:137-142` and explaining "no inner timeout because audit-1 HTTP/2 keepalive (30s interval / 20s timeout) reaps dead peers". Streaming RPC contracts (subscribe especially) need this rationale to be discoverable at the call site — `feedback_server_await_timeouts.md` exists because someone missed this contract once.

---

### http2-keepalive-client-side-missing — Client connects don't request HTTP/2 keepalive

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-daemon/src/main.rs:137-142` — daemon's `Server::builder().http2_keepalive_interval(30s).http2_keepalive_timeout(20s)` so the daemon reaps idle subscribers.
  2. `crates/blit-app/src/client.rs:37-46` — CLI/TUI client builds `Endpoint::from_shared(uri).connect_timeout(CONNECT_TIMEOUT)` — NO `.keep_alive_while_idle(true)`, NO `.http2_keep_alive_interval(...)`.
  3. `crates/blit-core/src/remote/pull.rs:238-244` and `crates/blit-core/src/remote/push/client/mod.rs:307-314` — same omission on both bulk-transfer client constructors.

**Canonical**: Daemon-side keepalive only protects the daemon from leaking subscriber-side resources. If the daemon process crashes mid-subscribe, the CLI/TUI has no client-side keepalive to detect it — `stream.message()` will park until the OS TCP RST eventually arrives. The TUI's `forward_step` racing `tx.closed()` (`main.rs:5680-5685`) handles consumer-side cancel, but does NOT detect a silently-dead daemon.

**Recommendation**: Decide explicitly whether client-side keepalive is wanted. Documented owner decision is "keepalive, not idle-close" (memory `project_audit_decisions`) — extending that to the client side is consistent. Add `.keep_alive_while_idle(true).http2_keep_alive_interval(Duration::from_secs(30)).http2_keep_alive_timeout(Duration::from_secs(20))` to the shared `connect_with_timeout` helper (see finding `connect-with-timeout-duplicated-three-ways`).

---

### nodelay-error-handling-asymmetric — Push-receive silently ignores nodelay/keepalive errors; client-receive logs them

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-daemon/src/service/push/data_plane.rs:115-116` — `let _ = s2.set_tcp_nodelay(true); let _ = s2.set_keepalive(true);` — both errors silently dropped.
  2. `crates/blit-core/src/remote/transfer/data_plane.rs:80-90` — `socket.set_tcp_nodelay(true).context("setting TCP_NODELAY")?;` propagates the error as a hard `Err`. `socket.set_keepalive(true)` → `log::warn!` on failure with the exact error message.

**Canonical**: The client side. Per POST_REVIEW_FIXES §1.1 cited in the client-side comment, "Surface failures via log so a misconfigured run isn't silent". Same rationale should apply to the daemon side.

**Recommendation**: Replace `let _ =` with `log::warn!`-on-failure on the daemon side, matching the client-side pattern. Per `RULES.md` "Professional Honesty": silent failure of a perf-relevant socket option is a smell.

---

### bridge-vs-daemon-rpc-timeout-magnitudes — Bridge scrape budget is shorter than daemon connect budget

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-prometheus-bridge/src/main.rs:36` — `ONESHOT_TIMEOUT: Duration = Duration::from_secs(8)` bounds the whole `jobs::query` future (which internally does `connect_with_timeout` + `GetState` RPC).
  2. `crates/blit-prometheus-bridge/src/server.rs:47` — `SCRAPE_TIMEOUT: Duration = Duration::from_secs(8)` for the same call in HTTP-server mode.
  3. `crates/blit-app/src/client.rs:24` — `CONNECT_TIMEOUT: Duration = Duration::from_secs(30)` for the underlying `connect_with_timeout`. Plus `Endpoint::connect_timeout(30s)` inside.

**Canonical**: The bridge's 8s budget is owner-decided ("below Prometheus's 10s default", per `main.rs:31`) and IS the intended outer cap. But because the inner connect helper still uses 30s, a slow-but-eventually-resolving connect can run for the full bridge budget while the connect helper still thinks it has 22s of headroom. The behaviour is correct (outer timeout wins) but the constant pair (8s outer, 30s inner) is not co-designed.

**Recommendation**: Document the relationship at both sites — add a comment on `CONNECT_TIMEOUT` noting "callers may apply a tighter outer timeout (e.g. bridge ONESHOT_TIMEOUT = 8s)". Consider exposing `connect_with_timeout_within(timeout, uri)` so the bridge can pass its own 8s budget down rather than relying on outer cancellation. Same applies to any future caller that wants a tighter SLA.

## Low severity

### dead-watch-flag-interval-ms — `jobs watch --interval-ms` is a dead flag preserved for back-compat

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-cli/src/cli.rs:118-124` — `--interval-ms` defaults to 1000, doc-comment says "preserved for backward compat per comment but does nothing under the streaming Subscribe model". `code-cli.md` inventory flagged this as a smell.
  2. `crates/blit-cli/src/jobs.rs:174-368` — the watch loop never reads `args.interval_ms`. Confirmed by grep.

**Canonical**: Either remove the flag (back-compat break) or hide it (`hide = true` in clap derive) so users don't see it in help and tune-tune-tune with no effect.

**Recommendation**: Mark `--interval-ms` with `#[arg(long, default_value_t = 1000, hide = true)]` and add a `#[deprecated]` doc-comment. A user who passes `--interval-ms 50` today gets zero feedback that the knob is inert — UX hazard.

---

### watch-timeout-zero-means-forever — `--timeout-secs 0` semantics differ across surfaces

**Dimension**: Timeouts, retries, cancellation, stalls
**Instances**:
  1. `crates/blit-cli/src/cli.rs:125-129` — `jobs watch --timeout-secs` defaults to 0 meaning "wait forever".
  2. `crates/blit-daemon/src/service/core.rs:1027` — `recent_limit == 0` means "daemon default of 50" (the sentinel value is positional in the request, not absent).
  3. `crates/blit-cli/src/cli.rs:102-106,530-535` — `jobs list --recent-limit 0` = daemon default; profile `--limit` defaults to 50.

**Canonical**: There is no consistent rule across the CLI: in some places 0 = "use server default", in others 0 = "no limit", in others 0 = "wait forever". `--max-depth 0` in `du` likewise means "unbounded" (`crates/blit-daemon/src/service/core.rs:1156` — "`max_depth==0` → `None` (unbounded)"). Same numeric value, three different semantics across three flags.

**Recommendation**: Document the 0-sentinel meaning explicitly at each clap-derive site. The pattern is fine as long as it's documented; today it's only documented at one of three sites. Consider `Option<u64>` (clap's `value_parser` supports it) so an absent flag and a present-zero-value are disambiguated where meaningful.

---

### bridge-method-allowlist-404-not-405 — Non-GET request → 404 instead of 405

**Dimension**: Timeouts, retries, cancellation, stalls (collateral: error response shapes)
**Instances**:
  1. `crates/blit-prometheus-bridge/src/server.rs:269-278` — `request_target` returns `Some(target)` only for GET; HEAD/POST/PUT all fall through to None → 404 Not Found.
  2. The HTTP/1.1 standard says non-allowed methods should return 405 Method Not Allowed; the bridge inventory flagged this as a quirk (smell #7 in `code-bridge-proto.md`).

**Canonical**: 405 is the standard answer; the bridge intentionally chose 404 for code simplicity. Documented as a quirk, not a bug.

**Recommendation**: Either accept the quirk and add a `// non-standard: 404 instead of 405 for simplicity` note at the call site, or implement 405 with an `Allow: GET` header. Low because no real Prometheus scrape rig probes with HEAD/POST.

