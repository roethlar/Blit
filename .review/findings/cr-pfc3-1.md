# cr-pfc3-1: Disk-full errors are downgraded to per-member failures

**Severity**: HIGH — a destination filling during a shard contains every
subsequent member; at the landed slice a mirror exits 0 with an
incomplete backup (same silent-incomplete class as cr-pfc2-1).
**Status**: In progress — fixed, awaiting per-finding reviewer verdict
**Branch**: — (default-branch mode)
**Commit**: (filled at landing)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `ff38f0e6..3ae5bf4d`
(`.review/results/pfc-3-range.codex.json`).

## Evidence
`sink.rs:1146` routes shard write errors into the fold; `:1204-1207`
contains every error `failure_is_containable` accepts; `:193-205`
recognizes only `ReadOnlyFilesystem` + EROFS/ERROR_WRITE_PROTECT as
volume-level — `StorageFull`/ENOSPC and Windows ERROR_DISK_FULL are
not classified. `transfer_session/mod.rs:4646-4648` lets contained
failures through for mirrors.

## Predicted observable failure
Destination fills mid-shard: every remaining member is attempted and
contained; the mirror completes as a "successful" partial transfer
(warn lines only, until pfc-5's summary/exit-code land).

## What
Volume exhaustion is a volume-level condition wearing per-file errors,
exactly like write-protection — cr-pfc2-1's classifier needs the
disk-full/quota family. (cr-pfc2-1's Known gaps recorded this as an
open call; this finding settles it: fatal.)

## Approach
(planned) Extend `io_error_says_volume_unwritable`: kinds
`StorageFull` (+ `QuotaExceeded` if stable on the toolchain) and raw
codes unix ENOSPC 28 / windows ERROR_DISK_FULL 112 +
ERROR_HANDLE_DISK_FULL 39, cfg-gated like the existing lists. Guards on
BOTH the shard fold and the single-file path per the reviewer's
better_approach.

## Files changed
(filled with the fix commit)

## Guard proof
(planned) Synthesized disk-full chains through both paths return Err
even in a mirror; ordinary per-file errors still contain; red when the
disk-full family is removed from the classifier.

## Coder dispute (if any)
None.

## Known gaps
Transient-ENOSPC (space freed later) loses per-file continuation — the
deliberate trade: an exhausted volume fails thousands of members and
"success" would lie; convergence-on-retry re-runs after space is freed.

## Reviewer comments
(pending per-finding verification after the fix)

As built: kinds StorageFull + QuotaExceeded joined ReadOnlyFilesystem in `io_error_says_volume_unwritable`; cfg-gated raw backstops gained unix ENOSPC 28 / windows ERROR_DISK_FULL 112 + ERROR_HANDLE_DISK_FULL 39 (EDQUOT deliberately kind-carried — 122 linux vs 69 macOS is not one portable list; recorded in code docs). Guard: stripping the exhaustion family reds exactly the 2 new fatality tests (single-file path AND shard fold — the RED shard output reproduces the finding's silent partial success verbatim) while all 4 boundary tests stay green; SHA-verified restore. blit-core lib 460/0; workspace 1655/0.
