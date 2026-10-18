# G5b Windows legs — netwatch-01, run 39080085302's `19ace1ac` archives

**Date:** 2026-10-18 (UTC). Local times in `jobs list` and file listings are EDT on the Mac and on netwatch-01.
**Scope:** RELEASE_1_0 G5b, the four legs that involve Windows: mac→win push, win→mac pull, win→linux push, linux→win pull. The two Mac↔Linux legs are in `docs/bench/g5b-2026-10-17/`.
**Result:** Leg 1 PASS, Leg 2 PASS, Leg 3 PASS, Leg 4 PASS. Every recorded leg command exited 0.
- Legs 2 and 3 have a Windows source and a non-Windows destination, so they ran with `--drop-windows-metadata`. Without it the destination refuses by design (contract v5; see §6).
- Leg 1 ran under a program-scoped firewall rule. Under the 9031-only rule first used, it failed `DATA_PLANE_FAILED` (see §4).
- Both of those first attempts are recorded below as observations, not as leg results.

**Method:** the same as `docs/bench/g5b-dryrun-2026-10-14/README.md` and `docs/bench/g5b-2026-10-17/README.md`: the same tree layout and contents, manifests compared with `diff`. Daemons ran on Windows (legs 1, 2) and on magneto (legs 3, 4). The Mac was a client only.

`$S` = `/private/tmp/claude-501/-Users-michael-Dev-blit/a1d34dfa-b6fa-407a-815b-9dd6d2b4dbb6/scratchpad/g5b-win`
Raw evidence is in `$S/logs/`. Each step has a `.txt` file (command, UTC start and end, exit), a `.out` (stdout) and a `.err` (stderr). Windows steps also have Windows-side copies under `win-out*/`. Manifests, entry lists, per-entry details (`details-*.txt`: size, attributes, mtime, reparse points, name code units), daemon logs and cleanup records are there too.

The work on netwatch-01 ran in three sessions. Each one created `C:\Users\michael\blit-g5b-20261018\` and removed it again at the end:

| session | UTC | what | evidence |
|---|---|---|---|
| s1 | 00:16–00:22 | identities, Windows tree, the win→linux push without the flag (refused) | `logs/s1-refusal/` |
| s2 | 00:24–00:33 | Windows daemon behind a 9031-only rule (mac→win failed); legs 3 and 4 | `logs/s2-portrule/` |
| s3 | 00:34–00:38 | Windows daemon behind a program-scoped rule; legs 1 and 2 | `logs/s3-programrule/` |

## 1. Binary identities

Source: GitHub Actions run 39080085302 (roethlar/Blit). Workflow `CI`, event `push`, branch `master`, conclusion `success`, headSha `19ace1acf170a1c93b759e5881859019755c226f` (`logs/run-39080085302.json`). The artifacts are the run's three `Build Release` archives.

| | Mac (aarch64-apple-darwin) | Linux (x86_64-unknown-linux-gnu) | Windows (x86_64-pc-windows-msvc) |
|---|---|---|---|
| archive sha256 | `cad7cf6e36e4354fb1604332f7d872bcf3111a793e6dfd04a6a22adcce175ce0` | `bb5d06f44e1493f5008616730c05d354888780c295547d491dd95d48d84f7679` | `bc05520d14f6b1752ea37eb9b71c5d36cc367ea65267c4e850fd5149fe677ff4` |
| sidecar check | `shasum -a 256 -c` on Mac: `OK` | `shasum -a 256 -c` on Mac: `OK`; `sha256sum -c` on magneto after scp: `OK` (each session) | sidecar has CRLF line ends, so compared by hand: Mac `shasum` and Windows `Get-FileHash` both equal the sidecar (each session) |
| BUILD.txt | `19ace1acf170a1c93b759e5881859019755c226f` | `19ace1acf170a1c93b759e5881859019755c226f` | `19ace1acf170a1c93b759e5881859019755c226f` (CRLF) |
| `blit --version` | `blit 1.0.0+19ace1acf170` | `blit 1.0.0+19ace1acf170` (on magneto) | `blit 1.0.0+19ace1acf170` (on netwatch-01) |
| `blit-daemon --version` | `blit-daemon 1.0.0+19ace1acf170` | `blit-daemon 1.0.0+19ace1acf170` (on magneto) | `blit-daemon 1.0.0+19ace1acf170` (on netwatch-01) |
| blit sha256 | `e7f926811a90dfc7e329dfa2e5d64f26333c1b6ce97075ebbfe08a60542cdf67` | `34140a2c7494971030496aa227ecfd2219a3056235f3f7ce040adbd0d0da5f22` | `3cdb41c46834d2047d6018c849091033b60acbb52e449028c7aa0d1261a95c2a` |
| blit-daemon sha256 | `8585a7820a84a6988f31900acd676f3893fcf57e6f2e87acc6a22948b3a99be5` | `f6a556a16926f8b3f67adadf1328872bc309ab83cfdfde148a81c74867847ecd` | `960c2344b26ac41213cabac9b22e0d43a9d5ef160cab9aaa71a55d2e4d909247` |

All three platforms report `1.0.0+19ace1acf170`, the expected identity.
- The Linux binary hashes are the same on the Mac (unpacked there) and on magneto.
- The Windows `.exe` hashes are the same on the Mac and on netwatch-01 (`Expand-Archive`).
- Archive contents: the two binaries plus `README.md`, `LICENSE`, `CHANGELOG.md`, `BUILD.txt`.

Downloads are in `$S/dl/`. My scripts and configs are in `$S/scripts/`; the Windows helpers are ASCII-only PowerShell in `$S/scripts/win/`.

## 2. Hosts and daemons

### netwatch-01 (Windows)

- `ssh michael@10.1.10.98` reports hostname `netwatch-01`. The OS is **Microsoft Windows 11 Pro, Version 10.0.26300, build 26300**, not the 26100 recorded earlier. Windows PowerShell is 5.1.26100.9549.
- NTFS. The ssh token is elevated. Ethernet is `10.1.10.98`, with network profile Private.
- Windows resolves `magneto` to A `10.1.10.10` plus four AAAA addresses (`2601:189:4602:3357::185`, `fd4d:4022:4ecf:10::7c8`, `2601:189:4602:3357::7c8`, `fd4d:4022:4ecf:10::185`). The Windows client commands used `10.1.10.10`.

Daemon config `C:\Users\michael\blit-g5b-20261018\config.toml`, verbatim (the module path is a TOML literal string, so the backslashes stay as written):

```toml
[daemon]
bind = "10.1.10.98"
port = 9031
no_mdns = true

