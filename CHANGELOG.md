# Changelog

All notable changes to Blit are documented in this file.

## [1.0.0] - 2026-10-14

Blit 1.0: a transfer that one bad file cannot end, on either side of the
wire, and a log and a job for every run. Covers everything since `v0.1.3`
(tagged 2026-08-20). User-facing notes: `docs/RELEASE_NOTES_1_0.md`.

**Major version.** `blit-core`'s public API changed (review finding
cr-ssc2-2): `PreparedPayload::TarShard` gained `skipped` and
`PreparedPayload::Skipped` was added; `build_tar_shard` returns
`TarShardBuild`; `TransferSource::open_file` returns `OpenedSourceFile` and
`check_availability` was removed; `TransferSink` gained
`begin_record`/`RecordWriter`, `can_store_raw_names` and `note_raw_name`;
`TransferSummary` gained `failed_paths`/`failed_paths_truncated`;
`transfers::filter::FilterInputs` gained `source_root`, `ignore_case`,
`files_from_read` and `resolved`; `fs_enum::FileFilter` gained
`ignore_case` and `files_from_unnamed`, and `TransferSource` a provided
`files_from_unnamed`; `client::connect_with_timeout` and
`connect_transfer_client` return a `BlitClient<client::BlitChannel>`
(the channel behind `client::BoundedResponses`). The wire
protocol is contract 18: a peer on any other contract version (every 0.1.x
release) is refused at session open, by design; two 1.0.0 builds from any
channel talk to each other.

### Reliability: a file the source cannot deliver no longer ends the run

Previously any source-side per-file failure — a file locked or vanished
after the scan, a name that is not valid UTF-8, a file whose size changed
while it was being read — ended the whole transfer; a growing file inside
a small-file batch could even corrupt the batch and abort it (`tar shard
entry: numeric field was not a number … cksum`).

- Such a file is now skipped (before its record is announced) or
  retracted (after), recorded through the same per-file failure report as
  destination-side failures, and the rest of the manifest lands. Reasons
  recorded by the source start with `source:`. Nothing is ever finalised
  under a header its bytes do not match: single-file records end with an
  explicit ok/failed terminator on both carriers, and small-file batches
  are packed from exactly the bytes the manifest promised, so one
  changing file can never corrupt its neighbours.
- **End-of-run retry passes.** Files that failed are retried automatically
  after the main pass, re-scanned fresh: `--retries <N>` (`-R`, default 1,
  `0` disables) passes, `--retry-wait <SECONDS>` (`-W`, default 30) before
  each. Files that land on retry count once; survivors are reported once
  with `(retried)`. Mirror deletions run at the end of the main pass and
  never touch the subtree under a failed path. A `--detach` job runs on
  the daemons and has no retry passes; the command says so once.
- **Filenames that are not valid UTF-8 transfer**, rsync-style: the exact
  bytes travel beside the text, a Linux (or other byte-keyed) destination
  creates the real name and the file converges on re-run; a destination
  that cannot store the bytes (Windows; macOS) reports the file as
  `source: filename is not valid UTF-8; the destination cannot store it
  (rename it to transfer)` and continues. Two source names that collapse
  to one text name are reported as a lossy-name collision instead of
  silently overwriting each other. This retires the 0.1.2 known
  limitation "Non-UTF-8 source filenames fail to transfer".
- The local route no longer refuses a whole mirror because a file became
  unreadable between the scan and the copy; that file is reported and the
  mirror completes. A file that vanished before the copy on the local
  route is likewise reported, not fatal.
- A file that cannot be opened when the scan reaches it — on Windows, a
  file held open with no sharing, such as `NTUSER.DAT` or a live database
  — no longer ends the run at scan time. It is listed, reported per file
  (`source: cannot open: …`) if it still cannot be read, retried, and its
  destination counterpart is kept under mirror. A permission refusal at
  scan is handled the same way (D-2026-10-04-1), so one access-denied file
  no longer blocks a mirror's deletions.
- Wording: the end-of-run block now reads "N file(s) did not land at the
  destination:", and the `move` source-delete refusal says the same,
  since a failure may now be the source's.
- Protocol contract 6 → 7 (`docs/TRANSFER_SESSION.md`).

### Job logs, run records and saved jobs

Every run now leaves a record of what it did, on every machine that took
part, and a job you can list, read, re-run and retry.

- **A log for every run.** Local copies, pushes, pulls and remote-to-remote
  jobs each write a compressed JSON log (`.jsonl.gz`) as the run goes:
  every file copied, deleted and failed with its reason, what `-v` would
  have shown, phase start and end times, stall notices. The machine where
  the command was typed keeps its log in the per-user folder
  (`~/Library/Application Support/com.Blit.Blit` on macOS, `~/.config/blit`
  on Linux, `%LOCALAPPDATA%\Blit` on Windows); each daemon involved keeps
  its own under its state directory. One job ID, created where the command
  was typed, names the run on every machine. A run killed mid-way leaves a
  log that the next start finalizes as interrupted. Names that are not
  valid UTF-8 are logged by their exact bytes.
- **`blit jobs list [<host>]`** shows recent runs (and saved jobs) with
  their outcome and failed-file count; **`blit jobs log [<host>] <job-id>`**
  shows a log as text, or the raw events with `--json`. A `--detach` job's
  outcome is fetched from the daemon that ran it; **`blit jobs watch`**
  exits 1 and names the failed files when a detached job has any.
