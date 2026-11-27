# `--files-from` lists from any tool

**Status**: Draft
**Created**: 2026-11-26
**Supersedes**: nothing. Once approved it answers two of STATE's open questions (a `--files-from` line that is not a plain relative path; a list saved with a byte-order mark) and, for the list alone, replaces the line rule of `EXCLUDE_CASE_MATCHING.md` Q5 "a" (Q5 below).
**Decision ref**: — (once approved)

## Owner's requirement (2026-11-26, verbatim)

- "well I need it to work, so plan for that. then review the plan with codex until consensus."
- "lists from whatever tool in whatever format cannot erroneously delete files or skip files while reporting success or this tool is dangerous."
- The owner's working example, in their usual shell (PowerShell): `(Get-ChildItem D:\ -recurse).FullName > filelist.txt`, then that file given to blit as the list.

## Goal

`--files-from` is an exact selection for `copy` (local, push, pull, daemon to daemon): a list made by any common tool, in any common text format, copies every entry it names and nothing it does not. No list, however it was made, can make blit delete a file, copy a different file from the one a line names, or end in success while an entry the list names did not land.

## Pending owner decisions (each with the plan's recommendation)

- **Q1 — a line naming a folder.** (a) The folder itself, and in it only what the list also names [recommended]: exact, and what `rsync -a --files-from` and `find | cpio -p` do (measured). `(Get-ChildItem -Recurse).FullName` and `find` name every entry below a folder, so they copy the whole tree either way; a filtered list (`find -newer …`, `Get-ChildItem -Recurse | Where-Object …`) names a folder and only some of what is in it, and recursing would copy what it left out. A hand-written list holding only `photos` then makes the folder, empty. (b) The folder and everything in it, as `tar -T` and `Copy-Item -Recurse` do: the selection grows beyond the list.
- **Q2 — `mirror` with a list.** (a) Refused, as `move` with a list already is: a list names what to copy, and no deletion is decided by a list [recommended]. (b) Allowed for folder lines only, each listed folder mirrored from the source's own content.
- **Q3 — a line the run can never satisfy** (outside the source, climbing above it, in a path syntax the source's machine does not have). (a) The whole run stops before it changes anything [recommended]: such a line means the list was made for another place, or read wrongly. (b) That line is reported and the rest copied.
- **Q4 — a Unix line starting with `/`.** (a) A full path, like every other full path: inside the source it names that entry, outside it is Q3's [recommended]; rsync's `--files-from=list /` idiom works, `/` being the source. (b) rsync's reading: relative to the source, whatever the source is.
- **Q5 — spaces at a line's edges, and a line starting with `#`.** (a) A line is the name exactly as written: nothing stripped, no comment lines [recommended]. Measured today: with `a.txt` and `a.txt ` both in the source, the line `a.txt ` copies `a.txt` — the other file — and exits 0; a file named `#notes`, listed by `ls`, `dir /b` or `Get-ChildItem -Name`, is skipped as a comment, exit 0. `tar -T` and `cpio` read both lines exactly; rsync reads the spaces exactly and skips the `#` line. This replaces, for the list only, the rule ruled in `EXCLUDE_CASE_MATCHING.md` Q5 "a" (both ends stripped, `#` lines ignored); the pattern files keep it. A hand-written list with a comment line then reports that line as not found, exit 2. (b) Keep the stripping and the comments: both measured faults stay.
- **Q6 — an entry no transfer carries today: a symbolic link, a device, a FIFO.** Measured: `blit copy src/ dst/` copies no link and exits 0; only `check --help` says so. In a list, (a) such a line is a reported failure — `not carried: a symbolic link` — exit 2, until transfers carry links [recommended for this plan]. Whether transfers carry links before 1.0 is the same product question as empty directories (owner, 2026-11-26: empty directories change before 1.0) and belongs to that work.

## Non-goals

- `move` with a list: stays refused (`reject_move_filters`).
- Patterns in a list: a list holds names; patterns belong to `--include-from`.
- A new visible option (repo guidance, SIMPLE): no `--from0`-style flag. A format is taken only where the bytes have one reading; where they have two, the list is refused and the message says how to save it.
- Guessing an encoding: a legacy code page, or UTF-16 without a byte-order mark, is refused with the fix named.
- The retry passes' own lists (`retry_only`, `jobs retry`): they stay exact internal paths.
- Carrying empty directories, links and other non-regular entries: the transfer's manifest holds regular files only today (`crates/blit-core/src/remote/transfer/source.rs`, the `EntryKind::File` arm). That is work of its own (Dependencies).

## Measured today (2026-11-26; blit at `ebc9d81c`)

**Windows** (NETWATCH-01), the list made by `(Get-ChildItem <src> -Recurse).FullName > filelist.txt`, then `blit copy --files-from filelist.txt <src>\ <dst>\`:
- Windows PowerShell 5.1: `>` writes UTF-16 LE with a byte-order mark (`FF FE`). blit refuses the whole list — `stream did not contain valid UTF-8`, as a raw error report with a Rust backtrace hint — exit 1, nothing copied.
- PowerShell 7: `>` writes UTF-8 without a mark. Every line — a full path, files and folders alike — is reported `source: missing at retry (not found by the retry scan)`, then retried once after 30 s; exit 2, nothing copied.
- A relative folder line (`photos`): reported the same way, nothing inside it copied; the file line beside it copied.

**Linux** (rsync 3.5.2, GNU tar 1.35, GNU cpio 2.15), `copy --files-from`:

| line | blit | `rsync -a` | `rsync -ar` | `tar -T` | `cpio -p` |
|---|---|---|---|---|---|
| `a.txt` | copied | copied | copied | copied | copied |
| `./a.txt` | failed, exit 2 | copied | copied | copied | — |
| full path inside the source | failed | failed, exit 23 (read as `<src>/<full path>`) | failed | copied, under its whole path | — |
| `/a.txt` | failed | copied as `<src>/a.txt` | same | failed | — |
| `photos/../a.txt` | failed | copied | copied | copied | — |
| folder `photos`, alone | failed | the folder, empty | the folder and all in it | all in it | — |
| folder `photos` and `photos/selected.jpg` (`omitted.txt` beside it) | `selected.jpg`; the folder line failed, exit 2 | folder and `selected.jpg` | — | folder, `selected.jpg` **and `omitted.txt`** | folder and `selected.jpg` |
| `a.txt ` (a space at its end; `a.txt` exists too) | **`a.txt` copied — the other file — exit 0** | `a.txt ` | — | `a.txt ` | `a.txt ` |
| `#notes` (a file of that name) | **nothing copied, exit 0** | nothing copied, exit 0 | — | copied | copied |
| a name that is not UTF-8 | whole list refused, exit 1 | copied | — | — | — |
| UTF-8 byte-order mark | first line failed, exit 2 | — | — | — | — |
| CRLF line ends | works | — | — | — | — |
| NUL-separated (`find -print0`) | failed, exit 2 | copied (`--from0`) | — | — | — |

`find a b -print0` writes the bytes `61 00 62 00` — byte for byte the UTF-16 LE text `ab` without a mark.

**Deletion and loss, today:**
- `mirror --files-from`, default delete scope: deletes nothing outside the list (`rsync --delete --files-from` likewise).
- **`mirror -y --delete-scope all --files-from`**: the whole destination is in scope. With plain lines, every destination file the list does not name is deleted (exit 0); with the same lines written `./a.txt`, as `find .` writes them and which today name nothing, **nothing is copied and the entire destination is deleted** (exit 2). Not refused; the only warning is a note `-y` suppresses (`crates/blit-cli/src/transfers/mod.rs`).
- `move --files-from` is refused before anything happens.
- **Two names to one destination entry**: a source's `A` and `a`, copied to a filesystem that folds case — by a list naming both, or with no list at all — report `Copied: 2 file(s)`, exit 0, and the destination holds one file, `a`, with `A`'s bytes. Older than this plan and not only a list's; the selection must not do it (S6), and the same loss in a whole-tree copy is raised to the owner in STATE.
- A plain `copy` carries no symbolic link and no empty directory, exit 0 (Q6; the owner's ruling on empty directories).

**The code** (as of `ebc9d81c`):
- The CLI captures the list once, before the mirror's question, with `read_to_string` (`crates/blit-cli/src/run_log.rs` `capture_files_from`, called from `crates/blit-cli/src/main.rs`), UTF-8 only — but nothing validates it then: the lines are first classified when the filter is built at dispatch, after the question and the `starting …` banner. `check` reads the file separately (`FileFilter::load_files_from`).
- `list_line` (`crates/blit-core/src/fs_enum.rs`) trims each line and skips blank and `#` lines; no byte-order mark is handled.
- `split_listed` (`crates/blit-core/src/transfers/filter.rs`) keeps a line as a name only when every component is a plain name; every other line goes to `files_from_unnamed` and is reported at the end through `requested_but_unscanned` (`crates/blit-core/src/transfer_session/mod.rs`) with the retry pass's hard-coded reason, and is retried like any failure.
- Matching is exact on raw components (`allows_entry`); only regular files go on the manifest, so a folder line names nothing.
- A remote transfer sends names by their components, joined with `/` (`FilterSpec` fields 7, 8, 10–14; `relative_path_to_posix`); the receiving end refuses a name with an empty, `.` or `..` component (`operation_spec.rs` `listed_name`). The session contract and the daemon-to-daemon operation spec are versioned separately, and a frame is decoded up to 4 MiB.
- A saved job keeps the list's lines (`JobSpec.files_from: Option<Vec<String>>`) and replays them through a temporary file joined with newlines; `jobs retry` does the same and then overrides it with `retry_only`, so the user's list is re-read and re-classified on every retry.
- `check` builds the lines that name nothing and never reads them.

## Safety contract (what every slice keeps true)

- **S1 — Validated before anything.** The list is read once — the snapshot the CLI already captures before any question — and decoded, every entry parsed, and (on the source's side) resolved, before a job is saved, a question asked, a destination made, or anything copied or deleted. A list that cannot be read with one meaning, or a line under Q3's rule, stops the run then: exit 1, the list, the line or record number and the reason in plain words, the fix where there is one — never a raw error report. The file is not read again for that run.
- **S2 — Every entry is accounted for.** At the end each entry has landed, was already in place, or failed with its reason. Exit 0 only when every entry is in place. An entry missing at the source is a failure, reported as not found at the source, final from the first pass — no retry pass is spent on it, and no later pass, however it ends for other files, takes it out of the run's result (exit 2).
- **S3 — No deletion follows from a list.** No file is deleted because a list does not name it; a list may be incomplete. Under Q2 (a), `mirror` with a list is refused, whatever the delete scope.
- **S4 — Nothing outside the source, at the read.** A line never names an entry outside the source — not by `..`, a full path, or a link on the way. The check that orders S1's refusal is advisory; the authority is the read itself: each entry is opened by the contained route the source walk's own entries take (`SOURCE_SIDE_CONTAINMENT.md`), so an entry replaced after validation by a link out of the source is not read.
- **S5 — A line names one entry, exactly.** The entry at that path below the source, by the name as written (Q5) — never searched for elsewhere in the tree, never another file whose name differs by a space. Two names of one file (hard links) are two entries and both are copied; two lines that resolve to one path are one entry.
- **S6 — One destination entry per selected entry.** Two selected entries that would occupy one destination entry — `A` and `a` on a destination that folds case, two normalizations of one name — do not overwrite one another: the second is a reported failure, exit 2.

## Design

**The list's bytes** (one reader for `copy`, `mirror`'s refusal, `check`, and a saved job's first capture). A format is taken only where the bytes have one reading:
1. A byte-order mark names the encoding: UTF-8's is dropped; UTF-16 LE or BE is decoded (Windows PowerShell 5.1's `>` and `Out-File`). Text lines follow.
2. No mark and no NUL byte: text lines. UTF-8; on Unix a line that is not UTF-8 is the raw bytes of a name (a Unix name need not be text, and `find` writes it so). On Windows, where names are UTF-16, such a list is refused, the line named: save it as UTF-8.
3. No mark, and NUL bytes: NUL-terminated records (`find -print0`, `git ls-files -z`) — only when that is the one reading. It is refused as ambiguous when the same bytes read as UTF-16 text: when every record is a single byte and there are two or more (`find a b -print0` is `61 00 62 00`, the UTF-16 LE text `ab`); or when the bytes decode as UTF-16, in either byte order with no unpaired surrogate, to text that holds a line break. It is refused as unreadable when the bytes do not end with a NUL, or hold an empty record. The message says what was found and both ways out: save text with a byte-order mark or as UTF-8, or write the names one to a line.
- **A NUL-terminated record is a name exactly**: every byte of it — a newline, a `#`, a space at either edge.
- **A text line** ends at LF or CRLF. Under Q5 (a) it is otherwise the name exactly as written; an empty line names nothing and is skipped. A name that holds a line break, or ends in a carriage return, cannot be written as a text line: such a list is written NUL-terminated.

**A line's meaning.**
- A relative line is parsed into its components by the syntax of the machine that reads the list — `/` separates everywhere, `\` also on Windows — and `.` components are dropped: `./a/b`, `.\a\b` and `a/b` are one entry. A line `.` is the source itself. `..` is resolved within the source; a line that climbs above it is Q3's.
- A full path — `/srv/data/a.txt`; `D:\data\a.txt`, `\\server\share\a.txt`, `\\?\D:\data\a.txt` — keeps its syntax. Resolved on the source's machine, by the filesystem: where the source's root is one of its ancestors — recognized by the root's identity, the one thing identity is used for — it names the entry at its path below the root; anywhere else, or in a syntax the source's machine does not have, Q3. This is what makes `(Get-ChildItem …).FullName`, `find "$PWD"` and `realpath` lists work.
- A folder: Q1. An entry no transfer carries: Q6.
- Case and normalization: the source's filesystem resolves the line, so where it folds case `photos\A.TXT` names the stored `photos\a.txt`, copied under its stored name.

**Between machines.** The contract in force stays: a relative entry travels as its components (text, or raw bytes where the name is not text), never as a line to be parsed again — `sub\name` read on Windows is two components and arrives as two; read on Unix it is one name and arrives as one. A full path travels as its text with its syntax named in a field. The source's side resolves every entry and answers, entry by entry and for the list as a whole (S1), in structured fields — never words to parse (D-2026-11-24-1). Both version checks rise — the session contract and the daemon-to-daemon operation spec — and each end refuses, before a destination is made or changed, a peer or a spec without the new selection; an empty list that was supplied is a list, and copies nothing. The selection and the answers travel in bounded frames, well under the transport's 4 MiB decode limit, so a list's size never decides whether it can be sent.

**Saved jobs and retries.** The validated selection, not the file's lines, is what a job keeps: a versioned form holding each entry's components — raw bytes where a name is not text, a line break inside a name — and each full path with its syntax. A job saved before this plan keeps its lines and replays them by the rules it was saved under: its meaning does not change. A retry pass and `jobs retry` take only the exact failed paths (`retry_only`); the user's list is not read to make one, so a line a retry does not use can never fail it.

**Reporting.** S1's stop is one plain message, exit 1. An entry missing at the source: `<entry>: not found at the source`, exit 2, no retry pass; the retry pass's "missing at retry" is said of retries only. `check --files-from` reports a listed entry missing from both trees as missing — today it says nothing of it.

**Cost.** Resolving each entry is one lookup at the start; a list of plain relative names keeps today's path. Measured with 100,000 entries, local and through a daemon, before slice 3 lands.

## Dependencies

- **Folders and links on the manifest.** Delivering a listed folder that holds no listed entry, or a listed link, needs the transfer to carry directory and link entries — the empty-directory work the owner ruled for 1.0 (2026-11-26), not yet planned. Until it lands such a line is a reported failure (Q6's rule, and `not carried: an empty directory`), never dropped; the owner's example then exits 2 on a tree that holds an empty folder or a link, naming each. AC-3's two cases close with that work.
- **`SOURCE_SIDE_CONTAINMENT.md`**: the contained read S4 relies on.

## Acceptance criteria

- [ ] **AC-1 (Windows)**: `(Get-ChildItem <src> -Recurse).FullName > list` in Windows PowerShell 5.1 and in PowerShell 7, then `blit copy --files-from list <src>\ <dst>\` — the files and folders of the tree copied exactly, exit 0 (a tree of regular files in folders none of which is empty); with `<src>` a drive root and a folder below it; through a daemon, push and pull.
- [ ] **AC-2 (Unix)**: `find . > list`, `find . -type f > list`, `find "$PWD/src" -type f > list`, `find . -type f -print0 > list` — each copies exactly what it lists, exit 0, on Linux and macOS.
- [ ] **AC-3 (lines)**: `./` and `.\`; `..` within the source; a folder line under Q1 — a filtered list naming `photos` and `photos/selected.jpg` copies no `photos/omitted.txt`; the measured `a.txt ` and `#notes` lines copy the files they name (Q5); two hard-linked names both copied; a line in another case on a filesystem that folds (macOS, Windows, a folding filesystem on Linux); and, with Dependencies, an empty folder's line and a link's.
- [ ] **AC-4 (S1)**: a line outside the source; one climbing above it; a Windows path given to a Unix source; a UTF-16 list without a mark; `find a b -print0`; a NUL list with an empty record; a non-UTF-8 list on Windows — each stops the run before anything changes, exit 1, the line or record number in the message; source and destination byte for byte as before; no job saved; no backtrace.
- [ ] **AC-5 (S2)**: a listed entry missing at the source — reported as not found at the source, exit 2, no retry pass (the run does not wait), the rest copied; and where another file fails, then lands on a retry pass, the run still exits 2 for the missing entry.
- [ ] **AC-6 (S3)**: the measured `mirror -y --delete-scope all --files-from` wipe is impossible: under Q2 (a) `mirror` with a list is refused before the question, `copy` named in the refusal; no route deletes a destination file because a list does not name it.
- [ ] **AC-7 (S4)**: an entry replaced, after validation and before its read, by a link out of the source is not read: reported, exit 2.
- [ ] **AC-8 (S6)**: a list naming `A` and `a`, copied to a destination that folds case: one lands, the other is reported, exit 2; neither file's bytes stand under the other's name.
- [ ] **AC-9 (remote)**: AC-1, AC-4 and AC-5 hold for push, pull and daemon to daemon; a daemon's S1 refusal reaches the CLI as a structured reason, before the destination is made; an older peer, on either version check, refuses the new selection; a supplied empty list copies nothing on every route.
- [ ] **AC-10 (size)**: 100,000 entries with long names, local and over a real daemon on each route: sent and answered in bounded frames, and a list of relative names no slower than today (measurement recorded).
- [ ] **AC-11 (jobs)**: a saved job holding a name that is not text, and one with a line break in it, replays the same selection; a job saved before this plan replays as it did; in-command retries and `jobs retry` behave as before and never read the user's list.
- [ ] **AC-12 (`check`)**: `check --files-from` with a listed entry missing from both trees reports it and exits non-zero.

## Slices

1. **No deletion from a list (S3)** — `mirror` with `--files-from` refused at validation, before the question (Q2), the refusal naming `copy`; the measured wipe pinned as a test that fails today. Independent of the rest; it removes a live hazard.
2. **The reader and its order (S1)** — the bytes' one reading (marks, UTF-16, raw bytes on Unix, NUL records, the refusals); exact lines (Q5); the captured snapshot validated before a job is saved, a question asked or a banner printed; plain messages; `check` on the same reader.
3. **Entries (S4, S5)** — relative forms, full paths recognized by the root's identity, `..`, folder lines (Q1), the source's own line, entries not carried (Q6), hard-linked names, case through the filesystem, Q3's stop; the read by the contained route.
4. **Outcome (S2, S6)** — not found at the source, final and without a retry pass; destination collisions; `check`'s report.
5. **Remote** — components and full-path syntax on the wire, the source's side answering in structured fields, both version checks, bounded frames, the supplied empty list.
6. **Saved jobs and retries** — the versioned selection, old jobs by their own rules, retries on exact paths only.
7. **Documentation and evidence** — `--help`, the manual, the CHANGELOG; AC-1 on NETWATCH-01; AC-2 and AC-3 on macOS through CI.

## Open questions

Q1–Q6 above. Answered by this plan once approved: STATE's "what should a `--files-from` line that is no plain relative path mean?" and "a `--files-from` list saved with a byte-order mark silently loses its first name".

## Review history

- **`files-from-plan-r1`** (Codex openreview over `d8689340..448643ef`, 2026-11-27): `acceptable_with_changes`, capability_ok and guard_confirmed true — "The draft gets the order right. Refusing list-driven deletion removes the measured `mirror -y --delete-scope all` hazard independently of the parser work." Five changes required, all adopted in this revision: (1) which input bytes have one meaning — NUL records literal, ambiguity refused, the list's stripping and comment rules settled against exactness (the reader's rules; Q5, with the two faults measured); (2) what a folder line selects — Q1's recommendation is now the reviewer's, the folder and its listed entries, and the manifest dependency is stated (Dependencies, Q6); (3) a path's meaning across machines and at the read — components kept on the wire, full-path syntax named, containment at the open (S4), hard-linked names distinct (S5), two names to one destination entry detected (S6, measured as a live loss); (4) both version checks, the supplied empty list, bounded frames and 100,000 entries over a real daemon (AC-9, AC-10); (5) one lossless selection — the captured snapshot validated before saving or dispatching (S1), a versioned saved form, old jobs' meaning kept, `check` and the retry-pass exit cases (AC-5, AC-11, AC-12). An earlier dispatch the same day failed at once on the reviewer's usage limit and was sent again with the owner's approval.
