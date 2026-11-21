# cr-uv4-2: `find -delete` went on removing after its asker had gone

**Severity**: HIGH — a `find -delete` stopped with Ctrl-C keeps deleting across the tree on the daemon
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv4-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. The defect is in utilities slice 4 (`35adeacb`).

## Evidence
"With the response receiver already closed, the daemon removed my fixture and returned `Ok(())`. A successful deletion sends no reply, so the worker can continue across the tree after Ctrl-C. Check cancellation during the walk and before each removal."

Cause. The daemon's walks learn that nobody is listening from a send that fails. A listing sends what it finds, so it learns soon. `-delete` sends nothing for an entry that went — only for one that would not — and a walk whose tests hold for nothing sends nothing at all: neither had a send to fail, and both ran to the end of the tree. For `-delete` that means removing for an asker who has stopped the command.

## Intake
Admitted as the reviewer put it.

## What
- **Before each removal** the daemon asks whether the reply's receiver is still there (`Sender::is_closed`), and ends the walk if it is not.
- **During the walk**: `blit_core::find` gives its caller a word that carries nothing — `FindEvent::Walking` — before each directory it reads and after every 1,024 entries of one (`ENTRIES_BETWEEN_WORDS`). The daemon answers it by ending the walk when the receiver has gone. So a walk that finds nothing stops within one directory's 1,024 entries of the asker leaving, and so does a listing between two batches.
- The request then ends as every stopped stream does (`Cancelled`, "response channel closed"), sent to nobody.

## Guard proof
- `service::admin::…::find_stops_when_its_asker_has_gone` (the daemon): with the receiver closed before the request starts, `-delete` over a tree — everything found, and by name — removes nothing and ends `Cancelled`; so does `-delete` of a start that is a FILE, which is found before any directory is read (the removal's own check, with no word from the walk before it); a listing that finds nothing ends `Cancelled` where it used to walk to the end and return `Ok`; and with the receiver there the same `-delete` removes what it found and says nothing of it.
- `find::tests::the_walk_can_be_stopped_though_it_finds_nothing` (the walk): over four directories, one of 2,053 entries, with a test nothing passes, the caller has six words — one per directory, two more inside the long one — and nothing else; stopped at the first, the walk has read nothing and given the start alone, or nothing at all when contents come first.
- Mutations, each restored byte for byte: the check before a removal gone; the daemon's answer to the walk's word gone; the word before a directory below the start, the word inside a long directory, the word before the start — five, each red on the guard named for it.

## Files changed
- `crates/blit-core/src/find.rs` — `FindEvent::Walking`, `ENTRIES_BETWEEN_WORDS`; guard.
- `crates/blit-daemon/src/service/admin.rs` — `stream_find` asks before each removal and at each word; guard.

## Known gaps
- What was removed before the asker left is removed: stopping a `-delete` stops it, and undoes nothing.
- The check is made between entries. One removal already begun is finished, and up to 1,024 entries of one directory may be read (not removed) after the asker has gone.
- The daemon learns that the asker has gone when the connection's end reaches it. A machine that vanishes without closing its connection is noticed only when the transport's own timeouts say so; nothing here shortens that.
- `du` and `ls` are not changed here: `du` sends a row for every directory and `ls` reads one directory, so both learn from a send that fails; `rm` has its own check (`called_off`, cr-uv2-3).
- Not driven end to end with a real Ctrl-C: when the removal stops relative to the signal is a race no test here could hold still.