- **Saved jobs.** `blit copy|mirror|move … --save <name>` keeps the run as a
  saved job; `--export <file>` writes it to a file (both together work).
  `blit jobs save <job-id> <name>`, `blit jobs export <job-id|name> <file>`,
  `blit jobs run <name|file>` and `blit jobs delete <name>` manage them. A
  saved job reproduces its run from any working directory, with the
  `--files-from` list it ran with. A command refused for what it says —
  an option the verb does not take, a filter that cannot be used, a source
  and destination that are one path — saves and exports nothing, so a job
  already saved under that name stays as it was.
- **`blit jobs retry <job-id|file>`** starts a new run that sends exactly
  the files that failed, with the original options (a `move` runs whole
  again). It refuses a job from another machine, a run still going or
  waiting, and a run whose failures are not all named.
- **Retention.** Each machine keeps its newest 50 logs and 50 run records;
  `[jobs] keep` in the CLI's per-user `config.toml` and in the daemon's
  `config.toml` changes it. Saved jobs are never pruned.
- **Windows** keeps settings, jobs and logs under `%LOCALAPPDATA%\Blit`
  (the local profile, not the roaming one); files found in the old roaming
  folder are moved once.
- Logging costs no measurable time: a command's start touches no disk and
  the log closes alongside the run's record. If the log directory cannot
  be written the transfer still completes and the report says the log is
  incomplete.
- A dry run or `--null` run is logged as what it planned, not as copies.
- Wire protocol (contract 18): the job ID rides the session-open and
  delegated-pull requests; `GetJobLog` returns a daemon's log by ID; the
  finished-job record carries the failed-file count. In a filtered mirror,
  `ManifestPruned` tells the destination which source paths were excluded,
  so it protects their counterparts in the destination's own spelling. The
  filter at session open carries the run's case mode (`--ignore-case`), so
  a daemon at either end reads every pattern as the command did.

### Patterns: exact by default, `--ignore-case`, pattern files

- A pattern matches exactly as typed on every machine: `--exclude
  '*cache'` names `cache`, not `Cache`. `--match-case` states the default;
  `--ignore-case` ignores capital letters in every `--exclude` and
  `--include` pattern of the run — on the source walk, in a `mirror`'s
  deletion scope, and on a daemon at either end. Letters pair by Unicode
  simple case folding (`Übung`/`übung`, the sigmas; never `ß` with `SS`).
  `copy`, `mirror` and `check` take both options; a saved job keeps the
  one typed.
- When a `mirror` asks before deleting and no mode was typed, the question
  starts with one line built from your pattern: `"*Cache" matches Cache
  only; capital letters matter (--ignore-case would also match cache,
  CACHE).` `copy` prints it once on stderr and goes on. No line with
  either mode typed, with `-y`, or when no pattern holds a letter. The
  line describes the patterns the run uses: an exclude that cannot match
  anything is not spoken of, and an absolute exclude is described as what
  it names under the source. Where the option would make a pattern match
  LESS — a negated class such as `[!x]`, which then refuses `X` too — the
  line says so instead of showing a spelling: `(--ignore-case changes
  what it matches: [!x] would stop matching X).` A loss is shown before
  any gain; where a class could take less and no file shows it, the line
  says it `may`. An example — a spelling, a letter — is shown only when
  one name, put to the run's whole filter in both modes, is treated
  differently; otherwise the line states the rule and no more (`"a"
  matches exactly as typed; capital letters matter (--ignore-case ignores
  them).`, for `--exclude a --exclude A`). Only a pattern that reads the
  same both ways, such as `{a,A}`, is left out of it. Under `--delete-scope
  all`, where a filter chooses what is copied and protects nothing at the
  destination, the question says that instead.
- `?` and a bracket class match one byte in both modes. A class holding a
  letter beyond ASCII (`[aé].txt`) is refused with the spelling that works
  (`{a,é}.txt`), on the command line, in a pattern file, from a saved job
  and by a daemon at session open. A name that is not valid UTF-8 is now
  matched by its own bytes; it used to be matched through a lossy copy in
  which one bad byte counted as three, so `?.txt` did not name it and a
  `mirror` could delete it from under an exclude.
- `--exclude-from FILE` and `--include-from FILE` (repeatable) read one
  pattern per line: both ends of a line are stripped, blank lines and `#`
  comments skipped, a space wanted at an edge written `[ ]`. An rsync rule
  line (`- pattern`, `+ pattern`, `!`), a missing file, or text that is not
  UTF-8 refuses the run, naming the file and the line. A saved job keeps
  the patterns read, not the file.
- A `--files-from` list that holds no name (a file of comments) selects
  nothing at a daemon, as it does locally. It used to reach the daemon as
  no list at all, and the daemon applied the patterns the list overrides.
- A `--files-from` name reaches a daemon as the folders and file it names,
  whichever way the machine that read the list writes a path. A list read
  on Windows sent `sub\name.txt` as typed; a Linux or macOS daemon read
  that as one file name, missed the file in `sub`, and on a mirror could
  select an unrelated file called exactly that for deletion. A line that
  is no relative path of names (`./x`, `../x`, `/abs`) still names
  nothing and is still reported as a file that did not land.
- A source that is ONE FILE is filtered under that file's name
  everywhere. `check` asked its filter about an empty path instead: with
  the file named in `--files-from` it compared nothing and reported
  success on two different files, and with an exclude naming the file it
  compared what `copy` left out. `copy` with the file named in
  `--files-from` copied it and then exited 2 reporting it missing.
- A single-file copy whose file fails and is retried (`--retries`) is
  retried: the retry looked for it under an empty name, sent nothing,
  reported `source: missing at retry` and exited 2 with the file there
  and its destination free. `blit jobs retry` of such a run likewise.
