# G5b dry run — Mac ↔ Linux, CI binaries from 8844dfbd

**Date:** 2026-10-14 (UTC); local Mac times in `jobs list` are EDT.
**Scope:** pre-candidate dry run of RELEASE_1_0 G5b, two legs only (mac→linux push, linux→mac pull).
**Result:** Leg 1 PASS, Leg 2 PASS. Every command exited 0. There were no warnings during transfers and no stall notices.

`$S` = `/private/tmp/claude-501/-Users-michael-Dev-blit/a1d34dfa-b6fa-407a-815b-9dd6d2b4dbb6/scratchpad/g5b-dryrun`
Raw evidence is in `$S/logs/`: per-step `.txt` (command + exit), `.out` (stdout), `.err` (stderr), all manifests, `daemon.log`, `cleanup.txt`.

## 1. Binary identities

Source: GitHub Actions run 38474595246 (roethlar/Blit). Workflow `CI`, event `push`, branch `master`,
conclusion `success`, headSha `8844dfbd8f3700c1fd51fb1d6e0dc407bd43e164`.

| | Mac (aarch64-apple-darwin) | Linux (x86_64-unknown-linux-gnu) |
|---|---|---|
| archive sha256 (sidecar) | `a88e630fec05326fd7e56ee802cf77d667093eeb6f1e5855ffba0046e6312957` | `5edc5086e7b33925bb019c81de14aacecb065d37af6d6ccb6f4f429b9f1fc29a` |
| sidecar check | `shasum -a 256 -c` on Mac: `OK` | `shasum -a 256 -c` on Mac: `OK`; `sha256sum -c` on magneto after scp: `OK` |
| BUILD.txt | `8844dfbd8f3700c1fd51fb1d6e0dc407bd43e164` | `8844dfbd8f3700c1fd51fb1d6e0dc407bd43e164` |
| `blit --version` | `blit 0.1.3+8844dfbd8f37` | `blit 0.1.3+8844dfbd8f37` |
| `blit-daemon --version` | `blit-daemon 0.1.3+8844dfbd8f37` | `blit-daemon 0.1.3+8844dfbd8f37` |
| blit sha256 | `cbaa99781238f8eff4e54d264b134b88b532cfd376846d7bc2330e1f91cbb054` | `c37d9b29a655c6a7746a573998d9d7ee552f7e30608390eff1f63f4f2747f225` |
| blit-daemon sha256 | `7a5d95361bad39b15c10569779aa52093cc096ec10500068131a3b314a5f8481` | `6592dd5a30b0e03b747150978e93d7dd9a54ba5902af88bb9514d228e2853ceb` |

Both peers report the same identity, `0.1.3+8844dfbd8f37`.
Archive contents (both): `blit`, `blit-daemon`, `README.md`, `LICENSE`, `CHANGELOG.md`, `BUILD.txt`.
Downloads are in `$S/dl-mac/` and `$S/dl-linux/`. My scripts and config are in `$S/scripts/`.

## 2. Daemon on magneto

Host: `magneto` (Arch Linux, kernel 7.2.9-arch1-1, x86_64). From the Mac, `magneto` resolves to IPv4 `10.1.10.10` (interface `enp1s0f1`). The host's other IPv4 address is 10.1.10.253 on `enp1s0f0`. Port 9031 was free, so the default port was used.

Config `~/blit-g5b-20261014/config.toml`, verbatim:

```toml
[daemon]
bind = "10.1.10.10"
port = 9031
no_mdns = true

[[module]]
name = "g5b"
path = "/home/michael/blit-g5b-20261014/module"
read_only = false
comment = "G5b dry-run module"
```

