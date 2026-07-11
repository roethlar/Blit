# otp-10c-1 — codex verdict adjudication

reviewer: gpt-5.5 (codex exec, model gpt-5.6-sol, read-only; raw output
`.review/results/otp-10c-1.codex.md`)
slice commit: `f53f5a4`
codex verdict: FAIL — 3 findings (2 Medium, 1 Low)

## F1 (Medium) — the counter lost its only positive control — **Accepted**

`remote_remote.rs`: the deleted relay e2e was the only test asserting
`cli_data_plane_outbound_bytes > 0`; every survivor asserts `== 0`, and
`read_counters` maps a missing/unreadable file to zero — so silently
broken instrumentation (flag wiring, recorder, parser) would leave the
load-bearing delegated isolation pins green vacuously. Verified against
source: the recorder's four call sites all live in
`remote/transfer/data_plane.rs` send paths, so a local→remote push (the
CLI is the session SOURCE by design) is a legal positive probe through
the same flag, file, and parser.

Fix: `local_to_remote_push_is_the_positive_counter_control` (new e2e,
real daemon, 2 MiB payload, asserts `>= payload` bytes recorded).
**Guard-proven by temporary mutation**: with
`record_cli_data_plane_outbound_bytes` no-op'd, the test FAILS with
`counters=CounterValues { cli_data_plane_outbound_bytes: 0 }`; restored,
it passes. (First cut of the test asserted the wrong landing path —
the rsync no-trailing-slash rule nests the source directory — caught on
its own first run and corrected before the mutation proof.)

## F2 (Medium) — live guidance still advertises the removed flag — **Accepted**

All cited sites verified and fixed: `cli.rs` `--detach` help (two
mentions), `README.md` relay example, `docs/cli/blit.1.md` flag entry +
"silently relaying" phrasing (now states there is no fallback and names
the two-hop), `docs/DAEMON_CONFIG.md` "upgrade-or-relay" fallback text,
`docs/perf/remote_remote_benchmarks.md` relay leg/bullets/table row.
Root cause of the miss: the slice's docs grep was `head`-truncated —
the sweep below re-ran unbounded. Additionally fixed beyond codex's
list (same class, found by the unbounded sweep):
`docs/ARCHITECTURE.md` (module table, source-impl bullet, per-direction
wiring row) and `docs/WHITEPAPER.md` (combination table row, sources
bullet) still presented `RemoteTransferSource`/relay as the live
remote→remote path.

## F3 (Low) — stale relay references in surviving code/records — **Accepted**

- `transfer_session/mod.rs` filter-chokepoint comment and
  `transfer_session_roles.rs` `FilterIgnoringSource` comments retyped:
  the fake models "a source impl that ignores scan(filter)" (which the
  contract permits) rather than citing the deleted type as live.
- `docs/plan/LOCAL_ERROR_TELEMETRY.md` (Draft) route list: the
  `RemoteToRemoteRelay` variant annotated as deleted (D-2026-07-11-1)
  and dropped from the scope sentence.
- `REVIEW.md` `relay-1-subpath-double-join` (open, Low): **closed as
  moot** at `f53f5a4` — the relay scan whose rel_path double-join it
  reported was deleted with the flag.

Not churned (dated records, kept verbatim per their own conventions):
`docs/audit/**`, `docs/reviews/**`, Historical plans
(`BENCH_VERB_PLAN`, `PIPELINE_UNIFICATION`, `UNIFIED_RECEIVE_PIPELINE`,
`RELEASE_PLAN_v2`), `TODO.md` resolved `[x]` entries, `DEVLOG.md`.

## Sweep

Unbounded re-grep for `relay-via-cli|relay_via_cli|RemoteToRemoteRelay|
RemoteTransferSource` across crates/docs/scripts/proto now hits only:
self-referential removal notes (dispatch/remote.rs docs, DECISIONS,
REVIEW moot row, this record), the dated records above, and comments
inside files otp-10c-2 deletes whole (`remote/pull.rs`,
`grpc_fallback.rs` scope lists) — left for that slice's sweep.

fix sha: `27bef56`
suite after fixes: 1585 → 1586 (+1 positive control; gate green:
fmt, clippy -D warnings, cargo test --workspace 1586/0)