- `check` of a single file against a destination of another name
  compares the two. With the source named in `--files-from`, or a
  pattern naming either file, it looked for one file's name on the
  other side and reported the copy "Missing on destination".
- An `--exclude` reported as unable to match anything (an absolute path
  outside the source, or any absolute path when the source is on a
  daemon) is no longer part of the filter or sent to the daemon. It was
  sent as typed, and a daemon on another kind of machine could read the
  same text as a pattern: a Windows `\keep` is the name `keep` to a
  Linux daemon, which then excluded it.
- A filter that cannot be used — a pattern that is no glob or cannot be
  compiled, an unreadable size or age, a pattern file that cannot be read —
  is refused before `mirror` asks its question; the question used to come
  first. A daemon applies the same validation at session open. So is an
  `--exclude` whose rule cannot be compiled once its trailing `/` is
  dropped (`keep\/` on Linux and macOS, an escaped slash): it used to be
  matched in the typed case only, whatever the mode, and a `mirror
  --ignore-case` deleted `KEEP\` from under it.
- An `--include` is matched as a glob and nothing else: `--include
  '[a].txt'` no longer also names a file spelled `[a].txt`.
- A job file edited to hold two options no command takes together
  (`--match-case` with `--ignore-case`, `--checksum` with `--size-only`, …)
  is refused instead of run.
- Each distinct set of patterns is compiled once per process and shared: a
  command with 250 excludes starts in about half the time it took before.
- Under `mirror`, a glob names an entry by its spelling: `--exclude '*ß'`
  does not keep a destination-only `bSS`. This was listed as a known
  limitation; it is the rule, the same on every disk and the one rsync
  follows.

### `du` and `df` behave like the commands they are named after

**Behaviour change from 0.1.x.** The utility verbs exist so that a job on
a server's files runs on the server's own disk, in one request, instead
of paying a network filesystem's cost per file. From 1.0.0 each also
behaves like the Unix command it is named after; `du` and `df` are the
first two, `rm` the third, `find` the fourth (`ls` follows).

- **`blit df HOST:/module/path`** reports the filesystem that holds THAT
  path, asked of the operating system directly, as `df -k` prints it —
  `Filesystem 1K-blocks Used Available Use%`, the path as typed in the
  first column. It used to drop the path, look the module up in a list of
  disks and take the longest mount-point match: a module on its own
  filesystem could be reported with another disk's numbers. `-h` prints
  human-readable sizes. `--json` gives `total_bytes`, `used_bytes` and
  `available_bytes` (the old `free_bytes` could not say which it was).
- **`blit du HOST:/module/path`** prints `du`'s lines: one per directory,
  deepest first and the path given last, `SIZE<TAB>PATH` with each path
  beginning with what was typed, sizes in 1K blocks of allocated space.
  `-a` adds files, `-s` prints the total alone, `-d N` / `--max-depth=N`
  limits the lines to N levels below the path given, `-h` prints
  human-readable sizes. A file with several hard links is counted once.
  It used to print a `PATH BYTES FILES DIRS` table with a row for every
  file, the path given twice, byte lengths, and a depth counted from the
  module's root. On Linux the output equals the system `du`'s for the same
  tree and options.
- A directory the daemon cannot read is reported in `du`'s own words, the
  rest is still printed, and the exit status is 1. A name that is not
  valid UTF-8 is printed by its own bytes. A symbolic link is counted as
  a link and never followed.
- On Windows, `df` of a path that is not there says so; it used to
  answer with the numbers of the folder above it. (Fixed from the source;
  not yet run on Windows.)
- `-h` on these two commands is the commands' own `-h`; help is `--help`.
  An option neither command has is refused before anything is asked of a
  daemon.
- The two requests are new on the wire (`Du`, `Df`) and compare the
  contract version before reading anything, so a client and a daemon of
  different builds refuse each other here as they do for a transfer; the
  old `DiskUsage` and `FilesystemStats` requests are gone. The daemon no
  longer depends on the `sysinfo` library.
- For a program built on `blit-core`: `admin::du::stream` now takes
  `UsageOptions` and yields `DuEvent` rows and failures, each with an
  `EntryPath` that keeps a name's exact bytes; `admin::df::query` takes the
  path and returns `FilesystemSpace { total_bytes, used_bytes,
  available_bytes }`; the walk and the size arithmetic are public as
  `blit_core::disk_usage`.

### `rm` behaves like `rm`; a question nobody can answer fails the command

**Behaviour change from 0.1.x**, for `blit rm` and for every command that
asks before it deletes.

- **`blit rm [-r] [-f] HOST:/module/path...`** treats each path as the
  Unix `rm` does. A directory is refused with `Is a directory` unless `-r`
  (`-R`, `--recursive`) is given — it used to delete a whole tree without
  being told to. A path that is not there is an error unless `-f` — it
  used to exit 0 ("path may already be absent"). Any number of paths may
  be given, each processed whatever became of the others — two used to be
  a usage error. A path the daemon cannot reach (below a directory it may
  not search) or will not remove (the module's root, a path that leaves
  the module) is that path's failure and no more. The exit status is 0
  only when every path is gone.
- It prints what `rm` prints: nothing for what was removed, and
  `cannot remove 'PATH': REASON` for each entry that could not be, with
  the path as typed. A removal goes on past an entry it cannot remove and
  leaves the directories above it standing, as `rm -r` does — it used to
  stop at the first. A symbolic link is removed as a link and never
  followed. A recursive removal holds each directory open and removes
  what is in it by name, from the module's root down: a directory swapped
  for a link while it runs is removed as the link, and cannot turn the
  removal elsewhere. (On Windows the tree goes by the standard library's
  own anchored removal, which stops at the first entry it cannot remove.)
  `--json` gives `entries_removed`, `failures` and `unfinished`. On Linux
  the messages, the exit status and what is left on disk equal the system
  `rm`'s for the same tree and paths.
- **Every entry that could not be removed is named**, as the daemon meets
  it — a tree with a million of them prints a million lines, as `rm`
  does. If a daemon's answer stops before it says it is done, what it
  reported is printed, then `HOST: the removal did not finish: …`, and
  the exit status is 1 (`unfinished` in `--json`).
- **A path is taken exactly as typed**, by `rm`, `du`, `df` and `find`
  alike: a `/` after it, a `.` as its last name, a space at either end.
  `rm` refuses a path ending in `.` or `..` in `rm`'s words (`refusing to
  remove '.' or '..' directory: skipping 'PATH'`), `-rf` or not, and goes
  on to the next — `blit rm -rf HOST:/m/work/.` used to delete `work`. A
  path with a `/` after it is removed only if it is a directory: a file is
  `Not a directory` and stays, which `-f` passes over — `blit rm
  HOST:/m/doc/` used to unlink the file. `report ` with a space after it
  is that file and not `report`. `du link/` and `find link/.` measure and
  search the directory a link points to, and say `Not a directory` of a
  file typed so. One place is deliberately not `rm`: `rm -r link/`, on a
  link to a directory, says `Not a directory` as `rm` does and removes
  nothing, where `rm` first empties the directory the link points to.
- **It still asks first**, once for the whole command and before anything
  is sent, naming the paths: `Remove 'a' and 'b'? [y/N]`. `-y` skips the
  question, and so does `-f`, which means "never ask" in `rm` itself — so
  `blit rm -rf` behaves as `rm -rf` does and a bare `blit rm -r` asks.
- **`mirror`, `move` and `rm`: when no answer can be read, the command
  fails.** "No" still stops it with nothing done and exit status 0. With
  standard input closed — under cron, or in a script that forgot `-y` —
  each of them used to print "Aborted." and exit 0, and a job that did
  nothing looked like one that had run. Each now does nothing, exits 1
  and names the option that skips the question. **A script that relied on
  that exit status must pass `-y`.**
- **`blit move` from a daemon** asks for the source's removal as a
  recursive removal, by name, and reports the source removed only when
  every entry of it went; otherwise it names an entry that stayed and
  fails.
- The request is new on the wire (`Rm`) and compares the contract version
  before it removes anything; the old `Purge` request is gone. An option
  `rm` has and `blit rm` does not (`-i`, `-v`, `-d`, …) is refused by
  name before anything is asked of a daemon.
- For a program built on `blit-core`: `admin::rm::remove` takes the
  module, the paths and `RmOptions` and returns an `RmReport` (entries
  removed, and each `RmFailure` with its operand, path and reason);
  `delete_remote_path` now returns the count removed and fails when any
  entry stayed; `blit_core::deletion::remove_as_rm` is the removal itself.

### `find` behaves like `find`

**Behaviour change from 0.1.x: `blit find` takes `find`'s expression, and
none of the options it had.**

- **`blit find HOST:/module/path [-name PATTERN] [-iname PATTERN]
  [-type f|d|l] [-maxdepth N]`**, every test given holding. It prints one
  path per line and nothing else, each beginning with the path as typed;
  the path itself is an entry and comes first, a directory before what is
  in it. It used to print a padded `PATH BYTES TYPE` table of paths
  relative to the module.
- **`-name PATTERN` is matched against an entry's own name, as `find`
  matches it**: `*` is any run of characters (a leading dot included),
  `?` any one, `[...]` one of a set (ranges, `!` or `^` to negate,
  `[:alpha:]`-style classes, each as `find` has it under `C.UTF-8`: an
  em space is a `[:blank:]`, a no-break space is no `[:space:]`, a digit
  of another script is an `[:alpha:]`), `\` takes the next character as
  itself. As
  in `find`, a name matches when the pattern fits it read as characters
  or as bytes: `?.dat` finds `é.dat`, and so does `??.dat`. **`-iname`**
  compares one character at a time by its lowercase form, as `find` does
  — `-iname 'σ.sig'` finds `Σ.sig` and not `ς.sig`. This is `find`'s rule
  and not the one `--ignore-case` gives a transfer's patterns, which
  pairs all three: each command follows the tool it is named after.
- **What `find`, `du` and `rm` print can be given back to them.** Below
  the default root, `HOST://`, an entry is printed `HOST://child` (it
  was `HOST:/child`, which names a module). And a name that is not valid
  UTF-8, which these commands print by its own bytes, is now a path they
  take: as an operand of `find`, `du`, `df` and `rm`, and as a `-name`
  or `-iname` pattern — `blit find HOST:/m/t -name '*.tmp' | xargs -d
  '\n' blit rm -y` removes it. `find` keeps every `/` typed after its
  path in what it prints below it (`dir//` prints `dir//sub`), as `find`
  does.
