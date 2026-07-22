# Windows file attributes and alternate data streams are silently lost — and it depends on the file COUNT

**Status**: **RESOLVED.** Release rel-4 implemented contract v5; hosted Windows
run `29944148295` at `28cf989` passed the exact local and remote single-file and
tar-batched filesystem guards for attributes, named `$DATA` streams, metadata-
only repair, and stale-stream replacement.
**Found**: 2026-07-13, while benchmarking blit vs robocopy on a local
`D: -> E:` copy (`docs/bench/win-local-ab-2026-07-13/`). Surfaced by the codex
review of `4402987`, then reproduced directly.
**Severity**: correctness (silent, partial metadata loss in a backup/mirror
tool). **Not** a regression — this is unlanded Windows support, not something
that broke. It predates the unified session.

## Summary

`blit copy` **silently discards Windows file attributes (ReadOnly / Hidden /
System) and alternate data streams (ADS)** — but only sometimes. Whether a
given file keeps its metadata depends on **how many other small files happen
to be in the same transfer.** No warning, no error, exit code 0.

Copy a file alone → metadata survives. Copy it alongside 39 siblings →
metadata is gone.

## Resolution

Contract v5 adds bounded Windows attributes and named `$DATA` stream
descriptors/content to `FileHeader` and resume completion records. Windows
sources enumerate descriptors for the manifest, re-read and hash-check stream
content before payload send, and every local/remote file, tar, in-stream, TCP,
and resume sink validates then applies streams, mtime, and attributes in that
order. Destination metadata differences now request a repair even when primary
content would be skipped; explicit `ignore-existing` still skips. Non-Windows
destinations reject present Windows metadata before creating a file.

The platform-neutral validation, framing, need-claim, and mutation guards pass,
and every Windows target compiles under strict clippy. The tiny Windows-only
integration guard exercises single-file and 32-file tar local copies plus
remote copies, attributes, ADS contents, metadata-only repair, and stale-stream
replacement. Hosted Windows run `29944148295` at `28cf989` passed those exact
local and remote filesystem guards; later exact run `29951872658` at `6fb4d3f`
passed the complete Windows suite.

## Reproduced (netwatch-01, blit `f35702a`, local `D:` → `E:`)

**The controlled probe.** Identical 200 KiB files; the file **count** is the
only variable. A stream (`meta`) and the ReadOnly attribute are set on `f1.bin`
in both groups:

| transfer | path taken | ADS `meta` | ReadOnly |
|---|---|---|---|
| **40 files** | tar shard | **LOST** | **LOST** |
| **3 files** | `CopyFileExW` | preserved | preserved |

Source in both cases: `streams: :$DATA,meta`, `attrs: ReadOnly, Archive`.

Corroborating probe (a mixed tree, 4 tiny files) against robocopy:

| | source | blit | robocopy `/E` |
|---|---|---|---|
| ADS | `hidden_stream` | **LOST** | preserved |
| ReadOnly | set | **LOST** | preserved |
| Hidden | set | **LOST** | preserved |
| empty directory | present | absent | present |

(The empty directory is a **documented non-goal**, not this bug — see below.)

## Root cause

The planner forks the byte path on size and count
(`crates/blit-core/src/transfer_plan.rs:103-109`):

```rust
let use_tar = if options.force_tar { small_count >= 1 }
              else if small_count < 2 { false }
              else { small_count >= 32 || avg_small_size <= 128 * 1024 };
```

- **`use_tar == false`** → `PreparedPayload::File` → `copy_resolved_file_payload`
  (`remote/transfer/sink.rs:473`) → `windows_copyfile` → **`CopyFileExW`**
  (`copy/windows.rs:363`). That single Win32 call carries attributes and ADS
  for free. **Metadata survives.**
- **`use_tar == true`** → the file is tar-framed (`build_tar_shard`,
  `remote/transfer/payload.rs:254-293`) and written out by
  `safe_extract_tar_shard` → `fs::write` + `filetime::set_file_mtime`
  (`sink.rs:600-618`). That path carries **only mtime**. Nothing reads or
  writes Windows attributes; nothing reads or writes ADS —
  `build_tar_shard` never even opens the alternate streams. **Metadata is
  dropped.**

So the trigger is: **≥2 small files AND (≥32 of them OR average ≤128 KiB)** —
which is essentially *every real directory*: a source tree, a photo folder, a
documents folder. The 10 000-file benchmark fixture hits it; so does any
ordinary backup.

## What DOES survive

- **mtime** — stamped explicitly on the tar path (`sink.rs:606`).
- **Unix permissions** — set under `#[cfg(unix)]` (`sink.rs:610-618`). This
  gap is **Windows-only**; the Unix side of the same sink does carry its
  platform's mode bits.
- **File content and mtime-based comparison** are unaffected — this is not
  data corruption of file *contents*.

## Scope — BOTH routes, both MEASURED

- **LOCAL: proven** (the probes above).
- **REMOTE: proven** (2026-07-13). Windows → Windows over TCP via a loopback
  daemon on netwatch-01, same 40 × 200 KiB trigger:

  ```
  source  f1.bin  streams: :$DATA,meta   attrs: ReadOnly, Archive
  blit remote push exit: 0
  REMOTE  f1.bin  streams: :$DATA        attrs: Archive
  ```

  Same loss, **exit code 0**. This was an inference from code reading until it
  was measured; it is now established. The tar shard is the wire payload
  format and the same functions write the destination on both routes
  (`write_tar_shard_payload` → `safe_extract_tar_shard` → `fs::write` +
  `set_file_mtime`), so local and remote fail identically.

## NOT this bug

- **Empty directories** are absent from the destination, but that is a
  **documented design position**, not a defect: `blit check`'s own help text
  (`crates/blit-cli/src/cli.rs:20-35`) states the equivalence model skips
  "Symlinks, FIFOs, devices … Empty directories. Two trees differing only in
  those will be reported identical", and points the user at `diff -r` for full
  tree equivalence. blit models files, not directories. (Note
  `test_push_empty_directory` in `crates/blit-cli/tests/remote_transfer_edges.rs:177`
  only asserts the command *succeeds* — it never checks the directory arrived.
  It is a crash smoke test, not a fidelity test.)
- **ACLs** — robocopy does not copy them either without `/COPY:S`, so this is
  not a blit-vs-robocopy gap.

## Wire-contract disposition

`docs/TRANSFER_SESSION.md` was amended to contract v4 before implementation.
Same-build peers remain mandatory, so mixed v3/v4 sessions are refused at
open rather than silently dropping metadata.

## Interaction with the perf work

The historical benchmark compared a build that did less fidelity work than
the rel-4 implementation. Those timings remain root-cause evidence for the old
loss but are not a performance baseline for the corrected app. Optional local
small-file ceiling work remains post-release.
