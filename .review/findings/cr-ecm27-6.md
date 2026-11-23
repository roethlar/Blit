# cr-ecm27-6: on macOS a volume with no second names fails a resume where nothing stood

**Severity**: MEDIUM — where nothing stood at the name, a resume on a macOS volume that refuses hard links fails, though the volume may support `renameatx_np(RENAME_EXCL)`
**Status**: Verified on Unix — `combined-r32` (Codex openreview, 2026-11-23) reviewed `93ecf7b2..714ed0b7` — the `combined-r31` round, the in-place rework of D-2026-11-23-1 among it — with `guard_confirmed: true`: "The Unix implementation removes the staging directory and the dependency on atomic exchange. Creating the partial directly also preserves the destination directory's group inheritance. I would keep that design and the shared nonblocking checksum opener." (macOS by cross-clippy only, the reviewer's own and the agent's; never run on a Mac)
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r28` over `e025a3e2..10145127` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r28.codex.md`. It is the macOS half of cr-ecm26-1 (`7615d648`) and cr-ecm27-1.

## Evidence
"An absent-name resume fails on a macOS volume that refuses hard links even when the volume supports `renameatx_np(RENAME_EXCL)`. Use that operation and cover both vacant and occupied destinations." (`crates/blit-core/src/copy/file_copy/resume.rs`, `Resolved::publish_exclusive`: off Linux, a second name, and failing where there is none; `scripts/bench_ldt4_rigw.sh` already uses `renameatx_np`.)

## Restated
`combined-r29` (Codex openreview, 2026-11-22) reviewed `10145127..e3ea9517` — the five `combined-r28` fixes — with `guard_confirmed: true` restated it: "A vacant-name resume fails on a macOS volume that refuses hard links even when it supports `renameatx_np(RENAME_EXCL)`. Try that operation first and test both vacant and occupied destinations on such a volume." It stays blocked.

## Resolved
The resume's fresh partial is made in place now (cr-ecm30-8, D-2026-11-23-1): where nothing stood, it is made at the name by a creation that fails if the name is taken (`O_CREAT|O_EXCL`) — no rename that will not replace, no second name — so a volume without hard links takes it as any other. Nothing below is needed any more. macOS cross-clippy (`aarch64-apple-darwin`, `CARGO_FEATURE_PURE=1`) is clean; no Mac ran it.

## What it needed (superseded)
- In `Resolved::publish_exclusive`, on Apple targets, try `libc::renameatx_np(held, staged, held, name, libc::RENAME_EXCL)` first, as Linux tries `renameat2(RENAME_NOREPLACE)`. Treat `ENOTSUP`/`EINVAL` as "no such rename", which falls to the second name. Treat `EEXIST` as the file failing, with what stands there left as it is.
- Guards that run on macOS: a vacant name takes the partial, and an occupied one is left as it is and the resume fails. A volume that refuses hard links (an exFAT or FAT32 disk image) should take the partial through the exclusive rename.
- Check the target type-checks (`cargo clippy --target x86_64-apple-darwin` / `aarch64-apple-darwin`) and run the suite on a Mac.

## Known gaps
- Until then, off Linux, a filesystem with no second names fails a resume where nothing stood — safely: nothing is replaced.
