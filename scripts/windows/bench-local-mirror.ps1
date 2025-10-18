param(
    [string]$RepoRoot = "$PWD",
    [int]$SizeMB = 256,
    [switch]$KeepWork
)

$ErrorActionPreference = "Stop"

# Prepare workspace
$tempRoot = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), "blit_v2_bench_" + [System.IO.Path]::GetRandomFileName())
if ($KeepWork) {
    $workRoot = $tempRoot
} else {
    $workRoot = $tempRoot
}
[System.IO.Directory]::CreateDirectory($workRoot) | Out-Null

$srcDir = Join-Path $workRoot "src"
$dstV1 = Join-Path $workRoot "dst_v1"
$dstV2 = Join-Path $workRoot "dst_v2"
$logFile = Join-Path $workRoot "bench.log"
foreach ($dir in @($srcDir, $dstV1, $dstV2)) {
    [System.IO.Directory]::CreateDirectory($dir) | Out-Null
}

try {
    $v1Root = (Resolve-Path (Join-Path $RepoRoot "..")).Path
    $v2Root = (Resolve-Path $RepoRoot).Path

    $v1Bin = Join-Path $v1Root "target/release/blit.exe"
    $v2Bin = Join-Path $v2Root "target/release/blit-cli.exe"

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
        $lines = "hello world`n" * ($i + 1)
        [System.IO.File]::WriteAllText((Join-Path $subDir "file.txt"), $lines)
    }

    # Build bins
    Push-Location $v1Root
    try {
        cargo build --release --bin blit 2>&1 | Tee-Object -FilePath $logFile
    } finally { Pop-Location }

    Push-Location $v2Root
    try {
        cargo build --release --bin blit-cli 2>&1 | Tee-Object -FilePath $logFile -Append
    } finally { Pop-Location }

    $hyperfine = Get-Command hyperfine -ErrorAction SilentlyContinue

    if ($null -eq $hyperfine) {
        Write-Host "hyperfine not found; running sequential timings"

        function Measure-Step {
            param(
                [string]$Label,
                [scriptblock]$Command
            )
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            & $Command
            $sw.Stop()
            $msg = "{0}: {1:N3} s" -f $Label, $sw.Elapsed.TotalSeconds
            $msg | Tee-Object -FilePath $logFile -Append
        }

        Remove-Item $dstV1 -Recurse -Force -ErrorAction SilentlyContinue
        [System.IO.Directory]::CreateDirectory($dstV1) | Out-Null
        Measure-Step "v1 mirror" { & $v1Bin mirror $srcDir $dstV1 --ludicrous-speed }

        Remove-Item $dstV2 -Recurse -Force -ErrorAction SilentlyContinue
        [System.IO.Directory]::CreateDirectory($dstV2) | Out-Null
        Measure-Step "v2 mirror" { & $v2Bin mirror $srcDir $dstV2 }
    } else {
        Write-Host "Running hyperfine benchmarks" | Tee-Object -FilePath $logFile -Append
        & $hyperfine \
            --warmup 1 \
            --prepare "Remove-Item '$dstV1' -Recurse -Force -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force -Path '$dstV1' >\$null" \
            "$v1Bin mirror '$srcDir' '$dstV1' --ludicrous-speed" \
            --prepare "Remove-Item '$dstV2' -Recurse -Force -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force -Path '$dstV2' >\$null" \
            "$v2Bin mirror '$srcDir' '$dstV2'" | Tee-Object -FilePath $logFile -Append
    }

    Write-Host "Benchmark artefacts stored in: $workRoot"
    Write-Host "Log file: $logFile"

    if (-not $KeepWork) {
        Write-Host "Cleaning up workspace" | Tee-Object -FilePath $logFile -Append
        Remove-Item $workRoot -Recurse -Force
    }
} finally {
    if ($KeepWork -and -not (Test-Path $workRoot)) {
        [System.IO.Directory]::CreateDirectory($workRoot) | Out-Null
    }
}
