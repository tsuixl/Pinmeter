$ErrorActionPreference = 'Stop'
$testRoot = $env:PINMETER_PACKAGE_TEST_DIR
if (-not $testRoot) { throw 'Run through the reserved check-tools entry' }
$makensis = Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe'
if (-not (Test-Path -LiteralPath $makensis)) {
    Write-Output 'SKIP: NSIS is not installed yet; rerun check-tools after the first bundled build.'
    exit 0
}
$root = Join-Path $testRoot 'uninstall-hook'
New-Item -ItemType Directory -Force -Path $root | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
$helperSource = @'
using System;
using System.IO;
class CleanupFixture {
 static int Main(string[] args) {
  var root = Path.GetFullPath(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, ".."));
  if (args.Length != 1 || args[0] != "--uninstall-cleanup-request") return 9;
  File.WriteAllText(Path.Combine(root, "cleanup-called.txt"), args[0]);
  return Int32.Parse(File.ReadAllText(Path.Combine(root, "cleanup-exit.txt")));
 }
}
'@
$sourcePath = Join-Path $root 'CleanupFixture.cs'
$helperPath = Join-Path $root 'cleanup-fixture.exe'
[IO.File]::WriteAllText($sourcePath, $helperSource, $utf8)
$compiler = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
& $compiler /nologo /target:exe "/out:$helperPath" $sourcePath
if ($LASTEXITCODE -ne 0) { throw 'Cleanup fixture compilation failed' }
$installer = Join-Path $root 'fixture-setup.exe'
$hook = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../src/backend/host/network-control-hooks.nsh'))
$script = @'
Unicode true
Name "Pinmeter uninstall hook regression"
OutFile "@INSTALLER@"
RequestExecutionLevel user
SilentInstall silent
SilentUnInstall silent
Var UpdateMode
!define MAINBINARYNAME "Pinmeter"
!define PRODUCTNAME "Pinmeter"
; The fixture tests the cleanup hook only; it never touches a real application.
!macro CheckIfAppIsRunning BINARY PRODUCT
!macroend
!include "@HOOK@"
Section
 SetOutPath "$INSTDIR\network-control"
 File /oname=pinmeter-network-control.exe "@HELPER@"
 FileOpen $0 "$INSTDIR\kept.txt" w
 FileWrite $0 "application files must survive cleanup failures"
 FileClose $0
 WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd
Function un.onInit
 ReadEnvStr $UpdateMode "PINMETER_TEST_UPGRADE"
FunctionEnd
Section "Uninstall"
 !insertmacro NSIS_HOOK_PREUNINSTALL
 Delete "$INSTDIR\kept.txt"
SectionEnd
'@
$script = $script.Replace('@INSTALLER@', $installer).Replace('@HOOK@', $hook).Replace('@HELPER@', $helperPath)
$scriptPath = Join-Path $root 'fixture.nsi'
[IO.File]::WriteAllText($scriptPath, $script, $utf8)
& $makensis /V2 $scriptPath
if ($LASTEXITCODE -ne 0) { throw 'NSIS hook fixture compilation failed' }
function Run-Fixture([string]$Executable, [string]$Arguments) {
    $start = [Diagnostics.ProcessStartInfo]::new($Executable, $Arguments)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        if (-not $process.WaitForExit(15000)) { $process.Kill(); throw 'NSIS fixture timed out' }
        return $process.ExitCode
    } finally { $process.Dispose() }
}
$previousUpgrade = $env:PINMETER_TEST_UPGRADE
try {
    foreach ($scenario in @('success', 'missing', 'failure', 'upgrade')) {
        $installDir = Join-Path $root $scenario
        if ((Run-Fixture $installer ("/S /D=" + $installDir)) -ne 0) { throw 'Fixture installation failed' }
        [IO.File]::WriteAllText((Join-Path $installDir 'cleanup-exit.txt'), $(if ($scenario -eq 'success') { '0' } else { '7' }), $utf8)
        if ($scenario -eq 'missing') { Remove-Item -LiteralPath (Join-Path $installDir 'network-control/pinmeter-network-control.exe') }
        $env:PINMETER_TEST_UPGRADE = if ($scenario -eq 'upgrade') { '1' } else { '0' }
        $code = Run-Fixture (Join-Path $installDir 'uninstall.exe') ("/S _?=" + $installDir)
        $kept = Test-Path -LiteralPath (Join-Path $installDir 'kept.txt')
        $called = Test-Path -LiteralPath (Join-Path $installDir 'cleanup-called.txt')
        if ($scenario -in @('missing','failure')) {
            if ($code -eq 0 -or -not $kept) { throw "$scenario cleanup did not preserve files and fail uninstall" }
        } elseif ($code -ne 0 -or $kept) { throw "$scenario uninstall did not finish" }
        if ($called -ne ($scenario -in @('success','failure'))) { throw "$scenario invoked the wrong cleanup path" }
    }
} finally { $env:PINMETER_TEST_UPGRADE = $previousUpgrade }
Write-Output 'PASS: compiled NSIS hook preserves files on missing/failed helper, checks the uninstall command, allows success, and skips cleanup on upgrade (isolated fixtures).'
