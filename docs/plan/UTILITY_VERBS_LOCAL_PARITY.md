# Utility verbs: behave like the local commands, run on the daemon

**Status**: Active — owner: "activate" (2026-11-20, D-2026-11-20-2), on the draft written that day under the owner's open-ended mandate; Q1 (whether `rm` asks) is still open and slice 2 waits on it
**Created**: 2026-11-20
**Supersedes**: the command matrix and UX principles of `docs/plan/BLIT_UTILS_PLAN.md` (Historical) as the design intent for `rm`, `find`, `du`, `df` and `ls`
**Decision ref**: D-2026-11-20-2

## Goal

Each of `rm`, `find`, `du`, `df` and `ls` does its job on the daemon's own disk in one request, so a network filesystem's per-file cost is never paid, and behaves like the Unix command it is named after: for the options it accepts, the same meaning, the same shape of output and the same exit status. An option it does not support is refused by name, never ignored. Nothing else is added: no new verb, no option the local command does not have.

## Owner requirements (verbatim)

- 2026-11-20: "find should work identically to the unix find command. I don't know what check is."
- 2026-11-20: "the point to all of the utility commands that blit supports is to avoid the speed issues with running checks over nfs/smb. find, rm, etc. used to be part of blit-utils, and they need to behave like the local versions, just run directly from the daemon. that way deleting thousands of files doesn't pay the network fs tax. find should likewise work the same way. look at the old plans for blit-utils to make sure its understood and we're building the correct thing."
- 2026-11-20: "okay, sanity check and improve the utils scope. I don't want to make this a whole side project, I just need blit to do what I stated last message. do local fs ops via daemon to avoid speed issues with network filesystems. take that as a general "plan this" task. that old plan was written months ago and you're far more capable than the models then were, so you have an open-ended mandate."
- 2026-11-20, on the draft's open points: Q1 — "mirror has a prompt, so why wouldn't rm?"; Q2 — "a"; Q3 — "a"; Q4 — "b"; and "activate".

## Sanity check (2026-11-20)

### The old plans

Read: `docs/plan/BLIT_UTILS_PLAN.md`, `docs/plan/greenfield_plan_v6.md` §"Discovery & Admin Utilities" and §"Remote Services", `docs/plan/WORKFLOW_PHASE_3.md`, the first committed `greenfield_plan_v4.md` and `greenfield_plan_v5.md`, and the retired `blit-utils` manual page (`4e6f5886`). They specify "operator tooling for discovery, inspection, and maintenance of remote Blit daemons" as a command matrix of blit's own design. Neither half of the owner's statement is written in any of them — not the purpose (no network-filesystem cost) and not the rule (behave like the local command); a search of `docs/`, `README.md`, `TODO.md` and `DEVLOG.md` for a line pairing a network filesystem with one of these verbs found none. The verbs were built to that matrix.

### What is right

- **The architecture is the owner's.** Every one of these verbs is one request the daemon answers from its own disk (`crates/blit-core/proto/blit.proto`: `List`, `Find`, `DiskUsage`, `FilesystemStats`, `Purge`).
- **Deletion runs at local speed and through the reviewed engine.** `rm` goes through `blit_core::deletion` with canonical-root containment (`crates/blit-daemon/src/service/admin.rs`), refuses a whole module and refuses a read-only module (both run today).

### What is wrong (run 2026-11-20: release build of blit 1.0.0, a loopback daemon serving scratch folders on tmpfs, Linux)

