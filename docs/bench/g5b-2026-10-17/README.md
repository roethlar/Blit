# G5b candidate run — Mac ↔ Linux, run 38960754619's `de9a333f` archives

**Date:** 2026-10-17 (UTC); local Mac times in `jobs list` and file mtimes are EDT.
**Scope:** RELEASE_1_0 G5b with the candidate's own `Build Release` archives, the two Mac↔Linux legs only (mac→linux push, linux→mac pull). The four Windows legs are not covered here.
**Result:** Leg 1 PASS, Leg 2 PASS. Every command exited 0. There were no warnings during transfers and no stall notices.
**Method:** a repeat of `docs/bench/g5b-dryrun-2026-10-14/README.md`, step for step, with the candidate's archives in place of the dry run's.

`$S` = `/private/tmp/claude-501/-Users-michael-Dev-blit/a1d34dfa-b6fa-407a-815b-9dd6d2b4dbb6/scratchpad/g5b-20261017`
Raw evidence is in `$S/logs/`: per-step `.txt` (command, UTC start and end, exit), `.out` (stdout), `.err` (stderr), all manifests and entry lists, `daemon.log`, `cleanup.txt`.

## 1. Binary identities

Source: GitHub Actions run 38960754619 (roethlar/Blit). Workflow `CI`, event `push`, branch `master`,
conclusion `success`, headSha `de9a333f50277e0f01913cd49ab2a8c4d27af9f4`. Artifacts `blit-aarch64-apple-darwin` and `blit-x86_64-unknown-linux-gnu`, fetched with `gh run download 38960754619 -n <name>`.

| | Mac (aarch64-apple-darwin) | Linux (x86_64-unknown-linux-gnu) |
|---|---|---|
| archive sha256 (sidecar) | `6f26ed7660634f0fa4026a75c8edf1f1745b8f5ea19ee3f510371b5d1ad667e3` | `bac71f1dc2b0baf10c0eb9fb4761f453b0bf4bb4196b9b4ecd662b9702205d1f` |
| sidecar check | `shasum -a 256 -c` on Mac: `OK` | `shasum -a 256 -c` on Mac: `OK`; `sha256sum -c` on magneto after scp: `OK` |
| BUILD.txt | `de9a333f50277e0f01913cd49ab2a8c4d27af9f4` | `de9a333f50277e0f01913cd49ab2a8c4d27af9f4` |
| `blit --version` | `blit 1.0.0+de9a333f5027` | `blit 1.0.0+de9a333f5027` (run on magneto) |
| `blit-daemon --version` | `blit-daemon 1.0.0+de9a333f5027` | `blit-daemon 1.0.0+de9a333f5027` (run on magneto) |
| blit sha256 | `2d3e42a05eb5f9302abc7a0cd51d1d960325a9df6c42f9cf3bb6c85f8c837d34` | `66409c81f27062987ae6bbdf1bed6bebb9a5224453d8db442b96999952d3eb3f` |
| blit-daemon sha256 | `5cf3b59e5ac3f751b820014c174eed73df6b01ee36dd7a2aa78b13d8f815f691` | `35dad7007443a356dd882f7c13e058ac7a252e4b492939128a1b92ec96f41f1f` |

Both peers report the same identity, `1.0.0+de9a333f5027`, the expected one.
The Linux binary hashes are the same on the Mac (after unpacking) and on magneto (after scp and unpacking).
Archive contents (both): `blit`, `blit-daemon`, `README.md`, `LICENSE`, `CHANGELOG.md`, `BUILD.txt`.
Downloads are in `$S/dl-mac/` and `$S/dl-linux/`. My scripts and config are in `$S/scripts/`.

## 2. Daemon on magneto

Host: `magneto` (Arch Linux, kernel 7.2.9-arch1-1, x86_64). From the Mac, `magneto` resolves to IPv4 `10.1.10.10` (interface `enp1s0f1`; the Mac is `10.1.10.15` on `en0`). The host's other IPv4 address is 10.1.10.253 on `enp1s0f0`. Before the run, `~/blit-g5b-20261017` did not exist, port 9031 was free and no `blit-daemon` was running, so the default port was used.

Config `~/blit-g5b-20261017/config.toml`, verbatim:

```toml
[daemon]
bind = "10.1.10.10"
port = 9031
no_mdns = true

[[module]]
name = "g5b"
path = "/home/michael/blit-g5b-20261017/module"
read_only = false
comment = "G5b candidate-run module"
```

