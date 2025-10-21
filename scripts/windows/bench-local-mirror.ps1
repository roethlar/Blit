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

function Apply-IncrementalChanges {
    if ($script:IncrementalApplied) {
        return
    }
    if ($incrementalTouchCount -le 0 -and $incrementalDeleteCount -le 0 -and $incrementalAddCount -le 0) {
        return
    }

    Write-Log ("Applying incremental changes to source tree (touch={0}, delete={1}, add={2})..." -f $incrementalTouchCount, $incrementalDeleteCount, $incrementalAddCount)
    $files = Get-ChildItem -Path $srcDir -Recurse -File | Sort-Object FullName

    $touchTargets = @()
    if ($incrementalTouchCount -gt 0) {
        $touchTargets = $files | Select-Object -First $incrementalTouchCount
    }

    $remaining = if ($touchTargets.Count -gt 0) {
        $files | Select-Object -Skip $touchTargets.Count
    } else {
        $files
    }

    $deleteTargets = @()
    if ($incrementalDeleteCount -gt 0) {
        $deleteTargets = $remaining | Select-Object -First $incrementalDeleteCount
    }

    foreach ($file in $touchTargets) {
        try {
            Add-Content -Path $file.FullName -Value "`nupdated $(Get-Date -Format o)`n" -Encoding UTF8
        } catch {
            Write-Log ("[warn] failed to touch {0}: {1}" -f $file.FullName, $_.Exception.Message)
        }
    }

    foreach ($file in $deleteTargets) {
        try {
            Remove-Item $file.FullName -Force -ErrorAction Stop
        } catch {
            Write-Log ("[warn] failed to delete {0}: {1}" -f $file.FullName, $_.Exception.Message)
        }
    }

    if ($incrementalAddCount -gt 0) {
        $rngLocal = [System.Security.Cryptography.RandomNumberGenerator]::Create()
        $payload = New-Object byte[] $incrementalAddBytes
        $rngLocal.GetBytes($payload)
        $addRoot = Join-Path $srcDir "incremental_new"
        [System.IO.Directory]::CreateDirectory($addRoot) | Out-Null
        for ($i = 0; $i -lt $incrementalAddCount; $i++) {
            $filePath = Join-Path $addRoot ("new_{0:D6}.dat" -f $i)
            [System.IO.File]::WriteAllBytes($filePath, $payload)
        }
        $rngLocal.Dispose()
    }

    $script:IncrementalApplied = $true
    $totalFiles = (Get-ChildItem -Path $srcDir -Recurse -File | Measure-Object).Count
    Write-Log ("Incremental changes applied. Source now has {0} files." -f $totalFiles)
}

$workspaceDisposition = if ($Cleanup) { 'removed' } else { 'preserved' }
Write-Log ("Workspace: {0} (will be {1} on exit)" -f $workRoot, $workspaceDisposition)
Write-Log "Generating ${SizeMB} MiB synthetic payload..."

$smallFileCount = [int]([Environment]::GetEnvironmentVariable("SMALL_FILE_COUNT") ?? "0")
$smallFileBytes = [int]([Environment]::GetEnvironmentVariable("SMALL_FILE_BYTES") ?? "4096")
$smallFileDirSize = [int]([Environment]::GetEnvironmentVariable("SMALL_FILE_DIR_SIZE") ?? "1000")

$preserveDest = [int]([Environment]::GetEnvironmentVariable("PRESERVE_DEST") ?? "0")
$incrementalTouchCount = [int]([Environment]::GetEnvironmentVariable("INCREMENTAL_TOUCH_COUNT") ?? "0")
$incrementalDeleteCount = [int]([Environment]::GetEnvironmentVariable("INCREMENTAL_DELETE_COUNT") ?? "0")
$incrementalAddCount = [int]([Environment]::GetEnvironmentVariable("INCREMENTAL_ADD_COUNT") ?? "0")
$incrementalAddBytes = [int]([Environment]::GetEnvironmentVariable("INCREMENTAL_ADD_BYTES") ?? "1024")
$script:IncrementalApplied = $false

