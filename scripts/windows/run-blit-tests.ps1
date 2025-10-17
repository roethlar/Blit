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

    $encoding = New-Object System.Text.UTF8Encoding($false)
    $logWriter = New-Object System.IO.StreamWriter($logFile, $false, $encoding)
    try {
        & $Command 2>&1 |
            ForEach-Object {
                $logWriter.WriteLine($_)
                $_
            }
        $logWriter.Flush()
        $exitCode = $LASTEXITCODE
    } catch {
        $logWriter.Flush()
        $logWriter.Dispose()
        Write-Error "Step '$Name' failed. See $logFile"
        throw
    }
    $logWriter.Dispose()

    if ($exitCode -ne 0) {
        Write-Error "Step '$Name' exited with code $exitCode. See $logFile"
        throw "CommandFailed"
    }

    Write-Host "--> Logs: $logFile"
}

Run-Step "cargo fmt -- --check"      { cargo fmt -- --check }
Run-Step "cargo check"               { cargo check }
Run-Step "cargo test -p blit-core"   { cargo test -p blit-core }
Run-Step "cargo test workspace"      { cargo test }
