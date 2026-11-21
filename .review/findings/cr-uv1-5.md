# cr-uv1-5: `rm` listed at most 1,000 failures an operand, all in one reply

**Severity**: MEDIUM — entries that were not removed went unnamed, and a reply could outgrow what a message may hold after the removal had happened
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv1-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r1` over `b55efff7..49350572` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r1.codex.md`.

## Evidence
With 1,001 unremovable files, GNU named all 1,001. Blit returned 1,000 names and a truncation flag. Its cap also restarts for each operand, while all operands share one reply that can exceed the transport limit after deletion has occurred. Send indexed failures as they happen and finish with the removal count.

Cause. Slice 2 collected an operand's failures in memory, up to `MAX_LISTED_REMOVAL_FAILURES`, and the daemon put every operand's list in the one `RmReply`.

## Predicted observable failure
A tree with more than 1,000 entries that cannot be removed: the rest are not named, only "more entries could not be removed than are listed". Many operands, each with many failures and long paths: a reply larger than the message limit, refused by the transport — the removal done and nothing reported.

## Intake
Admitted as the reviewer put it.

## What
- **`Rm` answers with a stream — wire contract 17.** `RmReply` is `failures` (a batch of `RmFailure`, each with its operand's place) or, last, `done` with the count of entries removed. A batch is sent when what it holds reaches 64 KiB as it will be on the wire, or has waited 50 ms; a message is at most that and one failure more.
- **Every failure is told, as it is met** (`deletion::remove_as_rm_reporting`): the removal hands each to its caller at once and holds none. There is no cap and no flag. `remove_as_rm` remains for a caller that wants the answer whole.
- **A listener that has gone ends the removal where it is**: nothing more is removed for nobody.
- **An answer is complete only when it says so.** `admin::rm::remove` returns the count only on `done`; an answer that ends, or fails, before it is `RmError::Interrupted`, and one refused before anything was read is `RmError::Refused`. The CLI prints each failure when it arrives, in the order the operands were typed; for an interrupted answer it prints what was reported, then `HOST: the removal did not finish: …`, and exits 1.
- `--json` gives `entries_removed`, `failures` (every one) and `unfinished` (the daemons whose answer stopped early); `failures_truncated` is gone, there being nothing truncated.
- `blit move`'s source removal names the first entry left behind and counts the rest, holding only that.

Speed, the slice-2 bench again (200,000 empty files in 2,000 directories, median of 5, release, loopback): the system `rm -rf` 244.2 ms; the base build's `blit rm --yes` 245.9 ms; this tree's `blit rm -rf` 249.2 ms (255.5 ms when slice 2 landed).

## Guard proof
- `deletion::rm_tests::every_entry_that_cannot_be_removed_is_told` (Unix): 1,001 files in a directory that cannot be written — each told once, the count of what went exact; a listener that answers "gone" at the third hears no more and the removal ends as cancelled; and the whole answer holds 1,001.
- `blit-daemon` `rm_tests::a_removal_is_answered_in_bounded_messages_that_end_with_done`: what `stream_removal` sends for 5,000 operands that are not there and one that is — more than one message, none over the bound by more than an entry, every failure in order, and `done` once and last with the count.
- `rm.rs::rm_reports_every_entry_it_cannot_remove` (Linux), through the binary against the system `rm`: 1,001 lines, the same lines, the same exit status, the same tree left; `--json` lists 1,001.
- `remote_remote.rs::rm_says_so_when_the_answer_stops_before_it_is_done`: a stand-in daemon that reports one entry and then ends its answer, and one that then fails — the entry is printed, the command says the removal did not finish, exit 1; `--json` has one failure and one `unfinished`.
- `rm.rs::a_daemon_refuses_rm_from_another_contract` reads the answer as a stream and requires `done`.
- Mutations, each restored byte for byte: a message holding 1,000 failures and dropping the rest → the second and third guards red; a message not bounded by its size → the second; the daemon never saying it is done → the second and every end-to-end `rm` guard; an answer that ends without `done` taken as complete → the fourth; a listener that has gone not noticed → the first.

## Files changed
- `crates/blit-core/proto/blit.proto` (`Rm` streams; `RmReply`, `RmFailures`, `RmDone`), `transfer_session/mod.rs` (`CONTRACT_VERSION = 17`), `deletion.rs` (`remove_as_rm_reporting`, `Report`; the cap removed), `admin/rm.rs` (`remove`, `RmError`, `delete_remote_path`).
- `crates/blit-daemon/src/service/admin.rs` (`stream_removal`, `remove_each`, `FailureBatch`), `core.rs`, `mod.rs`.
- `crates/blit-cli/src/rm.rs` (`Said`); tests `rm.rs`, `remote_remote.rs`, and the stand-in daemons in `jobs_lifecycle.rs` and `small_frames.rs`.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- `--json` still builds one document, so it holds every failure in memory until the end; the plain output holds none.
- A removal whose asker goes away stops only when it next has a failure to report or a message to send; one with nothing to report runs to its end.
- With operands on several daemons, failures are printed daemon by daemon in the order each was first named, so an operand refused by `rm` itself can be printed after a later operand of an earlier daemon.
- On Windows a tree's removal reports one failure (cr-uv1-2), so the stream there carries at most one for a directory operand.
- Run on Linux only.