| # | Verb | Observed | The local command |
|---|---|---|---|
| 1 | `df` | A module on a 16 GiB tmpfs was reported as 1.82 TiB total, 865 GiB used — another disk's numbers. Cause, read from the code: it looks the path up in a list of disks from the `sysinfo` library and takes the longest mount-point match; a filesystem that library does not list falls through to its parent mount. | `df` asks the filesystem that holds the path. |
| 2 | `du` | `--max-depth 1` on `HOST:/rw/small/` printed only `.` and `small`: depth is counted from the module root, not from the path asked about. Every run prints the target twice (`.` and its own name) and one row per FILE. Sizes are byte lengths. | Depth counts from the operand; directories only unless `-a`; allocated space in 1K blocks. |
| 3 | `rm` | Deletes a directory tree with no `-r`. Asks unless `-y`; unanswered, it prints "Aborted." and exits 0. A path that does not exist exits 0 ("path may already be absent"). A wildcard is taken literally and exits 0. Two operands are a usage error. | `-r` required for a directory; asks nothing; a missing operand is an error unless `-f`; many operands. |
| 4 | `find` | Prints a padded table with a `PATH BYTES TYPE` header; paths are relative to the module, not to what was typed. Its options are blit's own. `--case-insensitive` pairs ASCII letters only; `?` matches one byte while the help says "character". | One path per line, beginning with the operand as typed; an expression of tests; in a UTF-8 locale `-iname` pairs `Ü` with `ü` and `?` matches one character. |
| 5 | `ls` | Prints a "Listing …" header and `FILE`/`DIR` rows; shows names beginning with `.`. | Names, one per line, sorted; dot-names hidden unless `-a`. |

The local column was run the same day with GNU findutils 4.11.0 and coreutils under `LC_ALL=C.UTF-8`: `rm` of a directory says "Is a directory" and exits 1; of a quoted wildcard or a missing name, "No such file or directory" and exits 1, removing the other operands all the same; `rm -f` of a missing name exits 0 in silence; `find -name '*'` lists names beginning with `.`; `find -delete` on a directory that is not empty says "Directory not empty" and exits 1; `du` counts a file with two hard links once; `ls` sorts `A`, `a`, `b` and hides `.dot`. The daemon's `find` cites the old plan as its specification in its own source ("Pattern matching is glob-based, matching `BLIT_UTILS_PLAN.md`").

### Speed (same run; 200,000 empty files in 2,000 directories; best of two for the walks, one run for the delete)

| Job | Local command | blit, daemon on loopback |
|---|---|---|
| walk and print every path | `find`: 40 ms | 361 ms |
| total size | `du -s`: 132 ms | 947 ms |
| delete the tree | `rm -rf`: 260 ms | 254 ms |

All three finish in under a second where a network filesystem would be paying a round trip per file, so the purpose is met; `rm` is at local speed. `find` and `du` cost about seven times the local command. Likely causes, not yet profiled: the daemon's `find` is handed full metadata for every entry and sends one message per entry (read from its source), filling size and time columns no test asked for; `du` sends a row for every file (seen in its output).

### Found beside the verbs (not in this plan's scope; each needs its own ruling)

- **No way to compare a local tree with a daemon's tree without transferring.** `check` reads both trees from the machine it runs on (given `HOST:/rw/small/` it answered "destination path does not exist"), and `mirror --dry-run` to a daemon answers "--dry-run is not supported for remote transfers". Verifying a copy that lives on a server therefore means mounting the share and hashing it over the network — the cost this plan exists to avoid. Open question Q3.
- **Ordinary refusals print a source location and a backtrace hint** ("Location: crates/blit-cli/src/rm.rs:27 … Run with RUST_BACKTRACE=1"), on every verb, in a release build.

## Non-goals

- The whole of any Unix command. Each verb gets the short option list below; the rest is refused by name. More is added only on a stated need.
- New verbs (`mv`, `cp`, `mkdir`, `stat`, …). A rename is one request on a network filesystem already; copies are the transfer verbs.
- Running a command on the daemon (`find -exec`, `-ok`). Never.
- `scan`, `list-modules`, `completions`, `profile`: no Unix namesake, unchanged.
- `check` and remote `--dry-run` (Q3).
- Wildcards in an operand. Locally the shell expands them before the command sees them; a quoted `'dir/*.jpg'` is a literal name to `rm`, which fails with "No such file or directory". `blit rm` does the same; selecting by pattern is `find … -delete`.
- Regular expressions (their own plan, `docs/plan/EXCLUDE_CASE_MATCHING.md` Q7).

## Constraints

