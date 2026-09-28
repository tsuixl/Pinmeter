$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/build-lock.ps1"
$projectRoot = [IO.Path]::GetFullPath((Split-Path $PSScriptRoot -Parent))
$arguments = @(ConvertFrom-Json $env:PINMETER_DESKTOP_ARGS)
$buildLock = Enter-PinmeterBuildLock $projectRoot
$reservation = $null
$exitCode = 1
try {
    $action = $arguments[0]
    if ($env:CARGO_TARGET_DIR -and -not [IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        throw 'CARGO_TARGET_DIR must be absolute so Cargo and the packager use the same output'
    }
    if ($action -eq 'build' -or $action -eq 'check-tools') {
        $requested = @($arguments | Where-Object { $_ -like '--package-dir=*' })
        if ($requested.Count -gt 1) { throw 'Only one package directory may be supplied' }
        $packagePath = ''
        if ($requested.Count) {
            $packagePath = $requested[0].Substring('--package-dir='.Length)
            if (-not [IO.Path]::IsPathRooted($packagePath)) { $packagePath = Join-Path $projectRoot $packagePath }
        }
        if ($action -eq 'build' -and @($arguments | Where-Object { $_ -match '^--(debug|target|profile)(=|$)' }).Count) {
            if ($requested.Count) { throw 'Numbered delivery requires a native Windows release build' }
        } else {
            $reservation = Reserve-PinmeterPackage $projectRoot $packagePath
            $env:PINMETER_PACKAGE_DIRECTORY = $reservation.Directory
            $env:PINMETER_PACKAGE_TEST_DIR = Join-Path $reservation.Directory 'tool-tests'
            Write-Host "Reserved package: $($reservation.Directory)"
        }
    }
    # Keep the shared compilation cache separate from packages users launch.
    if (-not $env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR = Join-Path $projectRoot 'src/backend/target/desktop-build' }
    $env:PINMETER_DESKTOP_WORKER = [string]$PID
    $env:PINMETER_HELPER_STAGE = 'tauri'
    & node "$PSScriptRoot/desktop.mjs" @arguments
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) { throw "Desktop operation failed: $exitCode" }
    if ($reservation -and (Test-Path -LiteralPath $env:PINMETER_PACKAGE_TEST_DIR)) {
        $testPath = [IO.Path]::GetFullPath($env:PINMETER_PACKAGE_TEST_DIR)
        if ((Split-Path -Parent $testPath) -ne $reservation.Directory) { throw 'Invalid test cleanup path' }
        Assert-PinmeterPlainPath $testPath
        $null = @(Get-PinmeterPlainFiles $testPath)
        Remove-Item -LiteralPath $testPath -Recurse -Force
    }
    if ($reservation -and $env:PINMETER_HOLD_DELIVERY -eq '1') {
        Write-Host "DELIVERY READY: $($reservation.Directory) (reservation retained until .delivery-release)"
        while (-not (Test-Path -LiteralPath (Join-Path $reservation.Directory '.delivery-release'))) {
            Start-Sleep -Milliseconds 500
        }
        Remove-Item -LiteralPath (Join-Path $reservation.Directory '.delivery-release')
    }
} catch {
    Write-Host $_.Exception.Message
    $exitCode = 1
} finally {
    if ($reservation) { $reservation.Mutex.ReleaseMutex(); $reservation.Mutex.Dispose() }
    Exit-PinmeterBuildLock $buildLock
}
exit $exitCode
