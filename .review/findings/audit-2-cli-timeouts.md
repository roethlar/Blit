# audit-2-cli-timeouts: Missing connection timeouts on CLI gRPC connections

**Severity**: Robustness
**Status**: Open
**Branch**: (none yet)

## What

Ground-up audit found that every CLI/admin-verb gRPC `connect()` call is made
without a `tokio::time::timeout()` wrapper. If a daemon is unreachable (DNS
slow, TCP handshake hangs, network partition), admin verbs and transfer
commands hang indefinitely. The only timeout in the entire CLI/app layer is
on the streaming `message()` call in `jobs watch`.

Affected call sites (all in `crates/blit-cli/src/` and `crates/blit-app/src/`):

- `completions.rs:75` — `BlitClient::connect`
- `admin/ls.rs:71` — `BlitClient::connect`
- `admin/du.rs:40` — `BlitClient::connect`
- `admin/find.rs:43` — `BlitClient::connect`
- `admin/df.rs:29` — `BlitClient::connect`
- `admin/rm.rs:23` — `BlitClient::connect`
- `admin/jobs.rs:24,63,126` — three `BlitClient::connect` sites
- `admin/list_modules.rs:28` — `BlitClient::connect`
- `transfers/remote.rs:335,467,474,695,840` — `RemotePullClient`/`RemotePushClient` connects
- `admin/jobs.rs:23-33` — `jobs::query()` has no internal timeout (bridge depends on this)

This violates the project's documented principle
(`feedback_server_await_timeouts.md`) and matches the pattern the reviewer
has reopened before (bridge-2 slowloris read, hung scrape).

## Approach

Wrap each `connect()` + first RPC call in `tokio::time::timeout(30_000, ...)`
(or a configurable value). A single helper in `blit-app` (e.g.
`connect_with_timeout`) would eliminate the duplication.

`jobs::query()` should also gain an internal timeout or accept a timeout
parameter, so callers (bridge, future consumers) don't all need their own
wrappers.

## Files changed

TBD by coder. Approximately 15 call sites affected.

## Tests

- Unit test: timeout fires → user-visible error message (not a panic)
- Existing admin-verb integration tests must still pass
