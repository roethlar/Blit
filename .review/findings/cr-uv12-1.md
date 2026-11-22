# cr-uv12-1: on Windows `find -type … -delete` drops the kind it found

**Severity**: HIGH — on Windows, a regular file put where a link `find -type l -delete` selected can be removed as if it were the link
**Status**: Admitted — NOT FIXED. Blocked: the fix is Windows-only code in the path that removes what `find` found, nothing in this session can run it, and no Windows machine answers (2026-11-22: `michael@10.1.10.222` and `michael@10.1.10.173`, "No route to host").
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r23` over `5d2f8def..24d4064f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r23.codex.md`. It is the Windows half of cr-uv11-1 (`6b965293`).

## Evidence
"Windows supplies no Unix parent identity, so the walker drops the known type. If a regular file replaces a selected link, `find -type l -delete` can remove that file. Preserve the type and check the entry without following links before removal." (`crates/blit-core/src/find.rs`, the `seen` built only from `frame.within`; `crates/blit-core/src/deletion.rs`, the non-Unix arm of `Remover::remove_found`.)

## What it needs (not done)
- The walk: carry the kind a `-type` test held in `Seen` whether or not the directory's identity is known — `within` becomes optional, or the kind travels beside it.
- The removal elsewhere than Unix: before it removes, look at the entry without following a link (`symlink_metadata`) and refuse — "it is not what was found: it was replaced since" — when its kind is not the one found, as the Unix arm does through the held directory.
- A guard that runs on Windows: a link selected by `-type l`, replaced by a regular file before its removal, survives, and the entry is reported.
- Run the suite on Windows. Neither half can be proven on Linux: on Unix the directory's identity is always known, so the walk's change is not seen there.

## Known gaps
- The Windows removal still acts on a path after checking it (cr-uv2-2, blocked too); a check by path narrows the window and does not close it.
