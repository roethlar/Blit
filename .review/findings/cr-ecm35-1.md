# cr-ecm35-1: the opener's own link resolution could leave the destination

**Severity**: HIGH — a file can be made outside the destination. Brought in by cr-ecm34-1, whose opener followed the name's links itself to know what it made:
- A dangling link passed the destination's containment check, which judged its lexical parent. The opener then made the file where the link led, outside the destination.
- A file that vanished between the opener's two opens was made by the second, recorded as one that stood there.
- The resolver refused a chain of 40 links, which Linux opens.
**Status**: Fixed on Unix; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm35-1`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-1 (`20779849`).

## Evidence
"By inspection, a dangling link can pass containment against its lexical parent and then resolve to a missing file outside the destination. If an existing file disappears between the two opens, the fallback can also create a file recorded as pre-existing. Reuse the held-directory resolver, make exclusive creation the only creating open, and retain the handle through cleanup. The new resolver also rejected a 40-link chain that Linux opened successfully."

## Intake
Admitted as the reviewer put it.

## What
On Unix, `copy::create_regular_within(dst, within)` replaces cr-ecm34-1's resolution. `create_regular(dst)` is it with no root.
- **One creating open.** A file is made only by a creation that fails if the name is taken, at the name itself (`O_EXCL`), never through a link.
- **A taken name is only opened.** It is opened without creating anything. A file gone between the two opens is therefore never made unrecorded: the name is tried again, a bounded number of times.
- **A dangling link** — a name taken whose open finds nothing, and that is a link — has its file made where it leads by the resolution a resume's partial uses (`resolve_destination`):
  - the chain followed as the system follows it, up to 40 links;
  - its end judged within `within`, the destination's canonical root;
  - the directory held, and the file made there exclusively.

  A link out of the destination makes nothing outside it, and the file fails.
- **Cleanup by identity.** A refused file the opener made is removed only while the place it was made still holds it, judged by identity: at the name (`remove_if_mine`), or in the held directory (`Resolved::remove_if`). Its handle is held through the removal.
- **The local copy passes its root**; the exported path-based copies pass none.
- **Unchanged:** the cheap path, a name absent or a file there. It is two opens at most, as before cr-ecm34-1, whose extra look at every taken name is gone. Off Unix the opener is unchanged.

## Guard proof
Tests:
- `file_copy::mode_tests::a_dangling_link_out_of_the_destination_makes_nothing_outside`: within the root, a link `../outside/new.bin` is refused, nothing is made outside, and the link stays.
- `a_chain_of_forty_links_to_nothing_has_its_file_made`.
- `a_file_gone_between_the_opens_is_made_only_exclusively`: a file removed between the opens, then refused, is not left.
- cr-ecm34-1's guards still hold.

Mutations, each restored byte for byte:

| Mutation | Red |
|---|---|
| The root not judged | the out-of-destination guard |
| The second open makes | the gone-between guard, and two others |
| One link fewer allowed | the forty-link guard |

Checks:
- Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.
- The workspace passes on Linux: 2,087 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `create_regular_within`, `acquire_to_write`, `MadeAt`, a test seam; guards; cr-ecm34-1's resolver removed.
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Resolved::remove_if`.
- `crates/blit-core/src/copy/mod.rs` — exports.
- `crates/blit-core/src/remote/transfer/sink.rs` — the local copy names its root.

## Known gaps
- A file made through a dangling link is made private (600) by the held directory's open, as a resume's partial is. The copy then gives it its source's mode, where it gives one (cr-ecm35-2 for the mmap copy).
- Off Unix the opener is unchanged: no Windows host is reachable.
