# sf-3a per-file receive-cost profile — 2026-08-13

**Status:** Complete analysis-only evidence for
`docs/plan/SMALL_FILE_CEILING.md` sf-3a. No product code changed.

## Bottom line

The unified receive sink pays substantial blit-controlled fixed work for every
small file on both measured destination roles. The two profiles independently
show the same filesystem contract: two path-containment walks, two opens/two
closes, about two stats, one parent-directory attempt, one timestamp update,
and one permission update per file. The first sf-3x cuts should reduce that
fixed work before considering a wire-visible tar-shard lane.

This evidence does **not** declare a hardware ceiling. `strace -f -c` perturbs
thread scheduling heavily, and its reported seconds are elapsed syscall time
summed across traced threads. They can exceed wall time, are not additive, and
are not predicted wall-clock savings. Call/error counts and the one-file raw
attribution are the decision evidence; every accepted cut still needs its own
untraced A/B and loopback proxy guard.

## Provenance and method

- Exact source: `382090265e8ad2b5898a53d9d280313540a020ea`
  (`blit 0.1.2+382090265e8a`).
- Release binary SHA-256:
  `blit` `a5e3f883300892b7532c05a782120f5831542fc31ea7c3b8f021c4bf4b276d12`;
  `blit-daemon`
  `984b6d4054c81af02b10c3288d4aab5726d1aaa60c2c8d5872d55af262641ee4`.
- Fixture: 100,000 regular files × 4,096 bytes = 409,600,000 bytes. Each
  completed destination was verified at exactly that count and byte total.
- Same-OS 10 GbE rig: magneto (Linux 7.1.6, Intel N200, four CPUs, NVMe Btrfs)
  ↔ skippy (Linux 6.12.95 TrueNAS, EPYC 7313, 32 CPUs, ZFS
  `generic-pool/video`). Port 9032; mDNS disabled.
- Daemon receive: magneto CLI pushed into a skippy daemon run under
  `strace 6.1 -f -c`. Traced wall: **146.635 s**, exit 0, 100,000 files.
- Client pull-write: skippy served the same shape from an untraced daemon;
  the magneto CLI/destination ran under `strace 7.0 -f -c`. Traced wall:
  **62.857 s**, exit 0, 100,000 files.
- A one-file daemon receive then ran under a filtered `strace -f -yy` to
  attribute the two opens and metadata tail. See `daemon-attribution.txt`.
- Aggregate summaries: `daemon-receive.strace.txt` and
  `client-pull-write.strace.txt`. Rows rounded below 0.01% are retained where
  named by the captured summary; unnamed zero-percent tail rows are omitted.

The 100,000-file size was derived from the committed 10,000-file baseline,
not selected by byte volume. It supplies 100,000 observations of every
per-file operation while 409.6 MB has a ~0.33 s 10 GbE wire floor, keeping the
profile shaped by file work rather than bulk throughput.

## Per-file syscall inventory

Counts are aggregate calls divided by 100,000 completed files. “Seconds” is
the aggregate threaded value reported by `strace`, included to rank observed
cost within one profile only.

| syscall/group | daemon receive | client pull-write | attribution |
|---|---:|---:|---|
| `readlink` | 35.523/file; 3,552,301 errors; 232.921 s | 27.145/file; 2,714,536 errors; 39.882 s | `safe_join_contained` canonicalizes from the filesystem root at both destination comparison and sink write; path depth explains the role difference. |
| `openat` | 2.001/file; 2060.663 s | 2.001/file; 15.231 s | One create/truncate, then one read-only reopen by `filetime` for descriptor-based `utimensat`. |
| `close` | 2.000/file; 98.139 s | 2.001/file; 7.851 s | One close after payload flush, one after the timestamp reopen. |
| `statx` | 1.999/file; 100,000 errors; 99.179 s | 1.999/file; 100,000 errors; 5.803 s | The missing-target compare contributes one error/file; the other call class includes filesystem/runtime checks such as `create_dir_all`'s existing-parent handling. Do not cut it without a focused attribution guard. |
| `mkdir` | 1.000/file; 99,901 errors; 63.458 s | 1.000/file; 99,899 errors; 4.101 s | `write_file_stream` calls `create_dir_all(parent)` for every file although only about 100 distinct fixture directories need creation. |
| `utimensat` | 1.000/file; 66.673 s | 1.000/file; 4.665 s | Required by `preserve_times`; currently follows the extra reopen. |
| `chmod` | 1.000/file; 95.143 s | 1.000/file; 4.521 s | Required by Unix permission preservation; currently path-based after timestamping. |
| `write`/`writev` | 1.078/file; 65.651 s | 1.055/file; 6.132 s | Payload/progress/control writes; not selected as an sf-3x cut by this profile. |
| `recvfrom` | 6.143/file; 40.386 s | 6.251/file; 11.405 s | Data-plane framing/network receive; materially fewer calls than containment traversal. |
| `futex` | 6.107/file; 5354.211 s | 0.409/file; 417.227 s | Thread waits dominate traced elapsed time, but ptrace changes scheduling; this is a sampling/phase-observer question, not an admitted cut. |
| `sched_yield` | 23.678/file; 736.766 s | 1.622/file; 4.971 s | Same caution as `futex`; the large role delta requires non-ptrace attribution before any worker-policy inference. |

There is no temp-file-plus-rename pattern on this path: the one-file trace
shows direct `O_CREAT|O_TRUNC`, and the 100,000-file daemon summary contains
only one `rename` total. The plan's provisional temp/rename candidate is
therefore rejected for the current unified streamed receive path.

## Ordered sf-3x candidates

Order is risk-adjusted implementation order, not raw syscall count.

1. **Remember parent-directory readiness inside one sink session.** Replace
   per-file `create_dir_all(parent)` with a concurrency-safe once-per-parent
   operation, while evicting/invalidating on an observed directory failure.
   Expected direct saving on this fixture: 99,899 client and 99,901 daemon
   failed `mkdir` calls (about one call/file), plus whatever existing-parent
   stat is proven by the slice's focused guard. This is the smallest semantic
   change and should be sf-3b.
2. **Keep the completed file descriptor through the metadata tail.** Convert
   the flushed Tokio file into/through a descriptor on which timestamp and
   permission preservation can run before final close, without reintroducing
   the documented deferred-write/mtime race. Expected saving: exactly one
   `openat` and one `close` per file — 200,000 syscalls per 100,000-file
   destination — while retaining one timestamp and one permission operation.
3. **Replace repeated full-path canonicalization with a secure contained-path
   primitive that amortizes directory traversal.** The upper-bound opportunity
   is 27.145 failed `readlink` calls/file on magneto and 35.523/file on
   skippy (2.71 M and 3.55 M calls here). A naive cache is forbidden: it would
   weaken the symlink-escape contract. Any slice must preserve fail-closed
   containment under concurrent path replacement, for example with
   handle-relative/no-follow resolution or an equivalently reviewed design,
   and must guard both compare and sink phases.

`statx`, `chmod`, and `utimensat` are accounted for but not independently
admitted as cuts. The first is not fully attributed; the latter two implement
requested metadata semantics. `futex`/`sched_yield` likewise require an
untraced phase/sampling measurement before they can justify scheduling work.

## Next gate

No sf-3x production change is authorized by this analysis. The recommended
first implementation slice is candidate 1 (parent-directory readiness), with
its own plan-approved code slice, loopback syscall/count proxy, red/green
proof, untraced rig A/B, and risk-based review selection.
