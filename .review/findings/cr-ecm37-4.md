# cr-ecm37-4: an absolute link within the destination was refused as leading out

**Severity**: MEDIUM — a copy the base made now fails. Brought in by cr-ecm36-1. On Linux the opener opens beneath the destination's root with `openat2(RESOLVE_BENEATH)`. The kernel answers `EXDEV` for every absolute link, including one that leads back into the destination, and the opener took that as "leads out". The base copied through an absolute link to a file inside the destination and exited 0. The head refused it, exited 2 and left the old bytes. The same link written relative worked.
**Status**: Reviewed, found incomplete — `combined-r39` (Codex openreview, 2026-11-24) reviewed `6ee7ed5e..86d69ee3` — the `combined-r38` fixes — with `guard_confirmed: true`: "The session-held directory, explicit failure index, structured unknown disposition and hash-based summary lookups are sound choices. I would keep them and require the following corrections." "If the destination is renamed and another directory takes its old name, the fallback used for internal absolute links or unavailable `openat2` can open the replacement. Directory creation and resume also use the pathname. Resolve, create and reopen relative to the session's held directory, including on fallback." The remaining work is cr-ecm38-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `4ea67c34`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r38` over `2c55cb3a..6ee7ed5e` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r38.codex.md`. It completes cr-ecm36-1 (`d5378b09`) in part.

## Evidence
"The base CLI copied through an absolute symlink to a file inside the destination and exited 0. The head reported that the link led outside, exited 2 and left the old bytes. The equivalent relative link worked. Resolve permitted targets against the held root before opening them."

## Intake
Admitted as the reviewer put it.

## What
- **On `EXDEV`**, `open_beneath` (`copy/file_copy/mod.rs`) now resolves the place against the destination with the resolver every other contained open uses (`resolve_destination`):
  - the chain of links at the name is followed;
  - the directory it ends in is canonicalized, checked to lie within the root, and held;
  - the file is opened by its name in that directory, never through a link.

  A place that lies within is opened; one that leads out is refused, as before.
- **The mode of a made file.** A file this resolution makes takes the mode any other made file takes (`0666` less the umask), through `Resolved::open_as`. `Resolved::open` keeps its private `0600` for a resume's partial.

## Guard proof
- `file_copy::mode_tests::an_absolute_link_within_the_destination_is_followed` (Linux):
  - an absolute link to a file within the destination is written through, and the link stays;
  - a file is made through an absolute link to a directory within the destination;
  - an absolute link to a directory outside is still refused, and nothing is made there.
- Mutation, restored byte for byte: every absolute link refused → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,111 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `open_beneath` on `EXDEV`; guard.
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Resolved::open_as`, `open_in_as`.

## Known gaps
- Linux only: elsewhere the opens go by the name, which follows such a link already.
