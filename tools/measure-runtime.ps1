param(
  [Parameter(Mandatory=$true)][string]$Binary,
  [Parameter(Mandatory=$true)][int]$ProcessId,
  [ValidateRange(10,28800)][int]$SecondsPerState = 600,
  [ValidateRange(0,600)][int]$WarmupSeconds = 120,
  [ValidateSet('visible','minimized','observed')][string]$Mode = 'observed',
  [string]$OutputDirectory = ''
)
# Read-only attachment: never starts, hides, restores or terminates the user's app.
$ErrorActionPreference = 'Stop'
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path (Split-Path $binaryPath -Parent) ('runtime-' + $Mode + '-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfff')) }
$null = New-Item -ItemType Directory -Path $OutputDirectory -Force
if ((Test-Path -LiteralPath (Join-Path $OutputDirectory 'runtime.csv')) -or
    (Test-Path -LiteralPath (Join-Path $OutputDirectory 'runtime-summary.json'))) {
  throw 'Measurement output already exists; choose a new directory to keep samples and provenance separate'
}
$rootProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId"
if (-not $rootProcess -or -not $rootProcess.ExecutablePath) { throw 'Cannot inspect the target process. Run the measurement with the same administrator rights as Pinmeter.' }
if (-not [string]::Equals($rootProcess.ExecutablePath, $binaryPath, [StringComparison]::OrdinalIgnoreCase)) { throw 'Process path does not match the requested release executable' }
$rootCreated = $rootProcess.CreationDate
$hash = (Get-FileHash -LiteralPath $binaryPath -Algorithm SHA256).Hash
$logical = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
if ($logical -le 0) { throw 'Logical processor count unavailable' }
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class PinmeterRuntimeWindows {
  private delegate bool EnumWindow(IntPtr window, IntPtr data);
  [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindow callback, IntPtr data);
  [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
  [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
  [DllImport("user32.dll")] private static extern bool IsIconic(IntPtr window);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr window, System.Text.StringBuilder text, int length);
  public static string State(int[] processes) {
    var ids = new HashSet<int>(processes); bool visible = false;
    EnumWindows((window, data) => {
      uint pid; GetWindowThreadProcessId(window, out pid);
      if (!ids.Contains((int)pid)) return true;
      var title = new System.Text.StringBuilder(256); GetWindowText(window, title, title.Capacity);
      if (title.ToString() == "Pinmeter" && IsWindowVisible(window) && !IsIconic(window)) visible = true;
      return true;
    }, IntPtr.Zero);
    return visible ? "visible" : "minimized";
  }
}
'@
$samples = [System.Collections.Generic.List[object]]::new()
$inventory = @{}
$failure = $null
$started = [DateTime]::UtcNow
$clock = [Diagnostics.Stopwatch]::StartNew()
$previous = @{}
$previousTime = 0.0
$measurementStart = 0.0
$warming = $true
$csvPath = Join-Path $OutputDirectory 'runtime.csv'
try {
  while ($true) {
    $processes = @(Get-CimInstance Win32_Process)
    $rootNow = $processes | Where-Object ProcessId -eq $ProcessId
    if (-not $rootNow -or $rootNow.CreationDate -ne $rootCreated) { throw 'Target exited or its PID was reused; measurement is incomplete' }
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    $null = $ids.Add($ProcessId)
    $created = @{ $ProcessId = $rootCreated }
    do {
      $changed = $false
      foreach ($item in $processes) {
        if ($ids.Contains([int]$item.ParentProcessId) -and $item.CreationDate -ge $created[[int]$item.ParentProcessId] -and $ids.Add([int]$item.ProcessId)) {
          $created[[int]$item.ProcessId] = $item.CreationDate
          $changed = $true
        }
      }
    } while ($changed)
    $actualMode = [PinmeterRuntimeWindows]::State([int[]]@($ids))
    if ($Mode -ne 'observed' -and $actualMode -ne $Mode) { throw "Window state changed: expected $Mode, observed $actualMode" }
    $memory = @(Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids.Contains([int]$_.IDProcess) })
    $cpuDelta = 0.0; $privateBytes = 0.0; $handles = 0; $threads = 0; $current = @{}
    foreach ($item in $processes | Where-Object { $ids.Contains([int]$_.ProcessId) }) {
      $process = Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
      if (-not $process) { continue }
      $identity = "$($item.ProcessId):$($item.CreationDate.Ticks)"
      $cpu = $process.TotalProcessorTime.TotalSeconds
      if ($null -eq $cpu) { throw "Cannot read CPU for process $($item.ProcessId)" }
      if ($previous.ContainsKey($identity)) { $cpuDelta += [Math]::Max([double]0, [double]$cpu - [double]$previous[$identity]) }
      elseif (-not $warming -and $item.CreationDate -ge $started.AddSeconds($previousTime)) { $cpuDelta += $cpu }
      $mem = @($memory | Where-Object IDProcess -eq $item.ProcessId)
      if ($mem.Count -ne 1) {
        $process.Refresh()
        if ($process.HasExited) { continue }
        throw "Private working set unavailable for live process $($item.ProcessId)"
      }
      $privateBytes += $mem[0].WorkingSetPrivate
      $handles += $process.HandleCount; $threads += $process.Threads.Count
      $current[$identity] = $cpu
      $inventory[$identity] = [pscustomobject]@{Pid=$item.ProcessId;ParentPid=$item.ParentProcessId;Path=$item.ExecutablePath;Created=$item.CreationDate}
    }
    $now = $clock.Elapsed.TotalSeconds
    if ($warming -and $now -ge $WarmupSeconds) { $warming = $false; $measurementStart = $now; $previous = @{} }
    if (-not $warming -and $previous.Count) {
      $row = [pscustomobject]@{Mode=$actualMode;ElapsedSeconds=$now-$measurementStart;IntervalSeconds=$now-$previousTime;CpuSecondsDelta=$cpuDelta;CpuPercent=$cpuDelta/($now-$previousTime)/$logical*100;PrivateWorkingSetMiB=$privateBytes/1MB;Processes=$current.Count;Handles=$handles;Threads=$threads}
      $samples.Add($row)
      $row | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8 -Append
      if ($samples.Count % 30 -eq 0) { Write-Output "$Mode samples=$($samples.Count), elapsed=$([int]$row.ElapsedSeconds)s, CPU=$([Math]::Round($row.CpuPercent,3))%, private=$([Math]::Round($row.PrivateWorkingSetMiB,1))MiB" }
    }
    $previous = $current; $previousTime = $now
    if (-not $warming -and $now-$measurementStart -ge $SecondsPerState) { break }
    Start-Sleep -Milliseconds 1000
  }
} catch { $failure = $_.Exception.Message }
$groups = foreach ($state in @('visible','minimized')) {
  $rows = @($samples | Where-Object Mode -eq $state)
  if (-not $rows.Count) { continue }
  $cpu = @($rows.CpuPercent | Sort-Object)
  $duration = ($rows.IntervalSeconds | Measure-Object -Sum).Sum
  $mean = ($rows.CpuSecondsDelta | Measure-Object -Sum).Sum / $duration / $logical * 100
  $memoryMean = ($rows.PrivateWorkingSetMiB | Measure-Object -Average).Average
  $cpuBudget = if ($state -eq 'visible') { 1.0 } else { 0.5 }
  $memoryBudget = if ($state -eq 'visible') { 180 } else { 120 }
  [pscustomobject]@{Mode=$state;Samples=$rows.Count;DurationSeconds=$duration;CpuMean=$mean;CpuP95=$cpu[[Math]::Min($cpu.Count-1,[Math]::Floor($cpu.Count*0.95))];CpuPeak=($cpu|Measure-Object -Maximum).Maximum;MemoryMeanMiB=$memoryMean;MemoryPeakMiB=($rows.PrivateWorkingSetMiB|Measure-Object -Maximum).Maximum;HandlesFirst=$rows[0].Handles;HandlesLast=$rows[-1].Handles;ThreadsFirst=$rows[0].Threads;ThreadsLast=$rows[-1].Threads;WithinBudget=$mean -le $cpuBudget -and $memoryMean -le $memoryBudget}
}
$report = [pscustomobject]@{Completed=($null -eq $failure);Error=$failure;Binary=$binaryPath;BinarySha256=$hash;RootPid=$ProcessId;RootCreated=$rootCreated;StartedUtc=$started;FinishedUtc=[DateTime]::UtcNow;LogicalProcessors=$logical;WarmupSeconds=$WarmupSeconds;RequestedSeconds=$SecondsPerState;RequestedMode=$Mode;BaselineEligible=($null -eq $failure -and $Mode -ne 'observed' -and $WarmupSeconds -ge 120 -and $SecondsPerState -ge 600);Groups=@($groups);Processes=@($inventory.Values)}
$json = $report | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText((Join-Path $OutputDirectory 'runtime-summary.json'), $json, [Text.UTF8Encoding]::new($false))
Write-Output $json
if ($failure) { throw $failure }
