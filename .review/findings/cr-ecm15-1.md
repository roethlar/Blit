# cr-ecm15-1: `--ignore-existing` was not held through the write

**Severity**: HIGH — a file put at a destination after the run looked is truncated and written over, by a transfer over the wire always and by a copy on this machine in a window
**Status**: Fixed, awaiting review (what runs only on Windows is not run: below)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm15-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. It completes cr-ecm14-2 (`e672e1c2`).

## Evidence
"The wire destination hard-codes the option to `false`. A file created after planning can therefore be truncated by a remote transfer. The local copy path acquires an exclusive handle, drops it, and reopens the pathname. Pass the negotiated option to the wire sink, retain ownership through writing and cleanup, and defer Windows attribute changes until the destination is acquired."

Three things, all in what cr-ecm14-2 built.
1. **The wire.** cr-ecm14-2 made every creation under `--ignore-existing` one that fails if the name is taken — in the sink. The sink a session's destination end writes through was built with `ignore_existing: false` whatever the run asked, under a comment that said the option was read by the local carrier alone; cr-ecm14-2 made that untrue and left the line.
2. **The copy on this machine.** It made the name by such a creation, DROPPED what it held, and then copied by the name (`copy_opened` opens the destination to truncate it). A file moved over the name in between was truncated and written over; and a copy that failed removed whatever stood at the name.
3. **Windows attributes.** `prepare_destination` clears the read-only bit of what stands at the destination before the write, and ran before the creation that might refuse the name: under `--ignore-existing` it changed a file that was then kept.

## Intake
Admitted as the reviewer put it.

## What
- `destination_session_inner` gives the wire's sink the negotiated option.
- `copy_and_stamp_opened`, under the option: the handle the creation returns is KEPT. `copy::copy_opened_into(src, made, len, sizer)` writes the bytes through it — on Linux the same cascade as `copy_opened` on the descriptors (`copy_file_range`, `sendfile`, the sparse copy, the buffered one); on every other system a buffered copy through the handle — and the time and mode it gives go through the handle too (`preserve_metadata_into`), as does the sink's own time stamp. `--resume` changes nothing there: a file just made holds nothing to resume. The name is never opened again.
- `PartialTarget::arm_made` notes WHICH FILE was made (Unix: device and number). Settling a failed write removes the destination only while that file stands at the name; and a write whose file no longer stands at its name when it is done has not landed — `THE_NAME_IS_TAKEN`, the file that stands there kept.
- `windows_metadata::prepare_new_destination`, used wherever a destination must be new (`prepare_to_write`: the stream, both shard paths, the copy): on Windows nothing is touched; elsewhere it is `prepare_destination` as before, which changes nothing and refuses what the platform cannot hold before anything is made.

## Guard proof
- `blit-daemon` `transfer_session_e2e::a_push_that_keeps_what_exists_writes_over_nothing_put_there_since`: a real push to an in-process daemon under the option, through a source that puts somebody's file at the destination after the destination has asked for the files and before a byte is sent — a small file (a shard member) and one of 4 MiB (a stream). Both are whole afterwards, the file nobody took is written, and two files are reported failed.
- `blit-core` `sink::…::a_name_made_by_a_run_that_keeps_what_exists_is_written_through_what_made_it`, with and without `--resume`: between the making of the name and the write (a test seam, `AFTER_CLAIM_HOOKS`) another file is moved over the name. It is whole afterwards — not written to, not removed — and the write reports the name taken; a name nobody takes is written, its bytes and its time.
- Mutations, each restored byte for byte: the wire's sink told the option is off (the push guard red); the name opened again by name to be written (the sink guard red); a failed write removes whatever stands at the name (red); what was written counts as delivered whatever stands at the name (red).
- The whole suite in the scratch copy before porting: 1,930 passed, 0 failed.
- Speed, `copy --ignore-existing` on this machine, median of five (Btrfs, twice): 20,000 small files 238.3 → 250.0 ms and 299.4 → 310.6 ms — about four in a hundred slower, for one look at what was made and one at the name when it is done; one file of 1 GiB 27.4 → 27.2 ms. A copy without the option is not touched.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — the wire's sink.
- `crates/blit-core/src/remote/transfer/sink.rs` — `copy_and_stamp_opened`, `PartialTarget`, `prepare_to_write`, `file_identity`, `identity_at`, the seam; guard.
- `crates/blit-core/src/copy/file_copy/mod.rs`, `metadata.rs`, `crates/blit-core/src/copy/mod.rs` — `copy_opened_into`, `preserve_metadata_into`.
- `crates/blit-core/src/windows_metadata.rs` — `prepare_new_destination`.
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — guard.
- `CHANGELOG.md`.

## Known gaps
- **Not run on Windows or macOS.** There the copy under the option is the buffered one through the handle: Windows' block clone and its sparse copy, and macOS' clone (which needs the destination absent), are not used under `--ignore-existing`. On Windows the system does not say which file is which in what the standard library gives, so a failed write there still removes what stands at the name, and a file moved over the name is not noticed when the write is done; and named streams and attributes are still set by name. `prepare_new_destination` does nothing on Windows by construction, and no guard here can see it. All of it compiles for Windows (cross-clippy) and none of it has been executed there.
- A destination written over the wire is made and written through one handle already (the stream) or made and written in one step (a shard member); neither notes which file it made, so a file moved over the name during a wire write is not noticed at the end, and a failed write removes what stands at the name. The window is one file's write.
- The link a transfer makes, and directories, are not made by this path.
- Seen while measuring, and NOT this finding's: a copy on this machine gives its destination mode 644 for a source of mode 640 (Linux, tmpfs and Btrfs, with and without the option, before and after this commit) — `copy_opened` skips the mode when its fast path succeeds. Recorded for the owner in `docs/STATE.md`; not changed here.