Two choices go slightly beyond the brief:
- `no_mdns = true` stops the daemon broadcasting itself on the LAN. mDNS plays no part in the test.
- The daemon was started with `STATE_DIRECTORY`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` and `XDG_STATE_HOME` pointed inside `~/blit-g5b-20261014/`. This kept its job logs, perf history and recents inside the work dir (`blit-core/src/config.rs` `daemon_state_dir()`). The pre-existing `~/.config/blit` on magneto has nothing newer than the test config.

Start command (run in `~/blit-g5b-20261014`):
`STATE_DIRECTORY=$W/state XDG_CONFIG_HOME=$W/xdg/config ... setsid nohup $W/dl/blit-x86_64-unknown-linux-gnu/blit-daemon --config $W/config.toml > $W/daemon.log 2>&1 < /dev/null &`

- PID 21469, recorded in `daemon.pid`.
- `ss -ltnp`: `LISTEN 0 128 10.1.10.10:9031 0.0.0.0:* users:(("blit-daemon",pid=21469,fd=9))`.
- Reachable from the Mac: `blit list-modules magneto` returned `g5b (rw)	/home/michael/blit-g5b-20261014/module` with exit 0.
- Full daemon.log for the whole run (two lines, nothing added during transfers):
  ```
  blitd: warn: no default root configured; server:// requests will be rejected until --root or config root is provided
  blitd v2 listening on 10.1.10.10:9031
  ```
  This warning is expected for a modules-only config.

## 3. Test tree (Mac, `$S/tree`)

The tree has 12 files across 5 directories (`.`, `a`, `a/b`, `a/b/c`, `d`), with 3 nested levels (`a/b/c`). It includes:
- an empty file: `empty.dat`
- a filename with spaces: `a/small one.txt`
- a unicode filename: `a/b/café.txt`, NFC `63 61 66 c3 a9`
- a dot-file: `.hidden`
- random data: `a/b/c/big.bin` (5 MiB), `a/medium.bin` (100 KiB), `d/mid2.bin` (37,000 B)
- small text files of a few bytes up to 49 B

Initial total: 5,382,443 bytes.
Manifests use `find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256` (`sha256sum` on Linux). Both tools print the same `hash  ./path` format, so manifests are compared with plain `diff`. That comparison also checks that filename bytes match.

## 4. Leg 1 — Mac → Linux (push)

| step | command | exit | stdout summary (verbatim) |
|---|---|---|---|
| 1a initial | `blit mirror $S/tree/ magneto:/g5b/pushed/ --yes` | 0 | `Transfer complete: 12 file(s), 5382443 bytes.` / `Destination: magneto:/g5b/pushed` |
| 1b after edit | same | 0 | `Transfer complete: 1 file(s), 39 bytes.` / `Destination: magneto:/g5b/pushed` |
| 1c converged | same | 0 | `Remote already up to date; nothing to upload (magneto:/g5b/pushed).` / `Transfer complete: 0 file(s), 0 bytes.` / `Destination: magneto:/g5b/pushed` |

Each run's stderr was only `starting mirror <src> -> magneto:/g5b/pushed`.

Manifest comparisons:
- **After 1a: PASS.** `manifest-mac-v1.txt` and `manifest-linux-leg1-1.txt` are byte-identical: 12 vs 12 files. A full `find` under `module/` shows only `pushed/` and the expected 5 dirs plus 12 files. There are no extra entries and no non-regular entries.
- **The 1b edit:** `d/readme.txt` changed from `readme v1\n` (10 B) to `readme v2 - content changed for leg 1b\n` (39 B). Its mtime moved from 22:31:00 to 22:34:51 local.
- **After 1b: PASS.** `manifest-mac-v2.txt` and `manifest-linux-leg1-2.txt` are identical: 12 vs 12 files.
- **1b transferred exactly 1 file: PASS.**
- **1c transferred 0 files (converged): PASS.**

## 5. Leg 2 — Linux → Mac (pull)

The destination `$S/pulled/` did not exist before the run. I created it new and empty (0 entries).

| step | command | exit | stdout summary (verbatim) |
|---|---|---|---|
| 2a initial | `blit mirror magneto:/g5b/pushed/ $S/pulled/ --yes` | 0 | `Pull complete: 12 file(s), 5382472 bytes -> $S/pulled/.` |
| 2b converged | same | 0 | `Pull complete: 0 file(s), 0 bytes -> $S/pulled/.` |

Each run's stderr was only `starting mirror magneto:/g5b/pushed -> $S/pulled/`.

- 5,382,472 = 5,382,443 − 10 + 39, which matches the updated tree.
- **Manifest after 2a: PASS.** `manifest-mac-v2.txt` and `manifest-pulled-1.txt` are identical: 12 vs 12 files.
- The full entry lists of `tree/` and `pulled/` (files and dirs) are identical. There are no non-regular entries.
- **After 2b: PASS.** `manifest-pulled-2.txt` is still identical, and 0 files were transferred (converged).

## 6. Job records

`blit jobs list` (Mac) exited 0, verbatim:

```
Jobs on this machine (5), newest first:
  8cdb2d744885e76fa5d1b4cabda9d9ff  2026-10-13 22:38:35  mirror  magneto:/g5b/pushed/ -> $S/pulled/  ok (0 copied, 0 deleted, 0 failed)
  590c0f629ae0f8dd209fa37a5712928f  2026-10-13 22:38:26  mirror  magneto:/g5b/pushed/ -> $S/pulled/  ok (12 copied, 0 deleted, 0 failed)
  34c736a4bad9e2990cd9f155d61ff851  2026-10-13 22:38:17  mirror  $S/tree/ -> magneto:/g5b/pushed/  ok (0 copied, 0 deleted, 0 failed)
  4006f3406e9985c1f59897166da485d9  2026-10-13 22:34:51  mirror  $S/tree/ -> magneto:/g5b/pushed/  ok (1 copied, 0 deleted, 0 failed)
  773adea2f3b645ff0ba89fdb5a8640f7  2026-10-13 22:31:09  mirror  $S/tree/ -> magneto:/g5b/pushed/  ok (12 copied, 0 deleted, 0 failed)