[[module]]
name = "g5b"
path = 'C:\Users\michael\blit-g5b-20261018\module'
read_only = false
comment = "G5b Windows-legs module"
```

**Kept alive:** a process started inside an ssh session is ended with that session, so the daemon ran in the foreground of a long-lived ssh opened in the background on the Mac. `STATE_DIRECTORY` pointed inside the work dir, as on magneto. The command, from the Mac:

`(nohup ssh -n -o BatchMode=yes -o ServerAliveInterval=30 michael@10.1.10.98 '$env:STATE_DIRECTORY="C:\Users\michael\blit-g5b-20261018\state"; & C:\Users\michael\blit-g5b-20261018\dl\blit-x86_64-pc-windows-msvc\blit-daemon.exe --config C:\Users\michael\blit-g5b-20261018\config.toml' > $L/daemon-win.ssh.log 2>&1 &)`

- s2: Mac ssh PID 49286, daemon PID 21024 (its parent: `powershell.exe -c …`).
- s3: Mac ssh PID 52283, daemon PID 20128.
- Each time, `Get-NetTCPConnection` showed `LISTEN 10.1.10.98:9031` owned by the daemon.
- `blit list-modules 10.1.10.98` from the Mac exited 0: `g5b (rw)	\\?\C:\Users\michael\blit-g5b-20261018\module`.

**Firewall:** one inbound rule named `blit-g5b`, removed at the end of each session.
- s2: `New-NetFirewallRule -DisplayName blit-g5b -Direction Inbound -Protocol TCP -LocalPort 9031 -Action Allow`.
- s3: `New-NetFirewallRule -DisplayName blit-g5b -Direction Inbound -Protocol TCP -Program C:\Users\michael\blit-g5b-20261018\dl\blit-x86_64-pc-windows-msvc\blit-daemon.exe -Action Allow`. Its application filter is that path; its port filter is TCP, local port Any.

**Daemon output**, all of it in each session (two lines, nothing added during transfers; their order varied between sessions):

```
blitd: warn: no default root configured; server:// requests will be rejected until --root or config root is provided
blitd v2 listening on 10.1.10.98:9031
```

The warning is expected for a modules-only config.

**Where the daemon wrote** (`STATE_DIRECTORY` = `…\state`):
- `machine-id`, `jobs\logs\*.jsonl.gz` and `perf_local.jsonl` went to `STATE_DIRECTORY`.
- `recents.jsonl` went to `%LOCALAPPDATA%\Blit`: 421 B in s2, 1,472 B in s3. The daemon has no config-dir override, and recents always live in `config_dir()`.
- At startup the daemon ran its one-time move of the old roaming folder (`migrate_roaming_dir`, `crates/blit-core/src/config.rs`). It moved the owner's pre-existing `%APPDATA%\Blit\Blit\config\perf_local.jsonl` intact into `%LOCALAPPDATA%\Blit\`: 2,412 B, mtime 2026-09-20T05:40:26.1417656Z, sha256 `b5f0ac5f3ae63040629ae995421b94047393d02ed62b01800abeb8df4c7e34ab`. It then removed the three emptied roaming folders. The file was restored at each cleanup (§10).
- Because `STATE_DIRECTORY` was set, the daemon never appended to that file.

**The Windows client** (`blit.exe`, legs 3 and 4) ran in the foreground of an ssh session through `$S/scripts/win/win-run.ps1`, which uses `Start-Process -NoNewWindow -Wait` with stdout and stderr redirected to files (raw bytes) and records the exit code. Every `blit.exe` call carried `--config-dir C:\Users\michael\blit-g5b-20261018\cfg`. That kept the client's job records inside the work dir, and it meant the client never ran the roaming-folder move.

### magneto (Linux)

Host `magneto` (Arch Linux, kernel 7.2.9-arch1-1, x86_64). It is `10.1.10.10` on `enp1s0f1` (also 10.1.10.253 on `enp1s0f0`). The Mac is `10.1.10.15` on `en0`. Before each start, `~/blit-g5b-20261018` did not exist, port 9031 was free and no `blit-daemon` was running.

Config `~/blit-g5b-20261018/config.toml`, verbatim:

```toml
[daemon]
bind = "10.1.10.10"
port = 9031
no_mdns = true

