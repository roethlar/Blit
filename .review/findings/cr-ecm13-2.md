# cr-ecm13-2: A retry decided again where its files go, and decided otherwise

**Severity**: HIGH — a retry writes the files that failed somewhere else, exits 0, and the place they failed at is left as it was
**Status**: Reviewed, found incomplete — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`. It keeps the "persisted retry destination"; a record from before the field is still replayed by reading the arguments again: "I replayed an old-format record for `copy src backup`, where the first run created `backup`. The retry wrote `backup/src/big`, left `backup/big` missing, and exited 0." The remaining work is cr-ecm14-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `bf4987de`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `exclude-case-r13` over `c99ad483..bd84649f` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/exclude-case-r13.codex.md`. Older than the plan (`blit jobs retry`, JOB_LOGS jl-4); found through cr-ecm12-2's guard.

## Evidence
"With `backup` initially absent and neither argument ending in `/`, `copy src backup` left `backup/big.bin` empty after a write failure. `jobs retry` wrote `backup/src/big.bin` and exited 0. The original partial was still empty."

Cause. Whether a source goes UNDER the destination by its name is decided when the command's two arguments are read (`resolve_destination`): a destination typed without a `/` is "put it in there" if a directory stands there and "this is its new name" if nothing does. A job keeps the two arguments as typed, and its retry reads them again — after the first run made the directory the answer turns on.

Reproduced here, both ways: a run that wrote `backup/…` retried into `backup/src/…`; and a run that wrote `backup/src/…`, `backup` removed since, retried into `backup/…`.

## Intake
Admitted as the reviewer put it.

## What
- A run's record keeps what it decided (`RunRecord.destination_nested`), noted where the command's route is chosen.
- `blit jobs retry` hands that answer to the command it runs, which then asks the filesystem nothing (`resolution::resolve_destination_as`). A copy's retry and a move's alike.
- A record from before this, and a run that ended before its arguments were read, hold no answer: such a run is retried as it was, by reading the arguments again.
- `blit jobs run NAME` is a new run and decides for itself, as it did.

## Guard proof
- `job_retry.rs::a_retry_writes_where_the_run_it_retries_wrote`: `copy src backup` with nothing there (one file failing for want of room, Unix) — the record says not nested, the retry lands the file in `backup/`, and there is no `backup/src`; and with `backup/src/` there and one file blocked — nested, `backup` removed, and the retry lands the file in `backup/src/`, with no `backup/blocked.txt`.
- `transfers::resolution::nesting_tests::where_a_source_goes_is_decided_once`: the same two arguments answered both ways as the directory comes and goes; resolved with an answer given, the filesystem is not asked.
- Mutations, each restored byte for byte: the retry asking the filesystem again; the run not keeping what it decided; the answer given not being the one used; a source going under the destination whatever was decided — four, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/transfers/resolution.rs` — `destination_nests`, `resolve_destination_as`; guard.
- `crates/blit-core/src/job_record.rs` — `RunRecord.destination_nested`.
- `crates/blit-cli/src/{cli,jobs,run_log}.rs`, `crates/blit-cli/src/transfers/{mod,remote_remote_direct}.rs`; `crates/blit-cli/tests/job_retry.rs`.
- `CHANGELOG.md`.

## Known gaps
- **A destination on a daemon was never asked**: without a `/` after it, it is "its new name" whether or not a directory stands there, in the first run and in every retry. That is as it was.
- A retry of a run recorded before this fix still reads the arguments again.
- The first half of the guard needs a write that fails where no directory can be put in the way; it runs on Unix only.
- Run on Linux only.
