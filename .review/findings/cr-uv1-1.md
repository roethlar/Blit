# cr-uv1-1: A utility verb's operand was tidied on the way, and another path was acted on

**Severity**: HIGH — `rm -rf HOST:/m/work/.` deleted `work`; `rm HOST:/m/doc/` unlinked a regular file; of two names differing by a space at the end, the wrong one was deleted
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv1-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`.

## Evidence
On Linux, `blit rm -rf HOST:/m/work/.` deleted `work` and its contents. `blit rm -y HOST:/m/doc/` unlinked a regular file. GNU `rm` refused both. With both `report` and `report ` present, asking Blit to remove the trailing-space name deleted `report` and returned 0. The reviewer: preserve whitespace, the final component, and trailing separators through validation and the RPC. The same correction matters for reads: `du -s` on a directory link ending in `/` reported 4 KiB where GNU reported 64 KiB.

Cause, three places that each made a tidier path of the one typed:
- `RemoteEndpoint::parse` trimmed the whole text, so a space at the end of a path was gone before any verb saw it.
- The verbs sent the path's COMPONENTS joined again (`rel_path.iter()`, `rel_path_to_string`), which drops a separator after the last name and every `.`.
- The daemon trimmed the path once more and normalized it again.

What the commands do with those forms was measured on this machine (GNU coreutils 9.12, findutils 4.11.0): `rm` refuses a last name of `.` or `..` whatever the options and goes on; `rm doc/` of a file is `Not a directory`, which `-f` passes over leaving the file; `du`, `df` and `find` resolve a link typed with `/` or `/.` after it and say `Not a directory` of anything else typed so.

## Predicted observable failure
As observed, and for any path ending in `/.` (shell completion and scripts produce them): a recursive forced removal of the directory itself.

## Intake
Admitted as the reviewer put it. One point decided here and put to the owner (`docs/STATE.md`): `rm -r link/`, a link to a directory typed with a separator. `rm` empties the directory the link points to and then says `Not a directory`. `blit rm` says the same words with the same exit status and removes nothing: it does not remove through a link (cr-uv1-2).

## What
- **The path goes as typed.** `RemoteEndpoint::parse` sets aside only what stands before the host; the path after the module is kept to the letter. `rm`, `du`, `df` and `find` send that text (`endpoints::module_and_typed_path`), and the daemon trims nothing.
- **What the writing says is read from that one text, by both ends** (`blit_core::admin::operand`): the last name, whether it is `.` or `..`, whether the path is written as a directory (a separator after it, or a final `.`), whether it holds no name at all (the module).
- **`rm`**: an operand whose last name is `.` or `..` is refused in `rm`'s words, `-rf` or not — by the CLI, which neither asks about it nor sends it, and by the daemon for any other asker — and the next operand is processed. An operand typed as a directory is removed only if it is one: a file is `Not a directory` and stays, and `-f` passes over that as it does a missing operand. A link typed so is `Is a directory` without `-r` when it points to one, `Not a directory` otherwise, and never gone through. Failures are printed in the order the operands were typed.
- **`du`, `df`, `find`**: the start is given to the system as it was typed — with a separator after it when it was typed as a directory — so a link is resolved and a file refused by the system itself, as for the command run on that disk. `find`'s name tests see `.` as the name of a start typed `dir/.`, as `find` does.
- **Wire contract 16.** No message changed; what the path in `Rm`, `Du`, `Df` and `FindPaths` means did, and a daemon of contract 15 would tidy it again and remove what was never named. The two refuse each other.

## Guard proof
- `admin::operand::tests::a_typed_path_says_how_it_was_written`: 21 paths, each with its last name and the three readings.
- `remote::endpoint::tests::a_path_is_kept_as_typed`: eleven operands with their path to the letter; a bare host trimmed; an empty one refused.
- `deletion::rm_tests::an_operand_typed_as_a_directory_is_removed_only_if_it_is_one` and `a_link_typed_as_a_directory_is_never_gone_through` (Unix): each row what `rm` does, by error kind; nothing removed, nothing a link points to touched.
- `blit-daemon` `typed_operand_tests::an_operand_is_removed_as_it_was_typed`: the reviewer's three as the request carries them — five spellings of a final `.`/`..` refused with the next operand removed, `doc/` with and without `-f`, the two `report`s — and `a_read_starts_where_the_path_as_typed_says`.
- Through the binary against the system commands (Linux): `rm.rs::rm_equals_the_system_rm` gains eleven commands (`t/dir/.`, `t/dir/..`, `t/./empty/`, `t/dir//`, `t/file.txt/` with and without `-f` and `-r`, `t/report ` and ` t/report`, `t/link/`), lines, exit status and what is left on disk; `du_df.rs::du_equals_the_system_du` gains eight operands and now compares what is said of an operand that cannot be measured; `df_equals_the_system_df` gains three; `find.rs::find_equals_the_system_find` gains seven starts, each with three expressions.
- On every OS: `rm.rs::rm_removes_what_was_typed_and_nothing_tidier` (the reviewer's three commands, the message to the letter, a command of nothing but refused operands asking nothing) and (Unix) `rm_never_removes_through_a_link_typed_as_a_directory`.
- Mutations, each restored byte for byte, eleven: the CLI tidying `rm`'s path as before; an operand ending in `.` sent on; the daemon not refusing one; the daemon dropping how an operand was typed; a name typed as a directory removed whatever it is; a link to a directory typed as one called no directory; the end of a remote path trimmed as before; a start typed as a directory asked for without its separator; the name of a start typed `x/.` read as `x`; `du`/`df`/`find` tidying the path before sending; a final `.` not saying directory. Each red on the guards named for it — the end-to-end ones against the system commands among them for all but two.
- The mutation driver stopped at the first failing test binary, so guards in a later binary were never asked; found here when three mutations left `find`'s guard "green". It now runs every binary (`--no-fail-fast`). It could only under-report: every earlier proof named guards that did go red.

## Files changed
- `crates/blit-core/src/admin/operand.rs` (new), `admin/mod.rs`; `remote/endpoint.rs` (`parse`); `endpoints.rs` (`module_and_typed_path`); `deletion.rs` (`remove_as_rm` takes `as_directory`); `transfer_session/mod.rs` (`CONTRACT_VERSION = 16`); `proto/blit.proto` (comments).
- `crates/blit-daemon/src/service/admin.rs` (`TypedOperand`, `read_start`, `read_operands`), `core.rs`.
- `crates/blit-cli/src/rm.rs`, `du.rs`, `df.rs`, `find.rs`; tests `rm.rs`, `du_df.rs`, `find.rs`.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md` (contract 16; the rule).

## Known gaps
- **`rm -r link/` is not `rm`'s**: nothing is removed through the link (above). The owner may rule otherwise.
- The transfer verbs read an endpoint through the same `parse`: a destination typed with a space at its end now names that name. Their own tidying of the path they send is unchanged.
- A doubled separator at the END of an operand is printed back as typed for the operand itself and as one for what is below it (`du dir//` → `dir//`, `dir/sub`; the system `du` prints `dir/`, the system `find` `dir//sub`). cr-uv1-4 reworks what is printed.
- A path with `..` anywhere but last is refused by the daemon as before (`path not allowed`), where the commands resolve it; a module is not left by name.
- On Windows a separator after a path is given to the system as typed and what it says of a file typed so is the system's. Nothing here was run on Windows or macOS. Run on Linux only.