[[module]]
name = "g5b"
path = "/home/michael/blit-g5b-20261018/module"
read_only = false
comment = "G5b Windows-legs module"
```

Start command, run in `~/blit-g5b-20261018` with `W` = that dir, exactly as in the earlier runs:

`STATE_DIRECTORY=$W/state XDG_CONFIG_HOME=$W/xdg/config XDG_DATA_HOME=$W/xdg/data XDG_CACHE_HOME=$W/xdg/cache XDG_STATE_HOME=$W/xdg/state setsid nohup $W/dl/blit-x86_64-unknown-linux-gnu/blit-daemon --config $W/config.toml > $W/daemon.log 2>&1 < /dev/null &`

- s1: PID 58055. s2: PID 58363, which served legs 3 and 4.
- Each was its own session leader with parent 1, and `/proc/<pid>/environ` confirmed all five variables.
- `ss -ltnp`: `LISTEN 0 128 10.1.10.10:9031 0.0.0.0:* users:(("blit-daemon",pid=58363,fd=9))`.
- From Windows, `blit.exe --config-dir … list-modules 10.1.10.10` exited 0: `g5b (rw)	/home/michael/blit-g5b-20261018/module`.
- Full daemon.log for each run: the same two lines as on Windows, with `10.1.10.10:9031`.

## 3. Test trees

**Mac, `$S/tree`.** Built new on 2026-10-18 at 00:16:07Z, with the same layout and text contents as the earlier runs and fresh `/dev/urandom` binaries.
- 12 files across 5 directories (`.`, `a`, `a/b`, `a/b/c`, `d`), with 3 nested levels.
- An empty file `empty.dat`; a name with spaces `a/small one.txt`; a Unicode name `a/b/café.txt`, NFC `63 61 66 c3 a9`; a dot-file `.hidden`.
- Random data: `a/b/c/big.bin` (5 MiB), `a/medium.bin` (100 KiB), `d/mid2.bin` (37,000 B). Small text files of 3 B up to 49 B.
- Initial total 5,382,443 bytes (`logs/manifest-mac-v1.txt`, `logs/entries-mac-tree-v1.txt`).

**Windows, `C:\Users\michael\blit-g5b-20261018\tree\`.** Built in s1 and again in s2, each time as the same 12 files as the Mac's v1 tree.
- The nine small files were written from base64 of the Mac files (`$S/scripts/win/win-mktree.ps1`).
- The name `café.txt` was built from `[char]0x00E9`. The three binaries were copied from the Mac with `scp`.
- Both times the Windows manifest and entry list were **byte-identical** to the Mac's: names in UTF-8, hashes, 17 entries. `café.txt` reads as UTF-16 `0063 0061 0066 00e9 …`, UTF-8 `63 61 66 c3 a9`, NFC=True. There are 0 reparse points.

**Manifests.** On Unix: `find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256` (`sha256sum` on Linux). Full entry lists: `find . -print | LC_ALL=C sort`.
On Windows, `$S/scripts/win/win-manifest.ps1` writes the same format:
- lowercase `Get-FileHash` hashes, `./`-relative paths with `/`;
- ordinal sort, which for these names equals the C-locale byte order;
- LF line ends, UTF-8 without BOM.
So every manifest is compared with plain `diff`, which also checks that filename bytes match.

## 4. Leg 1 — Mac → Windows (push)

Run in s3. The module directory on Windows was empty before 1a (`.` only).

| step | command | exit | stdout (verbatim) |
|---|---|---|---|
| 1a initial | `$S/dl/mac/blit-aarch64-apple-darwin/blit mirror $S/tree/ 10.1.10.98:/g5b/pushed/ --yes` | 0 | `Transfer complete: 12 file(s), 5382443 bytes.` / `Destination: 10.1.10.98:/g5b/pushed` |
| 1b after edit | same | 0 | `Transfer complete: 1 file(s), 39 bytes.` / `Destination: 10.1.10.98:/g5b/pushed` |
| 1c converged | same | 0 | `Remote already up to date; nothing to upload (10.1.10.98:/g5b/pushed).` / `Transfer complete: 0 file(s), 0 bytes.` / `Destination: 10.1.10.98:/g5b/pushed` |

Each run's stderr was only `starting mirror $S/tree/ -> 10.1.10.98:/g5b/pushed`.

- **After 1a: PASS.** `manifest-mac-v1.txt` and `manifest-win-leg1-1.txt` are identical: 12 vs 12 files. The entry lists are identical (17 entries).
  The module holds only `pushed\` and, under it, the expected 4 dirs and 12 files (18 entries counting `.` and `pushed`). There are 0 reparse points and no extra entries; every file is attribute `Archive` only.
- **The 1b edit:** `d/readme.txt` changed from `readme v1\n` (10 B) to `readme v2 - content changed for leg 1b\n` (39 B). Its sha256 went `8c49fa09…05bf` → `66172d3c…c6b0`, and its mtime moved from 20:16:07 to 20:36:06 local.
- **After 1b: PASS.** `manifest-mac-v2.txt` and `manifest-win-leg1-2.txt` are identical, and so are the entry lists. Exactly 1 file was transferred.
- **1c: PASS.** 0 files transferred (converged). The manifest, entry list and every size, mtime and attribute on Windows are unchanged.

**Observation, not this leg's result: the first attempt under a 9031-only firewall rule (s2).**
That rule is `New-NetFirewallRule … -LocalPort 9031 …`. It is also the Windows recipe that `docs/DAEMON_CONFIG.md` "Firewall Configuration" gave at the candidate `19ace1ac`. It opens only 9031, and so it **fails on the default data path**. The recipe has since been corrected (`d0a9a7a8`, docs only, after the candidate).

The same command as 1a exited **1** after 8 s (00:26:09–00:26:17Z) with no stdout. Its stderr, verbatim except that colour escapes are shown as `^[` and the trailing space blit prints after `Error:` is dropped:

```
starting mirror $S/tree/ -> 10.1.10.98:/g5b/pushed
Error:
   0: ^[[91mpushing to 10.1.10.98:/g5b/pushed^[[0m
   1: ^[[91msession DATA_PLANE_FAILED: dialing session data plane: dialing session data plane: connecting data plane 10.1.10.98:56416: Operation timed out (os error 60)^[[0m

Location:
   ^[[35mcrates/blit-core/src/transfer_session/mod.rs^[[0m:^[[35m1488^[[0m

Backtrace omitted. Run with RUST_BACKTRACE=1 environment variable to display it.
Run with RUST_BACKTRACE=full to include source snippets.
```

- **Why:** the default TCP data plane has the responder (the daemon) listen on a fresh ephemeral port that the client dials (`docs/TRANSFER_SESSION.md` "Transport selection"). Windows Defender Firewall drops that SYN.
- **Probe from the Mac** (`logs/s2-portrule/leg1-1-firewall-probe.txt`):
  - `nc -z 10.1.10.98 9031`: `Connection to 10.1.10.98 port 9031 [tcp/*] succeeded!` in 0.01 s.
  - `nc -z 10.1.10.98 9032`: `connectx to 10.1.10.98 port 9032 (tcp) failed: Operation timed out` after 3.02 s, i.e. dropped.
  - For comparison, magneto (no host firewall) answered a closed port with `Connection refused`.
- **Nothing was written:** the Windows module stayed `.` only.
- **Both ends recorded the failure.** Mac `blit jobs list`: `8672421b5804f90117e9001d6e0c59cd  2026-10-17 20:26:09  mirror  $S/tree/ -> 10.1.10.98:/g5b/pushed/  failed: pushing to 10.1.10.98:/g5b/pushed: session DATA_PLANE_FAILED: dialing session data plane: dialing session data plane: connecting data plane 10.1.10.98:56416: Operation timed out (os error 60) (0 copied, 0 deleted, 0 failed)`.
  `blit jobs list 10.1.10.98`: `t1792283170515-0  push  g5b/pushed  peer=10.1.10.15:58521  duration=7.8s  FAILED: dialing session data plane: dialing session data plane: connecting data plane 10.1.10.98:56416: Operation timed out (os error 60)`.
  The daemon's job log `8672421b….destination.1.jsonl.gz` carries the same run ID.
- The client did not fall back to the gRPC data plane.
- This was not retried under the same rule. Legs 1 and 2 ran in s3 under the program-scoped rule.

## 5. Leg 2 — Windows → Mac (pull)

Run in s3, right after Leg 1. The destination `$S/pulled-from-win/` did not exist before; I created it new and empty (0 entries). The Windows daemon is the source and the Mac the destination, so the command carries `--drop-windows-metadata` (see §6 for why).

| step | command | exit | stdout (verbatim) |
|---|---|---|---|
| 2a initial | `$S/dl/mac/blit-aarch64-apple-darwin/blit mirror 10.1.10.98:/g5b/pushed/ $S/pulled-from-win/ --drop-windows-metadata --yes` | 0 | `Pull complete: 12 file(s), 5382472 bytes -> $S/pulled-from-win/.` |
| 2b converged | same | 0 | `Pull complete: 0 file(s), 0 bytes -> $S/pulled-from-win/.` |

Each run's stderr was the flag's own warning, then the start line:
`warning: --drop-windows-metadata permanently discards Windows file attributes and named data streams` / `starting mirror 10.1.10.98:/g5b/pushed -> $S/pulled-from-win/`.

- 5,382,472 = 5,382,443 − 10 + 39, which matches the edited tree. The pulled files also sum to 5,382,472 bytes on disk.
- **Manifest after 2a: PASS.** `manifest-mac-v2.txt` and `manifest-pulled-1.txt` are identical: 12 vs 12 files. The entry lists are identical, there are no non-regular entries, and `café.txt` is `63 61 66 c3 a9`.
- **After 2b: PASS.** The manifest, entry list, sizes and mtimes are unchanged, and 0 files were transferred (converged).
- The source on Windows was unchanged after Leg 2 (`manifest-win-after-leg2.txt` is identical to `manifest-mac-v2.txt`).

## 6. Leg 3 — Windows → Linux (push)

Run in s2 against magneto's daemon (PID 58363). The module directory on magneto was empty before 3a.

| step | command | exit | stdout (verbatim) |
|---|---|---|---|
| 3a initial | `C:\Users\michael\blit-g5b-20261018\dl\blit-x86_64-pc-windows-msvc\blit.exe --config-dir C:\Users\michael\blit-g5b-20261018\cfg mirror C:\Users\michael\blit-g5b-20261018\tree\ 10.1.10.10:/g5b/pushed-from-win/ --drop-windows-metadata --yes` | 0 | `Transfer complete: 12 file(s), 5382443 bytes.` / `Destination: 10.1.10.10:/g5b/pushed-from-win` |
| 3b after edit | same | 0 | `Transfer complete: 1 file(s), 39 bytes.` / `Destination: 10.1.10.10:/g5b/pushed-from-win` |
| 3c converged | same | 0 | `Remote already up to date; nothing to upload (10.1.10.10:/g5b/pushed-from-win).` / `Transfer complete: 0 file(s), 0 bytes.` / `Destination: 10.1.10.10:/g5b/pushed-from-win` |

Each run's stderr was the flag's warning, then the start line:
`warning: --drop-windows-metadata permanently discards Windows file attributes and named data streams` / `starting mirror C:\Users\michael\blit-g5b-20261018\tree\ -> 10.1.10.10:/g5b/pushed-from-win`.

- **After 3a: PASS.** `manifest-win-tree-v1.txt` and `manifest-linux-leg3-1.txt` are identical: 12 vs 12 files. The entry lists are identical (17 entries).
  A full `find` under `module/` shows only `pushed-from-win/` and the expected 4 dirs and 12 files, at the expected sizes. There are no non-regular entries, and `café.txt` is `63 61 66 c3 a9` on magneto.
- **The 3b edit (on Windows):** `d\readme.txt` changed from `readme v1\n` (10 B) to `readme v2 - content changed for leg 3b\n` (39 B). Its sha256 went `8c49fa09…05bf` → `f97b08b8…d7b998`, and its mtime moved from 00:24:54.947Z to 00:28:51.583Z.
- **After 3b: PASS.** `manifest-win-tree-v2.txt` and `manifest-linux-leg3-2.txt` are identical, and so are the entry lists. Exactly 1 file was transferred.
- **3c: PASS.** 0 files transferred (converged). The manifest, entry list, sizes and mtimes on magneto are unchanged.

**Observation, not this leg's result: the contract-v5 refusal held (s1).**
The same push without the flag — `…\blit.exe --config-dir C:\Users\michael\blit-g5b-20261018\cfg mirror C:\Users\michael\blit-g5b-20261018\tree\ 10.1.10.10:/g5b/pushed-from-win/ --yes` — exited **1** (00:18:32Z) with no stdout. Its stderr, verbatim except that colour escapes are shown as `^[` and the trailing space blit prints after `Error:` is dropped:

```
starting mirror C:\Users\michael\blit-g5b-20261018\tree\ -> 10.1.10.10:/g5b/pushed-from-win
Error:
   0: ^[[91mpushing to 10.1.10.10:/g5b/pushed-from-win^[[0m
   1: ^[[91msession INTERNAL: this destination cannot preserve Windows file attributes and named data streams; rerun with --drop-windows-metadata to discard them explicitly^[[0m

Location:
   ^[[35mcrates\blit-core\src\transfer_session\mod.rs^[[0m:^[[35m1488^[[0m

Backtrace omitted. Run with RUST_BACKTRACE=1 environment variable to display it.
Run with RUST_BACKTRACE=full to include source snippets.
```

- **This is the documented policy** (`docs/TRANSFER_SESSION.md` "Cross-platform Windows metadata policy (contract v5)"): strict preservation by default.
  A Windows source attaches Windows metadata to every regular file; this tree carried only the default `Archive` attribute and no named streams. A non-Windows destination refuses the session before any write, and the message names the remedy.
- **Nothing was written:** magneto's module stayed `.` only.
- **Both ends recorded the failure.** Windows `blit.exe jobs list`: `2a957bb06c2c275927724a1d65727db2  2026-10-17 20:18:32  mirror  C:\Users\michael\blit-g5b-20261018\tree\ -> 10.1.10.10:/g5b/pushed-from-win/  failed: pushing to 10.1.10.10:/g5b/pushed-from-win: session INTERNAL: this destination cannot preserve Windows file attributes and named data streams; rerun with --drop-windows-metadata to discard them explicitly (0 copied, 0 deleted, 0 failed)`.
  `blit jobs list magneto`: `t1792282712295-0  push  g5b/pushed-from-win  peer=10.1.10.98:64342  duration=4ms  FAILED: this destination cannot preserve Windows file attributes and named data streams; rerun with --drop-windows-metadata to discard them explicitly`.
  Both job logs (`initiator` on Windows, `destination` on magneto) end `"outcome":"failed"` under the same run ID.

## 7. Leg 4 — Linux → Windows (pull)

Run in s2, right after Leg 3. The destination `C:\Users\michael\blit-g5b-20261018\pulled\` did not exist before; I created it new and empty (0 entries). The source is Linux, so there is no flag.

| step | command | exit | stdout (verbatim) |
|---|---|---|---|
| 4a initial | `C:\Users\michael\blit-g5b-20261018\dl\blit-x86_64-pc-windows-msvc\blit.exe --config-dir C:\Users\michael\blit-g5b-20261018\cfg mirror 10.1.10.10:/g5b/pushed-from-win/ C:\Users\michael\blit-g5b-20261018\pulled\ --yes` | 0 | `Pull complete: 12 file(s), 5382472 bytes -> C:\Users\michael\blit-g5b-20261018\pulled\.` |
| 4b converged | same | 0 | `Pull complete: 0 file(s), 0 bytes -> C:\Users\michael\blit-g5b-20261018\pulled\.` |

Each run's stderr was only `starting mirror 10.1.10.10:/g5b/pushed-from-win -> C:\Users\michael\blit-g5b-20261018\pulled\`.

- 5,382,472 B matches the edited Windows tree.
- **Manifest after 4a: PASS.** `manifest-win-tree-v2.txt` and `manifest-win-pulled-1.txt` are identical: 12 vs 12 files. The entry lists are identical.
  `pulled\` holds 12 files (all attribute `Archive`) and 4 dirs, with 0 reparse points. `café.txt` reads as UTF-16 `00e9`, NFC=True.
- **After 4b: PASS.** The manifest, entry list, sizes, mtimes and attributes are unchanged, and 0 files were transferred (converged).
- The source on magneto was unchanged after Leg 4 (`manifest-linux-after-leg4.txt` is identical to `manifest-win-tree-v2.txt`).

## 8. Job records

`blit jobs list` (Mac) exited 0. It printed `Jobs on this machine (50), newest first:`; the 50 is the record store's cap. The six newest lines are this run's, verbatim:

```
  0873603cf0dd8ecd82d72ef8e44e117a  2026-10-17 20:36:47  mirror  10.1.10.98:/g5b/pushed/ -> $S/pulled-from-win/  ok (0 copied, 0 deleted, 0 failed)
  83871f8f2136ed669c768146d908af44  2026-10-17 20:36:39  mirror  10.1.10.98:/g5b/pushed/ -> $S/pulled-from-win/  ok (12 copied, 0 deleted, 0 failed)
  fc799f5cb25cbbd5f55ff158b7689d27  2026-10-17 20:36:22  mirror  $S/tree/ -> 10.1.10.98:/g5b/pushed/  ok (0 copied, 0 deleted, 0 failed)
  4e454b851d98beef6e9f5d7882dadd26  2026-10-17 20:36:06  mirror  $S/tree/ -> 10.1.10.98:/g5b/pushed/  ok (1 copied, 0 deleted, 0 failed)
  a21478d86bdad18fcf1e742776b77ca2  2026-10-17 20:35:47  mirror  $S/tree/ -> 10.1.10.98:/g5b/pushed/  ok (12 copied, 0 deleted, 0 failed)
  8672421b5804f90117e9001d6e0c59cd  2026-10-17 20:26:09  mirror  $S/tree/ -> 10.1.10.98:/g5b/pushed/  failed: pushing to 10.1.10.98:/g5b/pushed: session DATA_PLANE_FAILED: dialing session data plane: dialing session data plane: connecting data plane 10.1.10.98:56416: Operation timed out (os error 60) (0 copied, 0 deleted, 0 failed)
```

(`$S` abbreviates the full scratch path in the real output.) The older lines are other local runs on this Mac.

`blit jobs list 10.1.10.98` (s3) exited 0, verbatim:

```
Daemon: blit 1.0.0 on 10.1.10.98 — uptime 1m 46s
Delegation: disabled
Modules: g5b

Active: (none)

Recent (5):
  t1792283807811-4  pull_sync  g5b/pushed  peer=10.1.10.15:58960  duration=35ms  ok
  t1792283799734-3  pull_sync  g5b/pushed  peer=10.1.10.15:58951  duration=102ms  ok
  t1792283783645-2  push  g5b/pushed  peer=10.1.10.15:58936  duration=29ms  ok
  t1792283767637-1  push  g5b/pushed  peer=10.1.10.15:58913  duration=34ms  ok
  t1792283748588-0  push  g5b/pushed  peer=10.1.10.15:58896  duration=120ms  ok

Counters: push=0 pull=0 purge=0 active=0 errors=0
```

`blit.exe --config-dir C:\Users\michael\blit-g5b-20261018\cfg jobs list` (Windows, s2) exited 0, verbatim:

```
Jobs on this machine (5), newest first:
  3541c00d60b2022225368f7d81dbab56  2026-10-17 20:29:50  mirror  10.1.10.10:/g5b/pushed-from-win/ -> C:\Users\michael\blit-g5b-20261018\pulled\  ok (0 copied, 0 deleted, 0 failed)
  a1693e8f97f295b235ed9b8e51e7a85c  2026-10-17 20:29:34  mirror  10.1.10.10:/g5b/pushed-from-win/ -> C:\Users\michael\blit-g5b-20261018\pulled\  ok (12 copied, 0 deleted, 0 failed)
  dd8347c805e07d2b0d9d48a9d58c17c6  2026-10-17 20:29:11  mirror  C:\Users\michael\blit-g5b-20261018\tree\ -> 10.1.10.10:/g5b/pushed-from-win/  ok (0 copied, 0 deleted, 0 failed)
  8909b1148a8d1bf3b4ecf81140c234bf  2026-10-17 20:28:56  mirror  C:\Users\michael\blit-g5b-20261018\tree\ -> 10.1.10.10:/g5b/pushed-from-win/  ok (1 copied, 0 deleted, 0 failed)
  b66c7e19d7d96a8b891b77964b9636e1  2026-10-17 20:28:34  mirror  C:\Users\michael\blit-g5b-20261018\tree\ -> 10.1.10.10:/g5b/pushed-from-win/  ok (12 copied, 0 deleted, 0 failed)
```

`blit jobs list magneto` (s2) exited 0, verbatim:

```
Daemon: blit 1.0.0 on magneto — uptime 2m 2s
Delegation: disabled
Modules: g5b

Active: (none)

Recent (5):
  t1792283390141-4  pull_sync  g5b/pushed-from-win  peer=10.1.10.98:56564  duration=5ms  ok
  t1792283373537-3  pull_sync  g5b/pushed-from-win  peer=10.1.10.98:56555  duration=12ms  ok
  t1792283350904-2  push  g5b/pushed-from-win  peer=10.1.10.98:56544  duration=4ms  ok
  t1792283336162-1  push  g5b/pushed-from-win  peer=10.1.10.98:56532  duration=4ms  ok
  t1792283314077-0  push  g5b/pushed-from-win  peer=10.1.10.98:56516  duration=10ms  ok

Counters: push=0 pull=0 purge=0 active=0 errors=0
```

- Every run is listed with correct copied counts.
- The daemons' job-log filenames carry the same run IDs as the clients' lists. Push runs are tagged `destination` and pull runs `source`; the Windows client's are tagged `initiator`.
- The counters are zero because `--metrics` was not set, as in the earlier runs.
- The failed first attempts' records are quoted in §4 and §6.

## 9. Anomalies and observations

- **None in any leg's recorded run.** All 10 leg commands exited 0. There were no stall notices. The only warnings were:
  - the expected warning printed by `--drop-windows-metadata` on legs 2 and 3;
  - the daemons' expected startup line `blitd: warn: no default root configured; …`.
- **The two first attempts described in §4 and §6** are observations, not leg results:
  - the contract-v5 refusal without `--drop-windows-metadata`, which held as documented;
  - `DATA_PLANE_FAILED` under the 9031-only firewall rule. At `19ace1ac`, the Windows recipe in `docs/DAEMON_CONFIG.md` "Firewall Configuration" opens only 9031 and so fails on the default data path. The same holds for its firewalld/ufw lines. Since corrected in `d0a9a7a8`.

Observations, not findings:
- **The Windows daemon outlived its ssh session.** In s2 and s3, killing the Mac-side ssh that held the daemon in its foreground left `blit-daemon.exe` running on netwatch-01: pid 21024 in s2, and pid 20128 in s3, which was still present 2 s later. Each was ended with `Stop-Process`. This contradicts the note in `.agents/machines.md` that a process started in an ssh session dies with it.
- **Remote copies carry whole-second mtimes.** The NTFS and APFS sources have sub-second mtimes (for example `00:24:54.9474109Z` on Windows), but every remote copy has whole seconds: magneto `20:24:54.0000000000`, the Windows module `00:16:07.0000000Z`, the Mac pull `1792283766.000000000` against the source's `1792283766.889859668`. This matches the wire format (`FileHeader.mtime_seconds`), and every converged re-run reported 0 files.
- **The refusal is labelled `session INTERNAL:`** in the client's error and in its job record, although `docs/TRANSFER_SESSION.md` calls it a precondition refusal of an unrepresentable request.
- **Colour escapes go to file-redirected stderr.** The error reports carry ANSI colour codes (`ESC[91m`, `ESC[35m`) even when stderr is a file. This was seen on Windows (`blit.exe` under `Start-Process -RedirectStandardError`) and also on the Mac (`2>` to a file).
- **Paths shown by `list-modules`.** `blit list-modules 10.1.10.98` shows the Windows module path with its verbatim prefix, `\\?\C:\Users\michael\blit-g5b-20261018\module`.
- **ACLs on the restored roaming folders.** The owner's file and its folders are back with the same bytes, size, attributes, creation and last-write times, and owner (§10). One thing differs: they now carry an inherited read entry `(A;OICIID;0x1200a9;;;S-1-5-21-59627942-1115594425-451533904-1002)`, an SID that is no current local account.
  - It is inherited from `%APPDATA%`. All 24 folders in `%APPDATA%` carry it now, and the original Blit folders were the only ones without it.
  - Removing it would mean breaking inheritance, so it was left.
  - The recreated folders first took `Administrators` as owner (an elevated token's default). `icacls /setowner netwatch-01\michael` put the original owner back.
- **Not exercised:** same-size content edits (each edit also changed size), `--detach`, delegation, and Windows files carrying Hidden, ReadOnly or System attributes or named streams (only the plain `Archive` bit was present).

## 10. Cleanup

**magneto** (`logs/s1-refusal/cleanup-magneto.txt`, `logs/s2-portrule/cleanup-magneto.txt`): for each daemon (58055, then 58363):
- `recorded pid=…`, `kill rc=0`, `pid … gone`;
- `no blit-daemon processes`, `9031 no longer listening`;
- the final daemon.log was the same two lines;
- nothing in `~/.config/blit` was newer than the test config;
- `rm -rf ~/blit-g5b-20261018` gave `rm rc=0`, and `ls: cannot access '/home/michael/blit-g5b-20261018': No such file or directory`.
- A final check after s3: still absent, no `blit-daemon`, 9031 free.

**netwatch-01**, after s2 and again after s3 (`logs/s2-portrule/cleanup-win*.txt`, `logs/s3-programrule/cleanup-win*.txt`):
- Killed the Mac-side ssh, then `Stop-Process` on the surviving `blit-daemon` (see §9).
- `$S/scripts/win/roaming-restore.ps1` moved `perf_local.jsonl` back from `%LOCALAPPDATA%\Blit` to `%APPDATA%\Blit\Blit\config\` and recreated the three folders. It then set every item's creation, last-write and last-access time to the values captured before the daemon started (file first, then folders innermost to outermost). `icacls /setowner` restored the folders' owner.
- Compared with the state captured before the first daemon start (`logs/s2-portrule/roaming-before.txt`), these are identical: items, attributes, creation and last-write ticks, length, owner and group, sha256 `b5f0ac5f3ae63040629ae995421b94047393d02ed62b01800abeb8df4c7e34ab`. The only difference is the inherited ACE above.
  After s3 the state is identical, ACE included, to the state at the start of s3. A byte copy of the original file is kept in `$S/logs/roaming-backup/`.
- `%LOCALAPPDATA%\Blit` held only `recents.jsonl` and was removed. `Remove-NetFirewallRule -DisplayName blit-g5b`. `Remove-Item -Recurse -Force C:\Users\michael\blit-g5b-20261018`.
- Final verification (`logs/s3-programrule/cleanup-win.txt`):
  - `workdir exists: False`, `LOCALAPPDATA\Blit exists: False`, `firewall rule blit-g5b present: False`;
  - `blit processes: 0`, `port 9031 listening: False`;
  - `roaming perf_local.jsonl: 2412 B, LastWriteTimeUtc 2026-09-20T05:40:26.1417656Z, CreationTimeUtc 2026-09-20T04:55:02.7926905Z, sha256 b5f0ac5f3ae63040629ae995421b94047393d02ed62b01800abeb8df4c7e34ab`.

No sudo was used. On netwatch-01 the test wrote outside its work dir only in three ways, each undone as described above:
- the daemon's own move of the roaming folder and its `recents.jsonl`;
- the `blit-g5b` firewall rule;
- the folder-owner reset.

On magneto nothing outside `~/blit-g5b-20261018/` was written. No blit daemon ran on the Mac, so the macOS Application Firewall was not touched. The Mac scratch dir is kept as evidence. Blit wrote its own job records, logs and perf history to the Mac per-user folder (`~/Library/Application Support/com.Blit.Blit`); nothing there was deleted.
