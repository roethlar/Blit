# cr-ecm33-4: macOS's `fcopyfile` fallback opens its destination waiting

**Severity**: HIGH
**Status**: Admitted — NOT FIXED. Blocked: macOS-only code that nothing here can run, and no Mac is reachable.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06`; `.review/results/combined-r34.codex.md`.

## Evidence
"A FIFO arriving after the writable probe can still block `fcopyfile`'s destination open indefinitely. Keep the clone fast path for an absent destination and pass a validated, nonblocking descriptor to the fallback."

## What it needs (not done)
On macOS, the path copier (`copy/file_copy/clone.rs`) and `mmap_copy_file`'s fallback go by name: the source is opened again, and the destination opened for `fcopyfile` waits on a FIFO. They should take the descriptors the copy opened and checked, as Linux now does (cr-ecm32-2, cr-ecm34-3), and be proved on a Mac.

## Restated by `combined-r36`
`combined-r36` (Codex openreview, 2026-11-24) reviewed `dc14afc4..d2494965` — the `combined-r35` fixes and prose-live-phase — with `guard_confirmed: true` restated it (the macOS pathname fallback): "The macOS pathname fallback and Windows handling of an `Unjustified` resume baseline remain open." Still blocked.
