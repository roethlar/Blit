# gate-put-race: the `--ignore-existing` guard's source marked its files put before it had put them

**Severity**: MEDIUM — a guard fails under load, now and then; the code is right.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix gate-put-race`)
**Reviewer**: none — found by this session's own gate, 2026-11-25; for the next openreview.

## Evidence
The gate for ci-windows-df-words, a change to an unrelated test, failed on its Btrfs pass. The failing guard was `service::transfer_session_e2e::a_push_that_keeps_what_exists_writes_over_nothing_put_there_since` (the daemon):
- "big.bin: the file put there is whole";
- the push reported `small.txt` kept, as it should be, and `big.bin` sent.

Alone, the guard passed 20 of 20 on Btrfs and on tmpfs.

The guard's source, `PutThereBeforeItsBytes`, writes another hand's files at the destination at its first `prepare_payload` or `open_file`. That is meant to come before a byte is sent. It marked the put done (`AtomicBool::swap`) before writing them. Under load, a second worker's `open_file(big.bin)` found it marked, and went on and sent its bytes while the first was still writing. The destination made `big.bin` exclusively, the name being free then, and held it (cr-ecm14-2, cr-ecm15-1). The guard's own late write then landed on the file the run had made.

With a 50 ms pause between the mark and the writes, the guard failed 5 of 5.

## Intake
Admitted. The fault is the guard's: its premise, "put there before a byte is sent", was not held by the guard itself. The run does what cr-ecm15-1 made it do.

## What
`put` runs once, with every caller waiting for it (`std::sync::Once`). It marks the put done after the files are written.

## Guard proof
- With the same 50 ms pause inside the put, the guard passes 5 of 5; without it, 10 of 10.
- The old put with the pause failed 5 of 5: the gate's failure, made on demand.
- Checks: native clippy is clean, and the guard passes.

## Files changed
- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — `PutThereBeforeItsBytes`.

## Known gaps
- None known.
