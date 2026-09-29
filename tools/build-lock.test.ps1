$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build-lock.ps1"
if (-not $env:PINMETER_PACKAGE_TEST_DIR) { throw 'Run through desktop.mjs check-tools' }
$fixture = Join-Path $env:PINMETER_PACKAGE_TEST_DIR 'lock-fixture'
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
$env:PINMETER_LOCK_LIBRARY = Join-Path $PSScriptRoot 'build-lock.ps1'
$env:PINMETER_TEST_PROJECT = Split-Path $PSScriptRoot -Parent
function Assert([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Child([string]$Code) {
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Code))
    & powershell.exe -NoProfile -NonInteractive -OutputFormat Text -EncodedCommand $encoded
    if ($LASTEXITCODE -ne 0) { throw "Child assertion failed: $LASTEXITCODE" }
}
Child @'
$ErrorActionPreference = 'Stop'
$env:PSModulePath = 'C:\Pinmeter-Test-Missing-Core-Modules'
. $env:PINMETER_LOCK_LIBRARY
$hash = Get-FileHash -LiteralPath $env:PINMETER_LOCK_LIBRARY -Algorithm SHA256
if ($hash.Hash.Length -ne 64) { throw 'Windows PowerShell inbox module recovery failed' }
'@
Child @'
$ErrorActionPreference = 'Stop'
. $env:PINMETER_LOCK_LIBRARY
$lock = Enter-PinmeterBuildLock $env:PINMETER_TEST_PROJECT
if ($null -ne $lock) { Exit-PinmeterBuildLock $lock; throw 'Expected inherited build lock' }
'@
Child @'
$ErrorActionPreference = 'Stop'
. $env:PINMETER_LOCK_LIBRARY
$env:PINMETER_BUILD_OWNER = ''
try { $lock = Enter-PinmeterBuildLock $env:PINMETER_TEST_PROJECT }
catch { if ($_.Exception.Message -like 'Another Pinmeter build*') { exit 0 }; throw }
Exit-PinmeterBuildLock $lock
throw 'Unrelated writer acquired an occupied build lock'
'@
# Occupancy fixtures model a machine with no application/driver processes. Real
# inaccessible elevated processes must conservatively prevent reuse in production.
function Get-CimInstance { param($ClassName) return @() }
$first = Reserve-PinmeterPackage $fixture
try {
    Assert ((Split-Path -Leaf $first.Directory) -eq 'test1') 'Did not select smallest number'
    $env:PINMETER_TEST_MUTEX = Get-PinmeterMutexName $first.Directory 'Package'
    Child @'
$mutex = New-Object Threading.Mutex($false, $env:PINMETER_TEST_MUTEX)
try {
    if ($mutex.WaitOne(0)) { $mutex.ReleaseMutex(); throw 'Reserved directory acquired by a second process' }
} finally { $mutex.Dispose() }
'@
} finally { $first.Mutex.ReleaseMutex(); $first.Mutex.Dispose() }
$file = Join-Path $first.Directory 'resource.dll'
Set-Content -LiteralPath $file -Value 'in use'
$handle = [IO.File]::Open($file, 'Open', 'ReadWrite', 'None')
try {
    $second = Reserve-PinmeterPackage $fixture
    try { Assert ((Split-Path -Leaf $second.Directory) -eq 'test2') 'Occupied file was not skipped' }
    finally { $second.Mutex.ReleaseMutex(); $second.Mutex.Dispose() }
} finally { $handle.Dispose() }
$reused = Reserve-PinmeterPackage $fixture
try {
    Assert ((Split-Path -Leaf $reused.Directory) -eq 'test1') 'Idle managed directory was not reused'
    Assert (-not (Test-Path -LiteralPath $file)) 'Old files survived reuse'
} finally { $reused.Mutex.ReleaseMutex(); $reused.Mutex.Dispose() }
$outside = Join-Path $fixture 'outside'
New-Item -ItemType Directory -Path $outside | Out-Null
Set-Content -LiteralPath (Join-Path $outside 'keep.txt') -Value 'keep'
$junction = Join-Path $reused.Directory 'linked'
New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
try {
    $safe = Reserve-PinmeterPackage $fixture
    try { Assert ($safe.Directory -ne $reused.Directory) 'Linked directory was reused' }
    finally { $safe.Mutex.ReleaseMutex(); $safe.Mutex.Dispose() }
    Assert (Test-Path -LiteralPath (Join-Path $outside 'keep.txt')) 'Linked target changed'
} finally { [IO.Directory]::Delete($junction) }
$rejected = $false
try { $null = Assert-PinmeterPackagePath $fixture $outside } catch { $rejected = $true }
Assert $rejected 'Path outside target accepted'
Write-Host 'PASS: inherited/build contention, directory reservation, occupied resources, safe reuse, linked and outside paths.'
