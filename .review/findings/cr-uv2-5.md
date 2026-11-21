# cr-uv2-5: Some names shown could not be given back, or were another file's

**Severity**: MEDIUM — a path the commands print is refused as an operand; a message, the question and `df`'s table name a file other than the one meant
**Status**: Reviewed, found incomplete — `utility-slices-r3` (Codex openreview, 2026-11-21) reviewed `eb0cc4d8..dd0a3179`, five of the six `utility-slices-r2` fixes, with `guard_confirmed: true`. The daemon's part is read on its own now; the PATH is still read by the receiving system's own path rules, on the daemon: on Linux `find` printed `\notes` and `C:receipt` and `du` refused both as Windows paths, and on Windows (from the source) `quarter\notes.pst` becomes two components, so `rm` can remove `quarter/notes.pst`. `--json` still holds a name that is not text as the text nearest it (the known gap below, now a finding). And the round-trip guard creates `bad\xff.dat` on any Unix, which APFS refuses. The remaining work is cr-uv3-3, -7 and -8
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `09c84a5f`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r2` over `4841b2d4..3cd49e40` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r2.codex.md`. It completes cr-uv1-4.

## Evidence
"Blit printed the valid Unix filename `quarter\notes.pst`, then classified that exact remote operand as local. For a name containing byte `0xff`, plain `df` output and an `rm` diagnostic both displayed the spelling of a distinct file containing U+FFFD. Parse the endpoint prefix independently of the filename bytes. Preserve those bytes in path output and use reversible quoting in prompts and diagnostics."

Cause.
- **The whole word was read as an endpoint.** The reader that tells a daemon from a local path looks for a `\` anywhere in the word — it is how `C:\dir` and `host:\module` are told apart — and with one in the PATH the word was "a local path" (or, by the host's shape, "remote paths must use forward slashes").
- **A name was put into text by replacing what is not text.** `rm`'s and `du`'s messages, `rm`'s question, `df`'s message and the first column of `df`'s table all showed `String::from_utf8_lossy` of the name: `bad\xff.dat` printed as `bad\u{fffd}.dat`, which is the name of another file that can stand beside it.

Measured: how GNU `rm`, `du`, `df` and `find` each write a name in a message, for twenty names that are not plain text (coreutils 9.12, findutils 4.11.0, `C.UTF-8`). `rm` and `du` always quote, in the form a shell reads back — `'plain.txt'`, `"it's"`, `'bad'$'\377''.dat'`, `'new'$'\n''line'`; `df` writes the same form only where a shell would not read the name as one word (`plain.txt`, `café`, `-dash` bare); `find` writes `‘bad\377.dat’`, a backslash as two.

## Predicted observable failure
`blit find HOST:/m/mail -name '*.pst' | xargs -d '\n' blit du` on a tree migrated from Windows with a `\` in a name: `requires a remote path (received local path: …)`. `blit rm` of a tree holding both `bad\xff.dat` and `bad�.dat`, one of which cannot be removed: the message does not say which.

## Intake
Admitted as the reviewer put it. Quoting is each command's own, as measured, since each "behaves like the local version".

## What
- **The daemon's part of an operand is read on its own** (`Operand::read`, `daemon_part`): `HOST://` or `HOST:/MODULE/`, found by the first `:/` in the bytes; only that much is given to the endpoint reader, and what follows is the path, whatever it holds — a `\`, a `://`, bytes that are no text. A word shaped like a daemon's that does not read as one now says why — `HOST:/module`: `module path must end with '/'` — where it was called a local path.
- **A name in a message is written as that command's reference writes it** (`blit_core::admin::quote`): `for_shell` for `rm` and `du` (always quoted) and `df` (quoted where needed), `for_find` for `find`. Each is exact for every byte, so two names are never written alike, and the first three read back in a shell. `rm`'s question quotes its operands the same way.
- **`df`'s table prints the operand by its own bytes**, as `find` and `du` print paths.
- What can be printed is the reference locale's `[:print:]` (`find_name::is_printable`, cr-uv2-6's tables).

## Guard proof
- `rm.rs::a_name_in_a_message_is_written_as_the_system_commands_write_it` (Linux), through the binary and beside the system's `rm`, `du`, `df` and `find`: twenty-two names that are not there — the byte `0xff` alone and among letters, a newline, a tab, controls, an apostrophe, a `"`, a `$`, a `\`, spaces at either end, a leading `-`, letters beyond ASCII, U+FFFD itself — the message of each is the system command's, and all eighty-eight are distinct.
- `rm.rs::a_name_these_commands_print_is_one_they_take` (Unix): `quarter\notes.pst` and `bad\xff.dat`, each printed by `find`, the line given to `du`, to `df` — whose table row begins with those bytes — and to `rm`, which removes that file.
- `admin::quote::tests::a_name_is_written_as_the_reference_writes_it`: twenty-four names, the three forms of each to the letter as measured; `two_names_are_never_written_alike`.
- `admin::operand::tests::what_a_path_holds_does_not_decide_what_the_word_is`: seven operands with `\`, `://`, `C:/`, `~` in the path; seven local words that stay local; five that say why they are no operand.
- `df::tests::the_table_is_dfs_for_one_filesystem`: the row holds the operand's bytes; a name of letters beyond ASCII is as wide as its letters.
- `find.rs::a_name_that_is_not_text_goes_round`: `rm`'s message for the name, quoted.
- Mutations, each restored byte for byte: the whole word read as an endpoint, as before; `rm`, `du`, `df` (message), `df` (table) and `find` each writing the text nearest the name; a name with an apostrophe never put in double quotes; what a shell minds not looked for — eight, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/admin/quote.rs` — new.
- `crates/blit-core/src/admin/operand.rs` — `Operand::read`, `daemon_part`; guard.
- `crates/blit-core/src/admin/mod.rs`, `crates/blit-core/src/find_name.rs` (`is_printable`).
- `crates/blit-cli/src/{rm,du,df,find}.rs` — messages, the question, `df`'s table.
- `crates/blit-cli/tests/rm.rs`, `crates/blit-cli/tests/find.rs` — guards.
- `CHANGELOG.md`.

## Known gaps
- **`--json` still holds text**: a path that is not UTF-8 is in JSON as the text nearest it, for `rm`, `du` and `find`. JSON has no other way to hold a name; a second field with the bytes was not asked for and is not added.
- **A transfer's endpoint is still read whole.** `blit copy HOST:/m/a\b out/` is refused on Unix as before; this finding is the utility commands', whose operands are bytes since cr-uv1-4. Put to the owner in `docs/STATE.md`.
- `HOST:/module` with no `/` after the module is still no operand — the endpoint grammar's rule — and now says so.
- `df` pads its first column by characters when the operand is text and by bytes when it is not; a wide character is counted as one.
- The quoting is GNU's for a `C.UTF-8` locale. On Windows the commands quote the same way; nothing has run there.
- Run on Linux only.
