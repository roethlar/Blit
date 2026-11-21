# cr-uv4-3: `find -delete` removed through a link in the middle of its path

**Severity**: HIGH — slice 4 said that nothing is removed through a link, and a link on the way to the path typed was looked through
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv4-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. The defect is in utilities slice 4 (`35adeacb`).

## Evidence
"With `alias` pointing to a directory inside the module, deleting `alias/entry` removed the target directory's entry. Canonicalizing the parent follows the link before the anchored remover sees it. Open each parent from the held module root without following links."

Cause. Slice 4 refused a start that was ITSELF a link typed with a separator after it, and kept that name unresolved when removing. For everything above the start it asked the system for the canonical path (`fs::canonicalize` of the start's directory) and handed THAT to the remover — so the remover, which opens each directory in the one before it and never through a link, was opening a path from which the link had already been taken out.

## Intake
Admitted as the reviewer put it, for `find -delete`.

Measured beside it, and NOT changed here (2026-11-21, this build through a loopback daemon): `blit rm -rf HOST:/m/alias/entry` removes through `alias` too — its operand's directory is resolved the same way (cr-uv1-2), three review rounds have passed over it, and it is what `rm` does. GNU `find alias/entry -delete` and `rm -r alias/entry` both remove through the link. After this fix `blit find … -delete` is stricter than `find`, than `rm`, and than `blit rm`. That is the reviewer's requirement for this command, and it leaves the two deleting verbs with different rules for a link in the middle of a path: a question for the owner, put beside the one already open on `rm -r link/` (`docs/STATE.md`). The drafting agent would have one rule for both and has not chosen it for `rm` unasked.

## What
- **Before the walk**, when `-delete` is asked: each directory ON THE WAY from the module's root to the start — every name of the path typed but its last — is looked at without looking through it (`link_on_the_way`). If one is a link, the request ends with one failure for the start, in words of this command's own, with nothing walked and nothing removed:
  `blit find: cannot delete ‘HOST:/m/alias/entry’: its path goes through the symbolic link ‘alias’, and -delete removes nothing through a link: name the directory itself` (exit 1). `find` has no words for this: it removes.
- **When removing**, the start's place is its names as typed below the module's root, with nothing resolved (`Deleting::at`). The remover opens that path one name at a time from the root, never through a link — so a link put on the way after the request was read is an error for each entry found below it, and no way through.
- Listing is unchanged: `blit find HOST:/m/alias/entry` lists through the link, as `find` does. The path named by its real directories deletes as before.

## Guard proof
- `crates/blit-cli/tests/find.rs::find_delete_removes_nothing_through_a_link_in_its_path` (end to end): the reviewer's tree; `-delete` of `alias/entry`, of it with a separator, with a name test, of a file two levels below the link and of a file directly below it — each exits 1 with exactly the one line, prints nothing, removes nothing; each of the same paths is listed without `-delete`; and `real/entry` named by itself is emptied and goes.
- `service::admin::…::find_delete_removes_nothing_through_a_link_standing_where_the_start_was` (the daemon), extended: with a link on the way to the start, an entry below it and the start itself fail to go and the target's entry stays; `link_on_the_way` finds the link, and takes neither the start's own name, nor a name that is not there, nor the root for one. Its earlier cases hold: a link standing where the start was, the directory itself, a link typed plainly going as the link.
- `find_delete_starts_where_the_system_find_starts` — seventeen ways of typing a start against the system `find` — holds unchanged.
- Mutations, each restored byte for byte: the link on the way not looked for (the end-to-end guard red: every entry then fails in the remover's own words, and nothing goes all the same); the start's place resolved before removal (the daemon guard red); the start's own name counted as on the way (the daemon guard and the comparison with `find` red).

## Files changed
- `crates/blit-daemon/src/service/admin.rs` — `link_on_the_way`; `Deleting::at` and its unresolved `start_at`; `stream_find`'s refusal; guard.
- `crates/blit-cli/tests/find.rs` — guard.
- `CHANGELOG.md` — the difference from `find`, said whole.

## Known gaps
- **`blit rm` still removes through a link in the middle of an operand's path** (above); the two verbs differ there until the owner says which rule both have.
- A module that reaches its directories through links of its own (`current -> releases/42`) cannot be cleaned by `find HOST:/m/current/logs … -delete`: the directory has to be named (`releases/42/logs`). `find` on the server's own disk has no such limit.
- The check before the walk and the walk are two looks at the path. A link put on the way between them is caught by the remover, entry by entry, in the remover's words — many lines where the check gives one.
- The WALK still reads by path: with a link put on the way mid-walk it lists what the link points to, and the remover then refuses each entry. Nothing is removed through it; what is reported is noise.
- Unix. On Windows the remover removes by a path checked to lie in the module (slice 4), not by names in held directories: the check before the walk is the whole of the rule there, and nothing ran on Windows.
