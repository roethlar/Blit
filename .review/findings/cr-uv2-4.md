# cr-uv2-4: `rm` asked to read directories it only passes through

**Severity**: MEDIUM — a path `rm` removes is refused with `Permission denied` and left
**Status**: Verified — `utility-slices-r3` (Codex openreview, 2026-11-21) reviewed `eb0cc4d8..dd0a3179`, five of the six `utility-slices-r2` fixes, with `guard_confirmed: true`: "I would keep the Unix search-only directory handles"
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `cade8d6c`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r2` over `4841b2d4..3cd49e40` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r2.codex.md`. It completes cr-uv1-2 on Unix.

## Evidence
"In a directory with mode `0300`, GNU `rm` removed a known file. Blit returned `Permission denied` and left its file behind. Use search-capable handles for ancestors and request read access when enumerating a directory."

Cause. cr-uv1-2 reaches an operand by opening every directory from the module's root down, each in the one before it. Each was opened to be READ — the one kind of handle the code had, a listing stream. Nothing on that way is listed: its directories are searched for the next name, and the last is asked what the operand is and to remove it. `rm` needs search permission on them and write on the last, and that is what the system checks; it never asks to read them.

Measured against the system `rm` (GNU coreutils 9.12), twelve trees — and a second difference of the same kind showed beside the reviewer's:

| tree | `rm` | blit before |
| --- | --- | --- |
| a file in a directory of mode 0300; a tree there; two such levels | removed | `Permission denied`, left |
| an EMPTY directory of mode 0300 or 0000, `-r` — as the operand, and inside a tree | removed | `Permission denied`, left |
| a directory of mode 0300 with something in it, `-r` | `Permission denied`, left | the same |
| a file in a directory of mode 0100, 0500 or 0600 | `Permission denied`, left | the same |

## Predicted observable failure
A drop directory others may write into and not list (mode 0300, or 1733 for other users): `blit rm HOST:/module/incoming/their-file` fails where `rm` on the server succeeds. `blit rm -r` of a tree holding an empty directory with its permissions taken away: the tree stays, with one line naming a directory `rm` would have removed.

## Intake
Admitted as the reviewer put it; the unreadable empty directory is the same finding — `rm` asked for less than blit did — and is fixed with it.

## What
- **A directory on the way to an operand is opened to be searched and no more** (`anchored::Passage`; `O_PATH` on Linux and Android, `O_SEARCH` on macOS and FreeBSD, and to be read, as before, where the system has neither). Entries in it are still opened, looked at and removed by name, in it and never through a link; nothing about the anchoring changes. What is done by name in an open directory is one trait (`ByName`) that both kinds of handle share.
- **A directory is opened to be read only where it is listed** (`anchored::Dir`): the operand's own tree.
- **A directory that cannot be read is removed if it is empty.** `rm` finds that out by removing it, and so does this: when a directory cannot be opened for its listing (`Permission denied`), it is removed by name as an empty directory; if that fails, the first error is the one reported — `Permission denied`, `rm`'s own word for it.

Speed, `rm -rf` of 200,000 files, release, medians of five: the system's 245.2 ms, blit before 250.7 ms, after 248.3 ms.

## Guard proof
- `rm.rs::rm_asks_of_a_directory_what_the_system_rm_asks` (Linux), through the binary and beside the system `rm`, the twelve trees above with `-r` and `-rf`: the same lines, the same exit status, the same entries left.
- `deletion::rm_tests::a_directory_is_asked_only_for_what_rm_needs_of_it` (Unix): a file one and two levels below mode 0300, and a tree there, removed; a file in a directory of mode 0100 kept, `Permission denied`; an empty directory of mode 0000 removed as an operand and inside a tree, with the counts; a full one of mode 0300 reported by its name and left.
- `deletion::rm_tests::a_held_directory_lists_and_removes_its_own_entries` and the three guards of cr-uv1-2 (a directory swapped for a link, mid-walk and on the way) pass unchanged on the new handles.
- Mutations, each restored byte for byte: a directory on the way opened to be read, as before → both guards red; an operand that cannot be read not tried as an empty directory → both; a directory in the tree likewise → both.

## Files changed
- `crates/blit-core/src/deletion.rs` — `anchored::{TO_SEARCH, ByName, Passage}`, `Dir` for listing only; `remove_operand`; guard.
- `crates/blit-cli/tests/rm.rs` — guard.
- `CHANGELOG.md`.

## Known gaps
- **macOS and FreeBSD are unproven.** `O_SEARCH` is what those systems name for this; the constant compiles for `aarch64-apple-darwin` (checked alone — the crate itself cannot be cross-checked here, a C dependency needs that system's compiler), and nothing has run there. Android: from the source.
- On any other Unix a directory on the way is still opened to be read.
- Windows is not this code (cr-uv2-2).
- A directory that can be read and not searched (mode 0600) is `Permission denied` for anything in it, as for `rm`.
- Run on Linux only.
