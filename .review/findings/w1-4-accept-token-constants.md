# w1-4-accept-token-constants — one shared accept/token timeout pair

**Branch**: `master`
**Commit**: `6a19e1d`
**Source**: `docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md` §W1.4 (ratified
D-2026-06-11-2), findings
`duplication-accept-token-timeout-quadruple` /
`constants-accept-token-timeout-quadruplication`.

## What

The audit found the accept(30 s)/token(15 s) bound pair declared four
times. One declaration died with the Pull RPC at ue-r2-1h; at HEAD
there were three:

- `push/data_plane.rs`: `DATA_PLANE_ACCEPT_TIMEOUT` /
  `DATA_PLANE_TOKEN_TIMEOUT` (the pair with the full R46-F7 rationale
  doc comments).
- `pull_sync.rs` module scope: `PULL_ACCEPT_TIMEOUT` /
  `PULL_TOKEN_TIMEOUT` (same values, R47-F5/R46-F7 comment).
- `pull_sync.rs` resume path, function-local: `ACCEPT_TIMEOUT` /
  `TOKEN_TIMEOUT` behind a `StdDuration2` alias.

Three names for one policy meant a future retune could miss a copy and
silently split the daemon's liveness behavior across paths.

## Approach

- The pair now lives in `blit_core::remote::transfer::socket` —
  `pub const DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT`
  (30 s / 15 s, values unchanged), carrying the push side's R46-F7
  rationale doc comments (the most complete of the three). Home
  rationale: the module is already the data-plane socket policy owner
  (w1-2/w1-3); these are the bounds for *establishing* those sockets.
  Re-exported from `remote::transfer` alongside
  `configure_data_socket`.
- All daemon uses renamed to the one shared name: push imports it
  (local pair deleted; `RESIZE_ARM_TTL = DATA_PLANE_ACCEPT_TIMEOUT`
  unchanged in meaning), pull_sync's module pair and the resume path's
  inline pair + alias deleted, every use site (accept timeouts, token
  reads, resize dial deadline at the controller, error messages) now
  reads the shared constants.

## Deliberately out of scope

- design-3 (client-side connect timeouts) — the shared pair is where
  that slice's bound can come from, but wiring it is design-3's work.
- No retuning: 30 s / 15 s preserved exactly.

## Tests

None added — the slice is compile-time constant wiring with
byte-identical values; there is no new behavior for a test to guard
(a value-equality test of a constant against itself is vacuous). The
bounded-accept / bounded-token behavior these constants feed is
already pinned by the existing timeout tests (audit-h3a family), which
all still pass.

Full suite: fmt clean, clippy clean (workspace, all targets,
`-D warnings`), `cargo test --workspace` 1446 passed / 0 failed /
2 ignored across 37 suites — unchanged from w1-3, no test count drop.

## Known gaps

- The gRPC control plane's 30 s connect bound (audit-2 family) remains
  its own constants in its own layer — deliberately: control-plane
  and data-plane policy families have different owners today, and
  fusing them is not in the ratified row.