As in the dry run:
- `no_mdns = true` stops the daemon broadcasting itself on the LAN. mDNS plays no part in the test.
- The daemon was started with `STATE_DIRECTORY`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_STATE_HOME` pointed inside `~/blit-g5b-20261017/`. `/proc/<pid>/environ` confirmed all five.

Start command (run in `~/blit-g5b-20261017`, `W` = that dir):
`STATE_DIRECTORY=$W/state XDG_CONFIG_HOME=$W/xdg/config XDG_DATA_HOME=$W/xdg/data XDG_CACHE_HOME=$W/xdg/cache XDG_STATE_HOME=$W/xdg/state setsid nohup $W/dl/blit-x86_64-unknown-linux-gnu/blit-daemon --config $W/config.toml > $W/daemon.log 2>&1 < /dev/null &`

- PID 53819, recorded in `daemon.pid`. It is its own session leader, and its parent is 1.
- `ss -ltnp`: `LISTEN 0 128 10.1.10.10:9031 0.0.0.0:* users:(("blit-daemon",pid=53819,fd=9))`.
- Reachable from the Mac: `blit list-modules magneto` exited 0 and printed:
  ```
  Modules on magneto:
  g5b (rw)	/home/michael/blit-g5b-20261017/module
  ```
- Full daemon.log for the whole run (two lines, nothing added during transfers):
  ```
  blitd: warn: no default root configured; server:// requests will be rejected until --root or config root is provided
  blitd v2 listening on 10.1.10.10:9031
  ```
  This warning is expected for a modules-only config.

## 3. Test tree (Mac, `$S/tree`)

Built new for this run, with the same layout and the same text contents as the dry run. The random binaries are fresh `/dev/urandom` data.
The tree has 12 files across 5 directories (`.`, `a`, `a/b`, `a/b/c`, `d`), with 3 nested levels (`a/b/c`). It includes:
- an empty file: `empty.dat`
- a filename with spaces: `a/small one.txt`
- a unicode filename: `a/b/café.txt`, NFC `63 61 66 c3 a9`, kept as those bytes on magneto and in the pulled copy
- a dot-file: `.hidden`
- random data: `a/b/c/big.bin` (5 MiB), `a/medium.bin` (100 KiB), `d/mid2.bin` (37,000 B)
- small text files of 3 B up to 49 B

Initial total: 5,382,443 bytes.
Manifests use `find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256` (`sha256sum` on Linux). Both tools print the same `hash  ./path` format, so manifests are compared with plain `diff`. That comparison also checks that filename bytes match. Full entry lists (`find . -print | LC_ALL=C sort`, files and dirs) are compared the same way.

## 4. Leg 1 — Mac → Linux (push)

The module directory on magneto was empty before 1a.

| step | command | exit | stdout summary (verbatim) |
|---|---|---|---|
| 1a initial | `blit mirror $S/tree/ magneto:/g5b/pushed/ --yes` | 0 | `Transfer complete: 12 file(s), 5382443 bytes.` / `Destination: magneto:/g5b/pushed` |
| 1b after edit | same | 0 | `Transfer complete: 1 file(s), 39 bytes.` / `Destination: magneto:/g5b/pushed` |
| 1c converged | same | 0 | `Remote already up to date; nothing to upload (magneto:/g5b/pushed).` / `Transfer complete: 0 file(s), 0 bytes.` / `Destination: magneto:/g5b/pushed` |

Each run's stderr was only `starting mirror $S/tree/ -> magneto:/g5b/pushed`.

Manifest comparisons:
- **After 1a: PASS.** `manifest-mac-v1.txt` and `manifest-linux-leg1-1.txt` are byte-identical: 12 vs 12 files. The entry lists of `tree/` and `module/pushed/` are identical (17 entries). A full `find` under `module/` shows only `pushed/` and, under it, the expected 5 dirs plus 12 files, at the expected sizes. There are no extra entries and no non-regular entries.
- **The 1b edit:** `d/readme.txt` changed from `readme v1\n` (10 B) to `readme v2 - content changed for leg 1b\n` (39 B). Its mtime moved from 11:31:34 to 11:32:15 local. Only that line of the Mac manifest changed.
- **After 1b: PASS.** `manifest-mac-v2.txt` and `manifest-linux-leg1-2.txt` are identical: 12 vs 12 files. The entry lists are identical. There are no non-regular entries.
- **1b transferred exactly 1 file: PASS.**
- **1c transferred 0 files (converged): PASS.** After 1c, `manifest-linux-leg1-3.txt` and the entry list are still identical.

## 5. Leg 2 — Linux → Mac (pull)

The destination `$S/pulled/` did not exist before the run. I created it new and empty (0 entries).

| step | command | exit | stdout summary (verbatim) |
|---|---|---|---|
| 2a initial | `blit mirror magneto:/g5b/pushed/ $S/pulled/ --yes` | 0 | `Pull complete: 12 file(s), 5382472 bytes -> $S/pulled/.` |
| 2b converged | same | 0 | `Pull complete: 0 file(s), 0 bytes -> $S/pulled/.` |

Each run's stderr was only `starting mirror magneto:/g5b/pushed -> $S/pulled/`.

- 5,382,472 = 5,382,443 − 10 + 39, which matches the edited tree. The pulled files also sum to 5,382,472 bytes on disk.
- **Manifest after 2a: PASS.** `manifest-mac-v2.txt` and `manifest-pulled-1.txt` are identical: 12 vs 12 files.
- The full entry lists of `tree/` and `pulled/` (files and dirs) are identical. There are no non-regular entries.
- **After 2b: PASS.** `manifest-pulled-2.txt` and the entry list are still identical, and 0 files were transferred (converged).
- The source on magneto was unchanged after Leg 2 (`manifest-linux-after-leg2.txt` is identical to `manifest-mac-v2.txt`).

## 6. Job records

`blit jobs list` (Mac) exited 0. It printed `Jobs on this machine (50), newest first:`. The five newest lines are this run's, verbatim:

```
  bd4dac88396de3e34cb0bbeb1373d385  2026-10-17 11:32:38  mirror  magneto:/g5b/pushed/ -> $S/pulled/  ok (0 copied, 0 deleted, 0 failed)
  f63128235818417ef233f130fc064d51  2026-10-17 11:32:30  mirror  magneto:/g5b/pushed/ -> $S/pulled/  ok (12 copied, 0 deleted, 0 failed)
  3c56addf287f503ee67ea516a96158a5  2026-10-17 11:32:22  mirror  $S/tree/ -> magneto:/g5b/pushed/  ok (0 copied, 0 deleted, 0 failed)
  ff456a118b32a3d3d851a8714c356123  2026-10-17 11:32:15  mirror  $S/tree/ -> magneto:/g5b/pushed/  ok (1 copied, 0 deleted, 0 failed)
  e38af30b32984f04cf8fc03d6b3a9a7d  2026-10-17 11:31:52  mirror  $S/tree/ -> magneto:/g5b/pushed/  ok (12 copied, 0 deleted, 0 failed)
