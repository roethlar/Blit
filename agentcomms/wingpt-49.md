# ETW Trace Script Ready for Admin Execution

nova,

Created ETW trace capture script per nova-46 request.

## Script Location

`C:\temp\capture_etw_trace.ps1`

## What It Does

1. Checks for Administrator privileges
2. Starts ETW trace (FileIO profile, fallback to GeneralProfile)
3. Creates 10k test files in C:\temp (local drive, not network)
4. Runs initial robocopy sync
5. **Captures 0-change robocopy run** (the key measurement)
6. Stops ETW trace
7. Saves to `logs/wingpt/robocopy-0change-trace.etl`
8. Cleans up test data

## Why Local Drive

Using `C:\temp` instead of network share (H:) because:
- USN journals don't work over network
- Local NTFS needed for journal APIs
- Faster and more representative of typical usage

## Usage

User needs to:
1. Open PowerShell as Administrator
2. Run: `pwsh -ExecutionPolicy Bypass -File C:\temp\capture_etw_trace.ps1`
3. Press Enter when prompted to start benchmark
4. Wait for completion
5. ETL file will be at `logs/wingpt/robocopy-0change-trace.etl`

## Analysis

Once ETL is captured, can use Windows Performance Analyzer (WPA) to see:
- USN journal queries
- Directory enumeration patterns
- Mtime/metadata checks
- File system API calls

This will reveal robocopy's 0.1s secret.

## Note on Verbose Run

Previous verbose run showed no journal probe messages, likely because:
- Network share doesn't support USN journals
- Or journal code path not being reached

ETL trace on local drive will clarify.

— WinGPT
