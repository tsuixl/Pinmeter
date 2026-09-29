$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build-lock.ps1"
$buildLock = Enter-PinmeterBuildLock (Split-Path $PSScriptRoot -Parent)
try {
$projectRoot = Split-Path $PSScriptRoot -Parent
$sensorSource = Join-Path $projectRoot 'src/backend/platform/sensors'
$cache = Join-Path $projectRoot 'src/backend/target/sensor-deps'
$output = Join-Path $projectRoot 'src/backend/target/sensors'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (-not (Test-Path -LiteralPath $compiler)) { throw '.NET Framework 4.7.2 or later is required to build the Windows CPU helper' }
New-Item -ItemType Directory -Path $cache, $output -Force | Out-Null
$archive = Join-Path $cache 'lhm-0.9.6.zip'
if (-not (Test-Path -LiteralPath $archive)) {
    Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases/download/v0.9.6/LibreHardwareMonitor.zip' -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '086D9F1B5A99E643EDC2CFAAAC16051685B551E4C5AC0B32A57C58C0E529C001') { throw 'LHM archive checksum mismatch' }
$package = Join-Path $cache 'lhm-0.9.6'
Expand-Archive -LiteralPath $archive -DestinationPath $package -Force
# CPU library dependency closure; no full LHM user interface is shipped or launched.
$libraries = @('LibreHardwareMonitorLib.dll', 'HidSharp.dll', 'DiskInfoToolkit.dll', 'BlackSharp.Core.dll', 'RAMSPDToolkit-NDD.dll', 'System.Memory.dll', 'System.Buffers.dll', 'System.Numerics.Vectors.dll', 'System.Runtime.CompilerServices.Unsafe.dll', 'System.Threading.AccessControl.dll', 'System.Security.AccessControl.dll', 'System.Security.Principal.Windows.dll')
foreach ($library in $libraries) { Copy-Item -LiteralPath (Join-Path $package $library) -Destination $output -Force }
Copy-Item -LiteralPath (Join-Path $package 'LibreHardwareMonitor.exe.config') -Destination (Join-Path $output 'pinmeter-sensors.exe.config') -Force
$libraryPath = Join-Path $output 'LibreHardwareMonitorLib.dll'
& $compiler /nologo /target:exe /platform:x64 /optimize+ "/out:$(Join-Path $output 'pinmeter-sensors.exe')" "/reference:$libraryPath" /reference:System.Web.Extensions.dll (Join-Path $sensorSource 'Program.cs') (Join-Path $sensorSource 'TemperatureSelection.cs') (Join-Path $sensorSource 'PawnIODriver.cs') (Join-Path $sensorSource 'PawnIODownload.cs')
if ($LASTEXITCODE -ne 0) { throw 'CPU helper compilation failed' }
Copy-Item -LiteralPath (Join-Path $package 'LibreHardwareMonitor.exe.config') -Destination (Join-Path $output 'pinmeter-gpu.exe.config') -Force
& $compiler /nologo /target:exe /platform:x64 /optimize+ "/out:$(Join-Path $output 'pinmeter-gpu.exe')" "/reference:$libraryPath" /reference:System.Web.Extensions.dll (Join-Path $sensorSource 'GpuProgram.cs')
if ($LASTEXITCODE -ne 0) { throw 'GPU helper compilation failed' }
& $compiler /nologo /target:exe "/out:$(Join-Path $cache 'selection-tests.exe')" (Join-Path $sensorSource 'TemperatureSelection.cs') (Join-Path $sensorSource 'tests/SelectionTests.cs')
if ($LASTEXITCODE -ne 0) { throw 'CPU selection tests compilation failed' }
& "$cache/selection-tests.exe"
if ($LASTEXITCODE -ne 0) { throw 'CPU selection tests failed' }
& $compiler /nologo /target:exe /platform:x64 "/out:$(Join-Path $cache 'driver-download-tests.exe')" /reference:System.Web.Extensions.dll (Join-Path $sensorSource 'PawnIODriver.cs') (Join-Path $sensorSource 'PawnIODownload.cs') (Join-Path $sensorSource 'tests/DriverDownloadTests.cs')
if ($LASTEXITCODE -ne 0) { throw 'Driver download tests compilation failed' }
& "$cache/driver-download-tests.exe"
if ($LASTEXITCODE -ne 0) { throw 'Driver download tests failed' }
Write-Output "CPU helper built at $output"
# Retired distribution resource: never let a previously cached installer enter a new bundle.
Assert-PinmeterPlainPath $output
$retiredInstaller = Join-Path $output 'PawnIO_setup.exe'
if (Test-Path -LiteralPath $retiredInstaller) { Remove-Item -LiteralPath $retiredInstaller -ErrorAction Stop }
Copy-Item -LiteralPath (Join-Path $sensorSource 'licenses') -Destination $output -Recurse -Force
if ($env:PINMETER_HELPER_STAGE -ne 'tauri') { foreach ($profile in @('debug', 'release')) {
    $resources = Join-Path $projectRoot "src/backend/target/$profile/sensors"
    New-Item -ItemType Directory -Path $resources -Force | Out-Null
    Assert-PinmeterPlainPath $resources
    $retiredInstaller = Join-Path $resources 'PawnIO_setup.exe'
    if (Test-Path -LiteralPath $retiredInstaller) { Remove-Item -LiteralPath $retiredInstaller -ErrorAction Stop }
    Get-ChildItem -LiteralPath $output | Copy-Item -Destination $resources -Recurse -Force
}
}
} finally { Exit-PinmeterBuildLock $buildLock }