- **FAST**: one request per verb, answered from the daemon's disk; the walk does no per-entry work the request did not ask for (no `stat` when no test or column needs it; no row sent that will not be printed).
- **SIMPLE**: the reference is GNU findutils and coreutils under the `C.UTF-8` locale (characters, code-point order, Unicode case pairs — the locale every measurement here used), the same on every client and daemon platform; no option the reference lacks, except the `--json` each verb already has.
- **RELIABLE**: refuse, never ignore — an unsupported option, test or operator stops the command before anything is read or deleted. Every delete goes through `blit_core::deletion` and its containment. Exit status is the local command's: 0 only when everything asked for was done.
- **Same build only** (D-2026-07-05-2): every wire change below bumps `CONTRACT_VERSION`.
- **These verbs shipped in 0.1.x.** Their behavior changes at 1.0.0 and the changelog says so (Q2).
- **Platform truth**: the pure pieces (expression parser, name matcher, depth and size arithmetic) are guarded on every OS. Guards that compare against the system's own `find`, `du` and `df` run on Linux only, where the system command IS the reference; macOS ships the BSD commands, which differ in defaults, and Windows has none. A Windows daemon reports a file's length where Unix reports allocated blocks.

## Acceptance criteria

- [ ] **AC-1 (`df`)**: `blit df HOST:/module/` reports the filesystem that holds the module's path — on Linux, the numbers `df -k` prints for that path — as `Filesystem 1K-blocks Used Available Use% Mounted on` with the operand in the first column and the last column omitted; `-h` prints human-readable sizes. A module on a filesystem other than the root one reports its own numbers (the run above reported another disk's).
- [ ] **AC-2 (`du`)**: `blit du HOST:/module/path` prints one line per directory, deepest first, the operand last, each `SIZE<TAB>PATH` with the path beginning with the operand as typed; `-a` adds files; `-s` prints the operand's line only; `-d N` (and `--max-depth=N`) limits lines to N levels BELOW THE OPERAND; `-h` prints human-readable sizes. Sizes are allocated space in 1K blocks where the daemon's OS reports allocation (Unix), the file's length rounded up to 1K elsewhere. A file with several hard links is counted once, as `du` counts it — a backup tree of hard-linked snapshots is exactly where counting each name would mislead. On Linux the output equals the system `du`'s for the same tree and options, hard links included. No duplicate line for the operand.
- [ ] **AC-3 (`rm`)**: `blit rm [-r] [-f] TARGET...`. A directory without `-r` (`-R`, `--recursive`) is refused with "is a directory", deletes nothing and makes the exit status 1; a missing operand is an error and exit 1 unless `-f`; the other operands are still processed; whether it asks first is Q1; exit 0 only when every operand was removed (or absent under `-f`). A whole module and a read-only module are refused as today. `-y` is accepted and does nothing, so 0.1.x commands keep working.
- [ ] **AC-4 (`find`, tests)**: `blit find TARGET [-name PAT] [-iname PAT] [-type f|d] [-maxdepth N]`, tests ANDed. Output is one path per line, each beginning with the operand as typed, nothing else. `-name` matches the base name with `*` (any run of characters, a leading dot included), `?` (one character), `[...]` and `\`; `-iname` ignores case by the fold `--ignore-case` uses (Unicode simple case folding, `docs/plan/EXCLUDE_CASE_MATCHING.md` Q1), so `-iname 'übung*'` finds `ÜBUNG.txt`; `?.dat` finds the composed `é.dat` and `??.dat` does not. Any other test, action or operator (`-o`, `!`, parentheses, `-exec`, …) is refused by name before the walk starts. On Linux the lines equal the system `find`'s for the same tree and expression, as sets. `--pattern`, `--case-insensitive`, `--files`, `--dirs` and `--limit` are refused with the spelling that replaces each.
- [ ] **AC-5 (`find`, acting)**: `-mtime [+|-]N` and `-size [+|-]N[ckMG]` with the reference's rounding; `-delete` removes each match, contents before their directory, prints nothing, leaves a matched directory that is not empty with an error and exit 1, never removes the module's root, and is refused on a read-only module. Running the same expression without `-delete` prints exactly what `-delete` would remove.
- [ ] **AC-6 (`ls`)**: `blit ls TARGET` prints names one per line, sorted by code point (`A`, `a`, `b`), hiding names that begin with `.`; `-a` shows them, with `.` and `..` as `ls` prints them; `-l` prints type letter, size, modification time and name (no permission or owner columns — a listing does not carry them, and the manual says so); `-h` with `-l` prints human-readable sizes. A file operand prints that file's line.
- [ ] **AC-7 (speed)**: the 200,000-file run above is repeated before and after each slice that touches a walk and recorded under `docs/bench/`; `find` with no test that needs it calls no `stat`, and `du` without `-a` sends no file rows — each pinned by a guard that counts, not by a stopwatch.
- [ ] **AC-8 (refusal, every verb)**: an option outside the lists above exits non-zero naming the option, before any request is sent.
- [ ] **AC-9 (words)**: `--help`, `docs/cli/blit.1.md`, `README.md` and `CHANGELOG.md` describe each verb as its Unix namesake with the supported options listed; `docs/plan/BLIT_UTILS_PLAN.md` points here.

## Design

### The rule, in one place

A small table per verb — the options accepted and what each maps to on the wire — drives parsing, the refusal message and the help text, so the three cannot disagree. `find`'s expression and the single-dash options are parsed by hand after clap has taken the target (clap has no single-dash long options).

### `df` (`crates/blit-daemon/src/service/admin.rs` `filesystem_stats_for_path`)

Ask the operating system about the path itself — `statvfs` on Unix, `GetDiskFreeSpaceExW` on Windows — in place of the `sysinfo` mount-list match. Total is `f_blocks`, used is `f_blocks − f_bfree`, available is `f_bavail`, as `df` computes them. `FilesystemStatsResponse` gains `available_bytes` (today's `free_bytes` cannot say which of the two it is).

### `du` (`disk_usage` in the same file)

Depth is counted from the start path. The daemon sends only the rows that will be printed (directories within the depth, files only under `-a`), each with allocated bytes (`st_blocks × 512` on Unix, the directory's own blocks included; the length elsewhere). On Unix a file whose link count exceeds one is remembered by device and inode and counted the first time only — the set holds only such files, as `du`'s does. `DiskUsageRequest` gains `all_files` and a depth with "unlimited" distinct from 0; `DiskUsageEntry` gains `allocated_bytes`.

### `rm` (`crates/blit-cli/src/rm.rs`, `crates/blit-core/src/admin/rm.rs`, the daemon's purge)

`PurgeRequest` gains `recursive` and `force`; the daemon — which is the side that knows what each path is — refuses a directory without `recursive` and reports each operand's outcome, so the CLI can print the local command's message per operand and set the exit status. `PurgeResponse` gains per-path results. Operands are grouped by daemon and module, one request each. The prompt's fate is Q1.

### `find` (`crates/blit-cli/src/find.rs`, the daemon's `find`)

- **Matcher**: a character-based `fnmatch` in `blit-core` (`*`, `?`, classes, `\`), on the base name; a name that is not valid text is matched byte by byte. It is not the transfer filters' glob engine: transfer patterns follow rsync's rule and count bytes (`docs/plan/EXCLUDE_CASE_MATCHING.md` Q10), `find` follows find's and counts characters — each verb answers to the tool it is named after.
- **Walk**: `stat` only when a test (`-size`, `-mtime`, or `-type` where the directory entry does not carry the type) needs it; entries are sent in batches.
- **`-delete`**: the daemon collects matches depth-first and hands them to `blit_core::deletion` with a non-recursive directory mode, so a directory goes only when empty.
- **Wire**: `FindRequest` gains the name pattern as typed with its case flag, `max_depth`, the size and time predicates and `delete`; the old `pattern` field is retired.

### `ls` (`crates/blit-cli/src/ls.rs`)

Printing only; `FileInfo` already carries name, kind, size and time.

### Alternative considered: have the daemon run the native commands

Owner, 2026-11-20: "unless it's simpler to just call the native equivalent commands. windows doesn't have analogs to find, du, df, etc., but delete does. or we could call powershell. that doesn't sound simpler, but if it is, it's an option." The drafting agent answered that it is not simpler, for the reasons below, and told the owner that activating the plan would accept that answer; the owner activated without objection.

- **It adds backends instead of removing one.** Windows has no `find`, `du` or `df`, so the daemon's own implementation is needed there whatever Unix does. Calling native tools would mean that implementation plus a GNU caller, a BSD caller for macOS (different defaults and option spellings) and whatever a NAS ships, each with its own output to parse — and the same `blit find` would answer differently from one daemon to the next.
- **`find` runs programs.** `-exec`, `-ok` and the `-f…` actions execute commands and write files, so a user's expression could not be passed through: the daemon would have to parse and allow-list it first, which is the parser this plan already needs, and one gap in that list would be command execution on the daemon's machine. No password or token check was found in the daemon's source (its authorization is by host address).
- **Deletion would leave the reviewed engine.** `rm` goes through `blit_core::deletion` and its per-entry containment today; a spawned `rm` or `Remove-Item` would not.
- **PowerShell is slower, measured** (2026-11-20, PowerShell 7.6.3 on Linux, the same 200,000-file tree; Windows itself not measured): starting it costs 51 ms against 1 ms for `find`; listing took 2,216 ms (`Get-ChildItem -Recurse`) against 55 ms for `find` and 361 ms for blit today; deleting took 728 ms (`Remove-Item -Recurse`) against 257 ms for `rm -rf` and 254 ms for blit.
- **A service's locale changes the answers.** The measurements above show GNU find pairing `Ü` with `ü` and counting characters only in a UTF-8 locale; a daemon started by the system often runs in the plain one.
- **What the instinct gets right is already the design.** Where the operating system has the answer, ask it: `df` becomes one `statvfs` / `GetDiskFreeSpaceExW` call, and deletion already uses the system's own delete calls at local speed. The rest is a walk blit already has (`FileEnumerator`) plus option parsing and printing, which a native caller would need too.

### Risks

- **Whether `rm` asks** (Q1, open). Today's prompt is the only thing between a typo and a deleted tree.
- **A second pattern matcher.** It is small and compared against the system `find` on Linux, but it is one more thing that decides what gets deleted once `-delete` exists.
- **0.1.x scripts break** on `find`'s old options and on `rm` of a directory without `-r` — loudly, with the replacement named (Q2).
- **The Linux-only comparison guards prove Linux.** macOS and Windows daemons are covered by the pure guards and by their own end-to-end runs; this is stated wherever a result is reported.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop. Each ships alone. Slices 1 to 3 fix what is wrong today; 4 to 6 are the rest of the parity. All six land before the 1.0 tag (Q4, owner "b").

1. **`df` and `du` say the right thing** (AC-1, AC-2, AC-7 for `du`): the two wrong answers measured above, their output shape and options. Wire change, contract bump.
2. **`rm`** (AC-3): `-r`, `-f`, several operands, no prompt, the local exit status. Wire change, contract bump.
3. **`find`, tests and output** (AC-4, AC-7 for `find`): the expression parser, the matcher, path-per-line output, the old options refused. Wire change, contract bump.
4. **`find` acts** (AC-5): `-mtime`, `-size`, `-delete`.
5. **`ls`** (AC-6).
6. **Words** (AC-9) — folded into each slice for its own verb; this slice is the manual page's overall rewrite and the pointer from the old plan.

AC-8 is met verb by verb as its slice lands.

## Open questions

- **Q1 — does `rm` ask before deleting?** The draft removed the prompt to match the local command. Owner, 2026-11-20: "mirror has a prompt, so why wouldn't rm?" — blit's own rule is that a command that deletes asks unless `-y` (`mirror`, `move`), and the draft set that aside for `rm` alone. The drafting agent's revised recommendation: keep the prompt, on the same footing as `mirror`'s; `-y` skips it as today, and so does `-f`, which means "never ask" in the local command — so `blit rm -rf` behaves exactly as `rm -rf` does, and a bare `blit rm -r` asks. Not yet ruled; slice 2 waits. — owner
- ~~**Q2 — the old spellings**~~ — **settled 2026-11-20 (owner "a")**: `find`'s old options and `rm` of a directory without `-r` are refused with a message naming the replacement; nothing is kept working hidden.
- ~~**Q3 — comparing a local tree with a daemon's tree**~~ — **settled 2026-11-20 (owner "a")**: not in this plan; making `--dry-run` work for a transfer to or from a daemon gets a plan of its own, which also decides what `check` is for. That plan is owed and not yet drafted. (`check` takes the transfer verbs' pattern options meanwhile: `docs/plan/EXCLUDE_CASE_MATCHING.md` Q11.)
- ~~**Q4 — before or after the 1.0 tag**~~ — **settled 2026-11-20 (owner "b")**: all six slices before the tag. The drafting agent had recommended 1 to 3 before and 4 to 6 after.
