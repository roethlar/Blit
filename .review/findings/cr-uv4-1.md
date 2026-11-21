# cr-uv4-1: a move's removal could take a file that is not the one that landed

**Severity**: HIGH — a file rewritten at the source within the second it was sent in is removed, and it was never copied
**Status**: Reviewed, found incomplete — `combined-r16` (Codex openreview, 2026-11-21) reviewed `35adeacb..13faafd2` — utilities slices 5 and 6 and the eleven `combined-r15` fixes — with `guard_confirmed: true`. Four things. The account misses what was asked for before the manifest's end: "I moved 2,048 ordinary, unchanged files. All 2,048 arrived with the correct contents, but 384 remained at the source and the command exited 1. Incoming requests consume entries from the same map that cleanup later snapshots." (older than this fix: cr-uv3-1's; the working agent met it the same hour, measuring a move of 20,000 files). The version is read apart from the bytes: "On the `move --checksum` path, a same-size edit in the same timestamp second can occur after hashing and before the receipt's separate stat … Windows also records zero for file identity and change time." A file with two names is refused at its second: "Removing the first name changed the inode's ctime, so the move rejected the second name as changed." And a failed name is struck twice over: "The receipt subtracts both the raw name and its display spelling." The remaining work is cr-uv5-1, -2, -3 and -4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `c4241da0`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r15` over `b8273ac1..35adeacb` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r15.codex.md`. It completes cr-uv3-1 (`57e8ee24`), whose design the review keeps ("source-issued move tickets").

## Evidence
"A receipt accepted a replacement with different contents, the same length, and a modification time 800 ms later in the same second. Cleanup would delete it. Tickets are built from manifest entries before delivery is confirmed. Bind them to the source root and acknowledged file version, then revalidate before removal."

Cause, in the three things the reviewer names:
- **The version.** An account held a file by its path, its size and its modification time in whole seconds — what the manifest writes. Anything of that size written in that second was "the file that was sent".
- **Before delivery is confirmed.** The account was complete, and answerable, from the moment the manifest was: before a byte of any file had gone, and whatever became of the session afterwards. That only a move which ended without failures presents its tickets was the asker's rule, kept by the asker (`refuse_source_delete_on_failures`); the daemon's account knew nothing of it.
- **The source root.** An account's paths are relative to what its session read, and nothing in it said what that was: presented with another path, it answered for that path's files of the same names.

## Intake
Admitted as the reviewer put it.

## What
- **A file is held as the file it was** (`blit_core::moved::Version`): its size, the time it was last written to the nanosecond, and on Unix the time it was last changed in any way and which file it is (device and inode). The change time is the strong one — no program sets it, and every write, rename-over and attribute change moves it. The removal reads the entry it is about to remove the same way, through the directory it holds open (`Met::version`), and takes it only if an account holds exactly that.
- **The account is written from this end's own disk**, when the manifest is complete (`account_of`): one look at each file on the manifest, off the runtime's threads, and a file goes in only while it is still the regular file the manifest wrote — of that size and that second. One written to since the walk, one that has gone, a link in its place: not in the account, so not removed.
- **The account is of one place**: the path its session read (`Account::of`, `Closed::is_of`). A removal's operand that is no account's place is refused whole — `the account of what was sent is of another path` — and nothing below it is looked at.
- **The account answers only once its session has ended well.** It is kept open under its ticket (`moved::keep`) and answers nothing; when the destination's summary reaches the source — the one confirmation a source ever gets — every path that summary says failed is struck from it, by text and by the bytes of a name that is not text, and it is confirmed (`moved::confirm`). A summary whose list of failed paths is not whole confirms nothing: the account is dropped (`moved::forget`). A session that faults, is cancelled or ends any other way leaves an account no removal can use, and the removal is refused with the source whole.
- The removal for a move waits up to five seconds for its accounts when every ticket is known and a session has not ended yet: the session's last word and the removal travel on two streams, one after the other, and may land either way round. A ticket nothing answers to is refused at once.
- **Wire contract 29.** No message changed; what the ticket stands for did. A daemon of contract 28 would let a removal take a file rewritten within its second.

