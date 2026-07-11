# Remote-to-Remote Benchmarks

This page records delegated remote-to-remote results. (It originally
compared direct delegation against the explicit CLI relay; the
`--relay-via-cli` path was removed at otp-10c-1, D-2026-07-11-1, so
delegation is the only remote→remote route and the relay leg is gone
from the harness.)

## Harness

Use:

```bash
cargo build --release

SRC_REMOTE=server-a:/bench/ \
DST_REMOTE=server-b:/bench/ \
SIZE_MB=512 \
RUNS=3 \
scripts/bench_remote_remote.sh
```

The destination daemon must opt in to delegated pull:

```toml
[delegation]
allow_delegated_pull = true
allowed_source_hosts = ["server-a.lan"]
```

The script writes `logs/bench_remote_remote_<timestamp>/results.csv` with:

- `direct`: the remote-to-remote path — the CLI asks the destination to pull from the source.
- `cli_data_plane_outbound_bytes`: counter from the CLI process (via `--diagnostics-counter-file`). Runs should report `0` — payload bytes never cross the CLI host.

## Results

No release benchmark captured yet. Fill this table from `results.csv` once run
on the target network.

| Date | Source | Destination | Payload | Mode | Avg MiB/s | Best MiB/s | CLI Data-Plane Bytes | Notes |
|------|--------|-------------|---------|------|-----------|------------|----------------------|-------|
| TBD | TBD | TBD | TBD | direct | TBD | TBD | 0 expected | |
