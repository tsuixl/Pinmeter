$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Get-PinmeterMutexName([string]$Path, [string]$Kind) {
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
    # Keep the build name compatible with the documented/manual reservation.
    return 'Global\Pinmeter.' + $Kind + '.' + ($full -replace '[^a-zA-Z0-9]+', '_')
}

function Test-PinmeterAncestor([int]$Owner) {
    $current = $PID
    for ($i = 0; $i -lt 32 -and $current -gt 0; $i++) {
        if ($current -eq $Owner) { return $true }
        $process = Get-CimInstance Win32_Process -Filter "ProcessId=$current" -ErrorAction Stop
        if (-not $process) { return $false }
        $current = [int]$process.ParentProcessId
    }
    return $false
}

function Enter-PinmeterBuildLock([string]$ProjectRoot) {
    $name = Get-PinmeterMutexName $ProjectRoot 'Build'
    $mutex = New-Object Threading.Mutex($false, $name)
    $acquired = $false
    try { $acquired = $mutex.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $acquired = $true }
    if (-not $acquired) {
        # Only a descendant of the live lock-owning coordinator may inherit it.
        $owner = 0
        if ([int]::TryParse($env:PINMETER_BUILD_OWNER, [ref]$owner) -and
            (Test-PinmeterAncestor $owner)) {
            $mutex.Dispose()
            return $null
        }
        $mutex.Dispose()
        throw 'Another Pinmeter build owns this workspace. Wait for it to finish; no outputs were changed.'
    }
    $previousOwner = $env:PINMETER_BUILD_OWNER
    $env:PINMETER_BUILD_OWNER = [string]$PID
    return [PSCustomObject]@{ Mutex = $mutex; PreviousOwner = $previousOwner }
}

function Exit-PinmeterBuildLock($Lock) {
    if ($null -ne $Lock) {
        $env:PINMETER_BUILD_OWNER = $Lock.PreviousOwner
        $Lock.Mutex.ReleaseMutex()
        $Lock.Mutex.Dispose()
    }
}

function Assert-PinmeterPlainPath([string]$Path) {
    $part = [IO.Path]::GetFullPath($Path)
    while ($part) {
        if (Test-Path -LiteralPath $part) {
            $item = Get-Item -Force -LiteralPath $part
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Linked path is not managed: $part"
            }
        }
        $parent = Split-Path -Parent $part
        if ($parent -eq $part) { break }
        $part = $parent
    }
}

function Get-PinmeterPlainFiles([string]$Directory) {
    # Inspect each level before descending; never follow a directory junction.
    foreach ($item in Get-ChildItem -Force -LiteralPath $Directory) {
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Linked content is not managed: $($item.FullName)"
        }
        if ($item.PSIsContainer) { Get-PinmeterPlainFiles $item.FullName }
        else { $item }
    }
}

function Assert-PinmeterPackagePath([string]$ProjectRoot, [string]$Directory) {
    $target = [IO.Path]::GetFullPath((Join-Path $ProjectRoot 'src/backend/target'))
    $full = [IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
    if ((Split-Path -Parent $full) -ne $target -or (Split-Path -Leaf $full) -notmatch '^test[1-9][0-9]*$') {
        throw "Not a numbered package inside target: $full"
    }
    Assert-PinmeterPlainPath $full
    return $full
}

function Assert-PinmeterPackageIdle([string]$Directory) {
    $prefix = $Directory.TrimEnd('\') + '\'
    $files = @(Get-PinmeterPlainFiles $Directory)
    $hasExecutable = @($files | Where-Object { $_.Extension -eq '.exe' }).Count -gt 0
    foreach ($process in Get-CimInstance Win32_Process) {
        if ($process.ExecutablePath -and $process.ExecutablePath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Process $($process.ProcessId) is using this package"
        }
        if ($hasExecutable -and -not $process.ExecutablePath -and $process.Name -match '^pinmeter.*\.exe$') {
            throw "Cannot determine the path of Pinmeter process $($process.ProcessId)"
        }
    }
    foreach ($driver in Get-CimInstance Win32_SystemDriver) {
        $driverPath = if ($driver.PathName) { $driver.PathName.Trim('"') -replace '^\\\?\?\\', '' } else { '' }
        if ($driverPath.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Driver $($driver.Name) still refers to this package"
        }
    }
    $handles = New-Object 'Collections.Generic.List[IDisposable]'
    try {
        foreach ($file in $files) {
            $handles.Add([IO.File]::Open($file.FullName, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None))
        }
    } finally {
        foreach ($handle in $handles) { $handle.Dispose() }
    }
}

function Reserve-PinmeterPackage([string]$ProjectRoot, [string]$Requested = '') {
    $target = Join-Path $ProjectRoot 'src/backend/target'
    Assert-PinmeterPlainPath $target
    New-Item -ItemType Directory -Force -Path $target | Out-Null
    for ($index = 1; $index -le 10000; $index++) {
        $candidate = if ($Requested) { $Requested } else { Join-Path $target "test$index" }
        $mutex = $null
        $owned = $false
        try {
            $candidate = Assert-PinmeterPackagePath $ProjectRoot $candidate
            $mutex = New-Object Threading.Mutex($false, (Get-PinmeterMutexName $candidate 'Package'))
            try { $owned = $mutex.WaitOne(0) }
            catch [Threading.AbandonedMutexException] { $owned = $true }
            if (-not $owned) { throw 'Directory is reserved by another task' }
            if (Test-Path -LiteralPath $candidate) {
                $children = @(Get-ChildItem -Force -LiteralPath $candidate)
                if ($children.Count -gt 0) {
                    $markerPath = Join-Path $candidate '.pinmeter-package.json'
                    Assert-PinmeterPlainPath $markerPath
                    if (-not (Test-Path -LiteralPath $markerPath)) { throw 'Historical directory has no verified ownership marker' }
                    $marker = Get-Content -Raw -Encoding UTF8 -LiteralPath $markerPath | ConvertFrom-Json
                    if ($marker.schema -ne 1 -or $marker.project -ne $ProjectRoot) { throw 'Package ownership does not match this workspace' }
                    Assert-PinmeterPackageIdle $candidate
                    # Revalidate immediately before recursive deletion, using one shell.
                    $candidate = Assert-PinmeterPackagePath $ProjectRoot $candidate
                    $null = @(Get-PinmeterPlainFiles $candidate)
                    Remove-Item -LiteralPath $candidate -Recurse -Force -ErrorAction Stop
                }
            }
            New-Item -ItemType Directory -Force -Path $candidate | Out-Null
            @{ schema = 1; project = $ProjectRoot; owner = $PID; task = $env:PINMETER_BUILD_TASK; started = [DateTime]::UtcNow.ToString('o') } |
                ConvertTo-Json | Set-Content -Encoding UTF8 -LiteralPath (Join-Path $candidate '.pinmeter-package.json')
            return [PSCustomObject]@{ Directory = $candidate; Mutex = $mutex }
        } catch {
            Write-Host "Skip $candidate : $($_.Exception.Message)"
            if ($owned) { $mutex.ReleaseMutex() }
            if ($mutex) { $mutex.Dispose() }
            if ($Requested) { throw }
        }
    }
    throw 'No safe package directory available'
}
