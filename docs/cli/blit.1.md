# BLIT(1) blit Manual
% Blit v2 Team
% 2026-11-21

## NAME
blit – local and hybrid-transport file transfer CLI

## SYNOPSIS
`blit copy [OPTIONS] <SOURCE> <DESTINATION>`
`blit mirror [OPTIONS] [--yes] <SOURCE> <DESTINATION>`
`blit move [OPTIONS] [--yes] <SOURCE> <DESTINATION>`
`blit scan [--wait <SECONDS>] [--json]`
`blit list <REMOTE> [--json]`
`blit list-modules <REMOTE> [--json]`
`blit ls [-a] [-l] [-h] [--json] <TARGET>`
`blit du [-a] [-s] [-d <N>] [-h] [--json] <REMOTE>`
`blit df [-h] [-k] [--json] <REMOTE>`
`blit rm [-r] [-f] [-y] [--json] <REMOTE>...`
`blit find [--json] [-y] <REMOTE> [-name <PATTERN>] [-iname <PATTERN>] [-type f|d|l] [-maxdepth <N>] [-size [+|-]<N>[c|k|M|G]] [-mtime [+|-]<N>] [-delete]`
`blit completions shell <SHELL>`
`blit completions remote <REMOTE> [--prefix <STR>] [--files] [--dirs]`
`blit profile [--limit <N>] [--json]`
`blit diagnostics perf [--limit <N>] [--enable|--disable] [--clear]`
`blit diagnostics dump [--json] <SOURCE> <DESTINATION>`

## DESCRIPTION
`blit` drives the v2 streaming transfer engine. It supports local transfers and
remote operations via a hybrid TCP/gRPC transport.

### Transfer Commands
- `copy` copies a `<SOURCE>` (file or directory) to `<DESTINATION>` without deleting extraneous files.
- `mirror` performs the same copy but removes files that are only present at the destination.
- `move` mirrors the source into the destination and then removes the original tree.

Both file and directory sources are supported. For files, the destination
resolution rules (below) determine whether the file is renamed, placed into
a directory, or treated as an exact target path.

### Destination Semantics (rsync-style)

`copy`, `mirror`, and `move` resolve the destination using rsync's
trailing-slash convention, applied identically regardless of which side is
local or remote:

| Invocation | Result |
|---|---|
| `blit copy SRC DEST/` or `blit copy SRC DEST` when `DEST` is an existing dir | `DEST/<basename(SRC)>/...` (nest) |
| `blit copy SRC/ DEST` or `blit copy SRC/. DEST` | `DEST/...` (merge contents) |
| `blit copy SRC DEST` when `DEST` does not exist | `DEST/...` (DEST becomes copy of SRC) |
| `blit copy file.txt DEST/` | `DEST/file.txt` |
| `blit copy file.txt renamed.txt` (renamed.txt does not exist) | `renamed.txt` (rename) |

The rule: a trailing slash (or `/.`) on the **source** means "copy the
contents of this directory". Without a trailing slash, whether the source's
basename is appended depends on the destination — if the destination has a
trailing slash or is an existing directory, the source is nested under it;
otherwise the destination path is used as the exact target.

On Windows, trailing `\` and `\.` are also recognized. For remote
destinations, only the trailing slash is consulted (no directory probe).


Any `<SOURCE>` or `<DESTINATION>` may be a local path or a remote endpoint:
- `server:/module/path` (explicit module export)
- `server://path` (default root export, if configured)
- `server` (implies default root)

