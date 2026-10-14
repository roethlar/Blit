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
  is named once on stderr and the run goes on. Excluded directories are
  never read, so an unreadable one no longer stops a mirror's deletions.
  Before 1.0 the absolute and bare-name forms silently matched nothing.

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
  list it ran with.
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

- **Wire contract 7.** A session is refused at open, with a plain
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
  channels update on tag publish, and the 1.0.0 crates are published with
  the tag. The 0.1.2 crates on crates.io predate the protocol gate and
  refuse every peer; `cargo install` them no further.
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
- **The daemon has no built-in TLS or user authentication** (unchanged).
  Module access controls and per-session data-plane tokens are not an
  authentication system: run it only on a trusted network or through an
  SSH tunnel or VPN.
