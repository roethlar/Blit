# cr-ecm25-2: a wire resume made a missing destination by its name, and learned which file it was after

**Severity**: HIGH — where nothing stood, a whole-file hold created the file at its name and only then learned its identity: a look that failed left a visible nine-byte file, and a folder replaced on the way could move where it was made
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm25-2`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r26` over `19025839..05ff50d7` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r26.codex.md`. It completes cr-ecm24-1 (`4f0978c2`).

## Evidence
"A whole-file grant creates the missing file by pathname and checks its identity afterward. An injected identity failure left a visible nine-byte file. A concurrent parent change can also redirect that creation. Prepare the file in the held directory and publish it with exclusive-create semantics."

## Intake
Admitted as the reviewer put it.

## What
- `hold_whole` resolves the destination (`resolve_destination`) and asks whether anything stands at the chain's end in the directory held (`Resolved::stands`). Either way the partial is staged there, private, at its length, its identity learned (`ready`), before it takes the name. Where something stands, it is renamed over it (`fresh`). Where nothing does, it takes the name only while nothing does (`fresh_exclusive`): a second name is made for it with `linkat`, which fails if the name is taken, and the staged name is let go. What came to stand there first is left as it is, and the file fails.
- On a filesystem with no second names (`linkat` refused with `EPERM` or `EOPNOTSUPP`), the staged file is renamed in only while a look just before finds nothing at the name.

## Guard proof
- `sink::tests::a_partial_where_nothing_stands_is_ready_before_it_takes_the_name` (Unix): with the look failing (the identity seam), nothing is left in the directory. With a file put at the name once the directory is held (the fresh-partial seam), that file is left as it is and the completion fails.
- Mutations, each restored byte for byte: the partial where nothing stands made by the name (red); the partial taking the name whatever stands there (red).
- The resume, sink and session tests (318) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `Resolved::fresh_exclusive`, `Resolved::stands`, `fresh_as`.
- `crates/blit-core/src/copy/mod.rs` — the fresh-partial seam to the crate's tests.
- `crates/blit-core/src/remote/transfer/sink.rs` — `hold_whole`; guard.

## Known gaps
- The fallback for a filesystem with no second names has no guard: no filesystem here refuses links. Its look and rename leave a window.
- Unix only.
