# cr-ecm34-1: a failed acquisition's cleanup removed by name, and missed a file made through a dangling link

**Severity**: HIGH — a destination file can be lost, or left behind unreported. Two errors, from cr-ecm33-5's first half:
- When the check of a file `copy::create_regular` had just made failed, it dropped the handle and removed the name, so a file put there since was removed in its place.
- Through a dangling link at the name, the exclusive create failed on the link. The fallback open then made the file the link leads to, recorded as not made, and a refused check left it there, empty, reported nowhere.
**Status**: Reviewed, found incomplete — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried." "By inspection, a dangling link can pass containment against its lexical parent and then resolve to a missing file outside the destination. If an existing file disappears between the two opens, the fallback can also create a file recorded as pre-existing. Reuse the held-directory resolver, make exclusive creation the only creating open, and retain the handle through cleanup. The new resolver also rejected a 40-link chain that Linux opened successfully." The remaining work is cr-ecm35-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `20779849`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`. It completes cr-ecm33-5 (`b3d952e1`) in part.

## Evidence
"After validation fails, `create_regular` drops its handle and unlinks `dst` by name. A replacement at that name can be deleted. In the other direction, I confirmed that opening through a dangling symlink can create its target, fail validation, and leave an unreported empty file because the fallback recorded `made = false`. Track creation at the resolved target and guard every fallible step through truncation."

## Intake
Admitted as the reviewer put it.

## What
On Unix, `create_regular`:
- **Follows the links first.** It follows the chain of links standing at the name (`links_resolved`, bounded as the system bounds it, `ELOOP`) to the name they lead to, and makes or opens the file there. A file made through a dangling link is therefore known to be made, and that is the name its cleanup removes. The link is left as it was.
- **Takes the made file's identity** right after the open, before anything else is asked of it.
- **Settles every fallible step to the cut.** The look, the regular-file check and the cut (`set_len(0)`) form one fallible step, so a failure anywhere in it is settled.
- **Removes only its own file.** A refused file it made is removed only while the name still holds it (`remove_if_mine`). A file put there since is somebody's and is kept. A made file whose identity could not be taken is not removed but reported, in the left-in-place words and with the `LeftInPlace` marker. One that could not be removed is reported the same way, as before.

Elsewhere, the opener is unchanged: Windows has no file identity here.

## Guard proof
- `file_copy::mode_tests::a_file_made_through_a_dangling_link_and_refused_is_not_left`: the file made through a dangling link and refused is removed, and the link stays.
- `file_copy::mode_tests::a_file_put_where_a_refused_one_was_made_is_kept`: a file renamed over the name before the cleanup looks is kept.
- `a_file_made_and_then_refused_is_not_left` still holds.
- Mutations, each restored byte for byte:
  - the links not followed first → red;
  - removed by name, whatever is there → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,081 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `create_regular`, `links_resolved`, `remove_if_mine`, a test seam; guards.

## Known gaps
- **A narrow race remains.** The name is looked at and then removed. A file renamed over it in between is removed, as with every removal by name on these systems: there is no removal by handle.
- **Windows unchanged.** It removes by name as before. No Windows host is reachable to prove an identity check there.
- **The cut has no guard of its own.** A failure of `set_len(0)` after making is settled with the rest, but would need an injected fault to prove.
