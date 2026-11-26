# `--files-from` lists from any tool

**Status**: Draft
**Created**: 2026-11-26
**Supersedes**: nothing. It answers two of STATE's open questions — a `--files-from` line that is not a plain relative path, and a list saved with a byte-order mark — once approved.
**Decision ref**: — (once approved)

## Owner's requirement (2026-11-26, verbatim)

- "well I need it to work, so plan for that. then review the plan with codex until consensus."
- "lists from whatever tool in whatever format cannot erroneously delete files or skip files while reporting success or this tool is dangerous."
- The owner's working example, in their usual shell (PowerShell): `(Get-ChildItem D:\ -recurse).FullName > filelist.txt`, then that file given to blit as the list.

## Goal

A list of files made by any common tool, saved in any common text format, works as `--files-from` for `copy` (local, push, pull, daemon to daemon): every entry it names is copied and nothing else. No list, however it was made, can make blit delete a file, or let a run end in success while an entry the list names did not land.

## Pending owner decisions (each with the plan's recommendation)

- **Q1 — a line naming a folder.** (a) The folder and everything in it, as `tar -T`, `rsync -r`, `Copy-Item -Recurse` and `robocopy /E` do [recommended]; (b) the folder alone, created empty, as `rsync -a` without `-r` does.
- **Q2 — `mirror` with a list.** (a) Refused, as `move` with a list already is: a list names what to copy, and a deletion is never decided by a list [recommended]; (b) allowed for folder lines only, each listed folder mirrored from the source's own content — never from the list.
- **Q3 — a line the run can never satisfy** (outside the source, climbing above it, unreadable). (a) The whole run stops before it changes anything [recommended]: such a line means the list was made for another place, or read wrongly; (b) that line is reported and the rest copied.
- **Q4 — a Unix line starting with `/`.** (a) A full path, like every other full path: inside the source it names that entry, outside it is refused [recommended]; rsync's `--files-from=list /` idiom works, since `/` is then the source; (b) rsync's reading: relative to the source, whatever the source is.

## Non-goals

- `move` with a list: stays refused (`reject_move_filters`).
- Patterns in a list: a list holds names; patterns belong to `--include-from`.
- A new visible option (repo guidance, SIMPLE): no `--from0`-style flag. The format is recognized from the file itself.
- Guessing a legacy code page: a Windows list that is neither UTF-8 nor UTF-16 is refused, with the fix named.
- The retry passes' own lists (`retry_only`, `jobs retry`): they stay exact internal names (see Design: retry).

## Measured today (2026-11-26)

