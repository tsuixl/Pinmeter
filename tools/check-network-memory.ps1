param(
    [Parameter(Mandatory=$true)][string]$Helper,
    [Parameter(Mandatory=$true)][string]$OutputDirectory
)
# Read-only COM inventory stress check. No firewall/driver/limit changes.
$ErrorActionPreference = 'Stop'
$helperPath = (Resolve-Path -LiteralPath $Helper).Path
$null = New-Item -ItemType Directory -Path $OutputDirectory -Force
$probePath = Join-Path $OutputDirectory 'inventory-memory-check.exe'
$sourcePath = Join-Path $OutputDirectory 'inventory-memory-check.cs'
$source = @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Reflection;
using System.Web.Script.Serialization;
class InventoryMemoryCheck {
    static void Collect() { GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect(); }
    static int Main(string[] args) {
        var assembly = Assembly.LoadFrom(args[0]);
        var method = assembly.GetType("Firewall").GetMethod("InspectOwned", BindingFlags.Static | BindingFlags.NonPublic);
        for (int i = 0; i < 10; i++) method.Invoke(null, null);
        Collect(); long baseline = GC.GetTotalMemory(false);
        var samples = new List<object>();
        var timer = Stopwatch.StartNew();
        for (int i = 0; i < 100; i++) {
            method.Invoke(null, null);
            if (i % 10 == 9) {
                Collect();
                using (var process = Process.GetCurrentProcess()) samples.Add(new { iterations = i + 1,
                    managed = GC.GetTotalMemory(false), privateBytes = process.PrivateMemorySize64,
                    working = process.WorkingSet64, handles = process.HandleCount });
            }
        }
        Collect(); long retained = GC.GetTotalMemory(false), growth = retained - baseline;
        // This detects the former unbounded dynamic COM cache, not the app's RAM budget.
        bool passed = growth < 16 * 1024 * 1024;
        Console.WriteLine(new JavaScriptSerializer().Serialize(new { passed, baseline, retained, growth,
            durationMs = timer.ElapsedMilliseconds, iterations = 100, samples }));
        return passed ? 0 : 1;
    }
}
'@
[IO.File]::WriteAllText($sourcePath, $source, [Text.UTF8Encoding]::new($false))
& (Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe') /nologo /platform:x64 /reference:System.Web.Extensions.dll "/out:$probePath" $sourcePath
if ($LASTEXITCODE -ne 0) { throw 'Read-only inventory memory probe compilation failed' }
$result = & $probePath $helperPath
$code = $LASTEXITCODE
$report = $result | ConvertFrom-Json
$report | Add-Member -NotePropertyName HelperSha256 -NotePropertyValue (Get-FileHash -LiteralPath $helperPath -Algorithm SHA256).Hash
$json = $report | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $OutputDirectory 'result.json'), $json, [Text.UTF8Encoding]::new($false))
Write-Output $json
if ($code -ne 0) { throw 'Firewall inventory retains excessive managed memory after repeated reads' }
