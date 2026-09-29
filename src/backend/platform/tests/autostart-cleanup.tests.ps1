$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../src/autostart.ps1" -DefinitionsOnly

function New-TestTask([string]$Name, [string]$Path, [string]$Arguments = '', [int]$Count = 1) {
    $actions = [pscustomobject]@{ Count = $Count; Action = [pscustomobject]@{ Type = 0; Path = $Path; Arguments = $Arguments } }
    $actions | Add-Member ScriptMethod Item { param($index) $this.Action }
    [pscustomobject]@{ Name = $Name; Definition = [pscustomobject]@{ Actions = $actions } }
}
function New-TestFolder($Tasks, [bool]$FailDelete = $false, [bool]$Retain = $false) {
    $folder = [pscustomobject]@{ Tasks = [Collections.ArrayList]@($Tasks); FailDelete = $FailDelete; Retain = $Retain }
    $folder | Add-Member ScriptMethod GetTasks { param($flags) $this.Tasks.ToArray() }
    $folder | Add-Member ScriptMethod DeleteTask {
        param($name, $flags)
        if ($this.FailDelete) { throw 'simulated access denied' }
        if (-not $this.Retain) {
            $item = @($this.Tasks | Where-Object { $_.Name -eq $name })[0]
            $this.Tasks.Remove($item)
        }
    }
    $folder
}
function Expect-Failure([scriptblock]$Action) {
    $failed = $false
    try { & $Action } catch { $failed = $true }
    if (-not $failed) { throw 'Expected cleanup to fail closed' }
}

$exe = 'C:\Apps\Pinmeter\Pinmeter.exe'
$owned = New-TestTask 'Pinmeter-Autostart-S-1-5-21-100' 'c:\apps\pinmeter\.\Pinmeter.exe'
$foreign = @(
    (New-TestTask 'Pinmeter-Autostart-S-1-5-21-200' 'C:\Portable\Pinmeter.exe'),
    (New-TestTask 'AnotherApp' $exe),
    (New-TestTask 'Pinmeter-Autostart-invalid' $exe),
    (New-TestTask 'Pinmeter-Autostart-S-1-5-21-300' $exe '--custom'),
    (New-TestTask 'Pinmeter-Autostart-S-1-5-21-400' $exe '' 2),
    (New-TestTask 'Pinmeter-Autostart-S-1-5-21-500' 'Pinmeter.exe')
)
$folder = New-TestFolder (@($owned) + $foreign)
Remove-PinmeterInstallationAutostart $folder $exe
if ($folder.Tasks.Count -ne $foreign.Count -or $folder.Tasks.Contains($owned)) { throw 'Installation ownership check failed' }
Remove-PinmeterInstallationAutostart $folder $exe
if ($folder.Tasks.Count -ne $foreign.Count) { throw 'Repeated cleanup changed unrelated tasks' }
Expect-Failure { Remove-PinmeterInstallationAutostart (New-TestFolder @($owned) $true) $exe }
Expect-Failure { Remove-PinmeterInstallationAutostart (New-TestFolder @($owned) $false $true) $exe }
Expect-Failure { Remove-PinmeterInstallationAutostart (New-TestFolder @()) 'Pinmeter.exe' }
Write-Output 'PASS: startup cleanup ownership, normalized paths, other installations, ambiguous actions, idempotence, deletion failure and residual verification (isolated task adapter).'
