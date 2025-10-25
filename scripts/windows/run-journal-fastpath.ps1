param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("NTFS", "ReFS")]
    [string]$Volume,

    # Optional override if the repo lives somewhere else.
    [string]$RepoRoot
)

$ErrorActionPreference = "Stop"

if (-not $RepoRoot) {
    $RepoRoot = (Resolve-Path (Join-Path -Path $PSScriptRoot -ChildPath "..\..")).Path
}

$driveRoot = switch ($Volume) {
    "NTFS" { "C:\" }
    "ReFS" { "D:\" }
}

$workspace = Join-Path -Path $driveRoot -ChildPath "temp\blit_journal_fastpath"
if (Test-Path -LiteralPath $workspace) {
    Remove-Item -LiteralPath $workspace -Recurse -Force
}
New-Item -ItemType Directory -Path $workspace | Out-Null

$src = Join-Path -Path $workspace -ChildPath "src"
$dst = Join-Path -Path $workspace -ChildPath "dst"
New-Item -ItemType Directory -Path $src | Out-Null
New-Item -ItemType Directory -Path $dst | Out-Null

Write-Host "Workspace     : $workspace"
Write-Host "Source dir    : $src"
Write-Host "Destination   : $dst"
Write-Host ""

Write-Host "Generating 5000 files..."
for ($i = 1; $i -le 5000; $i++) {
    $fileName = "file_{0:D5}.txt" -f $i
    $filePath = Join-Path -Path $src -ChildPath $fileName
    Set-Content -LiteralPath $filePath -Value "payload $i" -NoNewline
}

$env:BLIT_CONFIG_DIR = Join-Path -Path $workspace -ChildPath "config"
New-Item -ItemType Directory -Path $env:BLIT_CONFIG_DIR | Out-Null

$candidateBinaries = @(
    Join-Path -Path $RepoRoot -ChildPath "target\release\blit-cli.exe"
    Join-Path -Path $RepoRoot -ChildPath "target\x86_64-pc-windows-msvc\release\blit-cli.exe"
)
$blitCli = $candidateBinaries | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $blitCli) {
    throw "Unable to find blit-cli.exe. Build it first: cargo build --release -p blit-cli --bin blit-cli"
}

Write-Host ""
Write-Host "Using blit-cli : $blitCli"
Write-Host ""

function Invoke-BlitMirror {
    param(
        [string]$Label
    )

    Write-Host "== $Label =="
    & $blitCli "mirror" $src $dst "--verbose"
    if ($LASTEXITCODE -ne 0) {
        throw "blit mirror failed during '$Label' run (exit $LASTEXITCODE)."
    }
    Write-Host ""
}

Invoke-BlitMirror -Label "Initial sync"
Invoke-BlitMirror -Label "Zero-change sync"

Write-Host "Done. Results remain under $workspace"
