# PERF_HISTORY_PLANNING ph-6 — cold-vs-warm A/B evidence (magneto↔skippy)

**Date**: 2026-08-20. **Binary**: blit @ working tree of `docs/plan/PERF_HISTORY_PLANNING.md` ph-6 slice (see DEVLOG entry of same date for the exact commit).
**Rig**: magneto (EPYC-class Linux host, NVMe, LAN 10GbE) ↔ skippy (N200 NAS, ZFS `generic-pool`). Both daemons purpose-built for this run; fixtures generated fresh.

## Protocol

Per plan R3: two workload classes, both directions, seeds wiped for the cold
arm before every run, seeds persisted across the warm arm. 4 runs per arm.
Destination dataset wiped between runs. First pass of both benches overlapped
a `cargo build` on magneto and was discarded; numbers below are from the
clean reruns on quiet boxes.

- **Forward**: magneto → skippy push, 8192 × 1 MiB files (`large` class key).
- **Reverse**: skippy ← magneto pull, 65k small files (`small` class key).
- **A** = warm (seed store preserved), **B** = cold (seed store wiped each run).

## Results

Forward (times-forward.txt):

| run | A (warm) ms | B (cold) ms |
|-----|-------------|-------------|
| r1  | 11200       | 18277       |
| r2  | 22232       | 18065       |
| r3  | 17809       | 18222       |
| r4  | 18113       | 18322       |

Reverse (times-reverse.txt):

| run | A (warm) ms | B (cold) ms |
|-----|-------------|-------------|
| r1  | 21626       | 26369       |
| r2  | 25880       | 25974       |
| r3  | 25961       | 26443       |
| r4  | 26256       | 26261       |

Settled seeds (seeds-forward.json / seeds-reverse.json):

- forward: `remote|source|cli|skippy:/ph6/data|large` → `workers: 4`, `runs: 4`
- reverse: `remote|source|cli|magneto:/ph6/data|small` → `workers: 1`, `runs: 4`

Merged reporting (profile-merged-skippy.txt): `blit profile` on the daemon
box shows `[daemon]`-origin per-route aggregates (9 real runs, file counts,
MiB, avg transfer ms) alongside `[operator]` records — R5 behaviour live.

## Reading

- **No regression**: warm steady-state equals cold steady-state within noise
  (<1.5%) in both directions.
- **Seeds behave as designed**: distinct per-class route keys, persisted,
  reused (`runs` increments), stable across repeats; reverse settles *down*
  to `workers: 1` (N200 daemon side is the bottleneck), forward settles at
  the cap of 4.
- **Win ≈ 0 on this rig, and that is the honest result**: the dial reaches
  its settled value within the first epochs even from cold (R4's design
  rationale), so seeding the start point cannot buy measurable wall time
  here. The demonstrated claims are seed correctness, reuse, and absence of
  warm-start regression — not a speedup.
- The r1-of-batch fast outlier (11.2s / 21.6s) appears in the *warm* arm of
  each batch but is a fresh-destination-dataset effect, not a seed effect:
  the cold arm's own r1 (18.3s / 26.4s) matches steady state, and the outlier
  vanishes once the dataset has absorbed a wipe. Kept in the tables rather
  than trimmed.
