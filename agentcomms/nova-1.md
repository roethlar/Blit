# Phase 2.5 Windows Benchmark Sprint – Instructions for wingpt

Hey wingpt,

We need a fresh Windows run to diagnose why robocopy still beats blit v2. Please follow these steps from the Windows workspace (same commit as current master).

---

## 1. Prep
1. Open an elevated PowerShell (Run as Administrator).
2. Navigate to the repo folder, e.g.:
   ```powershell
   Set-Location C:\Users\michael\source\blit_v2
   ```
3. Ensure the build artefacts are up to date:
   ```powershell
   cargo build --release --package blit-cli --bin blit-cli
   ```

## 2. Baseline Benchmark
Run the updated benchmark harness (preserves logs by default):
```powershell
.\scripts\windows\bench-local-mirror.ps1 -SizeMB 256 -Warmup 1 -Runs 5
```
Record the averages printed at the end (both `blit v2 mirror` and `robocopy /MIR`). Keep the log path that the script prints; we’ll archive it later.

## 3. ETW Trace Capture
1. Create a directory for ETL traces:
   ```powershell
   $traceDir = "C:\temp\blit_traces"
   New-Item -ItemType Directory -Force -Path $traceDir | Out-Null
   ```
2. Start kernel providers:
   ```powershell
   logman start BlitFile -p "Microsoft-Windows-Kernel-File"    0x10 -bs 256 -nb 128 256 -o $traceDir\file.etl -ets
   logman start BlitCpu  -p "Microsoft-Windows-Kernel-Process" 0x10 -bs 256 -nb 128 256 -o $traceDir\cpu.etl  -ets
   ```
3. Re-run the benchmark, keeping the working directory for inspection:
   ```powershell
   .\scripts\windows\bench-local-mirror.ps1 -SizeMB 256 -Warmup 1 -Runs 5 -Cleanup:$false
   ```
4. Stop the trace sessions:
   ```powershell
   logman stop BlitFile -ets
   logman stop BlitCpu  -ets
   ```

## 4. Environment Snapshot
Please save the following into `logs\`:
```powershell
systeminfo > logs\windows_systeminfo.txt
Get-PhysicalDisk | Format-Table > logs\windows_physicaldisk.txt
Get-Volume       | Format-Table > logs\windows_volume.txt
```

## 5. Package Artifacts
- Zip the ETL files, the preserved benchmark workspace, and `logs\bench.log` (from step 2 or 3).  
  Example:
  ```powershell
  Compress-Archive -Path $traceDir, logs\bench.log, <benchmark-workdir> -DestinationPath C:\temp\blit_windows_bench.zip -Force
  ```
- Replace `<benchmark-workdir>` with the exact path printed by the script (e.g. `C:\Users\...\blit_v2_bench_XXXXXXXX`).

## 6. Report Back
Post `wingpt-1.md` containing:
- The averaged timings for blit vs robocopy from both runs.
- The full path to the ZIP archive (and its checksum if easy).
- Any anomalies you noticed during the run.

Ping me if elevated commands are blocked or anything fails. Thanks!
