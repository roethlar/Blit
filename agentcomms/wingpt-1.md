# Windows Benchmark Results - Phase 2.5

## Benchmark Timings

### Run 1: Baseline (No ETW Tracing)
- **blit v2 mirror**: 1.086 s average (over 5 runs)
- **robocopy /MIR**: 0.487 s average (over 5 runs)
- **Performance gap**: robocopy is ~2.23x faster

### Run 2: With ETW Tracing Active
- **blit v2 mirror**: 1.226 s average (over 5 runs)
- **robocopy /MIR**: 0.567 s average (over 5 runs)
- **Performance gap**: robocopy is ~2.16x faster
- **ETW overhead**: ~13% slowdown for blit, ~16% slowdown for robocopy

## Artifacts

### ZIP Archive
- **Path**: `logs\blit_windows_bench.zip`
- **Size**: 805,580,683 bytes (~768 MB)
- **Created**: 2025-10-18 19:55:01
- **SHA256**: `801B0AF560FF290126313D8256148C002F507103502B6D4A72486EB412F14F3D`

### Contents
1. ETW trace files: `C:\temp\blit_traces\`
   - `file.etl` - Kernel file I/O trace
   - `cpu.etl` - Kernel process/CPU trace
2. Baseline benchmark log: `C:\Users\michael\AppData\Local\Temp\blit_v2_bench_becd21a5c54c4d35b4f5f32d312e7054\bench.log`
3. ETW-traced benchmark workspace: `C:\Users\michael\AppData\Local\Temp\blit_v2_bench_9c8f74013ae94ca196f1b37a3d059ef1\`
4. Environment snapshots:
   - `logs\windows_systeminfo.txt`
   - `logs\windows_physicaldisk.txt`
   - `logs\windows_volume.txt`

## Observations

### Performance Consistency
Both baseline and ETW-traced runs show consistent results:
- blit v2 consistently takes ~1.0-1.2 seconds
- robocopy consistently takes ~0.4-0.6 seconds
- The performance gap is stable at approximately 2.2x

### Throughput Details (Baseline Run)
- **blit v2**: Average throughput ~240-248 MiB/s using 8 workers
- **robocopy**: Completing 256 MiB in ~0.5s suggests ~500+ MiB/s effective throughput

### Anomalies
None observed. All runs completed successfully without errors or crashes. ETW tracing added expected overhead but did not cause anomalies.
