# cr-ecm29-2: the occupant to replace was looked up twice, and an unchecked rename stood where there is no exchange

**Severity**: HIGH — a regular file that came to the name between the caller's decision to replace and `fresh_as`'s own second look became the expected occupant, and was replaced; and where there is no exchange (off Linux, or a Linux filesystem without one) the fallback `renameat` replaced whatever had come to stand there during staging
**Status**: Fixed on Linux, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm29-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It completes cr-ecm28-2 (`3c25cd73`).

## Evidence
"A regular file arriving between the decision to replace and the second lookup becomes the new expected occupant. On macOS and Linux filesystems without exchange support, the fallback `renameat` can also overwrite an entry that arrived during staging. Pin the original identity and fail the individual file wherever safe replacement is unavailable."

## Intake
Admitted as the reviewer put it. Where the decision to replace is made after the destination is opened (it is not a regular file once opened, has a second name, or may not be reused), the identity pinned is that of the handle opened and judged. That is the first look at what is being replaced. Pinning the earlier path look there would fail a FIFO swapped in for a reusable file after the look (cr-ecm27-3's guard), though what is replaced is exactly what was opened.

## What
- `Resolved::look` returns an `Entry` (mode, owner, device and inode), through `entry_at`. `Resolved::occupant`, the second look, is gone.
- `Resolved::fresh` takes `seen`, the `Entry` the caller judged. `fresh_as` carries it to publication unchanged, and no longer looks again. A link or a directory there is left as it is, and the file fails, as before.
- Callers:
  - `open_resume_partial_with` passes its look, or, once the destination is opened, `Entry::of` the opened handle.
  - The wire's `hold_whole` passes its look.
  - `fresh_partial_ready` looks once itself and passes that.
- `Resolved::publish_replacing` (Linux) swaps the partial in with `renameat2(RENAME_EXCHANGE)` and checks what came out against the pinned identity, as before. A change: when the name has gone (`ENOENT`), it reports `Replacing::Vacant`, and the partial takes the name only while nothing has it, through `publish_exclusive`.
- Where there is no exchange (`EINVAL`/`ENOSYS`/`EOPNOTSUPP`), and off Linux, nothing is replaced. The partial and its stage go, the destination is left as it is, and the file fails with "this filesystem cannot swap a fresh partial in for it". The rename over it is gone.
- `CHANGELOG.md`: the `--resume` entry says so.

## Guard proof
- `resume::tests::a_file_come_since_the_first_look_is_not_replaced` (Linux): after the caller's look, before the partial is made, a new regular file takes the name. The resume fails, and the newcomer is left.
- `sink::tests::the_wires_fresh_partial_leaves_a_file_come_since_its_look` (Linux): the same over the wire's whole-file hold. The newcomer is left and the file fails at completion.
- `resume::tests::where_there_is_no_exchange_nothing_is_replaced` (Linux): the exchange refused (`EXCHANGE_FAULTS`, `EINVAL`). The resume fails, saying so, the destination is as it was, and no stage is left.
- `resume::tests::a_destination_gone_while_its_partial_is_staged_takes_the_partial` (Linux): the destination is removed while the partial is staged. The partial takes the name, and the resume completes.
- Mutations, each restored byte for byte:
  - what is replaced looked at again before publication → red (both newcomer guards);
  - where there is no exchange, renamed over → red;
  - nothing there any more failing the file → red.
- The resume, sink and session tests (345), cr-ecm27-3's FIFO guard among them, and the four local-copy-mode tests pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `look`, `fresh`, `fresh_as`, `publish_replacing`, `Replacing`, `Entry::of`, `EXCHANGE_FAULTS`, `open_resume_partial_with`, `fresh_partial_ready`; guards.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_whole`; guard.
- `CHANGELOG.md` — the `--resume` entry.

## Known gaps
- A behaviour change the owner may want to weigh. Where the filesystem has no exchange (Linux's NFS and SMB clients among them) and on macOS, a `--resume` over a destination it may not reuse now fails that file and leaves the destination as it is. Such a destination is one too wide for its source, another user's, unreadable, or with a second name. The fallback used to rename over it, which could replace something that came there meanwhile.
- macOS has its own exchange (`renameatx_np` with `RENAME_SWAP`). Using it is macOS-only code that nothing here can check or run, so it is blocked with cr-ecm27-6 (no Mac).
- An entry that came to the name between the caller's look and the open is judged on what was opened. Patching a regular file this run may reuse in place is unchanged.
