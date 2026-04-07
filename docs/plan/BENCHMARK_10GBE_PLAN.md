# 10GbE Benchmark Test Plan

## Prerequisites

### Binaries
```bash
cd ~/dev/Blit
cargo build --release
```

### TrueNAS Setup
1. Create a dataset for benchmarks: `blit-bench` on your pool
2. Create NFS share exporting `/mnt/<pool>/blit-bench`
3. Optionally create SMB share for the same path
4. Copy daemon binary: `scp target/release/blit-daemon truenas:/tmp/`

### This Machine
```bash
# NFS mount
sudo mkdir -p /mnt/truenas
sudo mount -t nfs <truenas-ip>:/mnt/<pool>/blit-bench /mnt/truenas

# SMB mount (optional)
sudo mkdir -p /mnt/truenas_smb
sudo mount -t cifs //<truenas-ip>/blit-bench /mnt/truenas_smb -o user=<user>,vers=3
```

---

## Phase 1: Local-Only (validate unified pipeline)

No network needed. Confirms the refactored local→local path works and performs.

```bash
# Quick sanity test
./target/release/blit-cli copy /tmp/test_src /tmp/test_dst --yes

# Full local benchmark (3 workloads × blit + rsync)
REMOTE_HOST= SIZE_MB=512 SMALL_COUNT=5000 RUNS=3 ./scripts/bench_10gbe.sh
```

**What to check:**
- [ ] All three workloads complete without errors
- [ ] No-op mirror runs complete in <100ms (journal fast-path)
- [ ] blit matches or beats rsync on all workloads
- [ ] Results CSV looks sane in `logs/bench_10gbe_*/results.csv`

---

## Phase 2: Local → NFS/SMB Mount

Tests filesystem sink over network mounts (still local pipeline, no daemon).

```bash
NFS_MOUNT=/mnt/truenas SIZE_MB=512 SMALL_COUNT=5000 RUNS=3 ./scripts/bench_10gbe.sh
```

**What to check:**
- [ ] Large file throughput approaches 10 Gbps (~1.1 GB/s)
- [ ] Small file perf — NFS will be slower than local due to metadata RTTs
- [ ] SMB vs NFS comparison (if both mounted)

---

## Phase 3: Remote Push/Pull — Daemon on TrueNAS

Tests the TCP data plane over 10GbE.

**Start daemon on TrueNAS:**
```bash
ssh truenas '/tmp/blit-daemon --root /mnt/<pool>/blit-bench --port 9031'
```

**Run from this machine:**
```bash
REMOTE_HOST=<truenas-ip> SIZE_MB=1024 SMALL_COUNT=10000 RUNS=3 ./scripts/bench_10gbe.sh
```

**What to check:**
- [ ] TCP push large file: target >5 Gbps (>625 MB/s)
- [ ] TCP pull large file: similar throughput
- [ ] TCP vs gRPC fallback: TCP should be 2-5× faster
- [ ] First payload timing visible in `-v` output (target <1s for all workloads)
- [ ] Small file push/pull: tar shard batching should keep throughput reasonable

---

## Phase 4: Remote Push/Pull — Daemon on This Machine

Tests the reverse direction (TrueNAS as client).

**Start daemon on this machine:**
```bash
./target/release/blit-daemon --root /tmp/blit-bench-local --port 9031
```

**From TrueNAS (copy blit-cli over):**
```bash
scp target/release/blit-cli truenas:/tmp/
ssh truenas '/tmp/blit-cli copy /mnt/<pool>/blit-bench/src <this-machine-ip>:9031:/default/ --yes -v'
```

Or script it from here by SSHing commands. This validates both daemon directions.

---

## Phase 5: Stress Test

Push the limits to find the throughput ceiling.

```bash
# 4 GiB single file
REMOTE_HOST=<truenas-ip> SIZE_MB=4096 SMALL_COUNT=0 RUNS=3 ./scripts/bench_10gbe.sh

# 100k small files
REMOTE_HOST=<truenas-ip> SIZE_MB=0 SMALL_COUNT=100000 RUNS=1 ./scripts/bench_10gbe.sh
```

---

## Recording Results

After all phases, results are in `logs/bench_10gbe_*/`. To document:

1. Copy the best `results.csv` into `CHANGELOG.md` benchmark section
2. Update TODO.md — check off the three benchmark items
3. Note any issues found (throughput bottlenecks, errors, etc.)

## TODO Items Covered

- [ ] Benchmark remote fallback + data-plane streaming (line 78)
- [ ] Benchmark TCP data plane throughput targeting 10+ Gbps (line 98)
- [ ] Capture remote benchmark runs TCP vs gRPC fallback (line 116)
