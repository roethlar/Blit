# Blit 1.0.0 Release Notes

Covers everything shipped since 0.1.3 (tagged 2026-08-20). The 0.1.x
releases are described in `CHANGELOG.md`.

## Headline

1.0 is the reliability release. No single file can end a transfer any
more, on either side of the wire: a file that is locked, vanishes, changes
size while it is being read, or cannot be written is reported by name and
the rest of the run lands. Files that failed are retried automatically at
the end of the run. Every run leaves a log on every machine involved and
a job you can list, read, re-run and retry. Filenames that are not valid
UTF-8 transfer.

## Reliability: nothing a single file does ends the run

0.1.2 contained failures on the destination side. 1.0 contains them on
the source side too, and adds a retry pass.

- **Source-side failures are contained.** A file locked, vanished or
  changed since the scan is skipped (before its record is announced) or
  retracted (after), reported through the same per-file failure report
  as destination failures, with a reason that starts with `source:`. A
  file that grows inside a small-file batch can no longer corrupt its
  neighbours: batches are packed from exactly the bytes the manifest
  promised, and nothing is ever finalised under a header its bytes do not
  match.
- **End-of-run retry passes.** Files that failed are retried after the
  main pass, re-scanned fresh. `--retries <N>` (`-R`, default 1; `0`
  disables) sets the number of passes and `--retry-wait <SECONDS>` (`-W`,
  default 30) the wait before each. This is a default-behavior change: a
  run with failures now waits 30 seconds and tries those files once more
  before it reports. Files that land on retry count once; survivors are
  reported once, marked `(retried)`. The older `--retry`/`--wait` pair
  still re-runs the whole transfer after a transient failure.
- **Mirror deletions stay safe.** Deletions run at the end of the main
  pass and never touch the subtree under a failed path. A file that
  enumerated but could not be read keeps its destination counterpart.
- **A failed write never leaves a finished-looking file.** A streamed
  record or batch member whose write failed after bytes reached the
  destination is removed (a failure before the target existed leaves the
  previous version untouched); an interrupted resume keeps its partial
  marked. Retry passes and later runs therefore compare normally.
- **Windows: locked files no longer end the run.** A source file held
  open with no sharing (`NTUSER.DAT`, a live database) is listed, reported
  per file if it still cannot be read, and retried. A permission refusal
  at scan is handled the same way.
- **Ctrl-C during the retries keeps the report.** The retries stop, the
  files that did not land are printed, and the run exits 130. A `move`
  refuses its source delete and names the files.
- **Exit codes:** `0` clean, `2` completed with per-file failures, `130`
  interrupted; any other non-zero code is a hard failure. The end-of-run
  block reads `N file(s) did not land at the destination:`.
- A local single-file copy that could not open its destination (a
  read-only file, a folder in the way) could delete that destination
  without writing a byte. Fixed: the copy's cleanup is armed only once
  the destination is open for writing.

## Filters