- **`-type f|d|l`** and **`-maxdepth N`** as in `find`. A symbolic link is
  an entry of its own (`-type l`) and is never followed — except the path
  given when it is typed with a `/` after it, which names the directory
  the link points to, as it does to `find`.
- **The old options are refused, each with what replaces it**:
  `--pattern X` → `-name 'X'` (it was a glob against the whole path below
  the module, with `**/`; `-name` is against the entry's name, at any
  depth); `--case-insensitive` → `-iname`; `--files` → `-type f`;
  `--dirs` → `-type d`; `--limit N` → pipe through `head -n N`.
- **Any other word of `find` is refused by name** before anything is
  asked of a daemon — `-o`, `!`, parentheses, `-newer`, `-size`, `-exec`,
  `-print`, a `-type` other than the three. A test that was dropped in
  silence would find more than was asked for.
- A directory the daemon cannot read is listed, reported in `find`'s
  words (`blit find: ‘PATH’: Permission denied`) and makes the exit status
  1; the walk goes on. It used to be left out without a word. A name that
  is not valid UTF-8 is printed, and matched, by its own bytes. `--json`
  gives `paths` and `failures`.
- The walk asks the filesystem nothing no test needs: an entry's kind
  comes with its directory's listing, and paths travel in batches. When
  the reader goes away (`blit find … | head -1`) the command stops
  without a message.
