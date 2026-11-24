# cr-ecm36-1: the receivers opened without the destination's root, and the opener's opens went by the name

**Severity**: HIGH — a file can be made outside the destination. Two gaps after cr-ecm35-1:
- **No root.** The streamed-record receiver and the tar-member writer called the opener with no root. A dangling link at the name could lead their creation outside the destination.
- **Opens by name.** Even with a root, the opener's two ordinary opens went by the whole path. A directory on the way replaced by a link out, after the containment check judged it, could lead them out.
**Status**: Reviewed, found incomplete — `combined-r38` (Codex openreview, 2026-11-24) reviewed `2c55cb3a..6ee7ed5e` — the `combined-r37` fixes — with `guard_confirmed: true`: "Passing the root into the stream and tar writers, marking failed mmap copies through the open file handle, and preserving uncertainty after a source-only failure are sound changes." "The thread-local root descriptor is reused by pathname. If a daemon processes another job after a destination directory is rotated and recreated, a reused worker can write into the retired directory. Hold the root per session, validate cached directories by identity, and use held directories for parent creation and fallback opens." "The base CLI copied through an absolute symlink to a file inside the destination and exited 0. The head reported that the link led outside, exited 2 and left the old bytes. The equivalent relative link worked. Resolve permitted targets against the held root before opening them." The remaining work is cr-ecm37-1 and -4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `d5378b09`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-1 (`be844676`) in part.

## Evidence
"The streamed and tar receivers still call the rootless opener. By inspection, a dangling destination link can therefore cause creation outside the destination. Even the scoped opener uses the original pathname for its two ordinary opens. Pass the root through every receiver and open relative to the validated, held directory."

## Intake
Admitted as the reviewer put it.

## What
- **Every receiver names its root.** `create_to_write` takes the destination's canonical root, and every receiver passes it: the streamed record, the tar member (`write_shard_member`) and the local copy's exclusive branch. A link at the name to nothing then has its file made only within the destination (cr-ecm35-1).
- **Both opens beneath the root (Linux).** The opener's two opens (`copy::open_to_write`) are made beneath the root with `openat2` and `RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS`, from the root held (`O_PATH`):
  - the exclusive creation;
  - the open of what has the name.

  The kernel refuses any `..` or link on the way that leads out, and a refusal fails the file ("leads out of the destination"). This is the held, validated directory the reviewer asked for, in one call per open.
- **Fallbacks.**
  - Where `openat2` is missing (a kernel before 5.6, or a filter that refuses it), and off Linux, the opens go by the name as before.
  - A destination that is one file has no directory to hold, and is opened by the name.
- **The path beneath the root.**
  - A destination written beneath the root by its own name is passed as it is: the kernel does the judging.
  - Otherwise the directory's place beneath the root is found once and remembered: a few directories per thread, the root held once per thread. A stale entry only names, beneath the root, where the directory really is.

## Guard proof
- `file_copy::mode_tests::a_directory_on_the_way_that_leads_out_makes_nothing_outside` (Linux): with `dst/sub` a link to `../outside`, neither the opener nor `open_to_write` makes `outside/f.bin` or `outside/g.bin`.
- `sink::tests::the_receivers_make_nothing_outside_the_destination` (Unix): a streamed record and a tar member, their names dangling links to `../outside/…`, fail, and nothing is made outside.
- Mutations, each restored byte for byte:
  - the opens go by the name → red;
  - the stream receiver has no root → red;
  - a tar member has no root → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,097 tests over 57 binaries.

## The cost, measured for FAST
1,000 files of 160 KiB, local `copy`, against `7cc15338`, in alternating order:

| Case | Before | After |
|---|---|---|
| tmpfs, empty destination (45 runs each) | mean 37.4, median 37 ms | mean 38.3, median 38 ms |
| tmpfs, half-filled files (30 runs each) | mean 36.0 ms | mean 37.0 ms |
| Btrfs, half-filled (30 runs each) | mean 66.9 ms | mean 65.4 ms |

On tmpfs that is about one microsecond per file, about 2.5%. On Btrfs it is within noise. The cost is the `openat2` call itself; the per-file `canonicalize` of a first version cost more and was removed. `copy --resume` is unchanged.

## Files changed
- `crates/blit-core/src/copy/file_copy/mod.rs` — `open_to_write`, `open_beneath`, `place_beneath`, `held_root`; guard.
- `crates/blit-core/src/copy/mod.rs` — exports.
- `crates/blit-core/src/remote/transfer/sink.rs` — `create_to_write` and `write_shard_member` take the root; the receivers pass it; guard.

## Known gaps
- Off Linux, and on kernels without `openat2`, the opens still go by the name. Containment there is the check before the open, as everywhere else.
- The cost above is on the owner's FAST line: about 2.5% on tmpfs for 1,000 small files, none measurable on Btrfs.
