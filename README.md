# Blit 

Blit delivers a high-performance, extensible file enumeration, planning, transfer, and orchestration platform for robust local and remote backups, file migration, and cross-platform syncing. With both CLI and daemon interfaces, async-aware planning, and strong Windows support, Blit is designed for speed and reliability—whether run interactively or as part of automated workflows.

---

## Features

- **Modular Core Engine**
  Async file enumeration, planner, and one unified transfer session in `blit-core` for maximum performance and extensibility.
- **CLI and Daemon Binaries**
  Minimal, ergonomic command-line interface; full daemon/server for automation and concurrent requests.
- **Nothing a single file does ends the run**
  A file that fails on its own — locked, vanished, changed while being read, or not writable at the destination — is reported by name and the rest of the run lands. `--retries N` (`-R`, default 1) re-scans and retries exactly those files at the end of the run after `--retry-wait S` seconds (`-W`, default 30), robocopy-style. A run that completes with per-file failures exits `2` and names them; `--retry N`/`--wait S` re-run the whole transfer after a transient failure, re-applying the selected comparison so complete files skip.
- **Resumable Transfers**
  With `--resume`, eligible partial files continue through block-level Blake3 comparison across local, push, pull, and remote-to-remote transfers.
- **Job logs, run records and saved jobs**
  Every run writes a compressed JSON log and a run record on each machine involved, under one job ID. `blit jobs list|log|watch|cancel|save|export|run|retry|delete` read, re-run and retry them; `--save <name>`, `--export <file>` and `--detach` on transfers. Each machine keeps its newest 50 (`[jobs] keep`).
