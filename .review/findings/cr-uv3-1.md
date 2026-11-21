# cr-uv3-1: The removal that ends a move took what the move had not carried

**Severity**: HIGH — a `move` from a daemon exits 0 with an entry gone from the source that was never copied
**Status**: Reviewed, found incomplete — `combined-r15` (Codex openreview, 2026-11-21) reviewed `b8273ac1..35adeacb` — the seven `utility-slices-r3` fixes, the seven `exclude-case-r13` fixes and utilities slice 4 — with `guard_confirmed: true`. It keeps "source-issued move tickets" and finds the account too loose: "A receipt accepted a replacement with different contents, the same length, and a modification time 800 ms later in the same second. Cleanup would delete it. Tickets are built from manifest entries before delivery is confirmed." And the bound on what a daemon keeps does not hold: "A single account larger than the advertised 1 GiB limit evicts every older account and is then inserted in full." The remaining work is cr-uv4-1 and cr-uv4-5
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `57e8ee24`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `utility-slices-r3` over `eb0cc4d8..dd0a3179` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/utility-slices-r3.codex.md`. The removal is older than the plan; that it ends WELL is cr-uv2-1's doing.

## Evidence
"I moved `incoming/.` containing a regular file and a symlink to it. At the base, cleanup failed and the source link survived. At the head, the move exited 0, removed the source tree, and left only the regular file at the destination. Cleanup issues a fresh recursive `rm`. Remove only entries accounted for at the destination, then remove empty directories. An untransferred entry must survive and make the move report incomplete."

Cause. A move from a daemon copies, and then asks the daemon to remove the source: a recursive removal of whatever stands there by then. Nothing ties it to the transfer. A link, which no transfer carries, goes with the files; so does a file that arrives after the source was read.

## Intake
Admitted as the reviewer put it.

## What
**The daemon keeps an account of what each session sent, and the removal takes that.**

- A session opened as one whose source will be removed (`source_removed_after`, cr-uv3-6), on the end that is read from when it is the responder — a daemon — keeps an ACCOUNT of every file it put on its manifest (`blit_core::moved`): path, size and modification time, as the manifest wrote them. With `ManifestComplete` it hands the other end a TICKET for it. The end that writes hands the ticket on in its summary; between two daemons, so does the daemon that pulled, in the summary it gives the CLI.
- The removal presents the tickets of ALL the move's sessions — the first pass and every retry (`RmRequest.sent_tickets`, **wire contract 24**). The daemon finds every account before it touches anything; a ticket it does not hold refuses the request, with nothing removed.
- With accounts, an entry that is no directory is removed only if it is a regular file one of them holds — this path, at this size and time. A directory goes when that leaves it empty. Everything else stays where it is and is told, in words for what it is: `it is a symbolic link, which a move does not carry`; `it is not a regular file, and a move carries only those`; `it is not a file the move sent: it is new, or has changed since it was read`. The command names the first (`cannot remove 'incoming/link': …`), does not report the source removed, and exits 1.
- The CLI asks for no removal at all without a ticket from every session.
- "Accounted for at the destination" is every entry of the manifests: the removal is not asked for unless no file failed, and a move sends every entry it lists.

An account is a sorted list of 128-bit digests — sixteen bytes a file, beside the manifest the session already holds whole — kept in the daemon's memory for a day, a gibibyte of them at most.

## Guard proof
- `remote_move.rs::a_move_removes_what_it_sent_and_leaves_what_it_did_not` (Unix), through the binary, to this machine (`incoming/.`, the reviewer's) and between two daemons (`incoming/`): exit 1, the link named with its reason; the files at the destination and gone from the source with the directories they left empty (one of them never held a file); the link where it was, and not at the destination. Then a source with nothing a move does not carry goes whole, exit 0.
- `blit-daemon` `…::a_moves_removal_takes_what_its_sessions_sent` (Unix): two accounts; files they hold removed, with three directories left empty and two that were empty; a file no account holds, one in a directory of its own, one five bytes longer than it was sent, a link and a pipe each stay with the words for it, and the directories holding them stay untold; a source that is one file is held under no name; and a plain removal takes all of it.
- `rm.rs::a_removal_for_a_move_is_refused_without_the_account_of_what_was_sent`, to the daemon's own interface: a ticket nothing answers to, a short one and an empty one, each beside one — `FailedPrecondition`, nothing removed.
- `moved::…::an_account_holds_what_was_sent_and_a_ticket_asks_for_it`; `admin::rm::…::a_moves_removal_asks_for_nothing_without_a_ticket`.
- Every earlier guard of a move from a daemon passes unchanged: a source that was sent whole still goes whole.
- Mutations, each restored byte for byte: the daemon keeping no account; the account holding nothing; the ticket not handed on by the end that writes, or by the daemon that pulled; the removal asked for with no ticket; the daemon removing as for `rm` whatever tickets it is shown; a ticket nothing answers to taken for none; a link going with what was sent; a file no account holds going; a file held whatever its size; a single-file source, or what a directory holds, removed unasked — twelve, each red on the guards named for it.

## Files changed
- `crates/blit-core/src/moved.rs` (new), `crates/blit-core/src/lib.rs`.
- `crates/blit-core/proto/blit.proto` — `ManifestComplete.sent_ticket`, `TransferSummary.source_ticket`, `DelegatedPullSummary.source_ticket`, `RmRequest.sent_tickets`.
- `crates/blit-core/src/transfer_session/mod.rs` (`CONTRACT_VERSION = 24`), `crates/blit-core/src/remote/transfer/delegated_summary.rs`.
- `crates/blit-core/src/deletion.rs` — `remove_what_may_go`, `Keep`, `Met`; `crates/blit-core/src/admin/rm.rs` — `delete_remote_path`, `remove`.
- `crates/blit-daemon/src/service/{core,admin}.rs`; `crates/blit-cli/src/transfers/mod.rs`, `crates/blit-cli/src/rm.rs`.
- Tests: `crates/blit-cli/tests/{remote_move,rm}.rs`; four literals in other tests. `README.md`, `CHANGELOG.md`, `docs/RELEASE_NOTES_1_0.md`.

## Known gaps
- **A move whose source is on THIS machine is unchanged**: it removes the source tree whole when the copy ends well, a link in it included. That is the same loss, in the CLI's own removal; it is in `docs/STATE.md` among the owner's questions on what a move does with a link, and this account can serve it.
- **A daemon restarted between the copy and the removal refuses the removal**; the source stays whole and the move fails after copying. So does a move whose removal comes more than a day after its first session.
- A file is known by path, size and time in WHOLE seconds: one rewritten to the same size within the second it was read in is taken for the one sent. And an entry is looked at and then removed, one system call apart.
- **No guard runs a move that needs a retry**: that every pass's ticket is presented is by reading.
- An empty directory goes though nothing of it was sent — the reviewer's rule ("then remove empty directories").
- **Nothing ran on Windows.** There the tree is removed entry by entry by path, each asked about first; the standard library's whole-tree removal, held to handles, is still what a plain `rm -r` uses.
- Run on Linux only.
