# cr-pfc2-1: Root-wide write failures are incorrectly contained

**Severity**: HIGH — a mirror to a read-only destination volume can contain
every failed write and exit 0, silently leaving the backup incomplete.
**Status**: Open
**Branch**: — (default-branch mode; this repo lands fixes on `master`)
**Commit**: (filled in after commit)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `4d2b888f..575e47e7`
(`.review/results/pfc-2-range.codex.json`, attempt 2, 2026-07-31).

## Evidence
`crates/blit-core/src/remote/transfer/sink.rs:140-146` —
`destination_root_live` treats any existing directory as a live root;
`:157-165` — `per_file_failure` therefore converts volume-level errors such
as EROFS into a per-file outcome; `transfer_session/mod.rs:4633-4650` — the
pfc-2 interim interlock makes contained outcomes fatal only for non-mirror
sessions, so a mirror absorbs them all.

## Predicted observable failure
Mirror to an existing read-only mount (NAS share remounted ro,
write-protected volume): every write fails, every failure is contained,
no extraneous entries exist to delete → session returns success, CLI exits
0, destination unchanged. Silent incomplete backup.

## What
The per-file/fatal classifier keys only on whether the destination ROOT
path is a live directory, not on whether the failure's *kind* indicates the
whole volume is unwritable. Volume-level unwritability (read-only
filesystem / write-protected media) is a root-wide condition wearing a
per-file error's clothes.

## Approach
(planned) In the containment classifier (`per_file_failure` /
`destination_root_live`), walk the error chain for `std::io::Error` whose
kind or raw OS code indicates volume-level unwritability
(`ReadOnlyFilesystem` / unix `EROFS` 30 / windows `ERROR_WRITE_PROTECT`
19) and keep such errors session-fatal before any `SinkOutcome::failed`
is constructed. Ordinary per-file errors (permission on one file, path
occupied by a directory) contain as landed in pfc-2.

## Files changed
(filled in with the fix commit)

## Guard proof
(planned) Injected error chain carrying the read-only-volume kind through
the write path returns `Err` even in a mirror session (red when the
classifier revert makes it contain); an ordinary per-file error in the
same fixture still contains (boundary in both directions).

## Coder dispute (if any)
None.

## Known gaps
`StorageFull` (disk fills mid-mirror) is arguably also root-wide but is
transient and not named by the finding; left contained, noted for pfc-4/5
surfacing.

## Reviewer comments
(pending per-finding verification dispatch after the fix)
