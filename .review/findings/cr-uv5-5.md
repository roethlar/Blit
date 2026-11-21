# cr-uv5-5: two of `ls`'s operands were refused at the daemon as paths out of the module

**Severity**: MEDIUM — `ls`, `du` and `find` of a module that is one file fail; a file below a directory that cannot be searched ends the request with exit 1 and nothing in `--json`
**Status**: Reviewed, found incomplete — `combined-r17` (Codex openreview, 2026-11-21) reviewed `13faafd2..7912da16` — the nine `combined-r16` fixes and local-copy-mode — with `guard_confirmed: true`. "A request for the root passes containment on pathname equality alone. If that pathname resolves elsewhere after daemon startup, a plain listing can expose the other directory. Validate the current root against the configured export while preserving the correct error for a file operand typed with a trailing slash." The remaining work is cr-uv6-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `ca4e6855`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r16` over `35adeacb..13faafd2` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r16.codex.md`. Slice 5 of `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` (`f1a9055d`).

## Evidence
"`ls HOST:/one/` failed for a module exporting one regular file because the daemon appended `/.` to its path. Separately, a file below a directory without search permission produced exit 1 and empty JSON output. GNU `ls` returned 2. Preserve an empty module-relative path and return ordinary lookup failures through the listing response while enforcing containment."

Measured before the fix (Linux, GNU coreutils 9.12 beside it):

| typed | blit | the system's |
|---|---|---|
| `ls`, `du`, `find` of a module that is one file | exit 1, `path containment: path '…/one.txt/.' resolves to '…' which escapes module root` | `one.txt`; `4 one.txt`; `one.txt` — exit 0 |
| `ls locked/file` (`locked` mode 000), with and without `-l` and `--json` | exit 1, `path containment: canonicalize … Permission denied`, nothing on stdout | `ls: cannot access 'locked/file': Permission denied`, exit 2 |
| `du locked/file`, `find locked/file` | the same refusal, exit 1 | `du: cannot access …`, `find: ‘locked/file’: Permission denied`, exit 1 |

Causes, two.
1. The module itself was asked of the system as `root/.` (`TypedOperand::below`), which the system refuses for a file (`Not a directory`); the containment check then judged the path by its ancestors and found it above the module.
2. `path_safety::verify_contained` treats a path that is not there by its deepest ancestor that is, and any other failure to resolve it as a failed check. A directory that cannot be searched is such a failure, so the request was refused as if it had left the module.

And one thing the second fix laid bare, which was wrong on a local path too: `ls link` of a link that cannot be followed printed the link whatever the reason. Measured, GNU `ls` 9.12 does that for a link to nothing (`ENOENT`) and one that leads round in a ring (`ELOOP`) and for no other: a link to a file below a directory that cannot be searched, a link through a file, a link by a name too long are `cannot access`, exit 2. (With `-l` it is always the link.) Before this fix the daemon refused the first of those as a containment failure; `blit ls ./link` on this machine printed all three.

## Intake
Admitted as the reviewer put it. `du` and `find` share both causes — one function reads where a read starts — and are put right with `ls`; they are named in the changelog.

## What
- `TypedOperand::below`: the module itself is its root as it stands, with nothing after it (a separator still, when it was typed as a directory: `HOST:/one/.` of a file is `Not a directory`, as `ls file/.` is).
- `start_is_contained(module_root, start_abs)`, asked by `ls`, `du` and `find` in place of `verify_contained`: the module itself is in the module however it was typed; otherwise what is judged is the deepest part of the start the system LOOKS UP — the trailing names it fails on for any reason but not-there are taken off, and `verify_contained` judges the rest, as it always has for a path that is not there. In the module, the read goes on to the start, fails there as the system fails, and the command says so in its own words. Out of the module, the request is refused as before — whatever cannot be looked up beyond the link that leaves it.
- `listing::list`: a link operand that cannot be followed is the link's own line only for `ENOENT` and `ELOOP` (`shown_as_the_link`); any other reason is `ListEnd::Access`. Off Unix there is no `ls` to measure and it is as it was.
- `verify_contained` itself is unchanged: transfers ask it, and nothing here is theirs.

## Guard proof
- `blit-daemon` `typed_operand_tests::a_start_is_judged_by_the_part_of_it_that_can_be_looked_up`: the module itself, a directory and one file, typed `""` and `.`; starts past a link that leads round in a ring, in the module (accepted) and by a link out of it (refused, `PermissionDenied`, "path containment"); a path not there on either side. No permission bits in it, so it holds for root.
- `typed_operand_tests::a_read_starts_where_the_path_as_typed_says` (changed): the module is `/srv/module` and `/srv/module/`, where it expected `/srv/module/.` and `/srv/module/./`.
- `blit-core` `listing::tests::a_link_that_cannot_be_followed_is_itself_only_where_ls_shows_it`: to nothing and round in a ring, the link; through a file and by a name too long, `Access` with the system's reason; each the link under the long form.
- `crates/blit-cli/tests/ls.rs`:
  - `a_module_that_is_one_file_is_read_as_the_file_it_is` — a second daemon whose module is a regular file: `ls`, `ls -l`, `ls --json`, `du`, `find`; and `ls HOST:/test/.`, exit 2, `Not a directory`.
  - `a_start_that_cannot_be_looked_up_is_told_and_one_out_of_the_module_refused` (Linux, not as root) — the reviewer's case for `ls`, `ls --json` (the failure is in it), `du`, `find`; and the same path by a link out of the module, refused by all four with nothing printed.
  - `ls_equals_the_system_ls` — seven more operands compared with the system `ls` under nine combinations of options (twenty-six in all, 234 comparisons): a link round in a ring, a path through it, a link through a file, a link by a name too long, and — not as root — a file below a directory that cannot be searched, a link to it, that link typed as a directory.
  - `ls_of_a_local_path_equals_the_system_ls` — four of those on a local path.
- Mutations, each restored byte for byte, each red on the guards named: the module asked as `root/.` again; the module judged by its ancestors; the names that cannot be looked up not taken off; what could not be looked up accepted unjudged; a link that cannot be followed shown as the link whatever the reason.
- The test harness (`tests/common/mod.rs::spawn_daemon`) exports what is already there as it stands, so a module can be one file; it made the directory unconditionally.

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — `below`, `start_is_contained`, its three callers; guards.
- `crates/blit-core/src/listing.rs` — `shown_as_the_link`, `list`; guard.
- `crates/blit-cli/tests/ls.rs`, `crates/blit-cli/tests/common/mod.rs` — guards.
- `CHANGELOG.md`.

## Known gaps
- A start accepted because its looked-up part is in the module is then read by path: what stands there can change between the check and the read, as it can for a path that was not there (unchanged; `find -delete` and `rm` do not go this way — they open what they remove).
- The refusal for a path out of the module still names where the link leads (unchanged).
- `df` of a module that is one file, and `rm`, were not asked about and are not guarded here: `rm` reads its operands by another function and refuses the module's root by rule.
- Off Unix a link that cannot be followed is shown as the link, as before: nothing to measure against.
- Linux only. The end-to-end guard for a directory that cannot be searched does not run as root.
