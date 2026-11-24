# cr-ecm39-5: the held-root walk refused an alias of the root, lost directory-only syntax, and looked for a dangling link by its path

**Severity**: MEDIUM — copies the base made fail, and one makes the wrong kind of entry. Brought in by cr-ecm38-2 and cr-ecm37-1. The reviewer found three faults:
- **An alias of the root.** An absolute link within the destination that names it by another path (a second absolute alias for the same directory) was taken for a way out. The base copied through it; the head exited 2, both for an ordinary copy and for a resume.
- **Directory-only syntax.** A path written as a directory (`x/`) had its place found from the root by its names alone, losing the `/`, and the fallback made a regular file there.
- **A dangling link.** After the held root was renamed, a dangling link within it was looked for by its path, not found in what had taken the root's name, and its file never made.
**Status**: Reviewed, found incomplete — `combined-r41` (Codex openreview, 2026-11-24) reviewed `d7f5f5e9..b6e6fab9` — the `combined-r40` fixes — with `guard_confirmed: true`: "The per-failure identity and Linux `openat2` fast path are worth keeping. A 72-directory-deep resume completed with a 64-descriptor limit and produced the correct bytes. I would require the following changes." "An absolute alias to the destination root worked. An alias directly to a subdirectory of that root caused both ordinary copy and resume to exit 2 for a file inside the destination. Verify the resolved directory's ancestry against the held root for aliases to descendants." The remaining work is cr-ecm40-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `57ba8bb2`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r40` over `86d69ee3..d7f5f5e9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r40.codex.md`. It completes cr-ecm38-2 (`fbb002d5`) in part.

## Evidence
"The base copied through a second absolute alias for the same destination directory. The head exited 2 for both ordinary copy and resume. I also observed the fallback create a regular file for a path ending in `/`, and fail to fill an internal dangling link after the held root was renamed. Verify aliases by directory identity, preserve directory-only path syntax, and inspect dangling links relative to the held directory."

## Intake
Admitted as the reviewer put it.

## What
- **Aliases, by identity.** When an absolute link's target names neither of the root's names, the walk (`Walk::alias_place`) looks for the ancestor of that target that is the root itself: the same device and inode as the held root. Where it finds one, the rest of the target lies beneath the root; where none, the link leads out.
- **Directory-only syntax.** The opener's open beneath the root (`open_beneath`) refuses a name written as a directory, as the system refuses it by its name (`EISDIR`), before any place is found from it.
- **A dangling link.** The opener looks for a link at the name in the directory the held root leads to (`a_link_at`, `Resolved::is_link`), not by its path. The file it names is then made where it leads, from the held root.

## Guard proof
- `file_copy::mode_tests::an_absolute_link_through_an_alias_of_the_root_is_followed` (Linux): absolute links within the root that name it through a link to its parent.
  - A file one leads to is written through.
  - A directory is made through another.
- `file_copy::mode_tests::a_name_written_as_a_directory_is_no_file` (Linux): `x/` is refused and nothing is made, with `openat2` and without.
- `file_copy::mode_tests::a_dangling_link_is_filled_in_the_root_held` (Linux): with the root renamed and another directory at its name, a dangling link has its file made where it leads in the held root, and the link is left.
- Mutations, each restored byte for byte:
  - an alias of the root taken for a way out → red;
  - a name written as a directory opened as a file → red;
  - a dangling link looked for by its name → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,138 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Walk::alias_place`, `Resolved::is_link`; `typed_as_directory` shared.
- `crates/blit-core/src/copy/file_copy/mod.rs` — `open_beneath`, `a_link_at`; guards.

## Known gaps
- Finding an alias stats each ancestor of the link's target by its path. That runs only for an absolute link that names neither of the root's names.
