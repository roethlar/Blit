# Utility verbs: behave like the local commands, run on the daemon

**Status**: Active — owner: "activate" (2026-11-20, D-2026-11-20-2), on the draft written that day under the owner's open-ended mandate; every open question is settled (Q1 by D-2026-11-20-3); revised the same day after Codex openreview `plan-utility-verbs-2026-11-20-r1`, all six changes adopted (D-2026-11-20-4)
**Created**: 2026-11-20
**Supersedes**: the command matrix and UX principles of `docs/plan/BLIT_UTILS_PLAN.md` (Historical) as the design intent for `rm`, `find`, `du`, `df` and `ls`
**Decision ref**: D-2026-11-20-2

## Goal

Each of `rm`, `find`, `du`, `df` and `ls` does its job on the daemon's own disk in one request, so a network filesystem's per-file cost is never paid, and behaves like the Unix command it is named after: for the options it accepts, the same meaning, the same shape of output and the same exit status. An option it does not support is refused by name, never ignored. Nothing else is added: no new verb, no option the local command does not have.

## Owner requirements (verbatim)

- 2026-11-20: "find should work identically to the unix find command. I don't know what check is."
- 2026-11-20: "the point to all of the utility commands that blit supports is to avoid the speed issues with running checks over nfs/smb. find, rm, etc. used to be part of blit-utils, and they need to behave like the local versions, just run directly from the daemon. that way deleting thousands of files doesn't pay the network fs tax. find should likewise work the same way. look at the old plans for blit-utils to make sure its understood and we're building the correct thing."
- 2026-11-20: "okay, sanity check and improve the utils scope. I don't want to make this a whole side project, I just need blit to do what I stated last message. do local fs ops via daemon to avoid speed issues with network filesystems. take that as a general "plan this" task. that old plan was written months ago and you're far more capable than the models then were, so you have an open-ended mandate."
- 2026-11-20, on the draft's open points: Q1 — "mirror has a prompt, so why wouldn't rm?"; Q2 — "a"; Q3 — "a"; Q4 — "b"; and "activate". Then, on Q1 put again with options: "a" (keep the prompt; `-y` and `-f` both skip it).
- 2026-11-20: "a, then codex review the plan", naming "gpt-6.2-astra max"; and on the review's six changes and the six choices put with them: "adopt all recommended".

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
- **SIMPLE**: the reference is GNU findutils and coreutils under the `C.UTF-8` locale (characters, code-point order, Unicode case pairs — the locale every measurement here used), the same on every client and daemon platform; no option the reference lacks, except the `--json` each verb already has and `-y`, blit's answer to the deletion question.
- **A verb that deletes asks first** (Q1, D-2026-11-20-3; D-2026-11-20-4): blit's rule for `mirror` and `move` holds for `rm` and for `find … -delete` — one question before anything is sent, skipped by `-y`, and for `rm` also by `-f`, which means "never ask" in the local command. This is the one place these verbs follow blit and not the local command. Answering no stops the command with exit 0, as today; when NO answer can be read (end of input — a script that forgot `-y`), the command stops with exit 1 and says which option to pass. That exit rule is one rule for all four verbs that ask: `rm`, `find … -delete`, `mirror` and `move`, the last two of which exit 0 there today.
- **RELIABLE**: refuse, never ignore — an unsupported option, test or operator stops the command before anything is read or deleted. Every delete goes through `blit_core::deletion` and its containment. Exit status is the local command's: 0 only when everything asked for was done.
- **Real trees** (D-2026-11-20-4): an entry the daemon cannot read is reported in the local command's words and makes the exit status 1 while the walk goes on — the walker these verbs use today drops such errors in silence (`FileEnumerator::enumerate_local_streaming`). A symbolic link is an entry of its own and is never followed, as `find`, `du` and `rm` treat it by default. A name that is not valid text travels as its exact bytes beside its printable form, as transfer file headers already carry it, so what is printed, matched and deleted is the real name. A delete goes on past an entry that fails and reports each failure.
- **Same build only** (D-2026-07-05-2), enforced where it is not today (D-2026-11-20-4): the contract check runs only when a transfer session opens, and the utility requests skip it — an older daemon would ignore a new `recursive` field and delete a directory the new client never asked it to. Every utility request therefore moves to a request name an older daemon does not have (it answers "unimplemented", reported as the same-build refusal) and carries the contract version, which the daemon checks before it touches the disk, refusing an older or newer client. The old request names are removed in the same contract.
- **These verbs shipped in 0.1.x.** Their behavior changes at 1.0.0 and the changelog says so (Q2).
- **Platform truth**: the pure pieces (expression parser, name matcher, depth and size arithmetic) are guarded on every OS. Guards that compare against the system's own `find`, `du` and `df` run on Linux only, where the system command IS the reference; macOS ships the BSD commands, which differ in defaults, and Windows has none. A Windows daemon reports a file's length where Unix reports allocated blocks.

