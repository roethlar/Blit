# cr-uv2-1: A remote move reported success and left its source

**Severity**: HIGH — `move` exits 0 with the source in place: the user believes the data moved, and a copy was made of something that was asked for as another thing
**Status**: Reviewed, found incomplete — `utility-slices-r3` (Codex openreview, 2026-11-21) reviewed `eb0cc4d8..dd0a3179`, five of the six `utility-slices-r2` fixes, with `guard_confirmed: true`. Carrying the directory requirement to the daemon stands. Four things do not: the removal that ends a move asks for the whole path afresh, so what the transfer did NOT carry is removed with what it did — `move incoming/.`, holding a file and a link to it, now exits 0 with the tree gone and only the file at the destination (before this fix the removal failed and the link survived); the request between a CLI and a daemon for a transfer between two daemons kept its own version, so a new CLI with old daemons, and the reverse, still copy `one.txt/` as a file; a module that is itself one file, typed `HOST:/single/.`, is copied; and the daemon takes "a complete scan is required" for "the source will be removed". The remaining work is cr-uv3-1, -2, -5 and -6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `7e0cff32`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r2` over `4841b2d4..3cd49e40` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r2.codex.md`. It completes cr-uv1-1.

## Evidence
"I moved a regular file spelled `single.txt/`. Blit copied its contents, exited 0, and left both files in place. The transfer drops the trailing slash, while cleanup preserves it and suppresses the resulting directory error under `force`. Validate the directory requirement before copying and use the same validated source for both remote cleanup routes."

Cause. A move from a daemon is a transfer, then a removal, and each read the source off the endpoint for itself. The transfer tidied the path on its way to the wire (`single.txt/` → `single.txt`) and the daemon copied the file. The removal, since cr-uv1-1, asks for the path as typed: `rm -rf single.txt/`, a directory that is not there, which under `-f` is no failure — as for `rm`.

Measured before the fix (loopback daemon; `mv`, `cp -r` and `rsync -a` beside it):

| source typed | `mv` / `cp -r` / `rsync` | blit, from a daemon |
| --- | --- | --- |
| `single.txt/`, `single.txt/.` | `Not a directory`, nothing copied | file copied; move exits 0, nothing removed |
| `link-to-file/` | `Not a directory` | `Pull complete: 0 file(s)`, exit 0 |
| `link-to-dir/` (move) | `mv`: `Not a directory`, nothing moved | everything copied, nothing removed, exit 0 |
| `dir/.` (move) | — | everything copied; `cannot remove 'dir/.': refusing to remove '.' or '..' directory`, exit 1 |

The third row is the same defect by another road: `link/` on a link to a directory IS that directory to the transfer, and to the removal it is a link that is never gone through (cr-uv1-1).

## Predicted observable failure
`blit move HOST:/module/report.pdf/ /archive/report.pdf` in a script: exit 0, the file now in both places. `blit move HOST:/module/current/ /archive/` where `current` is a link to a release directory: exit 0, the release copied and still on the server.

## Intake
Admitted as the reviewer put it, with the link case added: it is the finding's own sentence — success reported, source in place — and would have been the next round's.

## What
- **The source's path goes to the wire as it was typed** (`path_posix::source_path_to_posix`, `session_client::source_module_path`): one `/` after it when it was typed as a directory — a separator after it, or `.` as its last name. For a transfer between two daemons the delegated request carries the same. **Wire contract 19**: no message changed; a daemon of contract 18 would drop the mark and copy the file.
- **The daemon refuses a source that is not what it was typed as, before anything is copied** (`refuse_source_not_as_typed`): `source 'single.txt/': Not a directory`, for `copy`, `mirror` and `move`. A source that is not there is still the scan's to report.
- **A move's source typed as a directory must be a directory itself.** A link to one is refused before anything is copied — `mv` refuses the same operand — saying to name the directory, or the link without the `/`. A `copy` removes nothing and goes through the link as before. The daemon knows a move by the one thing a move asks of a source: a complete scan (`require_complete_scan`).
- **The removal asks for the path the session was opened with** (`admin::rm::delete_remote_path(remote)`): one function says what that is, and both routes — to this machine, and between two daemons — call it with the endpoint alone. `move HOST:/m/dir/. out/` now moves the directory's contents and removes it.

