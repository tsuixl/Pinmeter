$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build-lock.ps1"
$buildLock = Enter-PinmeterBuildLock (Split-Path $PSScriptRoot -Parent)
try {
$projectRoot = Split-Path $PSScriptRoot -Parent
$source = Join-Path $projectRoot 'src/backend/platform/network-control'
$output = Join-Path $projectRoot 'src/backend/target/network-control'
$cache = Join-Path $projectRoot 'src/backend/target/network-control-research'
New-Item -ItemType Directory -Force -Path $output, $cache | Out-Null
$archive = Join-Path $cache 'WinDivert-2.2.2-A.zip'
if (-not (Test-Path -LiteralPath $archive)) {
    Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/basil00/WinDivert/releases/download/v2.2.2/WinDivert-2.2.2-A.zip' -OutFile $archive
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne '63CB41763BB4B20F600B6DE04E991A9C2BE73279E317D4D82F237B150C5F3F15') { throw 'WinDivert archive checksum mismatch' }
Expand-Archive -LiteralPath $archive -DestinationPath $cache -Force
$dist = Join-Path $cache 'WinDivert-2.2.2-A'
$driver = Join-Path $dist 'x64/WinDivert64.sys'
if ((Get-AuthenticodeSignature -LiteralPath $driver).Status -ne 'Valid') { throw 'WinDivert driver signature invalid' }
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
& $compiler /nologo /target:exe /platform:x64 /optimize+ "/out:$(Join-Path $output 'pinmeter-network-control.exe')" /reference:System.Web.Extensions.dll /reference:Microsoft.CSharp.dll (Join-Path $source 'Program.cs') (Join-Path $source 'PacketEngine.cs') (Join-Path $source 'WfpBlock.cs') (Join-Path $source 'Diagnostics.cs')
if ($LASTEXITCODE -ne 0) { throw 'Network control helper compilation failed' }
Copy-Item -LiteralPath (Join-Path $dist 'x64/WinDivert.dll'), $driver, (Join-Path $dist 'LICENSE'), (Join-Path $source 'THIRD-PARTY-NOTICES.txt') -Destination $output -Force
# Ship the upstream distribution (including headers/examples/licenses) alongside replaceable DLL/driver.
Copy-Item -LiteralPath $archive -Destination $output -Force
$sourceArchive = Join-Path $cache 'WinDivert-2.2.2-source.zip'
if (-not (Test-Path -LiteralPath $sourceArchive)) {
    Invoke-WebRequest -UseBasicParsing -Uri 'https://codeload.github.com/basil00/WinDivert/zip/refs/tags/v2.2.2' -OutFile $sourceArchive
}
if ((Get-FileHash -LiteralPath $sourceArchive -Algorithm SHA256).Hash -ne '65EC79C9E6AFA99F648A3F4D1F6DB794640B40D0B65BD438770EA503EE14ECB7') { throw 'WinDivert source checksum mismatch' }
Copy-Item -LiteralPath $sourceArchive -Destination $output -Force
& (Join-Path $output 'pinmeter-network-control.exe') --self-test
if ($LASTEXITCODE -ne 0) { throw 'Network control adapter tests failed' }
if ($env:PINMETER_HELPER_STAGE -ne 'tauri') { foreach ($profile in @('debug', 'release')) {
    $destination = Join-Path $projectRoot "src/backend/target/$profile/network-control"
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    foreach ($file in Get-ChildItem -LiteralPath $output -File) {
        $targetFile = Join-Path $destination $file.Name
        # Windows may keep an unchanged signed driver loaded after the last handle closes.
        if ((Test-Path -LiteralPath $targetFile) -and ((Get-FileHash -LiteralPath $targetFile).Hash -eq (Get-FileHash -LiteralPath $file.FullName).Hash)) { continue }
        Copy-Item -LiteralPath $file.FullName -Destination $targetFile -Force
    }
}
}
Write-Output "Network control helper built at $output"
} finally { Exit-PinmeterBuildLock $buildLock }
