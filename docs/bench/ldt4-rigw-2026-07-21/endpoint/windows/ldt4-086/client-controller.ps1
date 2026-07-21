$ErrorActionPreference = 'Stop'
function Normalize-Ldt4CommandLine([string]$value) {
  if ($null -eq $value) { return '' }
  return (($value.Replace([char]92,[char]47).Replace([string][char]34,'') -replace '\s+',' ').Trim()).ToLowerInvariant()
}
$dir = 'D:/blit-test/ldt4-sessions/ldt4-20260721T224319Z-96a4e3b03caf/logs/ldt4-086'
$process = $null
$stdoutStream = $null
$stderrStream = $null
try {
  $gate = $dir + '/client-launch.ok'
  for ($wait = 0; $wait -lt 150 -and -not (Test-Path -LiteralPath $gate); $wait++) { Start-Sleep -Milliseconds 100 }
  if (-not (Test-Path -LiteralPath $gate)) { throw 'client launch gate timed out' }
  $stdoutStream = [IO.File]::Open(($dir + '/client.out'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  $stderrStream = [IO.File]::Open(($dir + '/client.err'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  $info = [Diagnostics.ProcessStartInfo]::new()
  $info.FileName = 'D:/blit-test/bins/406a7e5/blit.exe'
  $info.UseShellExecute = $false
  $info.RedirectStandardOutput = $true
  $info.RedirectStandardError = $true
  foreach ($argument in @('copy','D:/blit-test/rigw-module/src_mixed','10.1.10.54:9031:/ldt4/active/mixed/','--yes')) { [void]$info.ArgumentList.Add($argument) }
  $info.Environment['BLIT_TRACE_SESSION_PHASES'] = '1'
  $info.Environment['BLIT_TRACE_RUN_ID'] = 'ldt4-086'
  $process = [Diagnostics.Process]::new()
  $process.StartInfo = $info
  $clock = [Diagnostics.Stopwatch]::StartNew()
  if (-not $process.Start()) { throw 'client Process.Start returned false' }
  $pidStream = [IO.File]::Open(($dir + '/client.pid'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { $bytes=[Text.Encoding]::ASCII.GetBytes([string]$process.Id); $pidStream.Write($bytes,0,$bytes.Length); $pidStream.Flush($true) } finally { $pidStream.Dispose() }
  $observed = Get-CimInstance Win32_Process -Filter "ProcessId=$($process.Id)" -ErrorAction Stop
  $actualPath = if ($observed.ExecutablePath) { $observed.ExecutablePath.Replace([char]92,[char]47) } else { '' }
  $actualCommand = Normalize-Ldt4CommandLine $observed.CommandLine
  $expectedCommand = Normalize-Ldt4CommandLine 'D:/blit-test/bins/406a7e5/blit.exe copy D:/blit-test/rigw-module/src_mixed 10.1.10.54:9031:/ldt4/active/mixed/ --yes'
  if ($observed.Name -ine 'blit.exe' -or $actualPath -ine 'D:/blit-test/bins/406a7e5/blit.exe' -or $actualCommand -cne $expectedCommand) { throw "client identity mismatch: $actualPath $actualCommand" }
  $identity = "run_id=ldt4-086`npid=$($process.Id)`nparent_pid=$PID`nexecutable=$actualPath`ncommand=$actualCommand`ntrace_session_phases=1`ntrace_run_id=ldt4-086`n"
  $identityStream = [IO.File]::Open(($dir + '/client-identity.txt'),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try { $bytes=[Text.UTF8Encoding]::new($false).GetBytes($identity); $identityStream.Write($bytes,0,$bytes.Length); $identityStream.Flush($true) } finally { $identityStream.Dispose() }
  $stdoutCopy = $process.StandardOutput.BaseStream.CopyToAsync($stdoutStream)
  $stderrCopy = $process.StandardError.BaseStream.CopyToAsync($stderrStream)
  $process.WaitForExit()
  $stdoutCopy.GetAwaiter().GetResult()
  $stderrCopy.GetAwaiter().GetResult()
  $clock.Stop()
  $elapsed = [Math]::Max([int64]1,[int64][Math]::Round($clock.Elapsed.TotalMilliseconds))
  $resultStream = [IO.File]::Open(($dir + '/client-result.txt'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  try { $bytes=[Text.Encoding]::ASCII.GetBytes("R|$elapsed|$($process.ExitCode)|$($process.Id)|reaped`n"); $resultStream.Write($bytes,0,$bytes.Length); $resultStream.Flush($true) } finally { $resultStream.Dispose() }
  exit 0
} catch {
  if ($process -and -not $process.HasExited) { $process.Kill($true); $process.WaitForExit() }
  $safe = ([string]$_).Replace("`r",' ').Replace("`n",' ')
  $resultStream = [IO.File]::Open(($dir + '/client-result.txt'),[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
  try { $bytes=[Text.UTF8Encoding]::new($false).GetBytes("E|$safe`n"); $resultStream.Write($bytes,0,$bytes.Length); $resultStream.Flush($true) } finally { $resultStream.Dispose() }
  exit 1
} finally {
  if ($stdoutStream) { $stdoutStream.Dispose() }
  if ($stderrStream) { $stderrStream.Dispose() }
  if ($process) { $process.Dispose() }
}
