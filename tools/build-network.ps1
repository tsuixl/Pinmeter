$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build-lock.ps1"
$buildLock = Enter-PinmeterBuildLock (Split-Path $PSScriptRoot -Parent)
try {
$projectRoot = Split-Path $PSScriptRoot -Parent
$source = Join-Path $projectRoot 'src/backend/platform/network'
$output = Join-Path $projectRoot 'src/backend/target/network'
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if (-not (Test-Path -LiteralPath $compiler)) { throw '.NET Framework x64 compiler is required' }
New-Item -ItemType Directory -Path $output -Force | Out-Null
& $compiler /nologo /target:exe /platform:x64 /optimize+ "/out:$(Join-Path $output 'pinmeter-network.exe')" /reference:System.Web.Extensions.dll /reference:System.Drawing.dll (Join-Path $source 'Program.cs') (Join-Path $source 'NativeEtw.cs')
if ($LASTEXITCODE -ne 0) { throw 'Network helper compilation failed' }
if ($env:PINMETER_HELPER_STAGE -ne 'tauri') { foreach ($profile in @('debug', 'release')) {
    $resources = Join-Path $projectRoot "src/backend/target/$profile/network"
    New-Item -ItemType Directory -Path $resources -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $output 'pinmeter-network.exe') -Destination $resources -Force
}
}
Write-Output "Network helper built at $output"
} finally { Exit-PinmeterBuildLock $buildLock }
