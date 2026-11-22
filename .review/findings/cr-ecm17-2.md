# cr-ecm17-2: a failed write's cleanup through a link reported removed what it had left

**Severity**: HIGH — a failed write through a link leaves its partial in the file the link names and reports it removed, so nothing tells a later run it is an incomplete copy
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm17-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r18` over `7912da16..ca377805` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r18.codex.md`. It completes cr-ecm16-2 (`064534f4`), which carried cr-ecm15-1's follow-up.

## Evidence
"I aborted an ordinary overwrite through an unchanged, in-module symlink. The head reported the partial removed, yet reading the same name returned the four partial bytes. The handle identifies the referent and `symlink_metadata` identifies the link. Track that distinction. If cleanup cannot establish ownership, mark and report the held partial. The second unlink after dropping the handle also needs that protection."

cr-ecm16-2's cleanup compared the file its handle holds with what stands at the name, looked at as itself (`symlink_metadata`). An ordinary write opens a name that is a link THROUGH it, so it holds the file the link names; the comparison saw the link, took it for somebody else's file, left everything and reported the partial removed. And after a first removal that failed, the cleanup let the handle go and removed the name again without asking again.

## Intake
Admitted as the reviewer put it.

## What
`settle_failed_target`, for a write that still holds its file, tells three cases apart:
- the name IS the file held — removed, as ever;
- the name is a LINK to the file held — not removed (the link is not the run's to remove, and removing it would leave the partial anyway): the partial is marked through the handle, one byte longer than the source, and reported left in place (`INCOMPLETE_LEFT_IN_PLACE`), so a later run sees it;
- somebody else's file — left alone; nothing of the write's is at the name.
Before the second removal (after the first failed and the handle was let go), it asks again whose file stands there.

## Guard proof
- `sink::…::a_record_given_up_through_a_link_reports_its_partial_left`: an ordinary record through an in-module link, given up — reported left in place, not removed; the link stays; the file it names is one byte longer than the source.
- `sink::…::a_second_removal_is_asked_whose_file_stands_there`: the first removal fails (its folder may not be written to); at a seam before the second, the folder is opened and another file moved over the name — it is whole afterwards. (Not as root, which removes whatever a folder's mode.)
- `sink::…::a_record_given_up_removes_only_the_file_it_made` (strengthened): with somebody else's file at the name, nothing is said to be left there.
- Mutations, each restored byte for byte: a link to the file held taken for somebody else's (the link guard red); taken for the file (the given-up guard red); the second removal not asked (red).

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `settle_failed_target`, a seam; guards.

## Known gaps
- A write with no handle left (a record dropped with a write in flight) cannot tell a link from its file; it removes the name, as before.
- A link to a file the run made and that is OUTSIDE the module cannot arise: the destination's containment refuses a link that leaves it before anything is opened.
- Unix only; on Windows nothing says which file is which (cr-ecm16-2's open half).