**Windows** (NETWATCH-01, blit at `ebc9d81c`), the list made by `(Get-ChildItem <src> -Recurse).FullName > filelist.txt`, then `blit copy --files-from filelist.txt <src>\ <dst>\`:
- Windows PowerShell 5.1: `>` writes UTF-16 LE with a byte-order mark (`FF FE`). blit refuses the whole list — `opening files-from list …: stream did not contain valid UTF-8`, printed as a raw error report with a Rust backtrace hint — exit 1, nothing copied.
- PowerShell 7: `>` writes UTF-8 without a mark. Every line — a full path, files and folders alike — is reported `source: missing at retry (not found by the retry scan)`, then retried once after 30 s; exit 2, nothing copied.
- A relative folder line (`photos`): reported the same way, nothing inside it copied; the file line beside it copied.

**Linux** (this host; blit at `ebc9d81c`, rsync 3.5.2, GNU tar 1.35), `copy --files-from`:

| line | blit | `rsync -a` | `rsync -ar` | `tar -T` |
|---|---|---|---|---|
| `a.txt` | copied | copied | copied | copied |
| `./a.txt` | failed, exit 2 | copied | copied | copied |
| full path inside the source | failed | failed, exit 23 (read as `<src>/<full path>`) | failed | copied, under its whole path from `/` |
| `/a.txt` | failed | copied as `<src>/a.txt` | same | failed |
| `photos/../a.txt` | failed | copied | copied | copied |
| folder `photos` | failed | the folder, empty | the folder and all in it | all in it |
| a name that is not UTF-8 | whole list refused, exit 1 | copied | — | — |
| UTF-8 byte-order mark | first line failed, exit 2 | — | — | — |
| CRLF line ends | works | — | — | — |
| NUL-separated (`find -print0`) | failed, exit 2 | copied (`--from0`) | — | — |

**Deletion with a list, today:**
- `mirror --files-from` with the default delete scope deletes nothing outside the list; `rsync --delete --files-from` deletes nothing either.
- **`mirror -y --delete-scope all --files-from`** brings the whole destination into scope: with plain lines, every destination file the list does not name is deleted (exit 0); with the same lines written `./a.txt` — as `find .` writes them, and which today name nothing — **nothing is copied and the entire destination is deleted** (exit 2). Not refused; the only warning is a note `-y` suppresses (`crates/blit-cli/src/transfers/mod.rs` 547–561).
- `move --files-from` is refused before anything happens.

**The code** (as of `ebc9d81c`):
- The CLI reads the list with `read_to_string` (`crates/blit-cli/src/run_log.rs` `capture_files_from`), UTF-8 only; lines are split and trimmed by `list_line` (`crates/blit-core/src/fs_enum.rs`), no byte-order mark handled. `check` reads it separately (`FileFilter::load_files_from`).
- The list is first read when the filter is built at dispatch — after the mirror's question and the `starting …` banner. Validation reads no list file.
- `split_listed` (`crates/blit-core/src/transfers/filter.rs`) keeps a line as a name only when every component is a plain name; every other line goes to `files_from_unnamed` and is reported at the end through `requested_but_unscanned` (`crates/blit-core/src/transfer_session/mod.rs`), with the retry pass's hard-coded reason. Those failures are retried like any other.
- Matching is exact on raw components (`allows_entry`); only files go on the manifest, so a folder line names nothing.
- A remote transfer sends the classified names (`FilterSpec` fields 7, 8, 10–14); the receiving end refuses a name with an empty, `.` or `..` component.
- `jobs retry` writes the saved list back to a temporary file and sets `--files-from` again, so the original list is re-read — and re-classified — on every retry, though `retry_only` then replaces it.

## Safety contract (what every slice keeps true)

- **S1 — Read before anything.** The list is read, decoded and every line resolved before any question is asked or anything is copied or deleted. A list that cannot be read, or a line under Q3's rule, stops the run then: exit 1, the list and the line number named, in plain words — never a raw error report.
- **S2 — Every line is accounted for.** At the end each line's entry has landed, was already in place, or failed with its reason. Exit 0 only when every line's entry is in place. An entry missing at the source is a failure (exit 2), reported as not found at the source, and not retried — waiting does not make it appear.
- **S3 — No deletion follows from a list.** No file is deleted because a list does not name it — a list may be incomplete (a tool that skipped a folder it could not read, a list cut short). Under Q2 (a), a list with `mirror` is refused, `--delete-scope all` included.
- **S4 — Nothing outside the source.** A line never names an entry outside the source — not by `..`, a full path, or a link on the way. Resolved by the filesystem and judged by identity (the root found among the real path's ancestors, as `alias_place` judges a link today), never by comparing text.
- **S5 — A line names one place.** A line names exactly the entry at that path under the source; it is never searched for by name elsewhere in the tree.

## Design

**Reading the file** (one reader for `copy`, `mirror`'s refusal, `check`, a saved job's replay):
- A byte-order mark decides the encoding: UTF-8's is dropped; UTF-16 LE or BE is decoded (Windows PowerShell 5.1's `>` and `Out-File` default).
- No mark: UTF-8 (PowerShell 7, `find`, most tools). On Unix a line that is not UTF-8 is the raw bytes of a name — a Linux name need not be text, and `find` writes it so. On Windows, where a name is UTF-16, a list neither UTF-8 nor UTF-16 (a `cmd` `dir /b > list` in the console's code page, with a name beyond ASCII) is refused, the line named and the fix said: save the list as UTF-8.
- A NUL in a list without a mark: a list whose bytes are NUL-separated names (`find -print0`) is read so. A list whose bytes look like UTF-16 without a mark — a NUL in every other byte of plain characters — is refused, the fix said. The rule is a byte pattern, decided once for the file.
- Lines end in LF or CRLF. Each line is trimmed of whitespace at both ends; blank lines and lines starting with `#` are skipped — the rule the list and the pattern files share today (owner Q5 "a", `EXCLUDE_CASE_MATCHING.md`). A name that starts with `#` is listed as `./#name`.