```

(`$S` above abbreviates the full scratch path that appears in the real output; see `logs/jobs-list.out`.)
All five runs are listed with correct copied counts.

As extra evidence, `blit jobs list magneto` exited 0. It showed:
- `Daemon: blit 0.1.3 on magneto`
- `Delegation: disabled`
- 5 recent transfers, all `ok`: 3 `push` and 2 `pull_sync`, from peer 10.1.10.15, each taking 1.1–1.7 s
- `Counters: push=0 pull=0 purge=0 active=0 errors=0` (counters are off because `--metrics` was not set)

The daemon's job-log filenames in `state/jobs/logs/` carry the same five job IDs as the Mac list. Push runs are tagged `destination` and pull runs `source`.

## 7. Anomalies

- **None in any transfer.** No warning, no stall notice, no non-zero exit.
- The only warning seen was the daemon's startup line `blitd: warn: no default root configured; server:// requests will be rejected until --root or config root is provided`. It is expected for a modules-only config.
- Observation, not a finding: the daemon wrote `recents.jsonl` under `$XDG_CONFIG_HOME/blit/`, not under `$STATE_DIRECTORY`. Job logs and `perf_local.jsonl` did go to `$STATE_DIRECTORY`. This matches the code: `crates/blit-daemon/src/recents_store.rs` says recents live at `config_dir()/recents.jsonl`. *Assumption, unverified:* under a systemd unit using `StateDirectory=` with `ProtectSystem=strict`, the recents file would go somewhere other than the state directory.
- Not exercised: same-size content edits (the 1b edit also changed size), Windows legs, `--detach`, and delegation.

## 8. Cleanup (magneto)

From `logs/cleanup.txt`:
- `recorded pid=21469`, the `blit-daemon --config ...` process
- `kill rc=0`, then `pid 21469 gone`
- `pgrep`: `no blit-daemon processes`
- `9031 no longer listening`
- `rm -rf ~/blit-g5b-20261014`: `rm rc=0`
- `ls: cannot access '/home/michael/blit-g5b-20261014': No such file or directory`

No sudo was used. Nothing outside `~/blit-g5b-20261014/` on magneto was written. The Mac scratch dir is kept as evidence. The Mac per-user folder (`~/Library/Application Support/com.Blit.Blit`) was only read and written by blit itself; nothing there was deleted.