- The request is new on the wire (`FindPaths`) and compares the contract
  version before reading anything; the old `Find` request is gone.
- For a program built on `blit-core`: `admin::find::stream` takes a
  `FindQuery` (name tests, kinds, depth) and yields `FindEvent::Found`
  paths and failures, each an `EntryPath`; `FindParams` and the
  `FindEntry` row are gone. The walk and the matcher are public as
  `blit_core::find` and `blit_core::find_name`.

### Fixes

- A transfer of very many small or empty files through a daemon, and a
  listing (`blit find`, `blit du`) read slower than it arrives — through a
  pager, on a slow terminal — no longer ends with "transfer transport peer
  closed" or "h2 protocol error". The HTTP/2 library closes a connection
  that holds too many small frames unread, and every blit stream is made
  of small frames; the connection window, from which the library derives
  that limit, is now the largest HTTP/2 allows on both ends. Present on
  the development branch since 2026-10-17 and in no release. With it, a
  connection carries at most eight streams at once (a ninth request waits
  its turn), and a client holds at most eight responses it has not read
  to the end or dropped — a ninth request waits for one of them, whether
  or not their streams have ended. That bounds what one connection can
  hold unread: 8 MiB on a daemon, 16 MiB on a client. A program built on
  `blit-core` that multiplexes more long-lived streams on one channel, or
  keeps more than eight responses unread, needs another channel.
- A local single-file copy armed its failure cleanup before opening the
  destination, so a destination it could not open (a read-only file, a
  folder in the way) could be deleted without a byte written. The cleanup
  is armed only once the destination is open for writing.
- Ctrl-C during the retry wait or a retry pass killed the process before
  its report: the first pass's counts and the files that did not land were
  never printed. The retries now stop, the report prints, and the run
  exits 130; `move` refuses its source delete and names the files.
- A `--detach` job hid its per-file failures: `jobs list` showed no count
  and `jobs watch` exited 0. Both report them now (see Job logs above).
- A file whose write failed after bytes reached the destination was left
  at the source's size with a fresh mtime, so every later compare took it
  for a finished copy. A failed streamed record or shard member is now
  removed (a failure before the target existed leaves the previous version
  untouched) and an interrupted resume keeps its partial marked. Retry
  passes therefore compare like the main pass: a file that became current
  during the wait is not re-sent, a newer destination file is kept, and
  `--ignore-existing` is honored except for this run's own incomplete copy.
