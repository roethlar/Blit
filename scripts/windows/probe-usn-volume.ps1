<#
.SYNOPSIS
  Enumerate the device paths for a given volume and attempt to query the USN journal.

.PARAMETER Drive
  Drive letter (e.g. C or C:) for the target volume. Defaults to C.
#>

param(
    [string]$Drive = "C"
)

$ErrorActionPreference = "Stop"

$driveLetter = $Drive.TrimEnd(':')
$volume = Get-Volume -DriveLetter $driveLetter -ErrorAction Stop

$candidates = @()

if ($volume.DriveLetter) {
    $candidates += "\\.\$($volume.DriveLetter):"
    $candidates += "$($volume.DriveLetter):"
}

if ($volume.Path) {
    $trimmed = $volume.Path.TrimEnd('\')
    if ($trimmed) {
        $candidates += $trimmed
        $candidates += ($trimmed + '\')
    }
}

if ($volume.UniqueId) {
    $guid = $volume.UniqueId.TrimEnd('\')
    if ($guid) {
        if ($candidates -notcontains $guid) {
            $candidates += $guid
        }

        if ($guid.StartsWith("\\?\") -and $guid.Length -gt 4) {
            $legacy = "\\.\" + $guid.Substring(4)
            if ($candidates -notcontains $legacy) {
                $candidates += $legacy
            }
        }
    }
}

$candidates = $candidates | Select-Object -Unique

Write-Host "Volume     : $($volume.FileSystemLabel) [$($volume.DriveLetter):]"
Write-Host "FS Type    : $($volume.FileSystem)"
Write-Host "Unique ID  : $($volume.UniqueId)"
Write-Host ""
Write-Host "Candidates:"
$candidates | ForEach-Object { Write-Host "  $_" }
Write-Host ""

foreach ($candidate in $candidates) {
    Write-Host "== fsutil usn queryjournal $candidate =="
    try {
        & fsutil usn queryjournal $candidate 2>&1
    } catch {
        Write-Warning $_
    }
    Write-Host ""
}
