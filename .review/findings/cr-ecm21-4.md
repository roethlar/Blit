# cr-ecm21-4: delivery and failure cleanup lost the writer's handle

**Severity**: HIGH — a failed wire resume marks a file renamed over its name one byte longer; a local resume whose identity cannot be read is acknowledged, and a move releases its source
**Status**: Fixed on Unix, awaiting review; the Windows half is BLOCKED (no Windows host reachable — `michael@10.1.10.222` and `michael@10.1.10.173`: "No route to host", 2026-11-22)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm21-4`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r22` over `b92c9a17..5d2f8def` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r22.codex.md`. It completes cr-ecm20-1 (`5b3bf24e`) and cr-ecm20-4 (`ae10d707`).

## Evidence
"The wire hold returns only device and inode. Failed-block cleanup checks the pathname and then reopens it to mark the partial, so a rename between those operations can extend somebody else's file. The local success path also turns a failed identity lookup into `None`, which permits acknowledgment. Retain the active descriptor for patching and cleanup, and require a successful destination identity check before a move releases its source. Windows also needs a handle-based file identity and directory walk. Its current paths can accept a replacement or descend through a changed pathname."

## Intake
Admitted as the reviewer put it. The Windows half — a handle-based identity and walk there — cannot be written and proven on this Linux host; it stays open with cr-uv6-3 and the Windows halves of cr-uv10-1 and cr-ecm20-4.

## What
- A wire resume's hold keeps the partial's handle itself (`HeldPartial`), not only which file it is: every block is written through it at its offset (`patch_held_block`), never opening the name; the completion cuts and stamps through it, and reports the file complete only while the name holds it (cr-ecm20-1's look); a failure's cleanup marks it through it — never whatever stands at the name. Where no hold holds the file (elsewhere than Unix), by the name, as before.
- A local resume whose identity cannot be read is not delivered: the look is an error, not a `None` that let any file at the name pass.

## Guard proof
- `sink::…::a_failed_wire_resume_marks_only_the_partial_it_holds`: a hold, a block through it, a file of the source's size renamed over the name, and a failed block — the completion fails and the other file is not marked.
- `sink::…::a_wire_resume_works_on_the_file_it_hashed_and_no_other` (cr-ecm20-1's) passes through the handle.
- Mutations, each restored byte for byte: a failed resume marking what stands at the name, as before (red); a block written by the name (red, cr-ecm20-1's guard).
- The sink, data-plane, session and resume tests (293) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `HeldPartial`, the holds, `patch_held_block`, `patch_file_block`, `finalize_resumed_file`, the block and completion payloads, the local route's tail; guard.

## Known gaps
- Windows (BLOCKED): blocks, the completion and the cleanup go by the name there, and delivery is not asked.
- A held handle is kept from the hold to the completion — one open file per resume record in flight.
- A local resume's identity look failing has no guard: nothing a test can do makes `fstat` of an open file fail.
