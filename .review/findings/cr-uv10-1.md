# cr-uv10-1: the daemon's walk was not anchored at the root its check verified

**Severity**: HIGH — a folder above a module replaced by a link after the check leads `ls`, `du` or `find` into another tree
**Status**: Verified on Unix — `combined-r22` (Codex openreview, 2026-11-22) reviewed `b92c9a17..5d2f8def` — the eight `combined-r21` fixes — with `guard_confirmed: true`: "Opening Unix module-root components from `/` … [is] sound"; the Windows half stays open, with cr-ecm21-4's Windows walk
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `1233e9de`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r21` over `c15ebf5a..b92c9a17` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r21.codex.md`. It completes cr-uv9-1 (`5a663780`).

## Evidence
"Unix reopens the checked module root by absolute pathname, so changing an ancestor can redirect that open. Windows still opens child directories by pathname after the check. Pass a verified root handle into the walk and use handle-relative descent on both platforms."

## Intake
Admitted as the reviewer put it, on Unix by the root's own form: a module root is a path with no link in it (cr-uv6-2 asks that it still resolves to itself), so it is opened from `/` one name at a time, never through a link — what a check of it could verify, the open itself now holds to. The Windows half — child directories opened relative to a held handle there — cannot be written and proven on this Linux host; it stays open.

## What
- `held::open_unlinked`: the directory at an absolute path with no link in it — the root a walk is given — opened from `/` one name at a time, none of them through a link. A folder above it replaced by a link is an error; it led the open, by the path, wherever the link pointed.
- `held::look_unlinked`: a root that is one file is looked at in its directory, opened the same way.
- `open_canonical` opens the root, and everything below it, from there.

## Guard proof
- `listing::tests::a_listing_is_not_led_away_by_a_link_above_its_root`: a module at `above/module`; `above` replaced by a link to a tree that has a `module` of its own — a listing of the root, and of a path below it, is an error with nothing of the other tree listed.
- Mutation, restored byte for byte: the root opened by its path (red).
- The core walk tests (32), the CLI's `ls`, `find`, `du`/`df` tests and the daemon's (220) pass — a module that is one file among them. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/held.rs` — `open_unlinked`, `look_unlinked`, `open_canonical`, `look_whole`.
- `crates/blit-core/src/listing.rs` — guard.

## Known gaps
- Windows (BLOCKED): the walk reads by path there, as before.
- A folder above the module replaced by another real folder (not a link) is the path as configured, and is read; nothing tells the two apart without a record of the module's identity, which would refuse a module folder that is legitimately made again.
- The daemon's check (`start_is_contained`) still looks by path; it decides what may be read, and the walk now reads only what its own opens reach.
