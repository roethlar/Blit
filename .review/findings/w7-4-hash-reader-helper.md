# w7-4-hash-reader-helper — one streaming hash loop owns the 256 KiB buffer

**Branch**: `master` (owner-authorized session 2026-06-12, "Continue with 1")
**Commit**: `6b2f433`
**Source finding**: duplication-file-hash-read-loop — `docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md`

## What

The 256 KiB read-loop-into-hasher pattern existed five times: three arms
of `hash_file`, `partial_hash_first_last`'s small-file branch, and a
hand-rolled Blake3 copy in the daemon's `build_file_header` (256 KiB
stack array, used because `hash_file`'s path signature couldn't take an
already-open `File`). Consolidated into one
`checksum::hash_reader(&mut dyn Read, ChecksumType)`.

## Approach

- `hash_reader` owns the loop + the new `HASH_READ_BUF_BYTES` const (the
  single statement of the buffer decision).
- `hash_file` = open + `hash_reader`. Behavior identical (same per-kind
  hashers, same chunking; MD5 arm keeps `hash_file`'s existing
  no-gate behavior — the `allow_md5` gate remains on `strong_checksum`,
  unchanged).
- `partial_hash_first_last` small-file branch delegates (whole-file
  Blake3 — byte-identical output); early-return restructure removes the
  pre-declared hasher from that path.
- Daemon `build_file_header` passes its open `File`; the `BufReader`
  wrapper is dropped (pure pass-through at 256 KiB read sizes); the
  heap buffer replaces the 256 KiB stack frame; hash errors now keep
  the eyre chain (`{:#}`).

## Files changed

- `crates/blit-core/src/checksum.rs`
- `crates/blit-daemon/src/service/pull.rs` (`build_file_header` only)

## Tests added

1 equivalence test (suite 1368 → 1369): `hash_reader` == `hash_file` ==
`strong_checksum` for Blake3/XxHash3/Md5 on a >256 KiB payload — pins
that the streaming loop and the one-shot implementations can never
silently diverge, and that the loop iterates more than once.

## Known gaps

- `strong_checksum` (slice-based) still has its own per-kind one-shot
  implementations — intentionally: it takes `&[u8]` and the `allow_md5`
  policy gate, and the equivalence test now chains it to `hash_reader`.
- The blocking single-file checksum call on the daemon's async path is
  w4-4's slice (spawn_blocking), untouched here.
