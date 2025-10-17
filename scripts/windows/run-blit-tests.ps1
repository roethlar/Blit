param(
    [string]$RepoRoot = "$PWD"
)

Set-Location $RepoRoot
$env:RUST_BACKTRACE = "1"

$logDir = Join-Path $RepoRoot "logs"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"

function Run-Step {
    param(
        [string]$Name,
        [scriptblock]$Command
    )

    Write-Host "==> $Name"
    $logFile = Join-Path $logDir "$($Name.Replace(' ', '-'))-$timestamp.log"

    try {
        & $Command 2>&1 | Tee-Object -FilePath $logFile
    } catch {
        Write-Error "Step '$Name' failed. See $logFile"
        throw
    }

    Write-Host "--> Logs: $logFile"
}

Run-Step "cargo fmt -- --check"      { cargo fmt -- --check }
Run-Step "cargo check"               { cargo check }
Run-Step "cargo test -p blit-core"   { cargo test -p blit-core }
Run-Step "cargo test workspace"      { cargo test }