**Note:** Remote paths must use forward slashes (`/`), not backslashes (`\`), regardless of platform.
Incorrect: `server:\module\path` — Correct: `server:/module/path`

Remote-to-remote transfers are supported (e.g., `blit copy server1:/mod/A server2:/mod/B`).
The CLI asks the destination daemon to pull directly from the source daemon,
so payload bytes flow source→destination and never cross the CLI host. The
destination daemon must opt in with `[delegation] allow_delegated_pull =
true`; if its gate rejects the request, the CLI fails with the daemon's
reason — there is no fallback route. If the destination cannot reach the
source but this host can reach both, run two commands: pull to a local
path, then push it.

### Admin Commands
- `scan` discovers blit daemons on the local network via mDNS.
- `list` smart-dispatches by target shape: a bare host (`server`,
  `server:9031`) routes to `list-modules`; a target with a module
  or path (`server:/module/`) routes to `ls`.
- `list-modules` lists modules exported by a daemon.
- `ls`, `du`, `df`, `rm` and `find` do on a daemon what the Unix
  commands of those names do on a local disk. See UTILITY COMMANDS.
- `completions shell <SHELL>` writes a clap-generated shell-completion
  script to stdout (bash/zsh/fish/powershell/elvish). Source it from
  your shell's rc file or completion directory.
- `completions remote <REMOTE>` returns daemon-side path completions
  via the `CompletePath` RPC. Used internally by the generated shell
  scripts for `server:/module/<TAB>`-style completion; usable
  directly for scripting too.
- `profile` prints local performance history records and the
  predictor's coefficients per transfer mode (`--json` for
  scripting). Reads `perf_local.jsonl` and the predictor state
  file; no network access.
- `jobs list [<host>]` lists the jobs run on this machine, newest first,
  with how each ended — a `--detach` job's outcome is fetched from its
  daemon; with a host, that daemon's active and recent transfers.
- `jobs log [<host>] <job-id>` shows a job's log: every file copied,
  sent, deleted and failed, with phase times. Without a host it reads
  this machine's log of the job; `jobs log <file>` reads a log file.
  `--json` prints the raw events; `--role` picks one of a daemon's logs.
- `jobs save <job-id> <name>` keeps a job run here as a saved job;
  `jobs export <job-id|name> <file>` writes a run (with how it went) or a
  saved job to a file; `jobs run <name|file>` runs a saved job or a job
  file again, only on the machine it was made on (a job file is given as
  a path, `./job.json`; a bare word is a saved job's name); `jobs delete <name>`
  deletes a saved job. `jobs retry <job-id|file>` sends again only the
  files a job run here failed to send, with its own options, as a new
  run (a move's retry runs the move again, so its source goes only once
  everything has landed). `copy`, `mirror` and `move` take `--save <name>`
  (keep this command as a saved job) and `--export <file>` (write its job
  and how the run went).
- `jobs watch <host> <job-id>` follows a daemon's job to its end;
  `jobs cancel <host> <job-id>` stops it.

## UTILITY COMMANDS
`ls`, `du`, `df`, `rm` and `find` do on a daemon what the Unix commands of
those names do on a local disk. The daemon does the work, on its own disk,
in one request, so a tree on a network share is listed, measured, searched
or removed without a network filesystem's cost for every file.

Each takes the options listed under it and means by them what its namesake
means; what it prints and its exit status are its namesake's. Any other
option, test or operator is refused by name before anything is asked of
the daemon. The reference is GNU coreutils and findutils under `C.UTF-8`,
on every platform; where a command differs from its namesake, its section
says so.

For all five:

- The path is `HOST:/MODULE/PATH`, or `HOST://PATH` for a daemon's default
  root. It is read as typed: a `/` after it names the directory — what a
  symbolic link there points to — as it does to the Unix commands.
- A symbolic link met below the path is an entry of its own and is never
  followed.
- A name that is not valid UTF-8 is printed, matched and removed as the
  bytes it is, and what these commands print is a path they take.
- What could not be read is reported on standard error in the namesake's
  words, and the rest of the answer is still given.
- The CLI and the daemon must be the same blit release: a daemon of
  another protocol contract refuses the request.
- `--json` prints one JSON document in place of the lines; a name that is
  not valid UTF-8 has its bytes beside its text (`raw`).
- `-h` is the namesake's `-h` where it has one (`ls`, `du`, `df`). Help is
  `--help`.

### ls
`blit ls [-a] [-l] [-h] [--json] <TARGET>`

Names, one per line and nothing else, sorted as `ls` sorts them (`A`, `a`,
`b`); names that begin with `.` are hidden. A path that is no directory is
one line: the path as typed. A symbolic link to a directory is listed
into, unless `-l` is given; with a `/` after it, it is the directory
either way.

- `-a`, `--all` — show names that begin with `.`, and `.` and `..`.
- `-l` — the long form: a `total` line (the entries' allocated space in 1K
  blocks), then for each entry the letter of what it is (`-` a file, `d` a
  directory, `l` a symbolic link, `p`, `s`, `c`, `b` as in `ls`), its
  size, when it was last written (`Nov 20 10:43`; `Jan  2  2025` for a
  time more than half a year past or still to come; this machine's time
  zone) and its name — a link with ` -> ` and where it points.
- `-h`, `--human-readable` — with `-l`, sizes in powers of 1024.

Exit status: 0; 1 when an entry could not be read; 2 when the path itself
could not be listed.

Unlike `ls`: one path to a command; one name to a line always, with no
columns and no quoting on a terminal; `-l` has no permission, link-count,
owner or group columns — a listing does not carry them; a module's `..` is
the module; a path that is a symbolic link out of its module is refused.
`TARGET` may be a path on this machine, written as a path (`./dir`); a
bare word is a host, whose modules are listed (`blit list` is the same
command).

### du
`blit du [-a] [-s] [-d <N>] [-h] [--json] <REMOTE>`

One line for each directory, an entry after everything below it and the
path given last: `SIZE<TAB>PATH`, each path beginning with the path as
typed, sizes in 1K blocks of allocated space. A file with several hard
links is counted once.

- `-a`, `--all` — a line for every file as well.
- `-s`, `--summarize` — the total for the path given, alone.
- `-d <N>`, `--max-depth=<N>` — lines only for entries at most N levels
  below the path given; the totals count everything.
- `-h`, `--human-readable` — sizes in powers of 1024 (`4.0K`, `12M`).

Exit status: 0; 1 when something could not be read.

Unlike `du`: one path to a command; a daemon on Windows counts a file's
length, where Unix counts the blocks allocated to it.

### df
`blit df [-h] [-k] [--json] <REMOTE>`

The filesystem that holds the path, as `df -k` prints it: the header
`Filesystem 1K-blocks Used Available Use%`, then one line with the path
as typed in the first column. A path on another mount than its module
reports that mount.

- `-h`, `--human-readable` — sizes in powers of 1024.
- `-k` — sizes in 1K blocks (the default).

Unlike `df`: one path to a command; the first column is the path asked
about, not a device's name, and there is no `Mounted on` column.

### rm
`blit rm [-r] [-f] [-y] [--json] <REMOTE>...`

Removes each path, one request to each daemon named. Nothing is printed
when everything went; each entry that could not be removed is reported as
`rm` reports it, the other paths are still processed, and the exit status
is 0 only when every path is gone. A symbolic link is removed as the link.
A wildcard in a path is not expanded: it is a name.

- `-r`, `-R`, `--recursive` — remove directories and what is in them. A
  directory without it is refused (`Is a directory`).
- `-f`, `--force` — a path that is not there is no error; never ask.
- `-y`, `--yes` — do not ask.

Unlike `rm`: **it asks once, before anything is removed**, as `mirror` and
`move` do, naming the paths; `-y` or `-f` skips the question. Answering no
removes nothing and exits 0; when no answer can be read (a script that
passed neither option) nothing is removed and the exit status is 1. A
module itself, and anything in a read-only module, is refused. A path that
is a symbolic link typed with a `/` after it (`link/`) is not removed
through: `rm` empties the directory the link points to, `blit rm` says
`Not a directory` and removes nothing.

### find
`blit find [--json] [-y] <REMOTE> [-name <PATTERN>] [-iname <PATTERN>] [-type f|d|l] [-maxdepth <N>] [-size [+|-]<N>[c|k|M|G]] [-mtime [+|-]<N>] [-delete]`

One path to a line, each beginning with the path as typed, for every
entry all the tests given hold for — the path itself included, a
directory before what is in it.

- `-name <PATTERN>` — the entry's own name fits the pattern: `*`, `?`,
  `[...]` (ranges, `!` or `^` to negate, `[:alpha:]`-style classes) and
  `\` for the next character as itself. A leading `.` is matched like any
  character. Quote the pattern from the shell.
- `-iname <PATTERN>` — the same, ignoring case as `find` does.
- `-type f|d|l` — a regular file, a directory, a symbolic link.
- `-maxdepth <N>` — enter nothing more than N levels below the path.
- `-size [+|-]<N>[c|k|M|G]` — the entry's size, in units rounded UP,
  against N: 512-byte blocks with no letter, bytes with `c`, then kibi-,
  mebi- and gibibytes. `+N` is more than N, `-N` less.
- `-mtime [+|-]<N>` — whole 24-hour periods since the entry was last
  written, any part of one dropped, by the daemon's clock.
- `-delete` — REMOVE every entry the tests hold for, and print nothing
  but what could not be removed: what is in a directory before the
  directory, a directory only when that leaves it empty (`cannot delete
  ‘PATH’: Directory not empty`, exit status 1), going on past what will
  not go. It must be the last word: in `find` a test written after it
  does not limit it, so anything after it is refused. The same expression
  without `-delete` lists what it would try to remove.
- `-y`, `--yes` — do not ask before `-delete`.

Exit status: 0; 1 when something could not be read or removed.

Unlike `find`: one path to a command; no operators (`-o`, `!`,
parentheses) and no other test or action — every test given must hold.
`-delete` **asks once before anything is removed**, with the same rule as
`rm` for no answer; it is refused in a read-only module and never removes
a module itself; and it removes nothing by a path that goes through a
symbolic link — `find link/ -delete` empties the directory the link
points to, and `find alias/entry -delete` removes through `alias`, where
`blit find` says so and removes nothing. Such a path is still listed.
Stopping the command stops the removal on the daemon; what had gone by
then is gone.

## OPTIONS
### Transfer Options
- `--dry-run`  
  Enumerate and plan the transfer without modifying the destination.

- `--checksum`  
  Force checksum validation for changed files (metadata comparison is the default).

- `--resume`  
  Use block-level comparison to continue eligible partial files. This applies
  to local, push, pull, and remote-to-remote transfers.

- `--retry <N>`
  Retry up to `N` times after a transient failure. Each attempt re-runs
  destination comparison, so normal comparison skips files now complete;
  flags that force copying still apply. With `--resume`, eligible partial
  files continue at block granularity. The default is 0 (no retries).

- `--wait <SECONDS>`
  Wait this many seconds between retries (default: 5).

- `--verbose`  
  Emit detailed transfer diagnostics.

- `--progress`  
  Show an interactive ASCII spinner while the transfer runs.

- `--force-grpc`
  Bypass the TCP data plane negotiation and stream payloads over gRPC.

- `--yes`, `-y` (mirror, move)
  Skip the confirmation prompt for destructive operations. By default, `mirror`
  prompts before deleting extraneous files at the destination, and `move` prompts
  before deleting the source after transfer.

### Admin Options
- `--wait <SECONDS>` (scan)  
  Duration to wait for mDNS responses (default: 2).

- The options of `ls`, `du`, `df`, `rm` and `find` are listed with each
  command under UTILITY COMMANDS.

## DIAGNOSTICS
`blit diagnostics perf` inspects and manages the local performance history.

- `--limit <N>` shows the most recent `N` entries (0 = all).
- `--enable` / `--disable` toggle capture.
- `--clear` removes the stored history file.

`blit diagnostics dump <SOURCE> <DESTINATION>` prints a pasteable snapshot of
what blit sees for an invocation — parsed endpoints, rsync destination
resolution, filesystem caps, free/total disk space, and (for local pairs)
whether source and destination are on the same device. No transfer is
performed. Intended for bug reports: `blit diagnostics dump SRC DST --json
> bug.log` captures everything a maintainer needs to reproduce the setup.

- `--json` emits machine-readable JSON instead of the human-readable default.

## CONFIGURATION DIRECTORY
- `--config-dir <PATH>` overrides the default configuration directory.

## FILES
The configuration directory is `~/Library/Application Support/com.Blit.Blit`
on macOS, `${XDG_CONFIG_HOME:-$HOME/.config}/blit` on Linux, and
`%LOCALAPPDATA%\Blit` on Windows (files in the old roaming
`%APPDATA%\Blit\Blit\config` are moved there once).

- `perf_local.jsonl` – local performance history.
- `settings.json` – persisted CLI settings.
- `config.toml` – `[jobs] keep = N` keeps the newest N job logs (default 50).
- `jobs/logs/` – this machine's job logs, one compressed JSON-lines file
  per run; every copy, mirror and move writes one.
- `jobs/saved/` – saved jobs, `<name>.json`, kept until deleted.
- `jobs/runs/` – this machine's jobs: for each run, what was run
  (`<id>.spec.json`) and how it went (`<id>.run.json`); `[jobs] keep`
  applies here too.

## SECURITY
Remote transfers do not include built-in TLS encryption. Data is transmitted in
plaintext over the TCP data plane and gRPC control plane. Operators are expected
to secure remote transfers through one of the following methods:

- **Trusted private network**: Run blit daemons only on isolated, trusted networks
  where traffic cannot be intercepted.

- **SSH tunnel**: Forward the daemon port through SSH:
  ```
  ssh -L 9031:localhost:9031 remote-host
  blit mirror /local/path localhost:/module/path
  ```

- **VPN**: Connect clients and servers via an encrypted VPN tunnel.

- **Reverse proxy with TLS**: Place the daemon behind a TLS-terminating reverse
  proxy (e.g., nginx, Caddy) for encrypted connections.

The daemon binds to `0.0.0.0` by default. In untrusted environments, use
`--bind 127.0.0.1` and access via SSH tunnel or VPN.

## SEE ALSO
`blit-daemon(1)`
