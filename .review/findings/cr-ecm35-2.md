# cr-ecm35-2: the Linux mmap copy left its source's mode unapplied and a short copy's partial in place

**Severity**: HIGH — a copy can look finished when it is not, and a private file can be copied readable by others. Brought in by cr-ecm34-3's fallback, and partly older:
- `mmap_copy_file` never gave its destination the source's mode. A `0600` source copied over a `0644` destination stayed `0644`.
- A copy that failed after the destination was cut to the source's size left it there. A source that shrank from eight bytes to three left eight bytes, ending in five zeroes.
**Status**: Reviewed, found incomplete — `combined-r37` (Codex openreview, 2026-11-24) reviewed `d2494965..2c55cb3a` — the `combined-r36` fixes — with `guard_confirmed: true`: "Exclusive creation, applying permissions through the open handle, and sharing failure accounting between the log and saved record are sound choices. The unretried count also works through the CLI. With one requested name over the wire budget, it reported one failure and one file not retried." "When the destination is a symlink to the file being written, cleanup compares the link's inode with the target's inode and returns the original error. A read failure can leave a partially written target at the advertised source length with no `LeftInPlace` marker. The sink already handles this case through its retained descriptor." The remaining work is cr-ecm36-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `795a9662`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r36` over `dc14afc4..d2494965` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r36.codex.md`. It completes cr-ecm34-3 (`71363fa9`).

## Evidence
"A `0600` source finished as `0644` on an existing destination. In a second probe, an eight-byte source shrank to three bytes. The copy returned an error and left an eight-byte destination ending in five zeroes. Apply permissions through the handle and remove or mark the partial on failure."

## Intake
Admitted as the reviewer put it.

## What
On Linux, `mmap_copy_file` copies in `copy_through`: the cut to the source's size, the kernel's copies, and the rest through the handles.
- **The mode.** The source's mode, from the opened source, is then given through the destination's handle, whichever path copied it.
- **A failure** from the cut on — a short source included — is settled by `settle_failed_copy`:
  - What the copy wrote is removed while the name still holds that file (by identity).
  - A file put at the name since is left as it is.
  - What cannot be removed is marked unfinished through the handle, one byte longer than the source, and the error carries the `LeftInPlace` marker (D-2026-11-24-1).

Elsewhere on Unix (macOS) the copy is unchanged: `std::fs::copy`, which gives the mode itself (cr-ecm33-4, blocked).

## Guard proof
- `file_copy::mmap::tests::the_copy_has_its_sources_mode`: through the kernel's copy and through the fallback, a `0600` source's copy over a `0644` file is `0600`.
- `a_short_copy_leaves_nothing_that_looks_finished`: a source truncated to three bytes before the fallback fails the copy, and nothing is left at the destination.
- Mutations, each restored byte for byte:
  - the mode not given → red;
  - a failure not settled → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,089 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mmap.rs` — `copy_through`, `settle_failed_copy`, the mode; guards.

## Known gaps
- An existing destination's old bytes are gone once it is cut, as with every in-place copy (D-2026-09-29-2). A failed copy removes the file rather than restoring them.
- macOS unchanged (cr-ecm33-4).
