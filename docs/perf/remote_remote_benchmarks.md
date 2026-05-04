# Remote-to-Remote Benchmarks

This page records direct delegation vs explicit CLI relay results.

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

- `direct`: default remote-to-remote path, CLI asks destination to pull from source.
- `relay`: explicit `--relay-via-cli`, CLI pulls from source and pushes to destination.
- `cli_data_plane_outbound_bytes`: env-gated counter from the CLI process. Direct runs should be `0`; relay runs should be roughly payload-sized.

## Results

No release benchmark captured yet. Fill this table from `results.csv` once run
on the target network.

| Date | Source | Destination | Payload | Mode | Avg MiB/s | Best MiB/s | CLI Data-Plane Bytes | Notes |
|------|--------|-------------|---------|------|-----------|------------|----------------------|-------|
| TBD | TBD | TBD | TBD | direct | TBD | TBD | 0 expected | |
| TBD | TBD | TBD | TBD | relay | TBD | TBD | payload-sized expected | |
