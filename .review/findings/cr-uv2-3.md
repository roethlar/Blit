# cr-uv2-3: A removal went on for an asker that had gone, and a lost request was called a refusal

**Severity**: HIGH — `rm` stopped with Ctrl-C keeps deleting on the daemon; and "none of these was removed" is said of a request whose outcome is not known
**Status**: Fixed, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-uv2-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r2` over `4841b2d4..3cd49e40` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r2.codex.md`. It completes cr-uv1-5.

## Evidence
"In a focused test I closed the receiver before the worker started. The worker deleted the queued file and discovered the closure when it tried to send `done`. Check cancellation before each operand and during successful traversal. On the client, a transport failure before response headers also leaves the outcome unknown. Report it as unfinished and reserve `Refused` for a confirmed rejection."

Cause.
- **The daemon learned that nobody was listening only by failing to send.** cr-uv1-5 made the answer a stream and ended the removal when a message could not be sent. A removal that meets no failure has nothing to send until it is done — so it ran to the end, every operand removed, and found the stream closed at `done`. The removal itself has a way to be called off; the daemon passed it nothing.
- **Every end of the request without an answer was a refusal.** The asker mapped any status in place of the answer to `Refused` — "none of the operands went" — and the front end printed `cannot remove 'X'` for each. A status the daemon answers with before it acts (a read-only module, another build, a path it does not allow) is that. A connection lost after the request went out is not: the daemon may have removed everything.

## Predicted observable failure
`blit rm -r HOST:/module/big-tree`, then Ctrl-C because the wrong tree was named: the client exits, the daemon removes the rest. And over a link that drops as the request is sent: `blit rm: cannot remove 'HOST:/module/tree': …unavailable…` for a tree that is gone.

## Intake
Admitted as the reviewer put it.

## What
- **A removal asks whether it has been called off before each operand is touched and before each entry below one** (`deletion::remove_as_rm_reporting`'s `called_off`, in place of a flag that only tests passed). The daemon's answer is `sender.is_closed()`: an asker that closes its stream is one that stopped the command, and the removal ends there, as a cancelled one.
- **A refusal is an answer** (`admin::rm::refused_before_acting`): the request ended with what the daemon says to a request it will not serve — invalid argument, not found, permission denied, failed precondition, unimplemented, unauthenticated. Anything else in place of an answer — unavailable, a timeout, an error with no name — is `Interrupted`: `the removal did not finish: the request was sent and no answer came (…)`, exit 1, listed under `unfinished` in `--json`, and no operand is said to be still there. A daemon that could not be reached at all is still a refusal: nothing was sent.

Speed, `rm -rf` of 200,000 files in 2,000 directories, release, medians of five: the system's `rm` 247.5 ms, blit before 246.6 ms, after 247.6 ms. The question is one read of an atomic for each entry.

## Guard proof
- `blit-daemon` `rm_tests::a_removal_nobody_waits_for_removes_nothing`: the reviewer's case — the answer's receiver dropped before the removal begins, a file and a tree as operands: the request ends cancelled, and both are whole.
- `rm_tests::a_removal_stops_at_the_operand_where_its_asker_went`: three files, the asker gone after the first: the first removed, the others not.
- `deletion::rm_tests::a_removal_called_off_stops_where_it_is`: called off before anything, a file and a tree are untouched; called off at the twentieth question, a directory of 100 files is left with 82 (Unix); never called off, it is removed.
- `admin::rm::tests::only_an_answer_is_a_refusal`: the six codes that are, nine that are not.
- `remote_remote.rs::rm_does_not_call_a_request_that_got_no_answer_a_refusal`, through the binary against a daemon that ends the request `unavailable`: exit 1, `the removal did not finish … no answer came`, no `cannot remove`; `--json` has no failure and one `unfinished`.
- Mutations, each restored byte for byte: an operand begun without asking → the three removal guards red; a removal that goes well not asked between entries → the `deletion` guard; the daemon not asking whether the answer is still read → the first daemon guard; the daemon not handing the question to the removal → both daemon guards; every end without an answer a refusal → the end-to-end guard; a lost connection counted as a refusal → the unit and end-to-end guards.

## Files changed
- `crates/blit-core/src/deletion.rs` — `remove_as_rm_reporting(…, called_off, …)`, both `remove_operand`s; guard.
- `crates/blit-daemon/src/service/admin.rs` — `stream_removal`, `remove_each`; two guards.
- `crates/blit-core/src/admin/rm.rs` — `refused_before_acting`, `RmError::Interrupted` for a request with no answer; guard.
- `crates/blit-cli/tests/remote_remote.rs` — the fake daemon's `no-answer` module; guard.
- `CHANGELOG.md`.

## Known gaps
- **What a called-off removal had already removed stays removed**, and the asker that went is not told how much: there is nobody to tell.
- **The entry being removed when the asker goes is still removed**; the question is asked before each, not during.
- **On Windows a tree is the standard library's to remove, whole**: called off, the removal stops before the operand or not at all (cr-uv2-2).
- The asker's own Ctrl-C closes the stream by ending the process; how soon the daemon sees the stream closed is the transport's — on loopback, at once. Not measured over a slow link.
- A proxy that answers for the daemon (403, 404) is read as the daemon's refusal.
- Run on Linux only.
