# sf-1 — Tripwire + stream-scaling harness

**Plan**: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4), slice sf-1.
**Status**: implemented, codex review pending.

## What

`scripts/bench_tripwires.sh` — makes the 2026-07-05 tool-comparison
baseline (`docs/bench/10gbe-2026-07-05/`) re-runnable against any
daemon host in one command, and adds the plan's stream-scaling probe
(files/s vs the stream count the transfer actually ran with). No
production code.

## Approach

Derived from `scripts/bench_10gbe.sh` (timing/generation/fresh-target
patterns) plus the session's ad-hoc comparison methodology recorded in
DEVLOG 2026-07-05 00:51 and DIAGNOSIS.md — the ad-hoc runner itself
was never committed, so this reconstructs it durably.

- **Matrix** (schema-identical to the committed
  `tool_comparison.csv`: `transport,direction,workload,run,ms,status`):
  blit / rsyncd / rsync-over-ssh / rclone-sftp × push/pull, and
  blit / rsync / rclone / `cp -a` × local, over the baseline's three
  workloads (1 GiB large, 10k×4 KiB small, 512 MiB+5k×2 KiB mixed).
  rclone runs its best measured LAN config (`--ignore-checksum`,
  tuned `--transfers`, sftp transport) per DIAGNOSIS.md. The harness
  matrix and the plan's tripwire list are the same set by
  construction (plan acceptance criterion).
- **One command**: `DAEMON_HOST=… REMOTE_ROOT=… REMOTE_BLIT_DAEMON=…
  ./scripts/bench_tripwires.sh`. By default it spins both daemons on
  the target host over ssh — blitd via `--root` (exports the
  per-invocation session dir as module `default`, no config file
  needed) and rsyncd via a generated config — and tears both down plus
  the session dir on exit. `SPIN_DAEMONS=0` targets already-running
  daemons. All tools share one data root (session methodology).
- **Fresh targets every run** (blit and rsync both no-op onto
  already-delivered content): local dests recreated, remote push
  targets are per-run never-seen subdirs; pull sources seeded once per
  workload (seeding writes leave the ARC warm — baseline was warm
  re-reads).
- **Scale probe**: fixed 4 KiB files at counts crossing
  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
  25k→8, 50k→10 expected); records files/s and the **measured** stream
  count (per-stream `stream complete` completion lines in blitd's
  stderr, `data_plane.rs:224`, delta-counted per push). Measured-vs-
  table divergence is exactly the sf-2 evidence the plan wants the
  curve to show.
- **Tripwire verdict is the exit code**: summary prints best-of per
  cell, blit vs fastest rival; any rival win → `TRIPPED` + exit 3.
  Also diffs blit cells against the committed baseline CSV (the ±10%
  regression criterion) when present.
- Missing tools (rsync/rclone locally or remotely) skip their cells
  with a note; a wedged tool is capped by `timeout` and recorded in
  the status column rather than hanging the run.

## Files

- `scripts/bench_tripwires.sh` (new, executable)

## Tests

Script-only slice — cargo suite unaffected (run anyway: fmt, clippy,
full workspace suite green; count vs 1479 baseline in verdict file).
Script verified by execution:

- `bash -n` clean.
- **Local-only e2e** (`SIZE_MB=32 SMALL_COUNT=500 RUNS=2 … matrix`):
  all local cells timed, CSV written, summary + baseline diff printed,
  exit 3 with `cp` tripping blit on tiny local copies (harness working
  as designed; rig verdicts belong to sf-4).
- **Full remote-path e2e over loopback** (ssh shim executing "remote"
  commands locally; real network transfers to a real spun blitd +
  rsyncd on 127.0.0.1): daemon spin-up, seeding, every push/pull cell,
  scale probe with stream counting (200 files → 1 stream measured),
  teardown verified (no stray daemons, session dir removed).
  rclone-sftp cells recorded status 1 in this rig-less test (no sftp
  auth to localhost) — the status column captured it and the run
  continued, which is the designed failure path.

## Known gaps

- The scale probe is push-only (the plan's target cell); pull scaling
  can be added when a pull-side per-stream log line exists.
- Stream counting needs the daemon's stderr (`SPIN_DAEMONS=1` owns it;
  otherwise `BLITD_LOG`); against a foreign daemon the column is empty
  rather than guessed.
- rclone-sftp cells assume ssh-agent/key auth to the host (same
  requirement the session had); no rclone config file is generated.
- Loopback e2e cannot validate 10 GbE-scale numbers — sf-4 is the rig
  re-measure slice.
- Observed during loopback testing, recorded for sf-2: a 1000-file
  push rode 1 stream where the proposal table says 2 — consistent
  with the DIAGNOSIS.md one-stream-for-10k-files gap; the daemon-side
  proposal call (`control.rs:798`) and its input manifest need the
  sf-2 pins.
