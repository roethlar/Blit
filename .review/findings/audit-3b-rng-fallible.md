# audit-3b-rng-fallible: `generate_token` returns Result instead of panicking

**Severity**: Robustness
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `eeb7c16`
**Parent finding**: `audit-3-panic-resilience` (part 2 of 2; audit-3a covered
the ActiveJobs mutex poisoning). Together these resolve audit-3.

## What

Part 2 of audit-3. `generate_token()` (data-plane handshake token) did
`SysRng.try_fill_bytes(&mut buf).expect("system RNG failed")`. The OS
cryptographic RNG is effectively always available, but `try_fill_bytes`
is fallible — a restricted/sandboxed container or fd exhaustion can deny
it. On failure the **spawned** data-plane task panicked
(`control.rs:218` spawns `accept_data_connection_stream` with the token),
so the control-plane stream would hang waiting for a data-plane
handshake that would never arrive (no error surfaced to the client).

## Approach

`generate_token() -> Result<Vec<u8>, Status>`: map the RNG error to
`Status::internal("system RNG unavailable: …")` and `?` it. All six
callers are RPC-handler helpers already returning `Result<_, Status>`,
so each propagates with a single `?`:

- `service/pull.rs:165,275`
- `service/push/control.rs:209,285`
- `service/pull_sync.rs:564,697`

So an RNG failure now becomes a clean `Status::Internal` RPC error
instead of a hung stream + panicked task.

## Files changed

- `crates/blit-daemon/src/service/push/data_plane.rs`: signature +
  `map_err`; test.
- `crates/blit-daemon/src/service/pull.rs`,
  `service/push/control.rs`, `service/pull_sync.rs`: `generate_token()?`
  at the 6 call sites.

## Tests

`blit-daemon` (+1):

- `generate_token_returns_full_length_random_token` — returns `Ok` with
  `TOKEN_LEN` bytes; successive tokens differ. The error arm
  (`Status::Internal`) is unreachable without injecting a failing RNG,
  which would require abstracting `SysRng` behind a trait — out of
  proportion to a propagate-instead-of-panic fix — so it is not
  fabricated. The value of the change is the type signature callers now
  rely on (no panic path).

## Scope

Completes `audit-3-panic-resilience` (with audit-3a). Daemon-only;
happy-path behavior is unchanged (RNG succeeds → same 32-byte token).

## Reviewer comments

(empty — pending review)
