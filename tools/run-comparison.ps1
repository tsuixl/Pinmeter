param([ValidateSet('cpu','memory','network')][string]$Scenario)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Threading;
public static class PinmeterTestLoad {
  static volatile bool running;
  static Thread[] workers;
  static byte[] memory;
  public static void StartCpu() { running=true;workers=new Thread[4];for(int i=0;i<workers.Length;i++){workers[i]=new Thread(()=>{double v=1;while(running){for(int n=1;n<100000;n++)v=Math.Sqrt(v+n);GC.KeepAlive(v);}});workers[i].IsBackground=true;workers[i].Start();} }
  public static void StartMemory(){memory=new byte[256*1024*1024];for(int i=0;i<memory.Length;i+=4096)memory[i]=1;}
  public static void Stop(){running=false;if(workers!=null)foreach(var thread in workers)thread.Join();memory=null;}
}
'@
$transfer=$null
try {
  if($Scenario -eq 'cpu'){[PinmeterTestLoad]::StartCpu()}
  if($Scenario -eq 'memory'){[PinmeterTestLoad]::StartMemory()}
  if($Scenario -eq 'network'){
    # Public vendor download, capped to 512 KiB/s; no local data is uploaded.
    $transfer=Start-Process -FilePath curl.exe -ArgumentList @('-L','--fail','--silent','--limit-rate','512K','--max-time','70','https://static.rust-lang.org/dist/rust-1.98.1-x86_64-pc-windows-msvc.tar.xz','-o','NUL') -PassThru -WindowStyle Hidden
  }
  & "$PSScriptRoot/compare-windows.ps1" -Samples 61 -Scenario $Scenario
  if($Scenario -eq 'network') {
    $csv = Import-Csv "$PSScriptRoot/../src/backend/target/measurements/compare-network.csv"
    $average = ($csv | ForEach-Object { [double]$_.ReferenceDownload } | Measure-Object -Average).Average
    if($average -lt 65536) { throw "Network load not established: average download $average B/s" }
  }
} finally {
  [PinmeterTestLoad]::Stop()
  if($transfer -and !$transfer.HasExited){$transfer.Kill();$transfer.WaitForExit()}
}
