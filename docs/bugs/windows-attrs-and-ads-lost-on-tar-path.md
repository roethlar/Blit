# Windows file attributes and alternate data streams are silently lost — and it depends on the file COUNT

**Status**: Confirmed, reproduced, **queued behind otp-12** (owner, 2026-07-13:
*"we started this as a linux alternative to robocopy, and full windows support
was always a goal… but obviously not landed. so, good, let's address that.
after this current phase is complete."*)
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

## Fixing it will touch the WIRE CONTRACT — flag before starting

The tar shard is not a local implementation detail; **it is the wire payload
format for small files**. Carrying attributes/ADS through it means extending
the shard header, which is a **frame change** →
`docs/TRANSFER_SESSION.md` must be amended through the codex loop **before**
any code (the same stop-and-amend rule `OTP12_PERF_FINDINGS.md` operates
under). Same-build-both-ends (D-2026-07-05-2) means no compatibility surface
is created, but the contract doc still governs.

Candidate directions (NOT a plan — a plan gets written when this is picked up):
1. Extend the tar shard header with a platform-metadata block (attributes +
   named streams). Wire change; preserves the batching win.
2. Route small files on the **local** carrier through `CopyFileExW` per file
   (no wire change, local only) — but that leaves the remote path broken and
   makes local/remote fidelity differ, which is worse than the status quo.
3. Post-pass: after extraction, re-apply attributes/ADS from the manifest.
   Requires the manifest to carry them — still a wire change, smaller blast
   radius than reshaping the shard body.

Direction 1 vs 3 is a real design decision and belongs to the owner.

## Interaction with the perf work

This makes the local benchmark's comparison **more** unfavourable to blit, not
less: blit is not doing *more* work than robocopy and paying for it — it is
doing **less** (no attributes, no ADS, no empty dirs) and **still** losing
1.9× at equal thread counts (`docs/bench/win-local-ab-2026-07-13/`). Any fix
here adds per-file work to the tar path, so it will make the small-file
numbers worse before they get better. Sequence accordingly:
`LOCAL_SMALL_FILE_PATH.md` (D-2026-07-13-2) and this finding should be planned
together when otp-12 clears.
