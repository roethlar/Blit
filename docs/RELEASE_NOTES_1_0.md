# Blit 1.0.0 Release Notes

Covers everything shipped since 0.1.1 (tagged 2026-07-23).

## Headline

1.0 is a reliability and performance release. A single bad file can no
longer take down an entire transfer, a real-world converged mirror check
that used to take 283.92 seconds now takes 55.57 seconds (about 5.1x
faster), and transfers show live, informative progress instead of a
scrolling spinner.

## Reliability: one bad file no longer aborts the whole transfer

Previously, a single file blit couldn't write — locked, permission denied,
an attribute it couldn't apply — killed the entire transfer, including
every file that had already been correctly copied or was still queued
behind it.

- **Per-file failure containment.** A file that fails to write is recorded
  and skipped; the rest of the transfer proceeds normally.
- **Named failures at the end of the run.** The summary lists every failed
  file and the reason, with a hint to re-run. The process exits with code
  `2` for "completed with failures" — distinct from a clean success (`0`)
  or a hard failure — so scripts can tell the three apart.
- **Move never deletes a source file whose copy failed.** If a file
  couldn't be written to the destination, its source copy is left in
  place; re-running the transfer will pick it up and converge safely.
- **Attribute-only mismatches are repaired in place, without re-sending
  file content.** If a destination file already matches the source by
  content but differs only in attributes (e.g. Windows flags an SMB
  server rewrote), blit now patches just the attributes instead of
  re-copying the whole file. This also fixes a real case: mirroring to a
  default-configuration Samba share used to fail outright, because Samba
  synthesizes a HIDDEN attribute on dot-files that could never converge.
- **A related Linux/macOS-only bug is fixed:** if a file's parent path
  component was itself an existing file (not a directory), that could
  abort an entire batch of files instead of failing only the one affected
  entry. Proven fixed on real Linux hardware, not just cross-compiled.

## Performance

- **SMB mirror, converged (no-op) check on a real ~46,000-file / ~8 GB
  workload: 283.92s -> 55.57s, roughly 5.1x faster.** This came from a
  chain of changes to how blit compares source and destination rather
  than from any new configuration: reusing file metadata already read
  during the directory scan instead of re-reading it per file, an
  automatically self-tuning concurrency level for the comparison stage
  (no configuration needed or exposed), scanning each destination
  directory once instead of once per file, and dropping a per-file check
  described below.
- **Local-to-local mirror on the same workload: roughly 2x faster**
  (17.68s -> 8.70s), from running more write workers in parallel during
  the apply stage.

All of the above is automatic — there is no configuration required or
available to get these numbers; blit tunes itself at runtime.

## Behavior change: default comparison no longer checks for extra data streams

This is the one user-visible behavior change in 1.0 and is worth reading
even if you skip the rest of these notes.

Every run — including a no-op run that verifies an already-mirrored tree
— used to ask the destination whether each file that already matched by
size and modification time also had matching Windows named data streams.
That check was expensive, and on the workload above it accounted for most
of the 283.92-second cost. It also never protected the data: whenever it
did find a stray or extra stream, the fix was always the same — replace
the entire file — so the check only ever bought earlier detection of
something that was going to be fully overwritten anyway.

As of 1.0, the default comparison (size + modification time) **no longer
performs that per-file stream check.** A stray or changed stream on an
otherwise size/mtime-matched destination file can now go undetected until
the next run made with `--checksum`.

- Streams are still transferred correctly whenever a file's content
  actually changes — this only affects detecting an out-of-band stream
  change on a file blit otherwise considers unchanged.
- Attributes are unaffected by this change and are still checked and
  repaired in place (see Reliability above).
- Run with `--checksum` to get the previous, exhaustive verification
  behavior (full checksum and stream comparison on every file, every
  run).

## CLI and terminal output

- **Live progress row.** Running with `-p` now shows one stable,
  in-place status row — current phase, files and bytes so far, current
  file — instead of a spinner that could be scrolled off-screen by other
  output. `-v` per-file lines scroll cleanly above it.
- **Colour.** The live row and any failure output are colour-coded
  (phase and outcome colours). Colour is automatically disabled when
  `NO_COLOR` is set, `TERM=dumb`, when output isn't going to an
  interactive terminal, or for `--json` output; piped or redirected
  output is byte-identical to before. There is no `--color` flag or
  theming to configure — this is not a user-facing option.
- **End-of-run failure block.** Runs with any per-file failures print a
  capped list of failed files and reasons at the end, with a hint to
  re-run to converge, and exit with code `2` (see Reliability above).

## Platforms and compatibility

- Release archives contain `blit` and `blit-daemon` for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`, each with a SHA-256 sidecar and the exact
  source commit in `BUILD.txt`.
- Both executables report the same `1.0.0+<commit>` build identity.
  Remote sessions require identical build identities on both ends and
  refuse mixed-version peers.
- The daemon has no built-in TLS or user authentication (unchanged):
  operate it only on a trusted network or through an SSH tunnel or VPN.

## Known limitations shipped open

Two pre-existing issues were triaged during this release and are shipping
documented rather than fixed:

- **Non-UTF-8 filenames fail to transfer.** A source filename that is
  valid on Linux/ext4 but not valid UTF-8 gets corrupted during blit's
  internal path handling and fails to transfer. On LOCAL transfers that
  failure is contained to the one file (the rest of the transfer
  proceeds and exits with code `2`, per Reliability above). On REMOTE
  transfers it is NOT contained: the corrupted path can make payload
  preparation fail at the source and abort the whole session. There is
  currently no workaround beyond renaming the affected files.
- **`--exclude` only matches paths relative to the transfer source, and
  directory patterns don't match their contents.** An absolute path, or
  a bare directory name intended to exclude everything under it, will
  silently match nothing — the pattern is accepted but excludes nothing,
  and blit does not warn that the pattern couldn't have matched. Use a
  source-relative glob with an explicit subtree wildcard instead, e.g.
  `--exclude '.java/**'` rather than `--exclude .java` or an absolute
  path.