## Acceptance criteria

- [ ] **AC-1 (`df`)**: `blit df HOST:/module/path` reports the filesystem that holds THAT path, as the local command does for the path it is given — a path below the module that sits on another mount reports that mount, where today the path after the module name is dropped; on Linux, the numbers `df -k` prints for that path — as `Filesystem 1K-blocks Used Available Use% Mounted on` with the operand in the first column and the last column omitted; `-h` prints human-readable sizes. A module on a filesystem other than the root one reports its own numbers (the run above reported another disk's).
- [ ] **AC-2 (`du`)**: `blit du HOST:/module/path` prints one line per directory, deepest first, the operand last, each `SIZE<TAB>PATH` with the path beginning with the operand as typed; `-a` adds files; `-s` prints the operand's line only; `-d N` (and `--max-depth=N`) limits lines to N levels BELOW THE OPERAND; `-h` prints human-readable sizes. Sizes are allocated space in 1K blocks where the daemon's OS reports allocation (Unix), the file's length rounded up to 1K elsewhere. A file with several hard links is counted once, as `du` counts it — a backup tree of hard-linked snapshots is exactly where counting each name would mislead — on Windows too, by the file identity blit already reads there. A file given as the operand prints its own line without `-a`. On Linux the output equals the system `du`'s for the same tree and options, hard links included. No duplicate line for the operand.
- [ ] **AC-3 (`rm`)**: `blit rm [-r] [-f] TARGET...`. A directory without `-r` (`-R`, `--recursive`) is refused with "is a directory", deletes nothing and makes the exit status 1; a missing operand is an error and exit 1 unless `-f`; the other operands are still processed; exit 0 only when every operand was removed (or absent under `-f`). Before anything is sent it asks once, naming the operands, as `mirror` and `move` ask (Q1); `-y` skips the question as today and so does `-f`, so `blit rm -rf` behaves as `rm -rf` does and a bare `blit rm -r` asks. A whole module and a read-only module are refused as today.
- [ ] **AC-4 (`find`, tests)**: `blit find TARGET [-name PAT] [-iname PAT] [-type f|d|l] [-maxdepth N]`, tests ANDed. Output is one path per line, each beginning with the operand as typed, nothing else. `-name` matches the base name with `*` (any run of characters, a leading dot included), `?`, `[...]` and `\`, exactly as the reference does (D-2026-11-20-4): a name matches when the pattern fits it read as characters OR read as bytes, so against `a.dat`, `ab.dat`, `abc.dat`, the composed `é.dat`, `éa.dat` and `€.dat`, `?.dat` finds `a.dat`, `é.dat`, `€.dat`; `??.dat` finds `ab.dat`, `éa.dat`, `é.dat`; `???.dat` finds `abc.dat`, `éa.dat`, `€.dat` (GNU find 4.11.0, `C.UTF-8`, run 2026-11-20). `-iname` ignores case as the reference does, one character at a time by its lowercase form: `-iname 'übung*'` finds `ÜBUNG.txt`, and `-iname 'σ.sig'` finds `Σ.sig` and `σ.sig` and NOT `ς.sig` (same run) — this is find's rule, not the fold `--ignore-case` uses for transfers, which pairs all three. Any other test, action or operator (`-o`, `!`, parentheses, `-exec`, …) is refused by name before the walk starts. On Linux the lines equal the system `find`'s for the same tree and expression, as sets. `--pattern`, `--case-insensitive`, `--files`, `--dirs` and `--limit` are refused with the spelling that replaces each.
- [ ] **AC-5 (`find`, acting)**: `-mtime [+|-]N` and `-size [+|-]N[ckMG]` with the reference's rounding; `-delete` must be the last word of the expression and is refused anywhere else — in the reference its position changes what is removed (`find d -delete -name '*.tmp'` removed the whole of `d`, run 2026-11-20). It asks once before the request is sent unless `-y` is given (owner, D-2026-11-20-4), then tries each match, contents before their directory, in bounded batches, going on past an entry that fails; it prints nothing but errors, leaves a matched directory that is not empty with an error and exit 1, never removes the module's root, and is refused on a read-only module. Running the same expression without `-delete` prints every entry `-delete` would TRY to remove; the two sets differ exactly by the entries whose removal fails.
- [ ] **AC-6 (`ls`)**: `blit ls TARGET` prints names one per line, sorted by code point (`A`, `a`, `b`), hiding names that begin with `.`; `-a` shows them, with `.` and `..` as `ls` prints them; `-l` prints type letter (`-`, `d`, `l`), size, modification time and name, a symbolic link with ` -> target` (no permission or owner columns — a listing does not carry them, and the manual says so); `-h` with `-l` prints human-readable sizes. A file operand prints that file's line, and a directory holding one entry prints that entry: the listing says which the operand was, which today's reply cannot.
- [ ] **AC-7 (speed)**: the 200,000-file run above is repeated before and after each slice that touches a walk and recorded under `docs/bench/`; `find` with no test that needs it calls no `stat`, and `du` without `-a` sends no file rows — each pinned by a guard that counts, not by a stopwatch.
- [ ] **AC-8 (refusal, every verb)**: an option outside the lists above exits non-zero naming the option, before any request is sent.
- [ ] **AC-9 (words)**: `--help`, `docs/cli/blit.1.md`, `README.md` and `CHANGELOG.md` describe each verb as its Unix namesake with the supported options listed; `docs/plan/BLIT_UTILS_PLAN.md` points here.
- [ ] **AC-10 (the question, D-2026-11-20-4)**: `rm` without `-y` or `-f`, `find … -delete` without `-y`, `mirror` and `move` without `-y`: answering no deletes nothing and exits 0; with standard input closed each deletes nothing, exits 1 and names the option to pass. Today all of them print "Aborted." and exit 0 there.
- [ ] **AC-11 (same build, D-2026-11-20-4)**: a client and a daemon of different contract versions refuse every one of the five verbs, in both directions, before anything is read or deleted — a newer client against an older daemon, and an older client against a newer daemon — each guarded. Both remote `move` paths that remove the source ask for a recursive removal explicitly and report the source removed only when every path's result says so.
- [ ] **AC-12 (real trees, D-2026-11-20-4)**: with one unreadable directory in the tree, `find` and `du` print the rest, report that directory and exit 1; a symbolic link to a directory is listed and counted as a link and nothing under its target is visited or removed; a name that is not valid text is printed, matched and removed as the name it is; on Windows a directory that `-delete` could not remove, because it is not empty, leaves the read-only attribute of everything inside it as it was.

## Design

### The rule, in one place

A small table per verb — the options accepted and what each maps to on the wire — drives parsing, the refusal message and the help text, so the three cannot disagree. `find`'s expression and the single-dash options are parsed by hand after clap has taken the target (clap has no single-dash long options).

### The same-build gate (AC-11)

One new request per verb, under names an older daemon does not serve, each carrying `contract_version`; the daemon compares it before any filesystem call and answers with the refusal a transfer gives. `Purge`, `Find`, `DiskUsage`, `FilesystemStats` and `List` are removed in the same contract, so an older client meets "unimplemented" against a newer daemon and a newer client meets it against an older one. `move`'s two source-removal paths (`crates/blit-cli/src/transfers/mod.rs`, through `delete_remote_path`) move to the new removal request with `recursive` set and read its per-path results. The mechanism is the drafting agent's; the requirement is AC-11.

### `df` (`crates/blit-daemon/src/service/admin.rs` `filesystem_stats_for_path`)

Ask the operating system about the path itself — `statvfs` on Unix, `GetDiskFreeSpaceExW` on Windows — in place of the `sysinfo` mount-list match. Total is `f_blocks`, used is `f_blocks − f_bfree`, available is `f_bavail`, as `df` computes them. The request carries the path below the module, which `crates/blit-cli/src/df.rs` drops today, and the reply gains `available_bytes` (today's `free_bytes` cannot say which of the two it is).

### `du` (`disk_usage` in the same file)

Depth is counted from the start path. The daemon sends only the rows that will be printed (directories within the depth, files only under `-a`), each with allocated bytes (`st_blocks × 512` on Unix, the directory's own blocks included; the length elsewhere). A file whose link count exceeds one is remembered by its identity and counted the first time only — device and inode on Unix, the volume and file index blit already reads on Windows (`crates/blit-core/src/transfers/filter.rs`) — and the set holds only such files, as `du`'s does. A file operand yields its own row. The request gains `all_files` and a depth with "unlimited" distinct from 0; each row gains `allocated_bytes`.

### `rm` (`crates/blit-cli/src/rm.rs`, `crates/blit-core/src/admin/rm.rs`, the daemon's purge)

The new removal request carries `recursive` and `force`; the daemon — which is the side that knows what each path is — refuses a directory without `recursive` and reports each operand's outcome, so the CLI can print the local command's message per operand and set the exit status. Operands are grouped by daemon and module, one request each. The prompt stays (Q1): the existing `confirm_destructive_operation` question, asked once for the whole command before any request is sent, skipped by `-y` or `-f`. That function gains the end-of-input rule (AC-10) for every caller, `mirror` and `move` included.

### Deletion on real trees (AC-12; `crates/blit-core/src/deletion.rs`)

`execute_deletion_plan` returns at the first failure and, on Windows, clears read-only attributes through a directory's whole subtree before either kind of directory removal. For these verbs it gains a mode that records a failure and goes on, and the attribute clearing moves to the recursive mode only, so an empty-only removal that fails changes nothing beneath it. Matches are deleted in bounded post-order batches, not collected whole.

### `find` (`crates/blit-cli/src/find.rs`, the daemon's `find`)

- **Matcher**: an `fnmatch` in `blit-core` (`*`, `?`, classes, `\`), on the base name, that accepts a name when the pattern fits it as characters or as bytes — the reference's observed rule (AC-4); a name that is not valid text has only the byte reading. `-iname` compares one character at a time by lowercase form. It is not the transfer filters' glob engine: transfer patterns follow rsync's rule (`docs/plan/EXCLUDE_CASE_MATCHING.md` Q1, Q10), `find` follows find's — each verb answers to the tool it is named after.
- **Walk**: the error-capturing enumeration (`enumerate_local_streaming_capturing`), its suppressed errors sent to the client as they occur; symbolic links reported as entries and never followed; `stat` only when a test (`-size`, `-mtime`, or `-type` where the directory entry does not carry the type) needs it; entries sent in batches, each with its exact bytes when the name is not valid text.
- **`-delete`**: accepted only as the last word; matches go depth-first, in bounded batches, to `blit_core::deletion` in its empty-only directory mode, so a directory goes only when empty (see Deletion on real trees).
- **Wire**: the new request carries the name pattern as typed with its case flag, the kinds wanted, `max_depth`, the size and time predicates and `delete`; the old `pattern` field goes with the old request.

### `ls` (`crates/blit-cli/src/ls.rs`, the daemon's listing)

The new listing reply says what the operand is (file, directory or symbolic link) and gives each entry its kind and, for a link, its target; today's reply carries only a directory flag and cannot tell a file operand from a directory holding one entry. Printing follows from that.

### Alternative considered: have the daemon run the native commands

Owner, 2026-11-20: "unless it's simpler to just call the native equivalent commands. windows doesn't have analogs to find, du, df, etc., but delete does. or we could call powershell. that doesn't sound simpler, but if it is, it's an option." The drafting agent answered that it is not simpler, for the reasons below, and told the owner that activating the plan would accept that answer; the owner activated without objection.

- **It adds backends instead of removing one.** Windows has no `find`, `du` or `df`, so the daemon's own implementation is needed there whatever Unix does. Calling native tools would mean that implementation plus a GNU caller, a BSD caller for macOS (different defaults and option spellings) and whatever a NAS ships, each with its own output to parse — and the same `blit find` would answer differently from one daemon to the next.
- **`find` runs programs.** `-exec`, `-ok` and the `-f…` actions execute commands and write files, so a user's expression could not be passed through: the daemon would have to parse and allow-list it first, which is the parser this plan already needs, and one gap in that list would be command execution on the daemon's machine. No password or token check was found in the daemon's source (its authorization is by host address).
- **Deletion would leave the reviewed engine.** `rm` goes through `blit_core::deletion` and its per-entry containment today; a spawned `rm` or `Remove-Item` would not.
- **PowerShell is slower, measured** (2026-11-20, PowerShell 7.6.3 on Linux, the same 200,000-file tree; Windows itself not measured): starting it costs 51 ms against 1 ms for `find`; listing took 2,216 ms (`Get-ChildItem -Recurse`) against 55 ms for `find` and 361 ms for blit today; deleting took 728 ms (`Remove-Item -Recurse`) against 257 ms for `rm -rf` and 254 ms for blit.
- **A service's locale changes the answers.** The measurements above show GNU find pairing `Ü` with `ü` and counting characters only in a UTF-8 locale; a daemon started by the system often runs in the plain one.
- **What the instinct gets right is already the design.** Where the operating system has the answer, ask it: `df` becomes one `statvfs` / `GetDiskFreeSpaceExW` call, and deletion already uses the system's own delete calls at local speed. The rest is a walk blit already has (`FileEnumerator`) plus option parsing and printing, which a native caller would need too.

### Risks

- **`rm -rf` deletes without asking**, as the local command does; a bare `rm -r` asks (Q1).
- **`mirror` and `move` change too.** With no answer available they exit 1 where they exit 0 today (AC-10): an unattended script that forgot `-y` and was silently doing nothing now fails loudly.
- **Every utility request is renamed** (AC-11), so any client built against the old names — the admin UIs included — must be rebuilt with the daemon, which the same-build rule already demands.
- **A second pattern matcher.** It is small and compared against the system `find` on Linux, but it is one more thing that decides what gets deleted once `-delete` exists, and its two-readings rule is the reference's observed behavior, not a documented one.
- **0.1.x scripts break** on `find`'s old options and on `rm` of a directory without `-r` — loudly, with the replacement named (Q2).
- **The Linux-only comparison guards prove Linux.** macOS and Windows daemons are covered by the pure guards and by their own end-to-end runs; this is stated wherever a result is reported.

## Slices

One coherent, testable change per slice — sized for the `.review/` loop. Each ships alone. Slices 1 to 3 fix what is wrong today; 4 to 6 are the rest of the parity. All six land before the 1.0 tag (Q4, owner "b").

1. **`df` and `du` say the right thing** (AC-1, AC-2, AC-7 for `du`, and the same-build gate of AC-11 for these two requests): the two wrong answers measured above, their output shape and options, `df`'s path, hard links on both platforms, an unreadable directory under `du` (AC-12). Wire change, contract bump.
2. **`rm` and the question** (AC-3, AC-10, AC-11 for removal and for `move`'s cleanup): `-r`, `-f`, several operands, the one question before deleting that `-y` or `-f` skips (D-2026-11-20-3), the end-of-input rule for `rm`, `mirror` and `move`, the local exit status, deletion that goes on past a failure. Wire change, contract bump.
3. **`find`, tests and output** (AC-4, AC-7 for `find`, AC-11 and AC-12 for `find`): the expression parser, the matcher, path-per-line output, links and unreadable entries, exact names, the old options refused. Wire change, contract bump.
4. **`find` acts** (AC-5, AC-10 for `-delete`, the Windows half of AC-12): `-mtime`, `-size`, `-delete`.
5. **`ls`** (AC-6, AC-11 for listing). Wire change, contract bump.
6. **Words** (AC-9) — folded into each slice for its own verb; this slice is the manual page's overall rewrite and the pointer from the old plan.

AC-8 is met verb by verb as its slice lands.

**Slice 1 LANDED 2026-11-21** (Linux gate; acceptance boxes are the owner's to tick). `blit du` and `blit df` are the Unix commands, run on the daemon's own disk:
- **`df`** (AC-1) asks the operating system about the path itself (`statvfs`; `GetDiskFreeSpaceExW` on Windows) and prints `df -k`'s line with the operand in the first column; the path below the module is no longer dropped; `-h`; `--json` gives total, used and available. The `sysinfo` dependency is gone.
- **`du`** (AC-2) prints `SIZE<TAB>PATH`, an entry after everything below it and the operand last and once, in 1K blocks of allocated space; `-a`, `-s`, `-d N`/`--max-depth=N` counted from the operand, `-h`; a file with several hard links counted once; a file operand prints its own line. On Linux its output is the system `du`'s byte for byte, once the operand is renamed, hard links included.
- **Real trees** (AC-12, for `du`): a directory that cannot be read is reported in `du`'s words, the rest is printed and the exit status is 1; a link is counted as a link; a name that is not UTF-8 is printed by its bytes.
- **Same build** (AC-11, for these two): the requests are `Du` and `Df`, names an earlier daemon does not serve, each carrying the contract version, which the daemon compares before it reads anything; `DiskUsage` and `FilesystemStats` are gone. **Wire contract 13.** Guarded both ways: a raw request one contract below and one above is refused naming both versions, and a stand-in daemon that does not know the names is reported as another build.
- **Refusal** (AC-8): an option neither command has is refused before anything is asked of a daemon.
- **Speed** (AC-7, `docs/bench/uv1-du-df-2026-11-21/`): the total over 200,000 files took 882 ms and takes 154 ms (system `du -s`: 130 ms); with `-a`, 233 ms (system: 199 ms). `du` without `-a` sends no row for a file, pinned by a guard that counts rows.
- **Words** (AC-9, for these two): `--help`, README, `docs/cli/blit.1.md`, CHANGELOG, release notes.
- Guards: `crates/blit-core/src/disk_usage.rs`, `admin/same_build.rs`, the daemon's `admin.rs`, `crates/blit-cli/tests/du_df.rs` (seven end-to-end, two against the system commands on Linux), `remote_remote.rs`. Twenty mutations, each red: nineteen by the guards predicted, one (1K blocks rounded down) by its unit guard alone.
- Not done here, and known: nothing ran on Windows or macOS (a Windows daemon reports a file's length where Unix reports allocated blocks; counting a hard-linked file once costs an open per file there, unmeasured). A link given as the operand WITH a separator after it (`du dir/`) is counted as the link; `du` follows it there, as `find` does from slice 3.

**Slice 2 LANDED 2026-11-21** (Linux gate). `blit rm` is the Unix command, run on the daemon's own disk, and the question every deleting verb asks has one rule:
- **`rm`** (AC-3): `blit rm [-r] [-f] TARGET...`. A directory without `-r` (`-R`, `--recursive`) is refused with `Is a directory` and nothing in it is touched; an operand that is not there is an error unless `-f`; the other operands are processed all the same; exit 0 only when every operand is gone; nothing printed for what was removed, `cannot remove 'OPERAND': REASON` for each entry that was not, with the operand as typed. A removal goes on past an entry it cannot remove and leaves the directories above it standing without a second line, as `rm -r` does. On Linux the messages, the exit status and what is left on disk equal the system `rm`'s over ten cases (a directory with and without `-r`, missing operands with and without `-f`, a link to a directory, a tree holding a directory that cannot be written).
- **The question** (AC-3, AC-10): `rm` asks once for the whole command, before anything is sent, naming the operands; `-y` skips it and so does `-f`. For `rm`, `mirror` and `move` alike, "no" stops the command with exit 0 and NO ANSWER (standard input closed) stops it with nothing done, exit 1, naming the option to pass — where all three printed "Aborted." and exited 0 (`crates/blit-cli/src/confirm.rs`).
- **Same build** (AC-11): the request is `Rm`, which carries `recursive` and `force` and the contract version, compared before anything is removed; `Purge` is gone. **Wire contract 14.** Guarded both ways, as for slice 1. `move`'s two source removals ask through it for a recursive removal by name and report the source removed only when every entry went (guarded with a source holding a folder that cannot be written: everything is copied, the removal goes on past it, the command fails naming the entry).
- **Real trees** (AC-12, for `rm`): a link is removed as the link, at the operand and below it, and nothing it points to is touched; a name that is not UTF-8 is removed as the name it is. A whole module and a read-only module are refused; the daemon refuses a module's root whoever asks.
- **Refusal** (AC-8): an option `rm` has and `blit rm` does not (`-i`, `-v`, `-d`, …) is refused by name before anything is asked.
- **Speed** (AC-7, `docs/bench/uv2-rm-2026-11-21/`): 200,000 files removed in 256 ms (before: 246 ms; system `rm -rf`: 249 ms). As first written the removal took 296 ms, removing each entry by its whole path; it now removes by name from the directory held open, as `rm` does.
- **Words** (AC-9, for `rm`): `--help`, README, `docs/cli/blit.1.md`, CHANGELOG, release notes — including the changed exit status of an unanswered `mirror` or `move`.
- Guards: `blit_core::deletion` (five), `crates/blit-cli/src/rm.rs`, `crates/blit-cli/tests/rm.rs` (eight end-to-end, one against the system `rm` on Linux), `remote_remote.rs`, `case_note.rs`. Twelve mutations, each red by the guards predicted.
- Not done here, and known: nothing ran on Windows or macOS (there entries are removed by path, and a read-only tree is cleared before a recursive removal, as before). An operand that is not valid UTF-8 cannot be typed (the command line takes text). `--json` reports `entries_removed`; nothing else counts them.

## Review

`plan-utility-verbs-2026-11-20-r1` (Codex openreview, owner-directed; `REVIEW.md`, `.review/results/utility-verbs-plan-r1.codex.md`): **acceptable_with_changes**, six material changes. The orchestrator checked each against the code and the reference commands; the owner adopted all six with the recommended choices ("adopt all recommended", D-2026-11-20-4). Where each landed: (1) the same-build gate — Constraints, AC-11, Design; (2) the question — Constraints, AC-10, slice 2, with `find -delete` now ruled by the owner; (3) `find` — AC-4 and AC-5 re-pinned to what GNU find does, `-delete` last; (4) real trees — Constraints, AC-12, Design; (5) `ls`, `df`, `du` wire — AC-1, AC-2, AC-6, Design; (6) the transfer filter's `--ignore-case` — `docs/plan/EXCLUDE_CASE_MATCHING.md` Design §Matching.

## Open questions

- ~~**Q1 — does `rm` ask before deleting?**~~ — **settled 2026-11-20 (owner "a", D-2026-11-20-3)**: it keeps the prompt; `-y` and `-f` both skip it. As put: the draft removed the prompt to match the local command. Owner, 2026-11-20: "mirror has a prompt, so why wouldn't rm?" — blit's own rule is that a command that deletes asks unless `-y` (`mirror`, `move`), and the draft set that aside for `rm` alone. The drafting agent's revised recommendation: keep the prompt, on the same footing as `mirror`'s; `-y` skips it as today, and so does `-f`, which means "never ask" in the local command — so `blit rm -rf` behaves exactly as `rm -rf` does, and a bare `blit rm -r` asks. The other options put: only `-y` skips it; or no prompt.
- ~~**Q2 — the old spellings**~~ — **settled 2026-11-20 (owner "a")**: `find`'s old options and `rm` of a directory without `-r` are refused with a message naming the replacement; nothing is kept working hidden.
- ~~**Q3 — comparing a local tree with a daemon's tree**~~ — **settled 2026-11-20 (owner "a")**: not in this plan; making `--dry-run` work for a transfer to or from a daemon gets a plan of its own, which also decides what `check` is for. That plan is owed and not yet drafted. (`check` takes the transfer verbs' pattern options meanwhile: `docs/plan/EXCLUDE_CASE_MATCHING.md` Q11.)
- ~~**Q4 — before or after the 1.0 tag**~~ — **settled 2026-11-20 (owner "b")**: all six slices before the tag. The drafting agent had recommended 1 to 3 before and 4 to 6 after.
