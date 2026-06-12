# w5-1-log-backend — stderr log backend (warn) in all 4 binaries + one prefix convention

**Branch**: `master` (owner-authorized branchless session 2026-06-11; AGENTS.md §8 forbids agent-created branches)
**Commit**: `56bda09` + follow-up `7145202` (sentinel refreshed; review range is `1adbe0c..7145202` minus the `d517935` bookkeeping commit)
**Source findings**: errors-log-facade-has-no-backend (reviewer: high), errors-stderr-prefix-babel (reviewer: medium) — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

Every `log::warn!`/`log::error!` in the workspace (~22 sites in blit-core,
including security-degradation warnings whose *only* surface is the warn) was
formatted and discarded because no binary installed a `log` backend. Stderr
prefixes were also babel: nine bracketed families plus unprefixed lines.

This slice installs a shared stderr backend in all four binaries and converges
the stderr prefix convention to `<binary>: ` (the style the bridge and
retry.rs already used), with log-facade lines rendered as
`<binary>: <level>: <message>`.

## Approach

- New `crates/blit-core/src/stderr_log.rs` (re-exported in lib.rs): a
  ~40-line `Log` impl on a `OnceLock` static. `init(binary)` is called first
  thing in each binary's `main()`. Default max level **warn**; `BLIT_LOG`
  (off|error|warn|info|debug|trace, case-insensitive) overrides; unparseable
  values fall back to warn. Idempotent (first install wins).
- Prefix convergence, by intent of each line:
  - **Genuine warnings/errors → log facade** (now visible): daemon mDNS
    advertise failure + config warnings (main.rs), recents-persist failure
    (active_jobs.rs), invalid pull/push data-plane tokens (pull.rs,
    push/data_plane.rs), tar shard worker error/panic (push/data_plane.rs),
    missing pull summary (core remote/pull.rs), file-logger write errors
    (core logger.rs), push skipping-unreadable (core push/client/helpers.rs —
    loses its former `.red()` coloring).
  - **Intentional operator/UX lines → eprintln with `<binary>: ` prefix**:
    daemon startup `[info]` lines, push data-plane accepted/token-accepted/
    aggregate-throughput lines (`blitd: …`), jobs watch failure/fallback
    lines and cancel-unsupported (`blit: …`), TUI drained config warnings
    (`blit-tui: …`).
  - **Unconditional per-file pipeline chatter → log::debug!** (visible with
    `BLIT_LOG=debug`): `[push] enqueue …` (2 sites ×2 variants),
    `[push] need-list includes …`, `[push-server] queued …`,
    `[push] daemon did not request N payload file(s)`. These printed one
    stderr line *per transferred file* unconditionally — spam proportional
    to file count and a per-file syscall on the hot path.
- **Deliberately untouched**: the trace-gated `[data-plane-client]` family
  (core transfer/data_plane.rs + the aggregate line in push/client/mod.rs).
  `remote_parity.rs:34` asserts the exact `[data-plane-client]` prefix under
  `--trace-data-plane`; converging it is a contract change a future slice
  can take deliberately. Unprefixed manifest-enumeration progress lines
  (helpers.rs:153/172) also untouched — deliberate stderr UX per R46-F4.

## Files changed

- `crates/blit-core/src/stderr_log.rs` (new) + `crates/blit-core/src/lib.rs`
- `crates/blit-core/src/{logger.rs, remote/pull.rs, remote/push/client/mod.rs, remote/push/client/helpers.rs}`
- `crates/blit-cli/src/{main.rs, jobs.rs}`
- `crates/blit-daemon/Cargo.toml` (+`log = "0.4"`), `src/{main.rs, active_jobs.rs, service/pull.rs, service/push/control.rs, service/push/data_plane.rs}`
- `crates/blit-prometheus-bridge/src/main.rs`, `crates/blit-tui/src/main.rs`
- `Cargo.lock`, `docs/STATE.md` (session-authorization note)

## Tests added

3 unit tests in `stderr_log.rs`: level strings render lowercase for the
prefix convention; unset/garbage `BLIT_LOG` defaults to warn; overrides
parse case-insensitively (off/error/debug/trace). Suite total grew
1331 → 1334; nothing removed.

## Follow-up commit (7145202)

The first sweep's grep missed multi-line `eprintln!` calls whose format
string starts on a continuation line. Converged in the follow-up: cli
perf-history warning + daemon stat warning → `log::warn!` (blit-cli
gains `log = "0.4"`); daemon pull accepted/aggregate + push
stream-complete/buffer-pool lines → `blitd: `; cli debug worker-limiter
+ verbose delegation lines → `blit: `. Deliberately left: the
jobs-watch ticker vocabulary (`[active]/[progress]/[done]/[not-found]/
[timeout]` — coherent single-command status output) and the client-side
throughput info lines (`[data-plane-client] aggregate`,
`[pull-data-plane] stream` — pair with the trace-gated family under
remote_parity.rs's prefix contract).

## Known gaps

- No end-to-end test asserting a `log::warn!` actually reaches a binary's
  stderr (would need a triggerable warn condition in a spawned binary;
  candidate for w9-6 harness-stderr-capture).
- blit-tui: a warn emitted while the alternate screen is up can smudge the
  frame until next redraw. Accepted tradeoff (slice mandates all 4 binaries);
  noted in main.rs comment.
- Per-file chatter demoted to debug is a deliberate behavior change
  (silent by default where it previously printed); flagged here for review.
- The `[data-plane-client]` trace family keeps its bracket prefix (test
  contract); full convention coverage needs a follow-up that updates
  `remote_parity.rs` in the same change.
- First `cargo test --workspace` run after the change showed 11 transient
  `blit_utils` failures ("daemon failed to listen") under cold-build parallel
  load; isolated re-runs and a clean full re-run are green (1334/0). Same
  harness contention class as tests-five-daemon-harness-clones → w9-3.
