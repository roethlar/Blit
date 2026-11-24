# cr-ecm34-3: the copy fallbacks resolved names again

**Severity**: HIGH — a copy can read or write another file than the one it opened and checked. `mmap_copy_file` opened its source and destination as regular files (cr-ecm33-6) and then, when its Linux fast paths fell short, finished with `std::fs::copy(src, dst)`. That resolves both names again, so a changed name selected another file, and a FIFO there could stall the copy. On macOS the path copier reopens the source and opens its destination for `fcopyfile` waiting: that half is cr-ecm33-4's.
**Status**: Reviewed, found incomplete — `combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true`: "The retained Linux copy handle, contract-v30 disposition flag, explicit display phase and separate unretried count are sound choices. The following gaps still affect what gets written, reported or retried." "A `0600` source finished as `0644` on an existing destination. In a second probe, an eight-byte source shrank to three bytes. The copy returned an error and left an eight-byte destination ending in five zeroes. Apply permissions through the handle and remove or mark the partial on failure." The remaining work is cr-ecm35-2
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `71363fa9`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r35` over `0f912b06..dc14afc4` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r35.codex.md`. It completes cr-ecm33-6 (`0c98acb3`) in part.

## Evidence
"`mmap_copy_file` falls back to `std::fs::copy(src, dst)`, which resolves both names again. The macOS path copier also reopens the source and uses a blocking destination open for `fcopyfile`. A changed name can select a different file, and a FIFO can stall the copy. Use descriptor-based fallbacks and derive the length from the opened source."

## Intake
Admitted. The Linux half is fixed here. The macOS half is the same code as cr-ecm33-4: macOS-only, with no Mac reachable.

## What
On Linux, when `copy_file_range` and `sendfile` fall short, `mmap_copy_file` copies the rest through the handles it opened and checked, from where the fast paths left both. It never uses the names again. The length is the opened source's (`metadata` of the handle), as before. A source that yields fewer bytes than that fails the copy as one that changed size, instead of leaving a hole.

Elsewhere on Unix (macOS) the fallback is unchanged: `std::fs::copy` by name, blocked with cr-ecm33-4.

## Guard proof
- `file_copy::mmap::tests::the_fallback_copies_what_was_opened_not_what_the_name_holds` (Linux): with the fast paths refused through a test seam, a file renamed over the source's name before the fallback is not what is copied. The opened source's bytes land.
- Mutation, restored byte for byte: the fallback copies by name → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,083 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/copy/file_copy/mmap.rs` — the fallback; a test seam; guard.

## Known gaps
- macOS: the `mmap_copy_file` fallback and the path copier's `fcopyfile` still go by name (cr-ecm33-4, blocked: no Mac).
