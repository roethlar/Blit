# Build optimized release binaries for Windows.
# Usage: .\scripts\windows\build-release.ps1 [-Target <TRIPLE>]

param(
    [string]$Target = ""
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)

Push-Location $RootDir

try {
    $CargoArgs = @("build", "--release")
    if ($Target) {
        $CargoArgs += @("--target", $Target)
        $OutDir = "target\$Target\release"
    } else {
        $OutDir = "target\release"
    }

    Write-Host "==> Building release binaries..."
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    $Binaries = @("blit-cli.exe", "blit-daemon.exe", "blit-utils.exe")

    Write-Host "==> Release binaries in $OutDir\"
    foreach ($bin in $Binaries) {
        $BinPath = Join-Path $OutDir $bin
        if (Test-Path $BinPath) {
            $Size = (Get-Item $BinPath).Length / 1MB
            Write-Host ("    {0}: {1:F1} MB" -f $bin, $Size)
        }
    }

    Write-Host "==> Done."
} finally {
    Pop-Location
}
