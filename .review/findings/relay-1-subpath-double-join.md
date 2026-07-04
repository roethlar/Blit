# relay-1: relay of a subpath source double-joins the endpoint rel_path

**Source**: ue-r2-1h self-review panel (wire-compat lens), 2026-07-03.
**Severity**: Low (latent user-facing break of `--relay-via-cli` with a
subpath source; the default delegated path is unaffected).
**Status**: Open — deferred out of ue-r2-1h (the port is faithful to
the deleted code, which had the identical bug).

## What

`blit copy --relay-via-cli host:mod/sub dst:…` scans the wrong path:
`crates/blit-app/src/transfers/remote.rs` (Endpoint::Remote arm) builds
`RemoteTransferSource` with `root = endpoint.path.rel_path` (= `sub`),
and the relay primitives (`scan_remote_files`/`open_remote_file`,
`crates/blit-core/src/remote/pull.rs::build_relay_session_spec`) join
`endpoint.rel_path.join(path)` — the same component twice. The daemon
receives `source_path = "sub/sub"` → `not_found` (or the wrong subtree
if `sub/sub` exists). Verified pre-existing: the deleted
Pull-RPC-based `scan_remote_files`/`open_remote_file` performed the
identical double join, so this is not a ue-r2-1h regression — but the
port kept it, and the new tests use module-root endpoints (e2e) or
assert the join semantics as-is (wire tests), so nothing pins the
subpath case correctly yet.

## Fix direction

Decide the semantic once: either the relay source's `root` is
endpoint-relative (then `RemoteTransferSource::new` should receive
`""`/`.` and the endpoint rel_path carries the subtree), or the
primitives take module-relative paths (then `build_relay_session_spec`
should not join `endpoint.rel_path`). Add a dual-daemon e2e with a
subpath source (`host:mod/sub`) asserting the relayed tree lands
correctly.