- **Comparison you choose**
  The default compare is size + modification time; `--checksum` compares content (Blake3) on both ends. Filters: `--include`/`--exclude` globs (an `--exclude` name matches at any depth and takes a directory's contents, a path is relative to the source, an absolute path under the source is made relative), `--files-from`, size and age bounds. Filenames that are not valid UTF-8 transfer by their exact bytes.
- **Hybrid Transport**
  TCP data plane by default, with an in-stream gRPC carrier for diagnostics or when direct TCP is unavailable.
- **Platform Optimization**
  Windows, Linux, and macOS optimized; per-filesystem capability detection (reflink, sparse, xattr) with platform-native fast-copy paths (`clonefile`, `copy_file_range`, `CopyFileEx`).
- **gRPC API**
  Robust proto definitions in `crates/blit-core/proto/blit.proto` enable remote orchestration and integrations.
- **Security Boundaries**
  Path containment, bounded wire records, and per-session data-plane tokens. The daemon has no built-in TLS or user authentication; operate it only on a trusted network or through an SSH tunnel or VPN.
- **Admin Utilities**
  Built into `blit`: daemon inspection and maintenance via mDNS discovery, module listing, remote `ls`/`find`/`du`/`df`/`rm`, shell completions, and performance profiling.
- **Developer Experience**
  Robust test suite, clear repo organization, scripting-friendly `--json` output on most commands, and colour that honours `NO_COLOR` and non-TTY output.
- **Extensive Documentation**
  Agent collaboration (`AGENTS.md`), process docs, session logs (`DEVLOG.md`), and roadmap (`TODO.md`).

---

## Repository Structure

```
.
├── crates/        # Rust workspace
│   ├── blit-core/
│   ├── blit-cli/  # Produces `blit` (admin verbs included)
│   └── blit-daemon/
├── scripts/       # Helper scripts (Windows, etc.)
├── docs/          # Workflow/process docs
├── AGENTS.md      # Agent and collaboration framework
├── DEVLOG.md      # Development log/context
└── TODO.md        # Feature roadmap/tasks
```

---

## Installation

Prebuilt binaries of `blit` and `blit-daemon` for x86_64 Linux,
Apple-silicon macOS and x86_64 Windows (macOS binaries are Developer
ID-signed and notarized; Windows and Linux binaries are unsigned — verify
the `.sha256` sidecar):

**macOS — Homebrew**

```sh
brew install roethlar/blit/blit-bin
```

**Windows — winget**

```powershell
winget install Roethlar.Blit
```

**Windows — Scoop**

```powershell
scoop bucket add blit https://github.com/roethlar/scoop-blit
scoop install blit
```

**Arch Linux — AUR** (`blit-bin` installs the release binaries;
`blit` builds from source):

```sh
paru -S blit-bin   # or: yay -S blit-bin
```

**Any platform — cargo** (builds from source, needs a Rust toolchain):

```sh
cargo install blit-transfer   # installs the `blit` command
cargo install blit-daemon
```

The crates are published with each release tag from 1.0.0 on. The
older 0.1.2 crates on crates.io predate the protocol gate and refuse
every peer; update them. Remote transfers require both peers to speak
the same protocol contract version (`blit scan` prints it as
`Protocol: v8`; `--json` names it `contract_version`). Any two builds of
the same release do, whatever channel installed them; a peer on a
different contract is refused at session open with both versions named.

**Any platform — GitHub Releases**: download the archive for your
platform plus its `.sha256` from
[Releases](https://github.com/roethlar/Blit/releases), verify, and
put the two binaries on `PATH`.

`blit --version` reports `<version>+<git sha>` for release archives and
package builds, so you can confirm exactly what you installed (a build
from a tree without git history reports `+unknown.<nonce>`; a dirty tree
appends `.dirty.<hash>`).

---

## Quick Start

### Prerequisites

- The current stable [Rust toolchain](https://www.rust-lang.org/tools/install)
  (the project does not declare an older minimum supported Rust version)
- No separate `protoc` install is required; the build uses a vendored compiler
- Windows, Linux, or macOS

### Building & Testing

```sh
git clone https://github.com/roethlar/Blit.git
cd Blit
cargo build                # Compile the full workspace
cargo test                 # Run all tests

# Windows (with scripting/log capture)
scripts/windows/run-blit-tests.ps1
```

### Usage

```sh
# Local mirror (copy + delete extraneous)
blit mirror ./source ./dest --yes

# Remote push to daemon
blit mirror ./local server:/module/ --yes

# Remote pull from daemon
blit mirror server:/module/ ./local --yes

# Remote-to-remote copy (daemon-to-daemon; destination daemon must allow delegation)
blit copy server-a:/module/data/ server-b:/module/data/

# Resume interrupted transfer (block-level comparison)
blit mirror ./source ./dest --resume

# Show progress
blit copy ./large_file.iso ./dest/ --progress

# Every run leaves a log and a record: list them, read one, retry its failed files
blit jobs list
blit jobs log <job-id>
blit jobs retry <job-id>

# Keep a transfer as a saved job and run it again later
blit mirror ./local server:/module/ --yes --save nightly
blit jobs run nightly
```

`blit move` (copy, then delete the source), `blit check` and
`blit diagnostics` round out the verbs; `blit --help` lists them all.

### Running the Daemon

```sh
blit-daemon --config /etc/blit/config.toml
```

See `docs/cli/blit-daemon.1.md` and `docs/DAEMON_CONFIG.md` for configuration details.

### Admin Utilities

```sh
# Discover daemons on the local network
blit scan

# List modules exported by a daemon
blit list-modules server

# Browse remote directories
blit ls server:/module/path

# Search for files on a remote daemon (glob pattern)
blit find server:/module/ --pattern "*.csv"

# Disk usage summary
blit du server:/module/path --json

# Filesystem statistics
blit df server:/module

# Remove remote files (with confirmation)
blit rm server:/module/path/old-data

# Show local performance history
blit profile

# Install shell-completion scripts (bash / zsh / fish / powershell / elvish)
blit completions shell bash > ~/.local/share/bash-completion/completions/blit
blit completions shell zsh  > "${fpath[1]}/_blit"
blit completions shell fish > ~/.config/fish/completions/blit.fish
```

`blit completions remote <REMOTE> [--prefix <STR>] [--files] [--dirs]`
asks a daemon for path completions under `<REMOTE>`, a module path such
as `server:/module/dir/` (a bare host is refused: the target must
include a module); `--prefix` narrows the returned set. It is a helper
for shell integrations; the generated completion scripts are static and
do not call it.