**A line's meaning** (after decoding):
- `a/b` — the path `a/b` under the source, as today. On Windows `\` and `/` both separate, as today.
- `./a/b`, `.\a\b` — `a/b`. A line `.` (the first line `find .` writes) is the source itself, a folder line (Q1).
- A full path — `/srv/data/a.txt`; `D:\data\a.txt`, `\\server\share\a.txt`, `\\?\D:\data\a.txt` — resolved by the filesystem; inside the source it names the entry at its path below the source; outside, Q3. This is what makes `Get-ChildItem … .FullName`, `find "$PWD"` and `realpath` lists work.
- `..` — resolved within the source; a line that climbs above it is Q3's.
- A folder — Q1.
- A link — the line names the link itself, copied as a link, as the walk copies it; a link on the way to a line's entry is followed only while it stays within the source (S4).
- Case and Unicode normalization — the filesystem resolves the line, so on a filesystem that folds case `photos\A.TXT` names the stored `photos\a.txt`, which is copied under its stored name.
- Two lines naming one entry name it once.

**Where lines are resolved.** The side that holds the source resolves them: the CLI for a local copy or a push; the source daemon for a pull or daemon to daemon. The decoded lines travel as written — bytes where they are not text — and the source side's verdict on each, and on the whole (S1), comes back as structured fields, never words to parse (D-2026-11-24-1). This changes the wire: a contract version, with an older peer refusing the new fields rather than reading them as names. A full path on a daemon source is a path on the daemon: one written on the client's machine names nothing there, and stops the run (Q3) before anything changes.

**Retry and saved jobs.** A retry pass and `jobs retry` keep their exact internal names (`retry_only`, `files_from_exact`); the user's list is not re-read to make a retry, so a list line a retry does not use can never fail it. A saved job keeps its list's lines, as today, and replays them through the reader of the build that runs it.

**Reporting.** A list problem at S1 is one plain message — the list, the line number, the line, why, and the fix where there is one — exit 1. A line whose entry is missing at the source: `<line>: not found at the source`, exit 2, no retry pass. The retry pass's "missing at retry" words are for retries only.

**Cost.** Resolving every line is one lookup per line at the start; a list of plain relative names keeps its fast path. Measured with a 100,000-line list before the slice lands, against today's.

## Acceptance criteria

- [ ] **AC-1 (Windows)**: `(Get-ChildItem <src> -Recurse).FullName > list` in Windows PowerShell 5.1 and in PowerShell 7, then `blit copy --files-from list <src>\ <dst>\` — the tree copied exactly, exit 0; also with `<src>` given as `D:\` and as a folder below it; also through a daemon (push and pull).
- [ ] **AC-2 (Unix)**: `find . > list`, `find . -type f > list`, `find "$PWD/src" -type f > list`, `find . -print0 > list` — each copies exactly what it lists, exit 0, on Linux and macOS.
- [ ] **AC-3**: `./` and `.\` lines; `..` within the source; a folder line (Q1); an empty folder's line (made, with the empty-directory work the owner ruled for 1.0); a link line; a line in another case on a filesystem that folds (macOS, Windows, `fold_run.sh` on Linux).
- [ ] **AC-4 (S1)**: a line outside the source; a line climbing above it; a UTF-16 list without a mark; a non-UTF-8 list on Windows — each stops the run before anything changes, exit 1, the line number in the message; source and destination byte for byte as before; no backtrace.
- [ ] **AC-5 (S2)**: a listed entry missing at the source — reported as not found at the source, exit 2, no retry pass (the run does not wait), every other line copied.
- [ ] **AC-6 (S3)**: the measured `mirror -y --delete-scope all --files-from` wipe is impossible; under Q2 (a) `mirror` with a list is refused before the question, with `copy` named; no test or route deletes a destination file because a list does not name it.
- [ ] **AC-7 (remote)**: AC-1, AC-4 and AC-5 hold for push, pull and daemon to daemon; a daemon's S1 refusal reaches the CLI as a structured reason; an older peer refuses the new fields.
- [ ] **AC-8 (retry)**: in-command retries and `jobs retry` behave as before; a saved job with a list replays; a list line a retry does not use never fails it.
- [ ] **AC-9 (cost)**: the 100,000-line measurement recorded; a list of relative names no slower than today.

## Slices

1. **No deletion from a list (S3)** — `mirror` with `--files-from` refused at validation, before the question (Q2), the refusal naming `copy`; the existing guards updated; the measured wipe pinned as a test that fails today. Independent of the rest, and the one that removes a live hazard.
2. **The reader (S1)** — byte-order marks, UTF-16, raw bytes on Unix, NUL-separated lists, the Windows refusal; read at validation, before any question or banner; plain messages; `check` and a saved job's replay on the same reader.
3. **Line forms (S4, S5)** — `./`, full paths judged by identity, `..`, folder lines (Q1), the source's own line, links, case through the filesystem, duplicates; Q3's stop.
4. **Reporting and retry (S2)** — not found at the source, without a retry pass; retries never re-read the user's list.
5. **Remote** — lines on the wire, the source side resolving and answering in structured fields; the contract version.
6. **Documentation and evidence** — `--help`, the manual, the CHANGELOG; AC-1 on NETWATCH-01, AC-2 and AC-3 on macOS through CI.

## Open questions

Q1–Q4 above. Answered by this plan once approved: STATE's "what should a `--files-from` line that is no plain relative path mean?" and "a `--files-from` list saved with a byte-order mark silently loses its first name".
