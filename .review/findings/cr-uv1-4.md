# cr-uv1-4: What `find`, `du` and `rm` printed could not always be given back

**Severity**: MEDIUM — a printed path named another thing, or was refused as an operand
**Status**: Reviewed, found incomplete — `utility-slices-r2` (Codex openreview, 2026-11-21) reviewed `4841b2d4..3cd49e40`, the seven `utility-slices-r1` fixes, with `guard_confirmed: true`. The round trip works: "I passed a non-UTF-8 path printed by `find HOST://` directly to `du` and `rm`. Both used the correct file, and its UTF-8 lookalike survived." Still not reusable or not exact: a name holding a backslash, printed by blit, is read back as a local path; and `df`'s output, `rm`'s diagnostics and the question show a name that is not text as its near-copy, which is another file's name. The remaining work is cr-uv2-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `a93294b0`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`.

## Evidence
Starting at `HOST://`, both `find` and `du` printed `HOST:/child`, changing the default-root address into named-module syntax. A non-UTF-8 name printed by `find` was also rejected when passed to `rm` or supplied as a literal `-name` pattern. The reviewer: preserve the endpoint prefix when appending children and carry OS-string arguments and raw path bytes through the request.

Cause, two things:
- **What is printed below an operand was the operand with its trailing separators trimmed, one separator, and the entry.** For `HOST://` the separators trimmed were the ones that END THE DAEMON'S PART: `HOST:` + `/` + `child`.
- **The command line took text only.** The operands and `find`'s expression were `String`s, so a word that is not UTF-8 was refused by the argument parser; and `Rm`, `Du`, `Df` and `FindPaths` carried a path as a `string`. These commands print a name that is not text by its own bytes (slice 1 onward), so they printed paths they could not take.

## Predicted observable failure
`blit find HOST:// -name x | xargs blit rm` asks for the module `x` (or whatever the first name is); a tree holding a name that is not UTF-8 cannot be cleaned up by `find … | xargs rm`, nor searched for that name.

## Intake
Admitted as the reviewer put it.

## What
- **An operand is read once, as typed and by its bytes** (`blit_core::admin::operand::Operand`): the daemon and the module as text, from the text the operand begins with; the path below the module as the bytes typed, and where in the operand it begins (`RemoteEndpoint::path_offset`). On Unix a command-line word is bytes; elsewhere it is text or refused. A byte that is no text in the daemon's name or the module's is refused by saying so.
- **What is printed below an operand** is made from those two parts. As `du` and `rm` print it (`Operand::below`): the daemon's part untouched, the path with the separators after it set aside, one separator, the entry — `HOST://child`, `HOST:/m/dir/child` for `dir/` and `dir` alike. As `find` prints it (`below_as_find`): the operand as typed, a separator unless it ends in one, the entry — every separator typed kept, `dir//` printing `dir//sub`, as `find` does.
- **A path that is not text has its bytes beside it on the wire — contract 18**: `RmRequest.raw_paths` (one for each path, or none), `raw_start_path` in `DuRequest` and `FindPathsRequest`, `raw_path` in `DfRequest`, by the convention the replies already follow (`raw_relative_path`). The daemon reads the bytes when they are sent and the text otherwise, and validates either the same way (`validate_wire_path_bytes`): relative, no `..`.
- **`find`'s expression is taken word by word as bytes**: a test's own name is text; the argument of `-name` and `-iname` is the pattern's bytes, which is how a pattern was already matched and sent.
- The helpers that read what a path's writing says (cr-uv1-1) work on bytes.

## Guard proof
- `admin::operand::tests::what_is_printed_below_an_operand_can_be_given_back`: twelve operands — the default root with and without a path, a module's root, separators and a `.` after a path, a space at its end, a port, an IPv6 host, a space before the host — each with its module, its path, what `du` and `find` print below it, and that either, read again, names the entry in the same module.
- `admin::operand::tests::a_path_that_is_not_text_is_taken_by_its_bytes` (Unix): the path, what goes on the wire (text and bytes), what is printed below it; text all through carries no second copy; a byte that is no text in the host or the module is refused; a local path that is not text is a local path.
- `remote::endpoint::tests::a_path_is_kept_as_typed` gains where the path begins for seven forms.
- `blit-daemon` `typed_operand_tests::a_path_that_is_not_text_is_read_by_its_bytes` (Unix): with a file named by bytes and one named by their near-copy on disk, the bytes name the first — as a start, and as an operand of a removal, which leaves the near-copy; one byte form for each path or none.
- `blit` `find::tests::a_pattern_that_is_not_text_is_taken_by_its_bytes` (Unix).
- Through the binary: `find.rs::a_name_that_is_not_text_goes_round` (Unix; against the system `find` on Linux) — the name is printed by its bytes; those bytes are a start, a literal `-name` pattern, a pattern with `*`, an `-iname` pattern; the line `find` printed is an operand `rm` removes, leaving the file beside it; and `rm` of it again says `No such file or directory`. `du_df.rs::du_equals_the_system_du` gains `du` and `df` of the name that is not text. `find_equals_the_system_find` gains the starts `t/sub//` and `t//`.
- Mutations, each restored byte for byte: the separators that end the daemon's part trimmed with the operand's → the first guard red; `find`'s separators collapsed to one → the first and the comparison with the system `find`; a path that is not text sent as its near-copy alone → the second, the round trip and `du`'s comparison; the daemon reading the text and not the bytes → its own guard, the round trip and `du`'s; a `-name` pattern sent as a near-copy → its guard and the round trip; `rm` sending no bytes → the round trip.

## Files changed
- `crates/blit-core/src/admin/operand.rs` (`Operand`, `word_bytes`; the helpers on bytes), `remote/endpoint.rs` (`path_offset`), `endpoints.rs` (`module_and_typed_path` removed), `admin/du.rs` (`EntryPath::typed`), `admin/df.rs`, `admin/find.rs`, `admin/rm.rs`, `proto/blit.proto`, `transfer_session/mod.rs` (`CONTRACT_VERSION = 18`).
- `crates/blit-daemon/src/service/admin.rs` (`typed_bytes`, `resolve_typed`, `read_start`, `read_operands`), `core.rs`.
- `crates/blit-cli/src/cli.rs` (operands and the expression as `OsString`), `du.rs`, `df.rs`, `find.rs`, `rm.rs`; tests `find.rs`, `du_df.rs`, `rm.rs`.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- `du.rs`'s unit test of what is printed (`a_printed_path_begins_with_the_operand_as_typed`) is removed with the function it tested; the three guards in `admin::operand` replace it (the workspace's count still rises).
- No end-to-end guard runs against a daemon's DEFAULT ROOT (`HOST://`): the test daemons serve a module. What is printed for that form is guarded where it is made.
- `du` prints an operand typed with several separators after it as typed (`dir//`), where the system `du` prints one (`dir/`); below it both print `dir/sub`.
- On Windows a command-line word and a path on disk are Unicode, so there is no name that is not text to give back; a word that is not valid Unicode is refused. A Windows daemon sent bytes that are not text refuses them (`validate_wire_path_bytes`). Nothing was run on Windows or macOS.
- `--json` output and the messages on stderr show a name that is not text as its near-copy: both are text.
- Run on Linux only.