$robocopyFlagsDefault = "/MIR /COPYALL /FFT /R:1 /W:1 /NDL /NFL /NJH /NJS /NP"
$robocopyFlagString = [Environment]::GetEnvironmentVariable("ROBOCOPY_FLAGS")
if ([string]::IsNullOrWhiteSpace($robocopyFlagString)) {
    $robocopyFlagString = $robocopyFlagsDefault
}
$robocopyFlags = $robocopyFlagString.Split(' ', [System.StringSplitOptions]::RemoveEmptyEntries)

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

    if ($smallFileCount -gt 0) {
        Write-Log ("Generating {0} small files ({1} bytes each)..." -f $smallFileCount, $smallFileBytes)
        $payload = New-Object byte[] $smallFileBytes
        $rng.GetBytes($payload)
        for ($i = 0; $i -lt $smallFileCount; $i++) {
            $bucket = [int][math]::Floor($i / [double][Math]::Max(1, $smallFileDirSize))
            $dirPath = Join-Path $srcDir ("small\\grp_{0:D4}" -f $bucket)
            [System.IO.Directory]::CreateDirectory($dirPath) | Out-Null
            $filePath = Join-Path $dirPath ("file_{0:D6}.dat" -f $i)
            [System.IO.File]::WriteAllBytes($filePath, $payload)
        }
        Write-Log "Small-file payload generated."
    }

    Write-Log "Building blit-cli (release)..."
    Push-Location $RepoRoot
    try {
        $previousActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = "Continue"
            & cargo build --release --package blit-cli --bin blit-cli 2>&1 |
                ForEach-Object { Write-Log $_ }
        } finally {
            $ErrorActionPreference = $previousActionPreference
        }
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

    $configDir = Join-Path $workRoot "blit_config"
    [System.IO.Directory]::CreateDirectory($configDir) | Out-Null
    $env:BLIT_CONFIG_DIR = $configDir
    Write-Log ("Using isolated config dir at {0}" -f $configDir)
    & $blitBin diagnostics perf --disable --clear 2>&1 | ForEach-Object { Write-Log $_ }

    $toolNames = New-Object System.Collections.Generic.List[string]
    $toolDest = @{}
    $toolLabel = @{}
    $toolSum = @{}
    $toolCount = @{}
    $toolBinary = @{}

    $dstBlit = Join-Path $workRoot "dst_blit"
    $toolNames.Add("blit")
    $toolDest["blit"] = $dstBlit
    $toolLabel["blit"] = "blit v2 mirror"
    $toolSum["blit"] = 0.0
    $toolCount["blit"] = 0
    $toolBinary["blit"] = $blitBin

    function Get-RobocopyPath {
        $cmd = Get-Command robocopy -ErrorAction SilentlyContinue
        if ($cmd -and $cmd.Source -and (Test-Path $cmd.Source)) {
            return $cmd.Source
        }
        if ($env:SystemRoot) {
            foreach ($sub in @("System32", "Sysnative")) {
                $candidate = Join-Path (Join-Path $env:SystemRoot $sub) "robocopy.exe"
                if (Test-Path $candidate) {
                    return $candidate
                }
            }
        }
        return $null
    }

    $robocopyPath = Get-RobocopyPath
    if ($robocopyPath) {
        $dstRobocopy = Join-Path $workRoot "dst_robocopy"
        $toolNames.Add("robocopy")
        $toolDest["robocopy"] = $dstRobocopy
        $toolLabel["robocopy"] = "robocopy /MIR"
        $toolSum["robocopy"] = 0.0
        $toolCount["robocopy"] = 0
        $toolBinary["robocopy"] = $robocopyPath
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
                $output = & $Binary mirror $Source $Destination 2>&1
                $exitCode = $LASTEXITCODE
            }
            "robocopy" {
                $exe = if ($Binary -and (Test-Path $Binary)) { $Binary } else { "robocopy" }
                $args = @($Source, $Destination) + $robocopyFlags
                Write-Log ("[robocopy] args: {0}" -f ($args -join ' '))
                $output = & $exe @args 2>&1
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
            [hashtable]$CountMap,
            [hashtable]$BinaryMap
        )

        if ($preserveDest -eq 1) {
            if (-not (Test-Path $Destination)) {
                [System.IO.Directory]::CreateDirectory($Destination) | Out-Null
            }
        }
        else {
            if (Test-Path $Destination) {
                Remove-Item $Destination -Recurse -Force -ErrorAction SilentlyContinue
            }
            [System.IO.Directory]::CreateDirectory($Destination) | Out-Null
        }
        $label = $LabelMap[$Tool]
        Write-Log ("[{0}] {1} run {2}/{3}: mirror -> {4}" -f $label, $Phase, $Index, $Total, $Destination)

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $binToUse = $null
        if ($BinaryMap.ContainsKey($Tool)) {
            $binToUse = $BinaryMap[$Tool]
        }
        if (-not $binToUse) {
            $binToUse = $Binary
        }

        $result = Invoke-ToolCommand -Tool $Tool -Source $Source -Destination $Destination -Binary $binToUse
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
                Invoke-ToolRun -Tool $tool -Phase "Warmup" -Index $i -Total $Warmup -Source $srcDir -Destination $toolDest[$tool] -Binary $blitBin -LabelMap $toolLabel -SumMap $toolSum -CountMap $toolCount -BinaryMap $toolBinary | Out-Null
            }
        }

        if (-not $script:IncrementalApplied) {
            Apply-IncrementalChanges
        }

        if ($Runs -gt 0) {
            Write-Log ("[{0}] Measured runs: {1}" -f $toolLabel[$tool], $Runs)
            for ($i = 1; $i -le $Runs; $i++) {
                Invoke-ToolRun -Tool $tool -Phase "Measured" -Index $i -Total $Runs -Source $srcDir -Destination $toolDest[$tool] -Binary $blitBin -LabelMap $toolLabel -SumMap $toolSum -CountMap $toolCount -BinaryMap $toolBinary
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

    & $blitBin diagnostics perf --enable 2>&1 | ForEach-Object { Write-Log $_ }
    Write-Log "Benchmark complete. Log: $logFile"

} finally {
    if ($Cleanup) {
        Write-Host "Cleaning up workspace: $workRoot"
        Remove-Item $workRoot -Recurse -Force -ErrorAction SilentlyContinue
    } else {
        Write-Host "Workspace preserved at: $workRoot"
        Write-Host "Log file: $logFile"
    }
}
