# Inside COMPARE: what the 273 seconds actually are, and what fixed 25% of it

**Status**: Evidence. 2026-07-31, netwatch-01 (32 logical CPUs).
Plan: `docs/plan/LOCAL_SMALL_FILE_PATH.md`. Follows `README.md` in this
directory, which established that COMPARE owns ~100% of a converged run.

## Sub-attribution (serial diff, same command as step (0))

46,041 files, `D:\Apps` → SMB `H:\apps`, `--dry-run`, wall 270.89 s:

| sub-phase | total | % wall | per file |
|---|---:|---:|---:|
| COMPARE (whole) | 270.83 s | 100.0% | — |
| **COMPARE_METADATA** | **168.18 s** | **62.1%** | **3.653 ms** |
| COMPARE_STAT | 49.00 s | 18.1% | 1.064 ms |

`destination_needs` performs exactly two destination I/O operations per
file: `std::fs::metadata` (the stat) and
`windows_metadata::destination_verdict` (durable attributes + named-stream
enumeration). The metadata read is reached whenever size and mtime already
match — which on a CONVERGED tree is *every file*, so the owner's no-op
mirror pays it 46,041 times.

The remaining ~54 s of COMPARE is the rest of the per-file work (path-safety
join, cross-platform support validation, status computation) plus chunk
overhead.

## The fix that worked, and how little it bought

The diff was a plain sequential `for header in chunk` inside one
`spawn_blocking` — roughly 138,000 blocking round trips issued strictly one
after another. It now runs `into_par_iter()` across the chunk.

| workload | serial | parallel | speedup |
|---|---:|---:|---:|
| SMB, 46,041 files (wall) | 273.57 s | **203.86 s** | 1.34× |
| local NVMe, 20,000 files (median of 3) | 1863 ms | **1366 ms** | 1.36× |

Consistent across both, and no regression on local disk. It lands.

## The negative result, which matters more

**Client-side concurrency is not the lever.** Under 32 rayon threads the
per-call costs exploded:

| sub-phase | serial per file | parallel per file | ratio |
|---|---:|---:|---:|
| COMPARE_METADATA | 3.653 ms | 88.513 ms | 24.2× |
| COMPARE_STAT | 1.064 ms | 26.497 ms | 24.9× |

Aggregate thread-time across those two phases rose to 5,295 s inside a
203.86 s wall — about 26× effective concurrency achieved, converted into
1.34× throughput. That is a saturated resource, not a scheduling problem:
the SMB metadata path serves roughly the same total operations per second no
matter how they are issued.

**So the binding constraint is the NUMBER of round trips per file, not their
order.** ~46k files × (1 stat + 1 metadata read) is ~92,000 destination
operations, and this server will serve them in ~200 s however they are
scheduled.

## What this says about the plan

`LOCAL_SMALL_FILE_PATH.md`'s L1–L4 remain falsified for this workload (all
apply-path; apply is zero on a converged run). This document adds a second
falsification: **"blit does not convert concurrency into throughput" is not
the whole story on the compare side** — here blit converts concurrency into
throughput about as well as the destination permits, which is barely at all.

The next lever is round-trip elimination, not parallelism. The obvious
candidate is directory-level enumeration: one `FindFirstFile`/`FindNextFile`
sweep per directory returns size, mtime and attributes for every entry in
it, which would collapse ~92,000 per-file operations into roughly one per
directory. That is how robocopy walks a destination. It is a real change to
compare semantics — named-stream enumeration still needs per-file work for
files that have streams, and the correctness of the metadata verdict
(rel-4/pfc-6) has to survive — so it is a scoped slice, not a tweak.

## Caveats

- One SMB server, one tree, one run per configuration on the SMB arm (the
  local arm is a median of 3). No variance estimate on the SMB numbers.
- `--dry-run`, so no attribute repair and no writes. The owner's first field
  run repaired 5,445 files; that cost remains unmeasured.
- The copying case remains unmeasured; this is the converged case only.
- Whether the ~4.7 ms/file is network RTT, server-side work, or the Windows
  SMB redirector is still not separated — only that it saturates.
- 32 threads is rayon's default pool on this machine. No sweep was run to
  find whether a smaller pool gives most of the 1.34× with less server load.
