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
$logFile = Join-Path $workRoot "bench.log"

[System.IO.Directory]::CreateDirectory($srcDir) | Out-Null
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

    $toolNames = New-Object System.Collections.Generic.List[string]
    $toolDest = @{}
    $toolLabel = @{}
    $toolSum = @{}
    $toolCount = @{}

    $dstBlit = Join-Path $workRoot "dst_blit"
    $toolNames.Add("blit")
    $toolDest["blit"] = $dstBlit
    $toolLabel["blit"] = "blit v2 mirror"
    $toolSum["blit"] = 0.0
    $toolCount["blit"] = 0

    if (Get-Command robocopy -ErrorAction SilentlyContinue) {
        $dstRobocopy = Join-Path $workRoot "dst_robocopy"
        $toolNames.Add("robocopy")
        $toolDest["robocopy"] = $dstRobocopy
        $toolLabel["robocopy"] = "robocopy /MIR"
        $toolSum["robocopy"] = 0.0
        $toolCount["robocopy"] = 0
    } else {
        Write-Log "robocopy not found; skipping robocopy baseline."
    }

    function Invoke-ToolCommand {
        param(
            [string]$Tool,
            [string]$Source,
            [string]$Destination,
            [string]$Binary
        )

        switch ($Tool) {
            "blit" {
                $output = & $Binary mirror $Source $Destination --no-progress 2>&1
                $exitCode = $LASTEXITCODE
            }
            "robocopy" {
                $args = @($Source, $Destination, "/MIR", "/NFL", "/NDL", "/NJH", "/NJS", "/NP")
                $output = & robocopy @args 2>&1
                $code = $LASTEXITCODE
                $exitCode = if ($code -ge 8) { $code } else { 0 }
            }
            default {
                throw "Unknown tool: $Tool"
            }
        }

        return [pscustomobject]@{
            Output = $output
            ExitCode = $exitCode
        }
    }

    function Invoke-ToolRun {
        param(
            [string]$Tool,
            [string]$Phase,
            [int]$Index,
            [int]$Total,
            [string]$Source,
            [string]$Destination,
            [string]$Binary,
            [hashtable]$LabelMap,
            [hashtable]$SumMap,
            [hashtable]$CountMap
        )

        if (Test-Path $Destination) {
            Remove-Item $Destination -Recurse -Force -ErrorAction SilentlyContinue
        }
        [System.IO.Directory]::CreateDirectory($Destination) | Out-Null
        $label = $LabelMap[$Tool]
        Write-Log ("[{0}] {1} run {2}/{3}: mirror -> {4}" -f $label, $Phase, $Index, $Total, $Destination)

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $result = Invoke-ToolCommand -Tool $Tool -Source $Source -Destination $Destination -Binary $Binary
        $sw.Stop()

        if ($result.Output) {
            foreach ($line in $result.Output) {
                if ($null -ne $line) {
                    Write-Host $line
                    Add-Content -Path $logFile -Value $line
                }
            }
        }

        if ($result.ExitCode -ne 0) {
            throw "$label exited with $($result.ExitCode) during $Phase run $Index."
        }

        $elapsed = $sw.Elapsed.TotalSeconds
        Write-Log ("[{0}] {1} run {2} completed in {3:N3} s" -f $label, $Phase, $Index, $elapsed)

        if ($Phase -eq "Measured") {
            $SumMap[$Tool] = $SumMap[$Tool] + $elapsed
            $CountMap[$Tool] = $CountMap[$Tool] + 1
        }
    }

    foreach ($tool in $toolNames) {
        if ($Warmup -gt 0) {
            Write-Log ("[{0}] Warmup runs: {1}" -f $toolLabel[$tool], $Warmup)
            for ($i = 1; $i -le $Warmup; $i++) {
                Invoke-ToolRun -Tool $tool -Phase "Warmup" -Index $i -Total $Warmup -Source $srcDir -Destination $toolDest[$tool] -Binary $blitBin -LabelMap $toolLabel -SumMap $toolSum -CountMap $toolCount | Out-Null
            }
        }

        if ($Runs -gt 0) {
            Write-Log ("[{0}] Measured runs: {1}" -f $toolLabel[$tool], $Runs)
            for ($i = 1; $i -le $Runs; $i++) {
                Invoke-ToolRun -Tool $tool -Phase "Measured" -Index $i -Total $Runs -Source $srcDir -Destination $toolDest[$tool] -Binary $blitBin -LabelMap $toolLabel -SumMap $toolSum -CountMap $toolCount
            }
        } else {
            Write-Log ("[{0}] No measured runs requested (Runs=0)." -f $toolLabel[$tool])
        }
    }

    foreach ($tool in $toolNames) {
        $count = $toolCount[$tool]
        if ($count -gt 0) {
            $avg = $toolSum[$tool] / $count
            Write-Log ("Average [{0}] over {1} measured run(s): {2:N3} s" -f $toolLabel[$tool], $count, $avg)
        }
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