- `--exclude` silently matched nothing for the forms people type: an
  absolute path (`--exclude /home/me/.java`) was never compared as one,
  and a bare directory name (`--exclude .java`) dropped only an entry with
  exactly that name, so its contents still transferred. Now a pattern
  with no `/` matches a file or directory name at any depth, and an
  excluded directory takes everything beneath it; a pattern with a `/` is
  relative to the source and excludes that path and its subtree; an
  absolute path under a local source is made relative to it (naming that
  one path) on every route and in `blit check`; an absolute pattern that
  cannot match anything is named once on stderr and the run goes on.
  An absolute path typed in another case than the disk stores it (macOS,
  Windows) excludes the entry anyway, an absolute glob written through a
  symlink (`/alias/*.txt`, macOS `/tmp`) is made relative too — names
  read from the disk match literally even when they hold `[` or `*`, a
  spelling the filesystem itself pairs with the stored name (final sigma,
  NFC for NFD) names that entry, and a typed pattern is glob syntax only
  (`data[1]` is a class naming `data1`) — and on Windows `\` in a relative
  pattern is a separator (`.\cache`, `cache\`), while elsewhere a typed
  escape (`a\*b`) is sent in a form every peer reads alike. An absolute
  path naming an entry the source stores under several names (a hard
  link) excludes every one of them and says so once on stderr. Under
  `mirror`, an excluded entry the destination stores in another spelling
  (`dst/Cache` for an excluded `src/cache`, `dst/x/ς` for an excluded `σ`,
  `dst/x/Cache` for `**/cache`) is out of the deletion scope too, and so
  is the directory holding it. The source also sends paths its glob
  excludes pruned; the destination protects its own spellings of those
  entries. Glob text keeps its typed class members, and `**` followed by
  a crossing glob keeps bounded matcher state in deep trees.
  Excluded directories are pruned, never read, so an unreadable one no
  longer leaves a mirror's scan incomplete. `*.log`, `**/*.log`, `dir/**`
  and exact relative paths match as before. (`blit-core`: `FilterInputs`
  gained `source_root`.)
- A `--detach` job that failed on the daemon showed only
  `delegated_pull handler failed` in `jobs list`; the reason reached the
  job's log alone. The recents record now carries the handler's own
  error, the same text the log records, so `blit jobs list <host>` says
  why (for example `FAILED: delegated transfer: session MODULE_UNKNOWN:
  module 'nosuchmod' not found`); the final subscribe event carries it
  too.

## [0.1.3] - 2026-08-20

Consolidation and self-tuning: the workspace is now three crates with
`blit-core` as the embeddable platform library, every install channel is
live (Homebrew, winget, Scoop, AUR, cargo, GitHub Releases), and
transfers record per-route performance history and warm-start their
tuning from it instead of re-learning every run. Covers all work landed
since `v0.1.2` (tagged 2026-08-08).

### Workspace consolidated to three crates

- `blit-app` and `blit-console-core` are folded into `blit-core`, the
  single platform crate (published on crates.io); the gRPC proto now
  ships inside it. `blit-tui`, `blit-gui`, and `blit-prometheus-bridge`
  are removed: Blit in this repo is the CLI plus the daemon, and UIs are
  developed out of tree. Shipped binaries are unchanged (`blit`,
  `blit-daemon`).
- On crates.io the CLI crate is named `blit-transfer` (the installed
  command is still `blit`).

### Install channels

- Live channels: Homebrew (`roethlar/blit/blit-bin`), winget
  (`Roethlar.Blit`), Scoop, AUR (`blit`, `blit-bin`), cargo
  (`blit-transfer`, `blit-daemon`), and GitHub Releases. The release
  pipeline updates the bot-maintained channels automatically on tag
  publish.

### Adaptive tuning from per-route performance history

- Every transfer session records per-route performance history (schema
  v3 with route identity, serialized writer, atomic compaction), on the
  client and on the daemon side, including remote client sessions and
  delegated coordinators.
- Settled tuning dials persist as per-route seeds: later runs warm-start
  the checker dial and session worker counts from those seeds, then keep
  adapting up and down under load instead of re-learning from scratch.
- Diagnostics report merged daemon + operator perf history with origin
  labels, so telemetry reflects both ends of a route.

### Protocol safety

- A session now refuses a peer with a mismatched protocol contract
  version up front instead of failing mid-transfer, and `blit scan`
  surfaces the protocol version.
- A local destination shaped like a daemon address (`host:/path`) gets a
  warning instead of being treated silently as a local path.

### Fixes

- Streamed-receive mtime pinning compared Unix-epoch seconds against
  1601-epoch seconds on Windows; metadata now stamps through the
  retained descriptor, and streamed receives cache parent readiness.
- Per-file error containment follow-ups: the containment decision is
  cached per session and resume-path descriptors are stamped.
- Progress output: the transfer-rate window stops measuring after the
  last payload byte (no decaying tail rates), stall reporting separates
  visible mid-payload stalls from silent tails, and summary colours are
  byte-observable.

## [0.1.2] - 2026-08-08

A patch version number, but not a small release: this covers everything
landed since 0.1.1 (tagged 2026-07-23). The headline items are per-file
failure containment, a large reduction in comparison cost on real mirror
workloads, live terminal progress, code-signed macOS and Windows binaries,
and one user-visible change to what the default comparison checks. Read
"Behavior change" below even if you skip the rest.

### Behavior change: default compare no longer inspects destination streams

Every run — including a no-op run over an already-mirrored tree — used to
ask the destination whether each file that already matched by size and
modification time also had matching Windows named `$DATA` streams. That
check is gone from the default comparison.

- The default comparison is now size + modification time only. A stray or
  altered named stream on an otherwise size/mtime-matched destination file
  can go undetected until a run made with `--checksum`.
- `--checksum` is unchanged and still performs the exhaustive per-file
  comparison, streams included.
- Streams are still carried whenever a file's content actually transfers.
  This affects detection of an out-of-band stream change on a file blit
  otherwise considers unchanged — not stream fidelity on copy.
- File attributes are unaffected: they are still compared, and an
  attribute-only divergence is now repaired in place (see Reliability).
- Rationale: the remedy for a detected stream divergence was always
  whole-file replacement, so the check never protected the destination
  data — it only found it sooner in order to overwrite it sooner, and it
  accounted for most of the comparison cost on a converged mirror.

### Reliability: one bad file no longer aborts the whole transfer

Previously a single file blit could not write — locked, permission denied,
an attribute it could not apply — ended the entire transfer, discarding
work already done and everything still queued.

- A file that fails is recorded and skipped; the rest of the transfer
  proceeds. Containment covers single-file paths, streaming receive,
  tar-batched shards, and resume block patching, on both local and remote
  routes.
- Runs that completed with failures print an end-of-run block naming the
  failed files and reasons (capped list, with an elided-count note and a
  re-run hint) and exit with status **2** — distinct from clean success
  (`0`) and from a hard abort, which keeps its own non-zero status.
  `--json` output carries both signals: the status and a `files_failed`
  count with a `failures` list.
- **`move` never deletes a source file whose copy did not land.** This
  gate now covers every move route, CLI and TUI. Re-running converges.
- Attribute-only divergence is repaired in place with no file content
  re-sent. This also fixes mirroring to a default-configuration Samba
  share, which synthesizes a HIDDEN attribute on dot-named files that
  previously could never converge.
- Volume-level problems deliberately stay fatal rather than being
  contained per file: a read-only destination mount or an exhausted volume
  ends the session instead of reporting thousands of individually failed
  files.
- Fixed (Linux/macOS): when a path component was an existing file rather
  than a directory, that could abort a whole batch instead of failing only
  the affected entry.

### Performance

Measured on one real workload — roughly 46,000 files / ~8 GB mirrored to
an SMB share — on the maintainer's hardware. These are that workload's
numbers, not a general guarantee.

- **Converged (no-op) SMB mirror check: 283.92 s → 55.57 s, about 5.1x.**
  This is comparison cost, from four changes together: reusing file
  metadata already read during enumeration instead of re-reading it per
  file, one directory scan per destination directory instead of one stat
  per file, a self-tuning concurrency level for the comparison stage, and
  dropping the destination stream check described above.
- **Local-to-local mirror of the same tree: 17.68 s → 8.70 s, about 2x**,
  from running the apply stage's writes in parallel instead of on one
  worker.
- All of it is automatic. No configuration was added and none is
  available: blit works these values out at runtime.

### CLI and terminal output

- Running with `-p` shows one stable in-place progress row — phase, files
  and bytes so far, current file — instead of a spinner that other output
  could scroll away. `-v` per-file lines scroll cleanly above it, and
  warnings no longer erase it.
- The phase names tell the truth about what is happening (enumerating,
  comparing, copying, deleting), and a `--dry-run` never claims to delete.
- The live row and the failure block are colour-coded. Colour is disabled
  automatically for `NO_COLOR`, `TERM=dumb`, non-interactive output, and
  `--json`; redirected output is byte-identical to before. There is no
  `--color` flag and no theming.
- The end-of-run summary separates files copied from files repaired, and
  labels its transfer rate as a whole-run average.
- The enumeration heartbeat on long scans is now quiet by default and
  audible under `--verbose` or `-p`, on both local and remote push routes.

### Packages, signing, and compatibility

- **macOS and Windows binaries in the release archives are code-signed for
  the first time.** macOS: Developer ID Application signature with
  hardened runtime and a secure timestamp, and the binaries are notarized
  by Apple. Windows: Authenticode via Azure Trusted Signing with an
  RFC 3161 timestamp. Linux binaries are not signed.
- Two honest caveats about that signing:
  - **Windows SmartScreen may still warn.** The signature is genuine and
    verifiable, but SmartScreen's "reputation" is earned from download
    volume over time, so a fresh certificate can still produce a warning
    on first download. Check the publisher in the file's Digital
    Signatures tab rather than relying on the absence of a prompt.
  - **The macOS notarization ticket is not stapled**, because a flat
    executable has no bundle to hold one. Gatekeeper resolves the ticket
    online instead, so a first run on a machine with no network path to
    Apple can still be blocked even though the binary is notarized.
- Releases are now built, signed, smoke-tested, and attached to the tag by
  CI rather than uploaded by hand. Publication is atomic: all three
  platform legs must succeed or no release is created or modified at all.
- Archive contents are unchanged in shape: `blit` and `blit-daemon` for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`, each archive with a SHA-256 sidecar and the
  exact source commit in `BUILD.txt`.
- Both executables report the same `0.1.2+<commit>` build identity, and
  remote sessions still refuse peers whose build identity differs. The
  wire contract version moved 5 → 6 (the summary now carries the failure
  report), so a 0.1.2 peer will not transfer with a 0.1.1 peer.

### Not in this release

Named here because the code is present in the tree and could otherwise be
mistaken for a shipped feature:

- **There is no GUI, and no new TUI.** Work on the Blit Console has begun
  as a library crate (`blit-console-core`: endpoint model, browse state,
  message/update loop, LAN daemon discovery, remote daemon browse), but no
  binary depends on it and none of it is reachable from any command. It
  ships in the source tree, not in any user-facing surface.
- Specifically, the daemon discovery and remote-browse functions inside
  that crate are **not** a new user feature. The shipped ways to find and
  browse daemons are the ones 0.1.1 already had and they are unchanged:
  `blit scan` (mDNS), `blit list <host>`, and `blit ls <host:module/path>`.
- The legacy TUI's F3 pane gained an internal picker mode with no caller
  and no keybinding — nothing opens it. TUI behavior is unchanged.
- Some default-off timing instrumentation was added for latency
  diagnostics. It is off unless explicitly enabled and changes nothing
  about a normal run.

### Known limitations

- **Non-UTF-8 source filenames fail to transfer.** A filename that is
  valid on Linux/ext4 but not valid UTF-8 is corrupted in blit's internal
  path handling. On LOCAL transfers the failure is contained to that one
  file (the run continues and exits 2, per Reliability above). On REMOTE
  transfers it is **not** contained: the corrupted path can fail payload
  preparation at the source and abort the whole session. The only current
  workaround is renaming the affected files.
- **`--exclude` only matches source-relative paths, and a directory
  pattern does not match that directory's contents.** An absolute path, or
  a bare directory name meant to exclude everything beneath it, silently
  matches nothing — the pattern is accepted and no warning is issued. Use
  a source-relative glob with an explicit subtree wildcard, e.g.
  `--exclude '.java/**'` rather than `--exclude .java`.
- Blit still has no built-in TLS and no user authentication. Module access
  controls and per-session data-plane tokens are not an authentication
  system. Run the daemon only on a trusted network or through an
  operator-managed VPN or SSH tunnel.
- Throughput is not a release claim. Hardware ceiling work, small-file
  tuning, and zero-copy receive remain open.
- Cross-OS daemon transfers were not smoke-tested as a matrix for this
  release. Automated testing covers all three platforms and same-OS
  loopback daemon transfers; the six directed cross-OS pairs were not run.

## [0.1.1] - 2026-07-23

### Packages and compatibility

- Release archives contain `blit` and `blit-daemon` for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and
  `x86_64-pc-windows-msvc`; each archive has a SHA-256 sidecar and embeds the
  exact source commit in `BUILD.txt`.
- Both executables report the same exact `0.1.1+<commit>` build identity.
  Remote sessions require identical build identities and refuse mixed builds
  before transferring data.
- Packaged-release validation checks safe extraction, checksum and build
  identity, CLI/daemon startup, one tiny local copy, and one tiny loopback
  remote copy with exact byte comparison and bounded teardown.

### Transfer correctness and operation

- Local, push, pull, and remote-to-remote operations now use one role-based
  transfer session instead of separate direction-specific transfer engines.
- Windows file attributes and named `$DATA` streams are preserved across
  supported local, TCP, in-stream, tar-batched, and resumed transfers. A
  non-Windows destination refuses Windows metadata unless the operator
  explicitly selects the warned metadata-drop option.
- Transfer progress now carries declared and completed file/byte totals, live
  served and delegated byte counts, and the final carrier through daemon state,
  events, persisted recent rows, CLI output, and TUI output.
- Retry re-runs destination comparison and skips files that already completed;
  `--resume` additionally enables block-level continuation for eligible partial
  files in every transfer layout.
- Failure handling now preserves the first actionable file/worker fault across
  shutdown races, bounds network and child-process waits, and reports daemon
  startup diagnostics without blocking on stderr.

### Security and known limitations

- Blit has no built-in TLS and no user authentication. Module access controls
  and per-session data-plane tokens are not an authentication system. Run the
  daemon only on a trusted network or through an operator-managed VPN or SSH
  tunnel.
- These packages are correctness- and smoke-tested on the three listed target
  triples. Hardware throughput ceilings are not release claims. Mac-to-Mac
  Thunderbolt testing, further small-file tuning, and zero-copy optimization
  remain post-release work.

## [0.1.0] - 2026-05-31

### Transfer Engine
- Local copy, mirror, and move operations with async orchestrator
- Remote push/pull via hybrid TCP data plane + gRPC control plane
- Remote-to-remote transfers (`blit copy server1:/mod/ server2:/mod/`)
- Block-level resumable transfers with Blake3 hashing (`--resume`)
- Multiplexed TCP data plane with auto-tuned stream counts
- Small file batching via tar shards (parallel unpacking on daemon)
- Async read-ahead pipeline with buffer pool (double-buffered I/O)
- Streaming manifest exchange for arbitrarily large file sets
- Adaptive performance predictor using online gradient descent

### Platform Support
- **macOS**: `clonefile()` CoW, FSEvents change journal, `statfs` FS detection
- **Linux**: `copy_file_range()`, metadata snapshot change journal, `statfs` FS detection
- **Windows**: `CopyFileExW`, USN Change Journal, ReFS block clone (when privileged)
- Filesystem capability probing for 12+ filesystem types (APFS, btrfs, XFS, ext4, ZFS, NFS, CIFS, NTFS, ReFS, etc.)
- Device-keyed capability cache

### CLI (`blit`)
- Commands: `copy`, `mirror`, `move`, `scan`, `list`, `du`, `df`, `rm`, `find`
- `diagnostics perf` for performance history management
- Progress spinner (`--progress`), verbose output (`--verbose`)
- `--dry-run`, `--checksum`, `--force-grpc`, `--workers`
- Destructive operations prompt unless `--yes` is supplied

### Daemon (`blit-daemon`)
- TOML configuration with `[[module]]` exports
- mDNS service discovery (`_blit._tcp.local.`)
- Admin RPCs: ListModules, List, Find, DiskUsage, FilesystemStats, CompletePath, Purge
- Hybrid transport: TCP data plane negotiation with gRPC fallback
- `--root` default export, `--no-mdns`, `--force-grpc-data`

### Admin Utilities (built into `blit`)
- Commands: `scan`, `list-modules`, `ls`, `find`, `du`, `df`, `rm`, `completions`, `profile`
- `--json` output for all inspection commands
- Human-readable byte formatting in `df` output
- Local path support for `ls`
- `blit list <bare-host>` smart-dispatches to module listing; `blit list <module/path>` falls through to `ls`
- `find --pattern <GLOB>` uses POSIX shell-glob syntax (`*`, `?`, `[abc]`, `**/`); `*` does not cross `/`. Pattern matches against both the relative path and the file-name basename so `*.csv` finds nested entries.
- `blit completions shell <SHELL>` generates static bash/zsh/fish/powershell/elvish completion scripts via `clap_complete`; pipe to your shell's completion directory.
- `blit completions remote <PREFIX>` is the existing daemon-backed remote-path completion; called internally by the generated shell scripts.
- Originally a separate `blit-utils` binary; merged into `blit` for a single install/distribution surface

### Performance History
- JSONL storage with schema versioning (v0/v1 migration)
- Capped at ~1 MiB with rotation
- Adaptive predictor with per-profile coefficients
- `blit diagnostics perf` for inspection and management

### Documentation
- Man pages: `blit(1)`, `blit-daemon(1)`
- Architecture guide (`docs/ARCHITECTURE.md`)
- Daemon configuration guide (`docs/DAEMON_CONFIG.md`)
- Performance roadmap (`docs/PERFORMANCE_ROADMAP.md`)

### Testing
- Integration tests: admin verbs (10), admin commands (21, in `crates/blit-cli/tests/blit_utils.rs`), remote transfers, transfer edges, parity, resume, move, remote-to-remote
- Unit tests: predictor regression (9), schema versioning (7), filesystem probing (7), mirror planner, checksum, enumeration
- GitHub Actions CI: fmt/clippy checks, tri-platform tests (Linux/macOS/Windows), release artifact builds

### Security
- Path traversal protection
- Block size limits
- Token verification (placeholder)
- Module-level read/write permissions
- No built-in TLS; operators use SSH tunnels, VPN, or reverse proxy

### Known Limitations
- TCP data plane throughput not yet benchmarked at 10+ Gbps (implementation complete, hardware testing pending)
- No built-in TLS encryption
- No authentication beyond module-level access control
- Windows ReFS block clone requires SeManageVolumePrivilege
