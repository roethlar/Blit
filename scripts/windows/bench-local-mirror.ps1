param(
    [string]$RepoRoot,
    [int]$SizeMB = 256,
    [int]$Runs = 5,
    [int]$Warmup = 1,
    [switch]$Cleanup
)

$ErrorActionPreference = "Stop"

$RepoRoot = if (-not $RepoRoot) {
    Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
} else {
    $RepoRoot
}
$RepoRoot = (Resolve-Path $RepoRoot).Path

# Prepare workspace
$guid = [Guid]::NewGuid().ToString("N")
$workRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("blit_v2_bench_$guid")
[System.IO.Directory]::CreateDirectory($workRoot) | Out-Null

$srcDir = Join-Path $workRoot "src"
$dstDir = Join-Path $workRoot "dst_blit"
$logFile = Join-Path $workRoot "bench.log"

foreach ($dir in @($srcDir, $dstDir)) {
    [System.IO.Directory]::CreateDirectory($dir) | Out-Null
}
New-Item -Path $logFile -ItemType File -Force | Out-Null

function Write-Log {
    param([string]$Message)
    Write-Host $Message
    Add-Content -Path $logFile -Value $Message
}

$workspaceDisposition = if ($Cleanup) { 'removed' } else { 'preserved' }
Write-Log ("Workspace: {0} (will be {1} on exit)" -f $workRoot, $workspaceDisposition)
Write-Log "Generating ${SizeMB} MiB synthetic payload..."

$previousPerf = $env:BLIT_DISABLE_PERF_HISTORY
$perfEnvSetByScript = $false

try {
    # Generate source payload
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    $payloadPath = Join-Path $srcDir "payload.bin"
    $buffer = New-Object byte[](1024 * 1024)
    $fs = [System.IO.File]::Open($payloadPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    try {
        for ($i = 0; $i -lt $SizeMB; $i++) {
            $rng.GetBytes($buffer)
            $fs.Write($buffer, 0, $buffer.Length)
        }
    } finally {
        $fs.Dispose()
    }

    for ($i = 0; $i -lt 32; $i++) {
        $subDir = Join-Path $srcDir ("dir_{0:D2}" -f $i)
        [System.IO.Directory]::CreateDirectory($subDir) | Out-Null
        $lines = ("hello world`n" * ($i + 1))
        [System.IO.File]::WriteAllText((Join-Path $subDir "file.txt"), $lines)
    }

    Write-Log "Building blit-cli (release)..."
    Push-Location $RepoRoot
    try {
        cargo build --release --package blit-cli --bin blit-cli 2>&1 | Tee-Object -FilePath $logFile -Append
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $blitBin = Join-Path $RepoRoot "target/release/blit-cli.exe"
    if (-not (Test-Path $blitBin)) {
        $blitBin = Join-Path $RepoRoot "target/release/blit-cli"
    }
    if (-not (Test-Path $blitBin)) {
        throw "blit-cli binary not found at $blitBin"
    }
    Write-Log "Binary ready: $blitBin"

    if ($Runs -lt 0 -or $Warmup -lt 0) {
        throw "Runs and Warmup must be non-negative integers."
    }

    if ([string]::IsNullOrEmpty($previousPerf)) {
        $env:BLIT_DISABLE_PERF_HISTORY = "1"
        $perfEnvSetByScript = $true
        Write-Log "Perf history disabled for benchmark runs (set BLIT_DISABLE_PERF_HISTORY=0 to keep history)."
    } else {
        Write-Log "Perf history env already set to '$previousPerf'."
    }

    function Invoke-BlitRun {
        param(
            [string]$Phase,
            [int]$Index,
            [int]$Total,
            [string]$Binary,
            [string]$Source,
            [string]$Destination
        )

        if (Test-Path $Destination) {
            Remove-Item $Destination -Recurse -Force -ErrorAction SilentlyContinue
        }
        [System.IO.Directory]::CreateDirectory($Destination) | Out-Null
        Write-Log ("{0} run {1}/{2}: mirror -> {3}" -f $Phase, $Index, $Total, $Destination)

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $cmdOutput = & $Binary mirror $Source $Destination --no-progress 2>&1
        $exitCode = $LASTEXITCODE
        if ($cmdOutput) {
            foreach ($line in $cmdOutput) {
                Write-Host $line
                Add-Content -Path $logFile -Value $line
            }
        }
        $sw.Stop()

        if ($exitCode -ne 0) {
            throw "blit-cli exited with $exitCode during $Phase run $Index."
        }

        $elapsed = "{0:N3}" -f $sw.Elapsed.TotalSeconds
        Write-Log ("{0} run {1} completed in {2} s" -f $Phase, $Index, $elapsed)
        return $sw.Elapsed.TotalSeconds
    }

    $measurements = @()

    if ($Warmup -gt 0) {
        Write-Log "Warmup runs: $Warmup"
        for ($i = 1; $i -le $Warmup; $i++) {
            Invoke-BlitRun -Phase "Warmup" -Index $i -Total $Warmup -Binary $blitBin -Source $srcDir -Destination $dstDir | Out-Null
        }
    }

    if ($Runs -gt 0) {
        Write-Log "Measured runs: $Runs"
        for ($i = 1; $i -le $Runs; $i++) {
            $duration = Invoke-BlitRun -Phase "Measured" -Index $i -Total $Runs -Binary $blitBin -Source $srcDir -Destination $dstDir
            $measurements += $duration
        }
    } else {
        Write-Log "No measured runs requested (Runs=0)."
    }

    if ($measurements.Length -gt 0) {
        $avg = ($measurements | Measure-Object -Average).Average
        Write-Log ("Average over {0} measured run(s): {1:N3} s" -f $measurements.Length, $avg)
    }

    Write-Log "Benchmark complete. Log: $logFile"

} finally {
    if ($perfEnvSetByScript) {
        Remove-Item env:BLIT_DISABLE_PERF_HISTORY -ErrorAction SilentlyContinue
    } elseif (-not [string]::IsNullOrEmpty($previousPerf)) {
        $env:BLIT_DISABLE_PERF_HISTORY = $previousPerf
    }

    if ($Cleanup) {
        Write-Host "Cleaning up workspace: $workRoot"
        Remove-Item $workRoot -Recurse -Force -ErrorAction SilentlyContinue
    } else {
        Write-Host "Workspace preserved at: $workRoot"
        Write-Host "Log file: $logFile"
    }
}
