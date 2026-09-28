param([ValidateRange(61,3600)][int]$Samples = 61, [ValidateSet('idle','cpu','memory','network')][string]$Scenario = 'idle')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$outputDir = "$projectRoot/src/backend/target/measurements"
New-Item -ItemType Directory -Force $outputDir | Out-Null
# Independent C# PDH/memory and .NET interface statistics reference, same semantics.
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class PinmeterReference : IDisposable {
  [StructLayout(LayoutKind.Explicit)] struct Value { [FieldOffset(0)] public uint Status; [FieldOffset(8)] public double Number; }
  [StructLayout(LayoutKind.Sequential)] struct Memory { public uint Size, Load; public ulong Total, Available, PageTotal, PageAvailable, VirtualTotal, VirtualAvailable, Extended; }
  [DllImport("pdh.dll", CharSet=CharSet.Unicode)] static extern uint PdhOpenQuery(string source, UIntPtr user, out IntPtr query);
  [DllImport("pdh.dll", CharSet=CharSet.Unicode)] static extern uint PdhAddEnglishCounter(IntPtr query, string path, UIntPtr user, out IntPtr counter);
  [DllImport("pdh.dll")] static extern uint PdhCollectQueryData(IntPtr query);
  [DllImport("pdh.dll")] static extern uint PdhGetFormattedCounterValue(IntPtr counter, uint format, out uint type, out Value value);
  [DllImport("pdh.dll")] static extern uint PdhCloseQuery(IntPtr query);
  [DllImport("kernel32.dll", SetLastError=true)] static extern bool GlobalMemoryStatusEx(ref Memory value);
  IntPtr query, counter;
  public PinmeterReference() { Check(PdhOpenQuery(null,UIntPtr.Zero,out query)); Check(PdhAddEnglishCounter(query,@"\Processor Information(_Total)\% Processor Time",UIntPtr.Zero,out counter)); }
  static void Check(uint code) { if(code!=0) throw new Exception("PDH: "+code); }
  public void Prime() { Check(PdhCollectQueryData(query)); }
  public double Cpu() { Prime(); uint type; Value value; Check(PdhGetFormattedCounterValue(counter,0x8200,out type,out value)); if(value.Status>1)throw new Exception("Counter: "+value.Status); return value.Number; }
  public double MemoryPercent() { var value=new Memory();value.Size=(uint)Marshal.SizeOf(value);if(!GlobalMemoryStatusEx(ref value))throw new Exception("Memory failed");return (value.Total-value.Available)*100.0/value.Total; }
  public void Dispose() { PdhCloseQuery(query); }
}
'@
$reference = [PinmeterReference]::new()
$info = [Diagnostics.ProcessStartInfo]::new()
$info.FileName = "$projectRoot/src/backend/target/debug/monitor-probe.exe"
$info.Arguments = [string]$Samples
$info.UseShellExecute = $false; $info.CreateNoWindow=$true; $info.RedirectStandardOutput=$true
$probe = [Diagnostics.Process]::Start($info)
$clock=[Diagnostics.Stopwatch]::StartNew(); $previous=$null; $rows=[System.Collections.Generic.List[object]]::new()
try {
  while(!$probe.StandardOutput.EndOfStream) {
    $line=$probe.StandardOutput.ReadLine(); if(!$line){continue}; $sample=$line | ConvertFrom-Json
    $guid=$sample.network_id -replace '^win:',''
    $adapter=[Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() | Where-Object { $_.Id.Trim('{}') -eq $guid } | Select-Object -First 1
    if(!$adapter){throw "Reference interface missing: $guid"}
    $stats=$adapter.GetIPStatistics(); $time=$clock.Elapsed.TotalSeconds
    if(!$previous){$reference.Prime();$previous=@{Received=$stats.BytesReceived;Sent=$stats.BytesSent;Time=$time};continue}
    $cpu=$reference.Cpu(); $memory=$reference.MemoryPercent()
    $download=($stats.BytesReceived-$previous.Received)/($time-$previous.Time)
    $upload=($stats.BytesSent-$previous.Sent)/($time-$previous.Time)
    if(@($sample.cpu,$sample.memory,$sample.download,$sample.upload) | Where-Object status -ne 'normal') { throw 'Probe returned unavailable sample' }
    $rows.Add([pscustomobject]@{Scenario=$Scenario;At=$sample.at_ms;Interface=$guid;Cpu=$sample.cpu.value;ReferenceCpu=$cpu;CpuAbsDifference=[Math]::Abs($cpu-$sample.cpu.value);Memory=$sample.memory.value;ReferenceMemory=$memory;MemoryAbsDifference=[Math]::Abs($memory-$sample.memory.value);Download=$sample.download.value;ReferenceDownload=$download;DownloadAbsDifference=[Math]::Abs($download-$sample.download.value);Upload=$sample.upload.value;ReferenceUpload=$upload;UploadAbsDifference=[Math]::Abs($upload-$sample.upload.value)})
    $previous=@{Received=$stats.BytesReceived;Sent=$stats.BytesSent;Time=$time}
  }
  $probe.WaitForExit(); if($probe.ExitCode -ne 0){throw 'Probe failed'}
} finally {
  $reference.Dispose()
  if(!$probe.HasExited){$probe.Kill();$probe.WaitForExit()}
  $rows | Export-Csv -NoTypeInformation -Encoding UTF8 "$outputDir/compare-$Scenario.csv"
}
if($rows.Count -ne $Samples-1){throw 'Missing comparison samples'}
$cpuMax=($rows.CpuAbsDifference | Measure-Object -Maximum).Maximum
$memoryMax=($rows.MemoryAbsDifference | Measure-Object -Maximum).Maximum
if($cpuMax -gt 5 -or $memoryMax -gt 0.5){throw "CPU/memory tolerance failed: $cpuMax / $memoryMax percentage points"}
foreach($direction in @('Download','Upload')) {
  $actual=($rows.$direction | Measure-Object -Average).Average
  $expected=($rows."Reference$direction" | Measure-Object -Average).Average
  if([Math]::Abs($actual-$expected) -gt [Math]::Max(4096,$expected*0.05)){throw "$direction mean tolerance failed"}
}
$rows | Measure-Object CpuAbsDifference,MemoryAbsDifference,DownloadAbsDifference,UploadAbsDifference -Average -Maximum | Format-Table Property,Average,Maximum
Write-Output "Samples=$($rows.Count); scenario label requires the caller to establish that workload."
