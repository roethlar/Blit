# local-ab-run.ps1 — ONE timed local D:->E: run on netwatch-01 (blit vs robocopy).
#
# Runs entirely on Windows so the ssh round trip stays OUTSIDE the timed window
# (the otp-2w F3 rule). Methodology matches the blit rig harnesses exactly —
# anything less is not comparable to the otp-12 numbers:
#   * cold caches before every run (standby-list purge);
#   * writeback DRAINED before the window opens (a dirty queue from the previous
#     run would otherwise be charged to this one);
#   * destination container precreated OUTSIDE the window on BOTH arms, so
#     neither tool pays a mkdir the other does not (otp-12b F5);
#   * durability keyed by the DESTINATION volume, never by the tool — the bug
#     that invalidated the first Linux session (docs/bench/otp12-perf README);
#   * the landed file count is returned so a tool that "succeeds" while writing
#     nothing is caught instead of scoring a fast time.
#
# Prints exactly one sentinel-framed record:  R:<ms>,<flush_ms>,<rc>,<files>,<drain>:R
# Anything else (a crash, pwsh noise) fails to parse and VOIDS the run — nothing
# can masquerade as a time.

param(
    [Parameter(Mandatory)][ValidateSet('blit', 'robocopy')][string]$Tool,
    [Parameter(Mandatory)][string]$Src,        # D:\...\src_mixed   (NO trailing slash)
    [Parameter(Mandatory)][string]$DestRoot,   # E:\blit-local-bench\<tag>
    [Parameter(Mandatory)][string]$BlitExe,
    [string]$DestDrive = 'E',
    [string]$PurgeScript = 'D:\blit-test\purge-standby.ps1'
)

$ErrorActionPreference = 'Stop'

# --- cold caches (untimed) --------------------------------------------------
& $PurgeScript | Out-Null

# --- drain writeback (untimed): quiet = <1 MB/s for 3 consecutive 2s samples -
Write-VolumeCache -DriveLetter $DestDrive
$drain = 'DRAIN-TIMEOUT'
$quiet = 0
for ($i = 0; $i -lt 60; $i++) {
    $w = (Get-Counter "\PhysicalDisk(_Total)\Disk Write Bytes/sec" -SampleInterval 2 -MaxSamples 1).CounterSamples[0].CookedValue
    if ($null -ne $w -and [double]$w -lt 1048576) { $quiet++ } else { $quiet = 0 }
    if ($quiet -ge 3) { $drain = "drained_$((($i + 1) * 2))s"; break }
}

# --- destination container, precreated OUTSIDE the window (both arms) --------
# blit nests a no-trailing-slash source under the destination, so `blit copy
# <Src> <DestRoot>` lands <DestRoot>\<leaf>. robocopy copies CONTENTS, so it is
# pointed at <DestRoot>\<leaf> directly. Both arms therefore land an identical
# tree and neither pays for the container.
$leaf = Split-Path $Src -Leaf
$dest = Join-Path $DestRoot $leaf
New-Item -ItemType Directory -Force -Path $dest | Out-Null

# --- the timed window -------------------------------------------------------
$sw = [Diagnostics.Stopwatch]::StartNew()
if ($Tool -eq 'blit') {
    & $BlitExe copy $Src $DestRoot --yes > $null 2>&1
    $rc = $LASTEXITCODE
}
else {
    # /MT:8 is robocopy's own default thread count. Retries bounded so a
    # transient error cannot hang the run for hours on the default /R:1000000.
    robocopy $Src $dest /E /MT:8 /R:2 /W:2 /NFL /NDL /NJH /NJS > $null 2>&1
    $rc = $LASTEXITCODE
}
$sw.Stop()
$ms = [int]$sw.Elapsed.TotalMilliseconds

# --- durability, self-timed, keyed by the DESTINATION volume ----------------
$flush = -1
try {
    $a = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    Write-VolumeCache -DriveLetter $DestDrive -ErrorAction Stop
    $b = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $flush = [int]($b - $a)
}
catch { $flush = -1 }   # caller VOIDS the run: a failed flush is not a fast run

# --- did the bytes actually land? -------------------------------------------
$files = (Get-ChildItem $dest -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count

Remove-Item -Recurse -Force $DestRoot -ErrorAction SilentlyContinue

# ${drain} braces are LOAD-BEARING: PowerShell parses a bare `$drain:R` as a
# SCOPE-qualified variable (like $env:PATH), so the closing sentinel silently
# vanishes and every run parses as a void. Same trap as bench_otp12_win.sh:520
# — documented there, and reproduced here anyway on the first run.
"R:$ms,$flush,$rc,$files,${drain}:R"
