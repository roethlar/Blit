# sf-3b parent-directory readiness — 2026-08-13

**Status:** Implementation and local/rig evidence complete; the plan-required
neutral review has not been invoked and the slice is not declared closed.

## Bottom line

The unified streamed receive sink now performs `create_dir_all` once per
destination parent per sink session instead of once per file. Concurrent first
users of one parent share an async once-cell, while different parents retain
independent initialization. An observed failure evicts only the cache
generation that saw it; if a previously ready parent disappears, the file-open
path recreates it and retries once.

The portable cost proxy went red before the cache (`16` concurrent files in one
parent produced `16` create attempts) and green afterward (`1`). A second guard
removes a cached parent between files and requires the later write to succeed
with exactly `2` total attempts.

## Implementation scope

- One session-owned map in `FsTransferSink`, keyed by full destination parent.
- The map mutex is held only for lookup/insert/invalidation, never across an
  await. Each value is its own `tokio::sync::OnceCell`, so unrelated parents do
  not serialize their filesystem work.
- Failed initialization and destination preparation/open failures invalidate
  by `Arc` identity. A late failure cannot evict a newer successful generation.
- A missing-parent create failure triggers one readiness refresh and one file
  create retry. Other create failures retain the existing per-file containment
  behavior.
- `resolve_destination` and its canonical containment check still execute for
  every wire path before this cache is consulted. The cache is readiness only,
  never path-safety authority.
- No CLI, wire, comparison, metadata, worker-policy, or local payload-path
  behavior changed.

## Untraced A/B

Three paired, alternating runs per destination role used the existing
10,000×4,096-byte fixture (40,960,000 bytes) on the magneto↔skippy 10 GbE rig.
Every run used a fresh destination and was verified at the exact file count and
byte total before cleanup. Timing covers the transfer command, not staging,
verification, or cleanup. These are regression/benefit observations, not a
hardware-ceiling claim: caches were not dropped, destination durability was not
forced, and three samples do not justify limiter attribution.

- Baseline source: `382090265e8ad2b5898a53d9d280313540a020ea`;
  `blit` SHA-256 `a5e3f883300892b7532c05a782120f5831542fc31ea7c3b8f021c4bf4b276d12`;
  `blit-daemon` SHA-256
  `984b6d4054c81af02b10c3288d4aab5726d1aaa60c2c8d5872d55af262641ee4`.
- Candidate base: `d5f5781ddc5503093a68df7cf23cdd779172d4a8` plus the sf-3b
  working-tree change; build identity
  `0.1.2+d5f5781ddc55.dirty.2976f89440c90d14`; `blit` SHA-256
  `72cb669ed53b61132c0113cf9a3b8211c26afd09d9cd54ca9183ff59ed0a8364`;
  `blit-daemon` SHA-256
  `487742b811de075e0dfd6b943704b3e9238fb7fa1b84d4ebf7761493fe04f4a6`.
- Live skippy address was `10.1.10.12`; the stale `10.1.10.143` default in an
  older harness caused connection-only preflight failures and produced no
  transfer. Only successful runs appear in `results.csv`.
- The fixture has 11 file-bearing parents, so the intended direct reduction is
  10,000 parent-create attempts to 11 per destination session (9,989 removed).

| destination role | baseline median (range) | candidate median (range) | delta |
|---|---:|---:|---:|
| skippy daemon receive, ZFS | 1,445 ms (1,433–1,452) | 1,447 ms (1,442–1,448) | +0.1% |
| magneto client receive, Btrfs | 768 ms (735–1,192) | 597 ms (582–720) | −22.3% |

The daemon arm is neutral within noise; the client arm shows a material benefit
but also a wide baseline range. The durable claim is therefore the proxy-proven
operation-count reduction with no observed wall regression, not a universal
22.3% speedup.

## Verification

- Focused sink module: 62 passed, 0 failed.
- `cargo fmt --all -- --check`: passed on magneto Linux.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed on magneto
  Linux.
- `cargo test --workspace`: passed on magneto Linux; the definitive warm rerun
  completed with exit 0.
- `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu
  --features blake3/pure -- -D warnings`: passed (compile-time Windows parity;
  no Windows runtime test was claimed).
- A/B staging, destinations, daemon, and listener were verified absent after
  cleanup.

## Remaining gate

`SMALL_FILE_CEILING` requires every sf-3x slice to pass a neutral review. No
review invocation was included in this implementation authorization, so that
owner-gated review remains before sf-3b is declared closed or sf-3c begins.
