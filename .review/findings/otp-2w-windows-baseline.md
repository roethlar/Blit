# otp-2w — OLD-path baseline on the owner-designated cross-direction rig

**What**: Completes otp-2. The Mac↔zoey session established that
hardware-asymmetric endpoints support per-direction verdicts only
(D-2026-07-05-1; codex otp-2 F1); the owner adjudicated the resulting
open question by designating a closer-spec pair — "mac to windows
would be closer spec. windows is faster, both have 10gbe" — and set
up OpenSSH on the host. This slice records the OLD paths' 12-cell
baseline on that pair: the rig for the otp-12 acceptance bar's
cross-direction half.

**Approach**:

- `scripts/bench_otp2w_baseline.sh` — the zoey methodology
  (cold/drained/durable/median-of-4/fresh-unique destinations)
  with the daemon-host half in PowerShell over SSH:
  standby-list purge for cold caches
  (`scripts/windows/purge-standby.ps1`, NtSetSystemInformation with
  SeProfileSingleProcessPrivilege), `Write-VolumeCache` for durable
  push windows, `Get-Counter` PhysicalDisk write-rate for the drain,
  WMI `Win32_Process.Create` for a daemon that survives ssh-session
  teardown (Windows OpenSSH kills session children — a smoke-proven
  failure), TOML literal-string paths (double-quoted TOML corrupts
  `\b`), one program-scoped inbound firewall rule (documented, named,
  removable).
- Same-commit builds both ends (`0f922de`): source delivered by git
  bundle (unpushed commits; pushes are owner-gated; a bundle is a
  file copy between the owner's machines), built natively on the
  host, the host's prior checkout state preserved via stash
  (`bench-cargo-lock`).
- Evidence: `docs/bench/otp2w-baseline-2026-07-10/` (README +
  summary.csv/runs.csv/drain-outcomes.txt). Zero drain timeouts;
  7 of 12 cells ≤ 2% spread, worst 11.9% (counts re-verified at the
  codex round — the first write-up said 8/12).

**Files**:

- `scripts/bench_otp2w_baseline.sh`, `scripts/windows/purge-standby.ps1` — NEW.
- `docs/bench/otp2w-baseline-2026-07-10/` — NEW (evidence).
- `docs/STATE.md`, `DEVLOG.md` — adjudication recorded; otp-2 closed;
  queue to otp-10.

**Tests**: none (harness + rig; no production code). Verification =
the recorded runs + a pre-run byte-identical smoke round trip; the
session-survival failure mode was reproduced live (Start-Process
daemon died with the ssh session; WMI-launched daemon survived).

**Known gaps**:

- The pair is close-spec, not identical (APFS vs NTFS write paths;
  Defender real-time scanning on the host was left at its normal
  state). The README records the consequence: old push trails old
  pull ×1.8–×2.7 here too, carrier-insensitive on large — whether
  that is platform write-path cost or old-code cost is exactly what
  otp-12's old-vs-new per cell discriminates; interleaved A/B keeps
  the Defender state identical across arms.
- The firewall rule and staged files persist on the host between
  sessions by design (re-runs); removal one-liners are documented.
- Windows-side cache purge empties the standby list but cannot evict
  NTFS metadata cached by the kernel the way drop_caches does on
  Linux; both directions of THIS rig share that property, so
  within-rig comparisons are unaffected.