- **`--exclude` matches the forms people type.** A pattern with no `/`
  matches a file or directory name at any depth, and an excluded
  directory takes everything beneath it (`--exclude .java`); a pattern
  with a `/` is relative to the source and excludes that path and its
  subtree (`--exclude a/.java`); an absolute path under a local source is
  made relative to it and excludes exactly that path (`--exclude
  /home/me/.java`). An absolute pattern that cannot match anything —
  outside the source, or any absolute pattern when the source is remote —
  is named once on stderr and the run goes on. An absolute path typed in
  another case than the disk stores it (macOS, Windows), or in a spelling
  the filesystem itself pairs with the stored name, still excludes the
  entry; an absolute glob written through a symlink (`/alias/*.txt`, macOS
  `/tmp`) is made relative too; on Windows `\` in a relative pattern is a
  separator (`.\cache`, `cache\`), and elsewhere a typed escape is sent
  in a form every peer reads alike. An absolute path naming an entry the
  source stores under several names (a hard link) excludes every one of
  them and says so once on stderr. Under `mirror`, an excluded entry the
  destination stores in another spelling (`dst/Cache` for an excluded
  `src/cache`, a Unicode-equivalent name, or an entry a glob pruned at the
  source) is out of the deletion scope too, and so is the directory
  holding it. Glob text is matched as written, including character
  classes. Excluded directories are
  never read, so an unreadable one no longer stops a mirror's deletions.
  Before 1.0 the absolute and bare-name forms silently matched nothing.
- **A pattern matches exactly as typed; `--ignore-case` ignores capital
  letters.** `--exclude '*cache'` names `cache` and not `Cache`, on every
  machine and whatever the disk does with case — the rule rsync follows.
  With `--ignore-case`, every `--exclude` and `--include` pattern of the
  run ignores capital letters, on the source and in what a `mirror`
  deletes alike, and on a daemon at either end. Letters of any alphabet
  pair one to one (`Übung` names `übung`; `ß` is not `SS`). `blit check`
  takes the same options, so it reads a tree as the transfer did.
- **`mirror` shows how your pattern will match before it asks.** When a
  `mirror` is about to ask before deleting and you gave patterns without
  saying how to read them, the question starts with one line:
  `"*Cache" matches Cache only; capital letters matter (--ignore-case
  would also match cache, CACHE).` `copy`, which asks nothing, prints the
  same line once as a note and goes on. Saying it yourself —
  `--match-case` (the default, stated) or `--ignore-case` — or `-y`
  leaves the line out. Where `--ignore-case` would make a pattern match
  less — a negated class such as `[!x]`, which then refuses `X` as well —
  the line says that instead: `(--ignore-case changes what it matches:
  [!x] would stop matching X).` With `--delete-scope all` a filter only
  chooses what is copied — every destination entry the filtered source
  lacks is deleted, excluded ones included — and the question says so.
- **`?` and a bracket class match one byte**, with or without
  `--ignore-case`, as in rsync and git: `?.txt` does not name `é.txt`. A
  class that holds a letter beyond ASCII (`[aé].txt`) could never match
  that letter, so it is refused with the spelling that works
  (`{a,é}.txt`). A name that is not valid UTF-8 is matched by its own
  bytes.
- **Pattern files.** `--exclude-from FILE` and `--include-from FILE`
  (repeatable; `copy`, `mirror` and `check`) read one pattern per line.
  Spaces at both ends of a line are dropped, blank lines and lines
  starting with `#` are skipped, and a space wanted at an edge is written
  `[ ]`; each line is then exactly the pattern it would be on the command
  line. A line that is an rsync rule (`- pattern`, `+ pattern`, `!`)
  refuses the run and names the line — a blit pattern file holds patterns
  and nothing else. A saved job keeps the patterns it read, not the file.
- **A filter that cannot be used is refused before anything is asked.**
  A pattern that is no glob, a size or an age that cannot be read, a
  pattern file that is missing: each now stops the command before
  `mirror`'s question. Before, the question came first.
