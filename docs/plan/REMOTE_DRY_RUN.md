# `--dry-run` and `check` against a daemon

**Status**: Draft (r1, 2026-11-20 — drafted from the owner's ruling of that day and the evidence gathered for it; no owner interview yet, every design choice below is the drafting agent's recommendation)
**Created**: 2026-11-20
**Supersedes**: nothing
**Decision ref**: pending (activation needs a `docs/DECISIONS.md` entry)

## Goal

Before a transfer to or from a daemon, a user can ask what it would do and get an answer without anything being changed; and after one, a user can ask whether the two trees are the same without mounting the server's disk — each machine reads its own files, and only the answer crosses the network.

## Owner requirements (verbatim)

- 2026-11-20, on the utility verbs: "the point to all of the utility commands that blit supports is to avoid the speed issues with running checks over nfs/smb. […] they need to behave like the local versions, just run directly from the daemon."
- 2026-11-20, asked what `check` should do: "I don't know what check is."
- 2026-11-20, `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` Q3, owner "a": comparing a local tree with a daemon's tree is not in that plan; making `--dry-run` work for a transfer to or from a daemon gets a plan of its own, which also decides what `check` is for (D-2026-11-20-2).

## What exists today (measured 2026-11-20 on Linux host `continuum`, blit 1.0.0 built at `a1fd6bae`, unless marked)

One pair of trees: `same.txt` in both, `new.txt` in the source only, `sub/changed.txt` differing in size, `extra.txt` in the destination only.

- **Local `mirror --dry-run` counts and names nothing.** It printed `Mirror (dry run) complete`, `Copied: 2 file(s), 0 B`, `Deleted: 1 file(s), 0 dir(s)`, exit 0, and changed nothing. `--verbose` added planner totals (`2 file(s), 17 B`), still no name. The byte count on the `Copied` line is 0 in a dry run.
- **Local `check` names everything.** `Differing (1): sub/changed.txt (size (14 B vs 3 B))`, `Missing on destination (1): new.txt`, `Missing on source (1): extra.txt`, exit 1 (0 identical, 2 errors). `--checksum` compares content; `--one-way` ignores destination-only entries; it takes the transfer verbs' filter options.
- **Neither reaches a daemon.** `mirror --dry-run` to a daemon: "--dry-run is not supported for remote transfers" (`blit_core::endpoints`, refused before any connection, either direction). `check` given `HOST:/module/path/`: "destination path does not exist" — it treats the endpoint as a local path (measured in `UTILITY_VERBS_LOCAL_PARITY.md`, §What exists).
- **So verifying a copy that lives on a server means mounting the share and reading it over the network** — the cost the owner's requirement names.

## Prior art (measured 2026-11-20 on `continuum`)

- **rsync 3.5.2 (`41bbd895`), a loopback rsync daemon, the same four files.** `rsync -an --delete --itemize-changes src/ rsync://HOST/mod/` printed one line per entry it would touch — `*deleting extra.txt`, `<f+++++++++ new.txt`, `<f.s....... sub/changed.txt` — exit 0, and the daemon's tree was byte-identical afterwards. The same from the daemon to a local directory (`>f+++++++++ …`), nothing written locally. With `-c` both ends hash their own files; `--stats` adds counts. rsync has no separate compare verb: a dry run IS its compare, and its exit code does not say whether anything differs.
- **robocopy** (its reference; not measured for this draft): `/L` lists what a run would copy, skip and delete without doing it, against a share as against a local path. It also has no compare verb.
- **rclone** (recalled, not measured for this draft): `--dry-run` on every transfer verb, AND a separate `rclone check` that compares two remotes by size and hash and exits non-zero on a difference.

Two shapes, then: one tool where the dry run is the comparison (rsync, robocopy), and one with both (rclone). blit already ships both verbs locally.

## Non-goals

- A general `diff -r`: `check` stays a statement about a transfer's two trees (its help says so today).
- Changing what a real transfer does. A dry run runs the same comparison and stops before the first write.
- Showing file contents or per-byte differences.

## Constraints

- **SIMPLE**: no new verb and no new option — `--dry-run` and `check` exist; this makes them work where they are refused. What each is FOR must be sayable in one sentence each (Q1).
- **FAST**: each machine reads its own disk. Nothing but names, sizes, times and (with `--checksum`) hashes crosses the network; no file content does.
- **RELIABLE**: a dry run can never write, create a directory or delete on either machine — enforced on the machine that owns the tree, not by the asker's good behaviour. What a dry run lists is what the real run would do, computed by the same code.
- **Same build only**: a new field at session open, a contract bump.
- **Platform truth**: guards run against a loopback daemon on every OS.

## Acceptance criteria

- [ ] To be written once Q1 to Q5 are ruled.

## Design (sketch, pending the rulings)

- The transfer session already has the two halves a comparison needs: the SOURCE sends its manifest, the DESTINATION answers with what it needs and, on a mirror, works out what it would delete. A dry run is that session opened with a `dry_run` mark: the destination computes the need list and the deletion list, writes nothing, and the session closes with a summary instead of a payload phase.
- Today's local dry run goes the full length of the session with a sink that discards writes. Stopping after the comparison instead would serve both routes with one rule, and is the more honest "nothing is written".
- The destination refuses to write under a dry-run open whatever the peer sends next.
- Names: the summary frame carries counts today. Listing names means the destination (which knows the needs and the deletions) reports them, bounded per frame, as the per-file failure list already is.
- `check HOST:/module/path/` (either side, or both, on a daemon) is the same session in dry-run form, presented as `check` presents its local result today — differing, missing on destination, missing on source — with `check`'s exit codes.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop.

1. To be cut after the rulings: the dry-run open and the destination's refusal to write; names in the result; the CLI on push and pull; `check` with a remote endpoint; words.

## Open questions

- **Q1 — what each verb is for.** (a) Both stay, each with one job: `--dry-run` answers "what would this command do?" and lists it; `check` answers "are these two trees the same?", lists the differences, and says so in its exit code [recommended: both exist, scripts use `check`'s exit code, and one engine serves both]; (b) `check` is removed and `--dry-run` is the comparison, as in rsync and robocopy — one concept fewer, but no exit code that says "different", and a verb leaves the 1.0 surface; (c) `check` stays local-only and only `--dry-run` learns the daemon. — owner
- **Q2 — does a dry run name files?** Today it counts. (a) It lists each entry it would copy, update or delete, one per line, as `check` and rsync do, with the counts after [recommended: a count of deletions is not enough to agree to a mirror]; (b) counts only, names with `--verbose`; (c) counts only, and `check` is the way to see names. — owner
- **Q3 — remote to remote.** A transfer between two daemons is delegated: the destination daemon pulls from the source daemon and the asking machine sees progress only. (a) `--dry-run` and `check` work there too, in the same slice set [recommended, unmeasured: the delegated route already relays a summary]; (b) local-to-daemon and daemon-to-local first, daemon-to-daemon refused by name until a later plan. — owner
- **Q4 — `check --checksum` on a daemon that refuses to hash.** A daemon can be started with server-side checksums off; a transfer with `--checksum` is refused at open there today. Recommended: `check --checksum` is refused the same way, in the same words. — owner
- **Q5 — before or after the 1.0 tag.** It changes nothing that works today and adds a contract bump. `docs/plan/UTILITY_VERBS_LOCAL_PARITY.md` is ruled "all before the tag"; this plan was split from it without that ruling. Recommended: before, because "verify the copy on the server" is the first thing a migration user does, and today the answer is "mount it". — owner
