param([ValidateRange(2, 60)][int]$Samples = 20)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$cache = Join-Path $projectRoot 'src/backend/target/sensor-deps'
$output = Join-Path $projectRoot 'src/backend/target/gpu-probe'
$archive = Join-Path $cache 'lhm-0.9.6.zip'
$package = Join-Path $cache 'lhm-0.9.6'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (-not (Test-Path -LiteralPath $compiler)) { throw '.NET Framework x64 compiler is required' }
if (-not (Test-Path -LiteralPath $archive)) { throw 'Run tools/build-sensors.ps1 first to obtain the pinned LHM dependency' }
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '086D9F1B5A99E643EDC2CFAAAC16051685B551E4C5AC0B32A57C58C0E529C001') { throw 'LHM archive checksum mismatch' }
New-Item -ItemType Directory -Path $output -Force | Out-Null
# Extract from the verified archive; do not change the running application's resources.
Expand-Archive -LiteralPath $archive -DestinationPath $package -Force
Get-ChildItem -LiteralPath $package -Filter '*.dll' | Copy-Item -Destination $output -Force
$exe = Join-Path $output 'gpu-probe.exe'
Copy-Item -LiteralPath (Join-Path $package 'LibreHardwareMonitor.exe.config') -Destination "$exe.config" -Force
$library = Join-Path $output 'LibreHardwareMonitorLib.dll'
$source = Join-Path $projectRoot 'src/backend/platform/sensors/probes/GpuProbe.cs'
& $compiler /nologo /target:exe /platform:x64 /optimize+ "/out:$exe" "/reference:$library" /reference:System.Web.Extensions.dll $source
if ($LASTEXITCODE -ne 0) { throw 'GPU probe compilation failed' }
$runName = 'gpu-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff')
$stdout = Join-Path $output "$runName.jsonl"
$stderr = Join-Path $output "$runName.stderr.log"
$process = Start-Process -FilePath $exe -ArgumentList @($Samples.ToString()) -WorkingDirectory $output -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
try {
    # Retain the native handle before exit so Windows PowerShell can retrieve ExitCode.
    $processHandle = $process.Handle
    if (-not $process.WaitForExit(($Samples * 5 + 20) * 1000)) {
        # Only this diagnostic and its bounded nvidia-smi child are terminated.
        & taskkill.exe /PID $process.Id /T /F | Out-Null
        throw "GPU probe timed out; partial evidence: $stdout"
    }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "GPU probe failed ($($process.ExitCode)); inspect $stdout and $stderr" }
    $records = @(Get-Content -LiteralPath $stdout -Encoding UTF8 | ForEach-Object { $_ | ConvertFrom-Json })
    if (@($records | Where-Object kind -eq 'sample').Count -ne $Samples -or $records[-1].kind -ne 'closed') {
        throw "Incomplete GPU probe output: $stdout"
    }
    Write-Output "GPU probe completed: $stdout"
} finally { $process.Dispose() }