After, from a daemon: `single.txt/`, `single.txt/.`, `link-to-file/` — exit 1, `source '…/': Not a directory`, nothing copied; `move link-to-dir/` — exit 1 before copying; `move dir/`, `move dir/.` — exit 0, contents landed, `dir` gone.

## Guard proof
- `remote_move.rs::a_file_typed_as_a_directory_is_neither_copied_nor_moved`: `single.txt/`, `single.txt/.`, `single.txt//` × `copy` to this machine, `move` to this machine, `move` to the daemon — refused in those words, nothing landed, the source whole; and `single.txt` is copied.
- `remote_move.rs::a_move_removes_the_path_its_session_read`: `dir/`, `dir/.`, `dir//`, `dir` — exit 0, the tree landed, the source gone.
- `remote_move.rs::a_move_through_a_link_typed_as_a_directory_is_refused_before_copying` (Unix): to this machine and to the daemon — refused, nothing copied, link and directory whole; a `copy` of the same goes through; the directory itself moves.
- Unit: `path_posix::tests::a_source_keeps_the_mark_that_it_was_typed_as_a_directory`, `session_client::…::a_source_endpoint_says_it_was_typed_as_a_directory`, `operation_spec::…::a_source_typed_as_a_directory_is_delegated_as_one`.
- Mutations, each restored byte for byte: the session that reads opened with the tidied path → the file and link guards red; the daemon not refusing a non-directory → the file guard; not refusing a move through a link → the link guard; a delegated source tidied → file, link and the delegated unit guard; the removal reading the typed path off the endpoint again → the same-path guard; a final `.` no mark → the file guard and all three unit guards.

## Files changed
- `crates/blit-core/src/path_posix.rs` — `typed_as_directory`, `source_path_to_posix`; guard.
- `crates/blit-core/src/remote/transfer/session_client.rs` — `source_module_path`; the pull session opens with it; guard.
- `crates/blit-core/src/remote/transfer/operation_spec.rs` — the delegated `source_path`; guard.
- `crates/blit-core/src/admin/rm.rs` — `delete_remote_path(remote)`.
- `crates/blit-cli/src/transfers/mod.rs` — both routes call it with the endpoint.
- `crates/blit-daemon/src/service/transfer.rs` — `refuse_source_not_as_typed`.
- `crates/blit-core/src/transfer_session/mod.rs` — `CONTRACT_VERSION = 19`.
- `crates/blit-cli/tests/remote_move.rs` — three guards.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- **A move on THIS machine is not changed**, and has the same two operands wrong in its own way (measured, not part of this finding): `blit move link-to-dir/ out/` and `blit move dir/. out/` copy everything, empty the directory, then fail removing the name typed (`Not a directory`, `Invalid argument`) and exit 1. Put to the owner in `docs/STATE.md`.
- **A link moved without a `/`** — `move link-to-dir out/` — copies what the link points to and removes the link, from a daemon and on this machine alike; `mv` moves the link. A link to a FILE is copied as nothing and then removed. Both are how transfers treat a link today, older than this plan; put to the owner in `docs/STATE.md`.
- If the owner rules that `rm -r link/` should empty the directory the link points to, as GNU `rm` does (the open question from cr-uv1-1), a move through such a link can be allowed in the same change.
- The refusal reads `session PROTOCOL_VIOLATION: source 'single.txt/': Not a directory`: the session's own prefix for a request it cannot serve.
- A client on Windows typing `sub\dir` had the removal ask a Unix daemon for the one name `sub\dir` while the transfer read `sub/dir`; it now asks for the same path. From the source; nothing ran on Windows.
- Run on Linux only.
