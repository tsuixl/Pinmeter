$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$inventory = [ordered]@{manufacturer=$null; model=$null; system=$null; system_detail=$null; boot_at_ms=$null; sections=@()}
function Query([string]$class, [string]$namespace = 'root/cimv2') {
    @(Get-CimInstance -Namespace $namespace -ClassName $class -OperationTimeoutSec 5 -ErrorAction Stop)
}
function Item([string]$name, [object[]]$details = @()) {
    if (-not [string]::IsNullOrWhiteSpace($name)) {
        @{name=$name.Trim(); details=@($details | Where-Object {-not [string]::IsNullOrWhiteSpace([string]$_)} | ForEach-Object {[string]$_})}
    }
}
function Section([string]$id, [scriptblock]$read) {
    try { $inventory.sections += @{id=$id; items=@(& $read); error=$null} }
    catch { $inventory.sections += @{id=$id; items=@(); error='系统未能提供此类信息'} }
}
function Gib($bytes) { if ($bytes -gt 0) { '{0:0.#} GiB' -f ($bytes / 1GB) } }
function Text($codes) { -join ($codes | Where-Object {$_ -gt 0} | ForEach-Object {[char]$_}) }
try {
    $computer = Query 'Win32_ComputerSystem' | Select-Object -First 1
    $inventory.manufacturer = $computer.Manufacturer
    $inventory.model = $computer.Model
} catch {}
try {
    $os = Query 'Win32_OperatingSystem' | Select-Object -First 1
    $inventory.system = $os.Caption
    $inventory.system_detail = "$($os.OSArchitecture) · $($os.Version)"
    $inventory.boot_at_ms = ([DateTimeOffset]$os.LastBootUpTime).ToUnixTimeMilliseconds()
} catch {}
Section 'board' {
    $bios = $null
    try { $bios = Query 'Win32_BIOS' | Select-Object -First 1 } catch {}
    Query 'Win32_BaseBoard' | ForEach-Object {
        $details = @($_.Manufacturer)
        if ($bios.SMBIOSBIOSVersion) { $details += "BIOS $($bios.SMBIOSBIOSVersion)" }
        Item $_.Product $details
    }
}
Section 'cpu' {
    Query 'Win32_Processor' | ForEach-Object {
        $details = @()
        if ($_.NumberOfCores -gt 0) { $details += "$($_.NumberOfCores) 核" }
        if ($_.NumberOfLogicalProcessors -gt 0) { $details += "$($_.NumberOfLogicalProcessors) 逻辑处理器" }
        Item $_.Name $details
    }
}
Section 'memory' {
    $sticks = @(Query 'Win32_PhysicalMemory')
    $slots = $null
    try { $slots = (Query 'Win32_PhysicalMemoryArray' | Measure-Object MemoryDevices -Sum).Sum } catch {}
    $total = ($sticks | Measure-Object Capacity -Sum).Sum
    $summary = @()
    if ($slots -ge $sticks.Count -and $slots -gt 0) { $summary += "$($sticks.Count) / $slots 插槽已使用" }
    if ($total -gt 0) { Item ("已安装 " + (Gib $total)) $summary }
    foreach ($stick in $sticks) {
        $kind = switch ($stick.SMBIOSMemoryType) { 26 {'DDR4'} 34 {'DDR5'} 24 {'DDR3'} 30 {'LPDDR4'} 35 {'LPDDR5'} default {$null} }
        $details = @((Gib $stick.Capacity), $kind, $stick.DeviceLocator)
        if ($stick.ConfiguredClockSpeed -gt 0) { $details += "配置速率 $($stick.ConfiguredClockSpeed)（系统报告值）" }
        $name = @(([string]$stick.Manufacturer).Trim(), ([string]$stick.PartNumber).Trim()) | Where-Object {$_}
        if (-not $name) { $name = @('内存模块') }
        Item ($name -join ' ') $details
    }
}
Section 'gpu' {
    Query 'Win32_VideoController' | ForEach-Object {
        $details = @()
        if ($_.DriverVersion) { $details += "驱动 $($_.DriverVersion)" }
        if ($_.Name -match 'Virtual|Indirect') { $details += '虚拟显示适配器' }
        Item $_.Name $details
    }
}
Section 'display' {
    $sizes = @(); $modes = @()
    try { $sizes = Query 'WmiMonitorBasicDisplayParams' 'root/wmi' } catch {}
    try { $modes = Query 'WmiMonitorListedSupportedSourceModes' 'root/wmi' } catch {}
    Query 'WmiMonitorID' 'root/wmi' | Where-Object {$_.Active} | ForEach-Object {
        $id = $_.InstanceName
        $name = Text $_.UserFriendlyName
        if (-not $name) { $name = '显示器（型号未提供）' }
        $details = @((Text $_.ManufacturerName))
        $size = $sizes | Where-Object {$_.InstanceName -eq $id} | Select-Object -First 1
        if ($size.MaxHorizontalImageSize -gt 0 -and $size.MaxVerticalImageSize -gt 0) {
            $diagonal = [Math]::Sqrt([Math]::Pow($size.MaxHorizontalImageSize, 2) + [Math]::Pow($size.MaxVerticalImageSize, 2)) / 2.54
            $details += ('约 {0:0.#} 英寸' -f $diagonal)
        }
        $mode = $modes | Where-Object {$_.InstanceName -eq $id} | Select-Object -First 1
        if ($mode -and $mode.PreferredMonitorSourceModeIndex -lt $mode.MonitorSourceModes.Count) {
            $preferred = $mode.MonitorSourceModes[$mode.PreferredMonitorSourceModeIndex]
            if ($preferred.HorizontalActivePixels -gt 0 -and $preferred.VerticalActivePixels -gt 0) {
                $details += "首选 $($preferred.HorizontalActivePixels) × $($preferred.VerticalActivePixels)"
            }
        }
        Item $name $details
    }
}
Section 'disk' {
    Query 'MSFT_PhysicalDisk' 'root/Microsoft/Windows/Storage' | ForEach-Object {
        $bus = switch ($_.BusType) {17 {'NVMe'} 11 {'SATA'} 7 {'USB'} 8 {'RAID'} default {$null}}
        $media = switch ($_.MediaType) {3 {'HDD'} 4 {'SSD'} default {$null}}
        Item $_.FriendlyName @((Gib $_.Size), $media, $bus)
    }
}
Section 'audio' { Query 'Win32_SoundDevice' | ForEach-Object { Item $_.Name } }
Section 'network' {
    Query 'Win32_NetworkAdapter' | Where-Object {$_.PhysicalAdapter -or $_.NetConnectionID} | ForEach-Object {
        $details = @($_.NetConnectionID)
        if ($_.Name -match 'Virtual|VPN|Tunnel|TAP|TUN|Hyper-V') { $details += '虚拟适配器' }
        Item $_.Name $details
    }
}
$inventory | ConvertTo-Json -Depth 6 -Compress
