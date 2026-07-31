# `--checkers`: runtime-discovered destination-comparison concurrency

**Status**: Evidence. 2026-08-01, netwatch-01 (32 logical CPUs).
Plan: `docs/plan/LOCAL_SMALL_FILE_PATH.md`.

## The correction this records

An earlier pass in this slice parallelised the destination diff on rayon's
GLOBAL pool, measured 26× effective concurrency for 1.34× throughput, and
concluded the destination was **saturated** — that concurrency was not the
lever. That conclusion was wrong, and it was drawn from a single datapoint at
one thread count on a pool that is CPU-sized, work-stealing, and shared with
the apply path. The caveat "no pool-size sweep was run" was recorded in the
same document that drew the conclusion.

The sweep, once actually run against a DEDICATED pool, says the opposite.

## The sweep

`blit copy 'D:\Apps\' 'H:\apps\' --dry-run --checkers N`, 46,041 files to the
owner's SMB share, converged tree so the run is pure comparison.

| configuration | wall | vs original |
|---|---:|---:|
| original (sequential, before this slice) | 273.57 s | — |
| `--checkers 1` | 226.40 s | 1.21× |
| `--checkers 8` | 170.94 s | 1.60× |
| `--checkers 16` | 165.80 s | 1.65× |
| **adaptive (no flag)** | **166.38 s** | **1.64×** |

Two things to read off it:

1. **The destination was never saturated.** 8 dedicated threads (170.94 s)
   beat 32 shared rayon-global threads (203.86 s) by 33 seconds. The earlier
   "saturation" was contention inside the wrong pool, not a limit of the SMB
   server.
2. **The curve knees between 8 and 16** — 170.94 → 165.80 s is 3% for double
   the concurrency, so there is little left past 16 on this destination.

Against the owner's original field measurement of **283.92 s**, the shipped
default is **1.71× faster**.

## Why it is discovered at runtime, not defaulted

The right concurrency is a property of the DESTINATION. A local NVMe target
wants very few; a high-latency SMB share wants many; a loaded server wants
fewer again. None of that is knowable before the transfer starts, and a
hard-coded 8 would be a guess that happens to suit one share.

`AdaptiveCheckers` follows the rule `dial.rs` already uses for stream
membership (D-2026-06-20-1/-2): start at a conservative floor, no probe
phase, no guess from workload shape — begin immediately and step one rung per
chunk on measured evidence. Throughput improves ⇒ climb. Throughput regresses
⇒ give the last step back and settle. Flat ⇒ settle. 5% either way is treated
as noise.

The adaptive run landed at 166.38 s, within **0.4%** of the best hand-tuned
value, without being told anything.

There is **no advertised flag** for this. Per D-2026-08-01-1, tuning the
program can determine at runtime must be determined at runtime — SIMPLE
constrains the user-facing contract, so a knob the user would have to reason
about is a cost with no benefit. `--checkers` exists `hide = true`, for
diagnostic runs like the sweep above and nothing else; a pinned value is
never second-guessed by the controller.

## Design notes

- **Dedicated pool, never the global one** (cr-ls1-6). Blocking destination
  I/O on rayon's shared pool could stall the tar-shard apply path and
  concurrent daemon sessions — a hazard invisible to any single-session
  benchmark.
- **Concurrency is bounded by SLICING**, not by resizing the pool: rayon
  pools have a fixed thread count, so the pool is built at the ladder ceiling
  and each slice admits at most the current limit. Idle threads park.
- **Duplicate destination paths in one chunk fall back to sequential**
  (cr-ls1-7b), because the diff can repair attributes in place and two
  concurrent repairs of one file is a write race.
- **Errors are selected positionally** (cr-ls1-7a), so which error surfaces
  does not depend on scheduling.

## Caveats

- One destination, one tree. The knee is a property of THIS SMB share; the
  controller exists precisely because it will differ elsewhere.
- Single run per configuration; no variance estimate.
- `--dry-run`, so no attribute repair and no writes. The copying case is
  unmeasured.
- The remote/wire carrier still diffs sequentially. The same latency argument
  applies, but a daemon serves many sessions at once — which is exactly the
  cr-ls1-6 hazard — so it needs a concurrent-session measurement before it
  gets a pool.
- The controller settles once and does not re-probe. A destination whose
  behaviour changes mid-run keeps the rung it settled on.
