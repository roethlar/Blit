# bench_10gbe.sh fixes — codex adjudication

**Slice commit**: `b9befb8` (endpoint grammar + client gRPC flag)
**Raw review**: `.review/results/bench-script-fix.codex.md`
(codex verdict: **NEEDS FIXES**, 2 High — the two-line fix itself
confirmed correct, no other stale CLI syntax found)
**reviewer: gpt-5.5**

## Finding 1 — run_timed re-runs measure incremental no-ops

> `scripts/bench_10gbe.sh:80` — High — `run_timed` repeats the same
> command `RUNS` times, but callers clean destinations only once
> before entering it. Since `blit copy` skips unchanged files by
> default, run 2..N for rows labeled `copy` are mostly no-op timings,
> so `avg`/`best` do not represent full-copy performance.

**Verdict: ACCEPTED.** Independently diagnosed during the session from
the run-1-vs-run-2 timing cliff (1 GiB "copy" runs of 934 ms then
4 ms); the session's gate numbers were taken from run-1/cold data and
targeted clean runs, so no reported figure was contaminated — but the
script itself produced misleading avg/best columns.

**Fix**: `run_timed_fresh` recreates the local destination before
every run; all local/NFS/SMB/pull/rsync copy rows use it (`noop` rows
deliberately keep bare `run_timed`).

## Finding 2 — shared module root invalidates the remote matrix

> `scripts/bench_10gbe.sh:178` — High — the remote matrix reuses one
> module-root endpoint for every workload and transport. TCP push
> populates `$REMOTE`; gRPC push then targets the same resolved
> paths, so size+mtime need-listing can skip payloads despite
> `--force-grpc`. Pull rows also all read the same accumulated module
> root, so `large`/`small`/`mixed` pull labels are not isolated.

**Verdict: ACCEPTED.** Same live diagnosis (the gRPC push phase
recorded 4 ms "transfers"); worked around during the session with
fresh per-transport subdirs and per-workload subpath pulls.

**Fix**: `push_timed` targets a fresh `push_<label>_r<run>/` module
subdir every run; pull rows read the isolated
`push_tcp_<workload>_r1/<src>/` subpath; a session note warns that
pushed bench dirs accumulate on the module for manual cleanup.

**Fix commit**: `92d6326`; validated end-to-end:
full matrix re-run on the real 10 GbE pair completed exit 0 with
avg≈best on every row (each run a true full transfer) — results in
`logs/bench_10gbe_20260704T203208/`.

## Methodology note (recorded for the benchmark record)

No `sync`/ARC eviction runs between iterations by design: the matrix
measures engine-vs-wire (ZFS absorbs pushes async; re-reads serve
from ARC; local ends are tmpfs). Correct isolation for the REV4
gates; disk-path variants (post-push `zpool sync` timing, cold-ARC
pulls via `primarycache`) are owner-gated follow-ups.
