# cr-ecm21-3: the fresh-private-partial rule, locally and through a link

**Severity**: HIGH — a local resume of a private source into a destination others can read patches it in place, for a reader who has it open; a wire resume through a link cut the file it names before it could set its mode
**Status**: Reviewed, found incomplete — `combined-r23` (Codex openreview, 2026-11-22) reviewed `5d2f8def..24d4064f` — the six `combined-r22` fixes — with `guard_confirmed: true`. "With a 256 KiB process file-size limit, Linux `copy --resume` exited 2 and replaced an 18-byte destination with an empty file. The rename succeeds before setting the unfinished length fails. Set the required mode and length on the staged file before renaming it, on both local and wire paths." "For `entry → middle → actual`, Linux `copy --resume` exited 0, replaced `middle` with a regular file, and left the old bytes in `actual`. Resolve the final referent within the destination boundary before placing the replacement." "A Linux acquisition probe showed that a mode-`0000` source reused the inode of a `0644` destination. If a privileged backup can read that source, an existing destination reader retains access to subsequent writes. Treat zero permission bits as a known mode. On the wire, use one metadata sample for reuse eligibility and the hash baseline so a permission change between the two reads cannot invalidate the decision." "A valid 255-byte basename made Linux `copy --resume` exit 2 with `File name too long` because the temporary name includes the entire basename plus a suffix. Generate a short sibling name independent of the destination basename and retry collisions." The remaining work is cr-ecm22-1, cr-ecm22-2, cr-ecm22-4 and cr-ecm22-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `abe9d4a1`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r22` over `b92c9a17..5d2f8def` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r22.codex.md`. It completes cr-ecm20-2 (`9ee5cb63`).

## Evidence
"A local resume from a `0600` source into a `0644` destination reuses the inode, so a reader who already has it open can read the new bytes. On the wire, a whole-file fallback through a destination symlink opens its referent with truncation before setting permissions. A subsequent permission failure can therefore leave the old contents gone. Apply the same source-aware acquisition rule locally and remotely. Where reuse is unsafe, establish a fresh private file before replacing the destination or its supported referent."

## Intake
Admitted as the reviewer put it.

## What
- `copy::fresh_partial(dst)`: a fresh private partial made BESIDE the file the name holds — or beside the file a link at the name names, which is replaced and the link kept — 600 from the moment it exists, and only then renamed over it. If it cannot be made, what stands there is as it was.
- `open_resume_partial(dst, source_mode)` (local): a destination that grants others what its source does not is not reused — a fresh partial takes its place, as one this run may not set the mode of does; `make_partial_private` no longer removes anything itself.
- The wire's hold, nothing hashed (`hold_whole`): anything at the name is replaced by a fresh partial so made; nothing there, one is made at the name, 600.

## Guard proof
- `resume::tests::a_resume_more_private_than_its_destination_takes_a_fresh_partial`: a 600 source into a 644 destination holding half of it — another file at the name, the source's bytes, a reader who had the old one open keeping only what it held; the same in a folder closed to new files — an error, the destination as it stood (bytes and mode).
- `sink::…::a_wire_partial_through_a_link_is_replaced_whole_or_not_at_all`: nothing hashed, through a link — the link stays and the file it names is a fresh 600 partial; the same with that file's folder closed to new files — the file as it was. Skipped, and says so, for a process a directory's mode does not bar.
- Mutations, each restored byte for byte: a resume more private than its destination reusing it (red); a wire partial through a link cut in place (red, and cr-ecm20-2's guard).
- The resume, sink and session tests (284) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs`, `crates/blit-core/src/copy/file_copy/mod.rs`, `crates/blit-core/src/copy/mod.rs` — `fresh_partial`, `open_resume_partial`, `make_partial_private`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — the local route's call, `hold_whole`; guard.

## Known gaps
- The fresh partial is made beside the file by a name of blit's own (`.<name>.blit-partial-<pid>-<n>`); one left by a process killed between making and renaming it is not cleaned up.
- A link whose target lies through further links is followed one step to name the file replaced.
- Unix only.