## Guard proof
- `moved::tests::a_file_rewritten_within_the_second_is_not_the_file_that_was_sent`: the reviewer's case — the same length, written 800 ms later in the same second — and each part of a version by itself: the write time alone, the change time alone (a time put back), another file of the same size and times, another device.
- `moved::tests::an_account_answers_only_once_its_session_has_ended_well`: nothing is answered before the session's end; then what the receiving end said did not land is not held (a name that is not text by its bytes, and its text is another path); a second word changes nothing; a forgotten account is never answered; and which tickets are worth waiting for.
- `moved::tests::an_account_is_of_the_place_its_session_read`, `a_version_is_read_from_the_file_whole` (a real file rewritten with as many bytes 800 ms later in the second; its time put back, the change time has moved).
- `transfer_session::…::a_moves_account_holds_each_file_as_it_stands_on_this_disk`: of seven files on a manifest, the one grown since, the one touched, the one gone and the one with a link in its place are not in the account; the three unchanged are, the name that is not text by its bytes.
- `transfer_session::…::what_did_not_land_is_the_summarys_whole_list_or_nothing`.
- `service::admin::rm_tests::a_moves_removal_takes_what_its_sessions_sent` (the daemon), extended: `same.txt`, sent and then rewritten with as many bytes 800 ms later in the same second, stays and is named; an account of `one.txt` lets nothing of `two.txt` go though `two.txt` is the very same file under another name (a hard link: the same bytes, times and identity).
- `crates/blit-cli/tests/remote_move.rs` (ten, unchanged) — a move from a daemon still removes what it sent: the account is confirmed by the session's end before the removal asks.
- Mutations, each restored byte for byte — eleven, each red on the guards named for it: a file held by its whole second; the change time not held; which file it is not held; an account answering as soon as it is kept; what did not land left in the account; an account of any place; a summary whose list is not whole confirming; the session's end confirming nothing (the end-to-end move red: its removal is refused); a file put in the account unseen; the removal asking an account of another path; the removal reading a file's time in whole seconds.

## Files changed
- `crates/blit-core/src/moved.rs` — `Version`; `Account::of`; open and confirmed accounts, `confirm`, `forget`, `pending`; guards.
- `crates/blit-core/src/deletion.rs` — `Met::version`, read whole on Unix and elsewhere.
- `crates/blit-core/src/transfer_session/mod.rs` — `account_of`, `not_landed`; the account written off the runtime's threads and confirmed at the summary; contract 29; guards.
- `crates/blit-daemon/src/service/admin.rs` — the removal asks the accounts of its operand's place, by version; guard.
- `crates/blit-daemon/src/service/core.rs` — the wait for a session's end.
- `crates/blit-cli/tests/rm.rs` — the refusal's words.
- `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`, `docs/TRANSFER_SESSION.md` — contract 29; what a move's removal takes.

## Known gaps
- **A file rewritten between the walk and the manifest's end, with as many bytes and in the same second**, goes into the account as it then stands. If the destination asked for it, that is the content it was sent; if the destination already held a file of that size and second and asked for nothing, the two differ and the source's is removed. The transfer's own rule calls those two the same file (size and whole-second time, unless `--checksum`), and the manifest carries no finer time to close it with.
- One more look at every file of a move's source (an `lstat` each, when the manifest is complete). Not measured.
- The wait for a session's end is five seconds and is not driven by a test: no test here makes the removal arrive first. A removal that outruns its session by more is refused, with the source whole, and may be asked again.
- The destination's word is what is struck: a file the destination reports as landed and does not hold is removed all the same. The account cannot see the destination's disk.
- On Windows a version is its size and its write time; there is no change time or file identity in what the standard library gives, and nothing ran there.
- A move whose source is on THIS machine still removes the source's whole tree (`fs::remove_dir_all`), link included — the open owner question beside cr-uv3-1.
- Accounts live in the daemon's memory: 32 bytes a file while a session is open, 16 once it has ended. The bound on them is cr-uv4-5.