- **A glob names an entry by its spelling.** Under `mirror`,
  `--exclude '*ß'` keeps a destination's `aß`; a destination-only `bSS`
  is not named by it and is deleted, on every disk, as rsync does. (A
  literal name, and an entry the source's walk left out, are still kept
  in the destination's own spelling.)

## `du` and `df` on a daemon behave like `du` and `df`

The utility verbs run on the server's own disk in one request, so a job
on a server's files never pays a network filesystem's per-file cost.
From this release each also behaves like the Unix command it is named
after. **This changes their output from 0.1.x.**

- **`blit df HOST:/module/path`** reports the filesystem that holds that
  path, in `df -k`'s shape with the path as typed in the first column;
  `-h` for human-readable sizes. Before, the path was dropped and a module
  on its own filesystem could be shown another disk's numbers.
- **`blit du HOST:/module/path`** prints what `du` prints: a line per
  directory, deepest first and the path given last, `SIZE<TAB>PATH` in 1K
  blocks of allocated space; `-a`, `-s`, `-d N` and `-h` as in `du`; a
  hard-linked file counted once. On Linux the output equals the system
  `du`'s. Before, it printed a table with a row for every file and byte
  lengths, and counted depth from the module's root.
- An unreadable directory is reported in `du`'s words and makes the exit
  status 1 while the rest is printed. `-h` is the commands' own; help is
  `--help`.

## `rm` on a daemon behaves like `rm`, and a question nobody answers is a failure

**This changes `blit rm` from 0.1.x.**

- **`blit rm [-r] [-f] HOST:/module/path...`** treats each path as `rm`
  does. A directory needs `-r` — before, a whole tree was deleted without
  it. A path that is not there is an error unless `-f` — before, it was
  reported as success. Several paths may be given; one that fails does not
  stop the others, and the exit status is 0 only when every path is gone.
  Nothing is printed for what was removed; each entry that could not be
  is reported as `rm` reports it (`cannot remove '…': Permission denied`),
  and a removal goes on past it. A symbolic link is removed and never
  followed. On Linux the messages, the exit status and what is left on
  disk equal the system `rm`'s.
- **It still asks first** — once, naming the paths, before anything is
  sent. `-y` skips the question and so does `-f`, so `blit rm -rf` behaves
  as `rm -rf` does.
- **`mirror`, `move` and `rm`: with nobody to answer the question, the
  command now fails.** Answering "no" still stops it with exit status 0.
  When no answer can be read at all — standard input is closed, as under
  cron or in a script that forgot `-y` — the command used to print
  "Aborted." and exit 0, so a job that did nothing looked like one that
  had run. It now does nothing, exits 1, and names the option to pass.
  **A script that relied on the old exit status must pass `-y`.**
- `blit move` from a daemon reports the source removed only when every
  entry of it went; an entry that could not be removed is named and the
  command fails.

## `find` on a daemon behaves like `find`

**This changes `blit find` from 0.1.x: its options are `find`'s own.**

- **`blit find HOST:/module/path [-name PATTERN] [-iname PATTERN]
  [-type f|d|l] [-maxdepth N]`** prints one path per line and nothing
  else, each beginning with the path as typed, for every entry all the
  tests given hold for — the path itself included, as `find` includes it.
  Before, it printed a `PATH BYTES TYPE` table of paths relative to the
  module.
- **`-name` matches an entry's own name, as `find` matches it**: `*`, `?`,
  `[...]` and `\`, a leading dot matched like any character. `-iname`
  ignores case the way `find` does. On Linux the lines equal the system
  `find`'s for the same tree and expression. Before, `--pattern` was a
  glob against the path below the module.
- **The old options are refused with what replaces each**: `--pattern X`
  → `-name 'X'`; `--case-insensitive` → `-iname`; `--files` → `-type f`;
  `--dirs` → `-type d`; `--limit N` → pipe through `head -n N`. Any other
  word of `find` (`-o`, `!`, `-newer`, `-exec`, …) is refused by name:
  a test that was silently dropped would find more than was asked for.
- A directory the daemon cannot read is reported as `find` reports it and
  makes the exit status 1; the rest is still listed. A symbolic link is
  listed and never followed. A name that is not valid UTF-8 is printed as
  the bytes it is.

## Filenames that are not valid UTF-8 transfer

A filename that is valid on Linux (ext4 accepts any bytes) but not valid
UTF-8 used to fail, and on remote transfers could abort the session. Now,
rsync-style, the exact bytes travel beside the text: a Linux or other
byte-keyed destination creates the real name and the file converges on
re-run; a destination that cannot store the bytes (Windows, macOS)
reports the file as `source: filename is not valid UTF-8; the destination
cannot store it (rename it to transfer)` and continues. Two source names
that collapse to one text name are reported as a lossy-name collision
instead of overwriting each other. Logs, run records and `jobs retry`
carry such names by their exact bytes.

## Job logs, run records and saved jobs

- **A log for every run.** Local copies, pushes, pulls and
  remote-to-remote jobs each write a compressed JSON log as the run goes:
  every file copied, deleted and failed with its reason, what `-v` would
  have shown, phase times and stall notices. The machine where the
  command was typed keeps its log in the per-user folder (`~/Library/
  Application Support/com.Blit.Blit` on macOS, `~/.config/blit` on Linux,
  `%LOCALAPPDATA%\Blit` on Windows); each daemon involved keeps its own
  under its state directory. One job ID names the run on every machine. A
  run killed mid-way leaves a log that the next start finalizes as
  interrupted.
- **`blit jobs list [<host>]`** shows recent runs and saved jobs with
  their outcome and failed-file count. **`blit jobs log [<host>]
  <job-id>`** shows a log as text, or the raw events with `--json`. A
  `--detach` job's outcome is fetched from the daemon that ran it, and
  **`blit jobs watch`** exits 1 and names the failed files when it has any.
- **Saved jobs.** `--save <name>` on `copy`, `mirror` or `move` keeps the
  run as a saved job; `--export <file>` writes it to a file. `blit jobs
  save|export|run|delete` manage them; `blit jobs run <name|file>` re-runs
  the same transfer from any working directory, with the `--files-from`
  list it ran with. A command that is refused for its own words saves
  nothing, so a job already saved under that name stays as it was.
- **A failed `--detach` job says why in `jobs list`.** The daemon's record
  of the job carries the handler's own error, the same text its log
  records, instead of the fixed `delegated_pull handler failed`.
- **`blit jobs retry <job-id|file>`** starts a new run that sends exactly
  the files that failed, with the original options (a `move` runs whole
  again). It refuses a job from another machine, a run still going, and a
  run whose failures are not all named.
- **Retention.** Each machine keeps its newest 50 logs and 50 run records
  (`[jobs] keep` in the CLI's per-user `config.toml` and the daemon's
  `config.toml`). Saved jobs are never pruned.
- **Windows** keeps settings, jobs and logs under `%LOCALAPPDATA%\Blit`;
  files found in the old roaming folder are moved once.
- Logging adds no measurable time to a command. If the log directory
  cannot be written the transfer still completes and the report says the
  log is incomplete.

## Protocol and compatibility

- **Wire contract 15.** A session is refused at open, with a plain
  message, when the two ends' protocol contract versions differ (every
  0.1.x release is another contract); upgrade both ends. Two 1.0.0 builds
  from any channel talk to each other. Both executables report
  `1.0.0+<commit>` from a release archive or package.
- **Release archives** contain `blit` and `blit-daemon` for
  `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin` (signed and
  notarized) and `x86_64-pc-windows-msvc` (not Authenticode-signed), each
  with a SHA-256 sidecar and the exact source commit in `BUILD.txt`.
- **Install channels:** Homebrew (`roethlar/blit/blit-bin`), winget
  (`Roethlar.Blit`), Scoop, AUR (`blit`, `blit-bin`), cargo
  (`blit-transfer`, `blit-daemon`) and GitHub Releases; the bot-maintained
  channels update on tag publish; the 1.0.0 crates.io packages are published
  separately after the tag. The 0.1.2 crates on crates.io predate the
  protocol gate and refuse every peer; `cargo install` them no further.
- **For embedders of `blit-core`:** the public API changed, which is why
  this is a major version. `PreparedPayload::TarShard` gained `skipped`
  and `PreparedPayload::Skipped` was added; `build_tar_shard` returns
  `TarShardBuild`; `TransferSource::open_file` returns `OpenedSourceFile`
  and `check_availability` was removed; `TransferSink` gained
  `begin_record`/`RecordWriter`, `can_store_raw_names` and
  `note_raw_name`; `TransferSummary` gained `failed_paths` and
  `failed_paths_truncated`; `transfers::filter::FilterInputs` gained
  `source_root`.

## Known limitations shipped open

- **The in-command retry passes do not re-send a file whose name is not
  valid UTF-8.** It stays reported as failed; `blit jobs retry` re-sends
  it.
- **A `--detach` job has no retry passes**; the command says so once. Use
  `blit jobs retry` afterwards.
- **A `move`'s source-side deletion on a daemon is not a job** and leaves
  no log.
- **A firewall must admit the daemon's data plane, not only its port.**
  For every transfer the daemon listens on a fresh ephemeral TCP port and
  the client dials it; a rule that opens only the control port (9031)
  lets a session open and then fails every transfer with
  `DATA_PLANE_FAILED`. Allow the program (macOS, Windows), open the
  ephemeral range, or run the daemon with `--force-grpc-data`
  (`docs/DAEMON_CONFIG.md`, Firewall Configuration). Found on a Windows
  daemon during the release checks; the recipes there were corrected.
- **The daemon has no built-in TLS or user authentication** (unchanged).
  Module access controls and per-session data-plane tokens are not an
  authentication system: run it only on a trusted network or through an
  SSH tunnel or VPN.
