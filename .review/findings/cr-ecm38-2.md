# cr-ecm38-2: the Linux opener's fallbacks, directory creation and the resume open went by the pathname

**Severity**: HIGH — a transfer writes into a directory that has since taken its root's name. cr-ecm37-1 made each transfer hold its root and open beneath it by `openat2`. Where that open cannot serve, three paths still went by the pathname:
- **The opener's fallback.** On `EXDEV` (an absolute link) or with no `openat2`, it went through the destination resolver, which finds the directory by its path from `/`.
- **Directory creation.** On those same cases, directories were made by `create_dir_all`.
- **The resume open.** A resume's partial was found the same way.

With the destination renamed and another directory made at its name, each of these reached the replacement.
**Status**: Reviewed, found incomplete — `combined-r40` (Codex openreview, 2026-11-24) reviewed `86d69ee3..d7f5f5e9` — the `combined-r39` fixes and log-named-text — with `guard_confirmed: true`: "The held directory, explicit links from diagnostics to filenames, and preservation of named failures withheld from a retry are worth keeping. I would require the following corrections." "The base copied through a second absolute alias for the same destination directory. The head exited 2 for both ordinary copy and resume. I also observed the fallback create a regular file for a path ending in `/`, and fail to fill an internal dangling link after the held root was renamed. Verify aliases by directory identity, preserve directory-only path syntax, and inspect dangling links relative to the held directory." "With one worker, a 64-descriptor limit, and a file 72 directories deep, the base completed the resume. The head failed with `Too many open files`. Use the kernel walk for ordinary paths and bound the descriptors retained by the fallback. It currently holds one per ancestor." The remaining work is cr-ecm39-5 and -6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `fbb002d5`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It completes cr-ecm37-1 (`6e49eb4a`) and cr-ecm37-4 (`4ea67c34`).

## Evidence
"If the destination is renamed and another directory takes its old name, the fallback used for internal absolute links or unavailable `openat2` can open the replacement. Directory creation and resume also use the pathname. Resolve, create and reopen relative to the session's held directory, including on fallback."

## Intake
Admitted as the reviewer put it.

## What
- **`resume::resolve_beneath`** (Linux) resolves a destination from the root the transfer holds, one name at a time.
  - **No link is followed by the kernel.** Each name is looked at with `fstatat` (no follow), and directories are entered with `openat` (`O_PATH`, no follow).
  - **A link is read** (`readlinkat`) and its target walked in its place:
    - a relative one from where it stands;
    - an absolute one from the held root, when it names a place beneath either of the root's names;
    - otherwise it leads out, as does a `..` above the root.
  - **The last name** is followed when it is a link, unless it is to be made.
  - **Limits.** The links followed count against the system's limit (cr-ecm36-8), and a link to a name written as a directory is no place for a file (cr-ecm24-6).
- **`resolve_destination`**, for every resume path and the opener, uses it whenever a root is held.
  - Its path-based resolution is kept, as `resolve_by_path`, for a root that holds no directory (one file) and off Linux.
  - The resume entry points (`open_resume_partial`, `fresh_partial` and their kin) and the sink's resume paths take the held root (`Within`), not its path.
- **The opener's fallback** (`open_resolved`) uses the walk for `EXDEV` and for no `openat2`. A name to be made is not followed: what stands there fails the exclusive creation.
- **Directory creation** (`make_beneath`) keeps the one `openat2` look for a directory that is there. Otherwise it walks from the held root, making each missing directory with `mkdirat` in the one found above it (`resume::make_dirs_beneath`).

## Guard proof
Each guard holds a root, renames it away, and makes another directory at its name (Linux).
- `file_copy::mode_tests::without_openat2_a_file_is_made_in_the_root_held`: with the kernel told it has no `openat2`, a file is made in the held root, not in what took its name.
- `file_copy::mode_tests::an_absolute_link_is_followed_within_the_root_held`: an absolute link to a file within the root is written through in the held root.
- `file_copy::mode_tests::directories_are_made_in_the_root_held`: directories are made in the held root:
  - through an absolute link within it;
  - with no `openat2`.
- `file_copy::mode_tests::a_resume_partial_is_made_in_the_root_held`: a resume's fresh partial is made in the held root.
- Mutations, each restored byte for byte:
  - the opener's fallback by the path → red (two guards);
  - directories by the name → red;
  - a resume resolved by the path → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,133 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs`:
  - `resolve_beneath`, `make_dirs_beneath` and the walk;
  - `resolve_destination_following`, `resolve_by_path`;
  - the entry points take `Within`.
- `crates/blit-core/src/copy/file_copy/mod.rs` — `open_resolved`, `make_beneath`; guards.
- `crates/blit-core/src/remote/transfer/sink.rs` — the resume paths take the held root.

## Known gaps
- Off Linux, resolution still goes by the path (cr-ecm33-4's line); no Mac or BSD host is reachable.
- A destination path that lies beneath neither of the root's names by its text is still resolved by its path. The sink's writes always lie beneath the root's own name.