```

(`$S` above abbreviates the full scratch path that appears in the real output; see `logs/jobs-list.out`.)
All five runs are listed with correct copied counts. The other 45 lines are older local runs on this Mac from earlier G3 fix and review work in the same scratchpad (2026-10-15 to 2026-10-17 00:10). The count is 50 because the record store keeps the newest 50 (`blit-core/src/job_log.rs` `DEFAULT_KEEP`).

`blit jobs list magneto` exited 0, verbatim:

```
Daemon: blit 1.0.0 on magneto — uptime 1m 30s
Delegation: disabled
Modules: g5b

Active: (none)

Recent (5):
  t1792251158136-4  pull_sync  g5b/pushed  peer=10.1.10.15:53770  duration=84ms  ok
  t1792251150673-3  pull_sync  g5b/pushed  peer=10.1.10.15:53751  duration=351ms  ok
  t1792251142676-2  push  g5b/pushed  peer=10.1.10.15:53727  duration=67ms  ok
  t1792251135848-1  push  g5b/pushed  peer=10.1.10.15:53705  duration=86ms  ok
  t1792251112998-0  push  g5b/pushed  peer=10.1.10.15:53661  duration=298ms  ok

Counters: push=0 pull=0 purge=0 active=0 errors=0
```

The counters are zero because `--metrics` was not set, as in the dry run.
The daemon's job-log filenames in `state/jobs/logs/` carry the same five job IDs as the Mac list. Push runs are tagged `destination` and pull runs `source`.

## 7. Anomalies

- **None in any transfer.** No warning, no stall notice, no non-zero exit. All 8 recorded commands exited 0.
- The only warning seen was the daemon's startup line `blitd: warn: no default root configured; server:// requests will be rejected until --root or config root is provided`. It is expected for a modules-only config.
- Observations, not findings:
  - The daemon reports transfer durations of 67–351 ms, against 1.1–1.7 s in the dry run on the same tree shape. This is not a G5b measure and was not investigated.
  - As in the dry run, the daemon wrote `recents.jsonl` under `$XDG_CONFIG_HOME/blit/`. Job logs, `perf_local.jsonl` and `machine-id` went to `$STATE_DIRECTORY`.
- Not exercised: same-size content edits (the 1b edit also changed size), Windows legs, `--detach`, and delegation.

## 8. Cleanup (magneto)

From `logs/cleanup.txt`:
- `recorded pid=53819`, the `blit-daemon --config ...` process
- `kill rc=0`, then `pid 53819 gone`
- `pgrep`: `no blit-daemon processes`
- `9031 no longer listening`
- the final daemon.log was still the same two lines
- `rm -rf ~/blit-g5b-20261017`: `rm rc=0`
- `ls: cannot access '/home/michael/blit-g5b-20261017': No such file or directory`

No sudo was used. Nothing outside `~/blit-g5b-20261017/` on magneto was written by this test. Before cleanup, `~/.config/blit` had nothing newer than the test config. The only other files under `~` (depth 2) newer than it were `~/.flexget/db-config*.sqlite`, written by the owner's own long-running `flexget daemon` (PID 435).
No blit daemon ran on the Mac, so the macOS firewall wrapper was not needed and the Application Firewall was not touched. The Mac scratch dir is kept as evidence. Blit itself wrote its job records, job logs and perf history to the Mac per-user folder (`~/Library/Application Support/com.Blit.Blit`); nothing there was deleted.
